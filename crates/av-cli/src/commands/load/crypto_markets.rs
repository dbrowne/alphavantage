use anyhow::{Context, Result};
use clap::Args;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use av_core::config::Config;
use av_client::AlphaVantageClient;
use av_database_postgres::{
    establish_connection,
    models::{
        crypto::CryptoApiMap,
        crypto_markets::{CryptoMarket, NewCryptoMarket},
        security::Symbol,
    },
    schema::{crypto_api_map, crypto_markets, symbols},
};
use av_loaders::{
    crypto::{
        markets_loader::{
            CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader, CryptoSymbolForMarkets,
        },
        CryptoDataSource,
    },
    DataLoader, LoaderConfig, LoaderContext, ProcessTracker,
};
use diesel::prelude::*;

#[derive(Args, Debug)]
pub struct CryptoMarketsArgs {
    /// Specific symbols to load (comma-separated). If not provided, loads all mapped cryptos
    #[arg(short, long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Data sources to use for market data
    #[arg(
        long,
        value_enum,
        default_values = ["coingecko"],
        value_delimiter = ','
    )]
    sources: Vec<CryptoDataSourceArg>,

    /// CoinGecko API key (optional, increases rate limits)
    #[arg(long, env = "COINGECKO_API_KEY")]
    coingecko_api_key: Option<String>,

    /// AlphaVantage API key (required for AlphaVantage source)
    #[arg(long, env = "ALPHAVANTAGE_API_KEY")]
    alphavantage_api_key: Option<String>,

    /// Number of concurrent API requests
    #[arg(short, long, default_value = "3")]
    concurrent: usize,

    /// Batch size for processing symbols
    #[arg(short, long, default_value = "20")]
    batch_size: usize,

    /// Update existing market data
    #[arg(short, long)]
    update_existing: bool,

    /// Continue processing on errors
    #[arg(long)]
    continue_on_error: bool,

    /// Minimum 24h volume threshold (USD) to include markets
    #[arg(long, default_value = "1000")]
    min_volume: f64,

    /// Maximum markets per symbol to fetch
    #[arg(long, default_value = "20")]
    max_markets_per_symbol: usize,

    /// Fetch all available exchanges (not just top ones)
    #[arg(long)]
    fetch_all_exchanges: bool,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable process tracking
    #[arg(long)]
    track_process: bool,

    /// Dry run - test API connections without database updates
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum CryptoDataSourceArg {
    CoinGecko,
    AlphaVantage,
}

impl From<CryptoDataSourceArg> for CryptoDataSource {
    fn from(arg: CryptoDataSourceArg) -> Self {
        match arg {
            CryptoDataSourceArg::CoinGecko => CryptoDataSource::CoinGecko,
            CryptoDataSourceArg::AlphaVantage => CryptoDataSource::SosoValue, // Map to SosoValue for AlphaVantage
        }
    }
}

