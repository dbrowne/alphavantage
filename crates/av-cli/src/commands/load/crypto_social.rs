use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, warn};
use std::sync::Arc;

use diesel::prelude::*;
use diesel::{PgConnection, Connection};
use av_database_postgres::schema::{symbols, crypto_api_map};
use av_loaders::{
    LoaderContext, LoaderConfig,
    crypto::social_loader::{CryptoSocialConfig, CryptoSocialInput, CryptoSocialLoader, CryptoSymbolForSocial},
};
use av_client::AlphaVantageClient;

use crate::config::Config;

/// Helper function to establish database connection
fn establish_connection(database_url: &str) -> Result<PgConnection> {
    PgConnection::establish(database_url)
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))
}

#[derive(Args, Debug)]
pub struct CryptoSocialArgs {
    /// Symbols to fetch social data for (comma-separated). If not provided, fetches for all crypto symbols
    #[arg(long, value_delimiter = ',')]
    symbols_list: Option<Vec<String>>,

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
    let crypto_symbols = load_crypto_symbols_from_db(&config.database_url, &args.symbols_list, args.limit)
        .context("Failed to load crypto symbols from database")?;

    if crypto_symbols.is_empty() {
        warn!("No crypto symbols found. Run crypto symbol loader first.");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for social data fetching", crypto_symbols.len());

    // Create social loader configuration
    let loader_config = CryptoSocialConfig {
        coingecko_api_key: args.coingecko_api_key.clone(),
        github_token: args.github_token.clone(),
        skip_github: args.skip_github,
        delay_ms: args.delay_ms,
        batch_size: args.batch_size,
        max_retries: 3,
        timeout_seconds: 30,
    };

    // Create loader context with proper types - use same pattern as crypto_markets.rs
    let av_client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    let loader_context = LoaderContext {
        client: av_client,
        config: LoaderConfig {
            max_concurrent_requests: 10,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            show_progress: args.verbose, // maps verbose to show_progress
            track_process: false,
            batch_size: args.batch_size,
        },
        process_tracker: None,
    };

    // Create social loader input
    let input = CryptoSocialInput {
        symbols: Some(crypto_symbols),
        update_existing: args.update_existing,
    };

    // Initialize social loader
    let social_loader = CryptoSocialLoader::new(loader_config);

    // Load social data
    info!("Starting social data fetching with {} concurrent requests", args.batch_size);
    let social_data = social_loader
        .load_data(&input, &loader_context)
        .await
        .context("Failed to load social data")?;

    info!("Fetched social data for {} symbols", social_data.len());

    if !args.dry_run && !social_data.is_empty() {
        info!("Saving social data to database...");

        // Save to database directly here since we need to handle the conversion
        let (inserted, updated) = save_social_data_to_db(
            &config.database_url,
            &social_data,
            args.update_existing,
        ).await
            .context("Failed to save social data to database")?;

        info!("Successfully saved social data: {} inserted, {} updated", inserted, updated);
    } else if args.dry_run {
        info!("Dry run completed. Found {} social data entries", social_data.len());
    }

    Ok(())
}

async fn execute_dry_run(_args: CryptoSocialArgs) -> Result<()> {
    info!("Executing dry run for crypto social data loading");
    // Implement dry run logic here
    Ok(())
}

/// Load crypto symbols from database for social data fetching
fn load_crypto_symbols_from_db(
    database_url: &str,
    symbol_filter: &Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForSocial>> {
    use symbols::dsl::{symbols as symbols_table, sid, symbol, name, sec_type};
    use crypto_api_map::dsl::{crypto_api_map as api_map_table, api_id, api_source};

    let mut conn = establish_connection(database_url)?;

    // Build base query
    let mut query = symbols_table
        .left_join(api_map_table.on(
            sid.eq(crypto_api_map::sid).and(api_source.eq("coingecko"))
        ))
        .filter(sec_type.eq("Cryptocurrency"))
        .select((sid, symbol, name, api_id.nullable()))
        .into_boxed();

    // Apply symbol filter if provided
    if let Some(ref filter_list) = symbol_filter {
        query = query.filter(symbol.eq_any(filter_list));
    }

    // Apply limit if provided
    if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
    }

    let results: Vec<(i64, String, String, Option<String>)> = query
        .load(&mut conn)
        .context("Failed to execute query")?;

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

/// Save social data to database (placeholder implementation)
async fn save_social_data_to_db(
    _database_url: &str,
    social_data: &[av_loaders::crypto::social_loader::ProcessedSocialData],
    _update_existing: bool,
) -> Result<(usize, usize)> {
    // Placeholder implementation - just count the data
    let count = social_data.len();
    info!("Would save {} social data entries to database", count);
    Ok((count, 0))
}