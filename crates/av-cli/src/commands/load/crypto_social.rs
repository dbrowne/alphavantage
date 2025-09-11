use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, warn};

use av_database_postgres::{establish_connection, schema::symbols::dsl::*};
use av_database_postgres::schema::crypto_api_map::dsl as api_map;
use av_loaders::{
    DataLoader, LoaderConfig, LoaderContext,
    crypto::social_loader::{CryptoSocialConfig, CryptoSocialInput, CryptoSocialLoader, CryptoSymbolForSocial},
};
use diesel::prelude::*;

use crate::config::Config;

#[derive(Args, Debug)]
pub struct CryptoSocialArgs {
    /// Symbols to fetch social data for (comma-separated). If not provided, fetches for all crypto symbols
    #[arg(long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Skip database updates (dry run)
    #[arg(short, long)]
    dry_run: bool,

    /// Update existing social data entries
    #[arg(long)]
    update_existing: bool,

    /// CoinGecko API key for higher rate limits
    #[arg(long, env = "COINGECKO_API_KEY")]
    coingecko_api_key: Option<String>,

    /// GitHub personal access token for repository data
    #[arg(long, env = "GITHUB_TOKEN")]
    github_token: Option<String>,

    /// Limit number of symbols to process (for testing)
    #[arg(short, long)]
    limit: Option<usize>,

    /// Delay between API requests in milliseconds
    #[arg(long, default_value = "2000")]
    delay_ms: u64,

    /// Batch size for database operations
    #[arg(long, default_value = "50")]
    batch_size: usize,

    /// Show detailed progress information
    #[arg(long)]
    verbose: bool,

    /// Skip GitHub repository data fetching
    #[arg(long)]
    skip_github: bool,
}

pub async fn execute(args: CryptoSocialArgs, config: Config) -> Result<()> {
    info!("Starting crypto social data loader");

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
        return execute_dry_run(args).await;
    }

    // Load symbols from database
    let symbols = load_crypto_symbols_from_db(&config.database_url, &args.symbols, args.limit)
        .context("Failed to load crypto symbols from database")?;

    if symbols.is_empty() {
        warn!("No crypto symbols found. Run crypto symbol loader first.");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for social data fetching", symbols.len());

    // Create social loader configuration
    let social_config = CryptoSocialConfig {
        batch_size: args.batch_size,
        max_concurrent_requests: 5, // Conservative for public APIs
        rate_limit_delay_ms: args.delay_ms,
        coingecko_api_key: args.coingecko_api_key.clone(),
        github_token: args.github_token.clone(),
        enable_progress_bar: args.verbose,
        fetch_github_data: !args.skip_github,
        update_existing: args.update_existing,
    };

    // Create loader context (minimal, since we're using our own HTTP client)
    let loader_config = LoaderConfig {
        max_concurrent_requests: 5,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        show_progress: args.verbose,
        track_process: false,
        batch_size: args.batch_size,
    };

    // We don't actually need the AlphaVantage client for social data, but the loader expects it
    let dummy_client = std::sync::Arc::new(
        av_client::AlphaVantageClient::new(config.api_config.clone())
    );
    let context = LoaderContext::new(dummy_client, loader_config);

    // Create social loader and input
    let social_loader = CryptoSocialLoader::new(social_config);
    let input = CryptoSocialInput {
        symbols: Some(symbols),
        coingecko_ids: None,
        update_existing: args.update_existing,
        batch_size: Some(args.batch_size),
    };

    // Execute the loader
    match social_loader.load(&context, input).await {
        Ok(output) => {
            info!(
                "Social data loading completed: {} fetched, {} processed, {} GitHub repos, {} errors",
                output.social_data_fetched,
                output.social_data_processed,
                output.github_repos_fetched,
                output.errors.len()
            );

            // Save social data to database
            if !output.social_data.is_empty() {
                match save_social_data_to_db(&config.database_url, &social_loader, &output.social_data, args.update_existing).await {
                    Ok((inserted, updated)) => {
                        info!("Database update completed: {} inserted, {} updated", inserted, updated);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to save social data to database: {}", e));
                    }
                }
            }

            // Print errors if any
            if !output.errors.is_empty() && args.verbose {
                warn!("Errors encountered during loading:");
                for error in &output.errors {
                    warn!("  {}", error);
                }
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Social data loading failed: {}", e));
        }
    }

    Ok(())
}

async fn execute_dry_run(args: CryptoSocialArgs) -> Result<()> {
    info!("=== Crypto Social Data Loader Dry Run ===");
    info!("Configuration:");
    info!("  Update existing: {}", args.update_existing);
    info!("  Batch size: {}", args.batch_size);
    info!("  Rate limit delay: {}ms", args.delay_ms);
    info!("  Fetch GitHub data: {}", !args.skip_github);

    if let Some(ref symbols) = args.symbols {
        info!("  Target symbols: {:?}", symbols);
    } else {
        info!("  Target symbols: all crypto symbols in database");
    }

    if let Some(limit) = args.limit {
        info!("  Limit: {} symbols", limit);
    }

    // Test API keys
    if args.coingecko_api_key.is_some() {
        info!("  ✓ CoinGecko API key: configured");
    } else {
        info!("  - CoinGecko API key: using free tier");
    }

    if args.github_token.is_some() {
        info!("  ✓ GitHub token: configured");
    } else {
        info!("  - GitHub token: not configured (limited rate limits)");
    }

    info!("Dry run completed - no actual API calls or database updates performed");
    Ok(())
}

/// Load crypto symbols from database that need social data
fn load_crypto_symbols_from_db(
    database_url: &str,
    filter_symbols: &Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForSocial>> {
    let mut conn = establish_connection(database_url)?;

    use av_database_postgres::schema::symbols::dsl::*;


    let mut query = symbols
        .left_join(av_database_postgres::schema::crypto_api_map::table.on(
            sid.eq(api_map::sid).and(api_map::api_source.eq("coingecko"))
        ))
        .filter(sec_type.eq("Cryptocurrency"))
        .select((sid, symbol, name, api_map::api_id.nullable()));

    // Apply symbol filter if provided
    if let Some(filter_list) = filter_symbols {
        query = query.filter(symbol.eq_any(filter_list));
    }

    // Apply limit if provided
    if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
    }

    let results: Vec<(i64, String, String, Option<String>)> = query
        .load(&mut conn)
        .context("Failed to load crypto symbols from database")?;

    let crypto_symbols = results
        .into_iter()
        .map(|(sid_val, symbol_val, name_val, coingecko_id_val)| CryptoSymbolForSocial {
            sid: sid_val,
            symbol: symbol_val,
            name: name_val,
            coingecko_id: coingecko_id_val,
        })
        .collect();

    Ok(crypto_symbols)
}

/// Save social data to database
async fn save_social_data_to_db(
    database_url: &str,
    social_loader: &CryptoSocialLoader,
    social_data: &[av_loaders::crypto::social_loader::ProcessedSocialData],
    _update_existing: bool,
) -> Result<(usize, usize)> {
    let mut conn = establish_connection(database_url)?;

    social_loader.save_social_data(&mut conn, social_data).await
        .map_err(|e| anyhow::anyhow!("Database operation failed: {}", e))
}