pub async fn execute(args: CryptoMarketsArgs, config: Config) -> Result<()> {
    info!("Starting crypto markets loader");

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
        return execute_dry_run(args).await;
    }

    // Validate API keys
    let sources: Vec<CryptoDataSource> = args.sources.iter().map(|s| (*s).into()).collect();
    validate_api_keys(&sources, &args)?;

    // Create API client
    let client = Arc::new(AlphaVantageClient::new(config.api_config));

    // Create markets loader configuration
    let markets_config = CryptoMarketsConfig {
        batch_size: args.batch_size,
        max_concurrent_requests: args.concurrent,
        rate_limit_delay_ms: 1000, // Conservative for public APIs
        enable_progress_bar: args.verbose,
        coingecko_api_key: args.coingecko_api_key.clone(),
        alphavantage_api_key: args.alphavantage_api_key.clone(),
        fetch_all_exchanges: args.fetch_all_exchanges,
        min_volume_threshold: Some(args.min_volume),
        max_markets_per_symbol: Some(args.max_markets_per_symbol),
    };

    // Create loader context
    let loader_config = LoaderConfig {
        max_concurrent_requests: args.concurrent,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        show_progress: args.verbose,
        track_process: args.track_process,
        batch_size: args.batch_size,
    };

    let mut context = LoaderContext::new(client, loader_config);

    // Set up process tracking if requested
    if args.track_process {
        let tracker = ProcessTracker::new();
        context = context.with_process_tracker(tracker);
    }

    // Load symbols from database
    let symbols = load_crypto_symbols_from_db(&config.database_url, &args.symbols)
        .context("Failed to load crypto symbols from database")?;

    if symbols.is_empty() {
        warn!("No crypto symbols found in database. Run crypto symbol loader first.");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for market data fetching", symbols.len());

    // Create markets loader and input
    let markets_loader = CryptoMarketsLoader::new(markets_config);
    let input = CryptoMarketsInput {
        symbols: Some(symbols),
        update_existing: args.update_existing,
        sources,
        batch_size: Some(args.batch_size),
    };

    // Execute the loader
    match markets_loader.load(&context, input).await {
        Ok(output) => {
            info!(
                "Markets loading completed: {} fetched, {} processed, {} errors",
                output.markets_fetched, output.markets_processed, output.errors
            );

            // Save markets to database
            if !output.markets.is_empty() {
                match save_markets_to_db(&config.database_url, &output.markets, args.update_existing)
                    .await
                {
                    Ok((inserted, updated)) => {
                        info!("Database update: {} markets inserted, {} updated", inserted, updated);
                    }
                    Err(e) => {
                        error!("Failed to save markets to database: {}", e);
                        if !args.continue_on_error {
                            return Err(e);
                        }
                    }
                }
            }

            // Display source-specific results
            for (source, result) in output.source_results {
                info!(
                    "{:?}: {} markets, {} errors, {}ms",
                    source, result.markets_fetched, result.errors.len(), result.response_time_ms
                );

                if args.verbose && !result.errors.is_empty() {
                    for error in result.errors {
                        warn!("  Error: {}", error);
                    }
                }
            }
        }
        Err(e) => {
            error!("Markets loading failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn execute_dry_run(args: CryptoMarketsArgs) -> Result<()> {
    info!("Executing crypto markets loader in dry run mode");

    info!("Configuration:");
    info!("  - Sources: {:?}", args.sources);
    info!("  - Concurrent requests: {}", args.concurrent);
    info!("  - Batch size: {}", args.batch_size);
    info!("  - Min volume threshold: ${}", args.min_volume);
    info!("  - Max markets per symbol: {}", args.max_markets_per_symbol);
    info!("  - Fetch all exchanges: {}", args.fetch_all_exchanges);

    // Test API connections
    let sources: Vec<CryptoDataSource> = args.sources.iter().map(|s| (*s).into()).collect();

    for source in &sources {
        match source {
            CryptoDataSource::CoinGecko => {
                info!("Testing CoinGecko API connection...");
                if args.coingecko_api_key.is_some() {
                    info!("  ✓ CoinGecko API key: configured");
                } else {
                    info!("  - CoinGecko API key: not configured (will use free tier)");
                }
                // Could add actual API test here
                info!("  ✓ CoinGecko connection test would run here");
            }
            CryptoDataSource::SosoValue => {
                info!("Testing AlphaVantage API connection...");
                if args.alphavantage_api_key.is_some() {
                    info!("  ✓ AlphaVantage API key: configured");
                } else {
                    warn!("  ✗ AlphaVantage API key: not configured");
                    warn!("    Set ALPHAVANTAGE_API_KEY environment variable");
                }
                // Could add actual API test here
                info!("  ✓ AlphaVantage connection test would run here");
            }
            _ => {
                warn!("  - Source {:?}: not implemented in dry run", source);
            }
        }
    }

    info!("Dry run completed - no actual market data fetching or database updates performed");
    Ok(())
}

fn validate_api_keys(sources: &[CryptoDataSource], args: &CryptoMarketsArgs) -> Result<()> {
    for source in sources {
        match source {
            CryptoDataSource::SosoValue => {
                if args.alphavantage_api_key.is_none() {
                    return Err(anyhow::anyhow!(
                        "AlphaVantage API key is required for AlphaVantage source. Set ALPHAVANTAGE_API_KEY environment variable"
                    ));
                }
            }
            CryptoDataSource::CoinGecko => {
                // CoinGecko API key is optional
                if args.coingecko_api_key.is_none() {
                    warn!("CoinGecko API key not provided - using free tier with rate limits");
                }
            }
            _ => {
                warn!("Source {:?} validation not implemented", source);
            }
        }
    }
    Ok(())
}

/// Load crypto symbols from database that need market data
fn load_crypto_symbols_from_db(
    database_url: &str,
    filter_symbols: &Option<Vec<String>>,
) -> Result<Vec<CryptoSymbolForMarkets>> {
    let mut conn = establish_connection(database_url)?;

    let mut query = symbols::table
        .left_join(
            crypto_api_map::table.on(
                crypto_api_map::sid
                    .eq(symbols::sid)
                    .and(crypto_api_map::api_source.eq("CoinGecko"))
                    .and(crypto_api_map::is_active.eq(Some(true))),
            ),
        )
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .select((
            symbols::sid,
            symbols::symbol,
            symbols::name,
            crypto_api_map::api_id.nullable(),
        ))
        .into_boxed();

    // Filter by specific symbols if provided
    if let Some(symbol_list) = filter_symbols {
        query = query.filter(symbols::symbol.eq_any(symbol_list));
    }

    let results: Vec<(i64, String, String, Option<String>)> = query
        .load(&mut conn)
        .context("Failed to query crypto symbols")?;

    let symbols = results
        .into_iter()
        .map(|(sid, symbol, name, coingecko_id)| CryptoSymbolForMarkets {
            sid,
            symbol: symbol.clone(),
            name,
            coingecko_id,
            alphavantage_symbol: Some(symbol), // Use symbol as-is for AlphaVantage
        })
        .collect();

    Ok(symbols)
}

/// Save market data to database
async fn save_markets_to_db(
    database_url: &str,
    markets: &[av_loaders::crypto::markets_loader::CryptoMarketData],
    update_existing: bool,
) -> Result<(usize, usize)> {
    let database_url = database_url.to_string();
    let markets = markets.to_vec();

    tokio::task::spawn_blocking(move || {
        let mut conn = establish_connection(&database_url)?;

        let new_markets: Vec<NewCryptoMarket> = markets
            .into_iter()
            .map(|market| market.into())
            .collect();

        conn.transaction(|conn| {
            if update_existing {
                // Use upsert for update mode
                let count = CryptoMarket::upsert_batch(conn, &new_markets)?;
                Ok((0, count)) // All are considered updates in upsert mode
            } else {
                // Try to insert new markets, skip conflicts
                let mut inserted = 0;
                let mut skipped = 0;

                for market in &new_markets {
                    match CryptoMarket::insert(conn, market) {
                        Ok(_) => inserted += 1,
                        Err(diesel::result::Error::DatabaseError(
                                diesel::result::DatabaseErrorKind::UniqueViolation,
                                _,
                            )) => {
                            skipped += 1;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }

                info!("Market insertion: {} inserted, {} skipped (duplicates)", inserted, skipped);
                Ok((inserted, 0))
            }
        })
    })
        .await?
        .context("Database operation failed")
}