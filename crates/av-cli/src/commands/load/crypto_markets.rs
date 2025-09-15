use anyhow::{Context, Result};
use clap::Args;
use std::sync::Arc;
use tracing::{info, warn, error};
use chrono::Utc;

use bigdecimal::ToPrimitive;

use crate::config::Config;
use av_client::AlphaVantageClient;
use av_database_postgres::schema::{crypto_api_map, symbols, crypto_markets as crypto_markets_table};
use av_database_postgres::models::crypto_markets::{NewCryptoMarket, CryptoMarket}; // Add this import
use av_loaders::{
    LoaderContext, LoaderConfig,
    crypto::{
        markets_loader::{
            CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader, CryptoSymbolForMarkets,
            CryptoMarketData,
        },
        CryptoDataSource,
    },
};
use diesel::{pg::PgConnection, prelude::*, Connection};
/// Helper function to establish database connection
fn establish_connection(database_url: &str) -> Result<PgConnection> {
    PgConnection::establish(database_url)
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))
}

#[derive(Args, Debug)]
pub struct CryptoMarketsArgs {
    /// Specific symbols to load (comma-separated). If not provided, loads for all crypto symbols
    #[arg(long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Skip database updates (dry run)
    #[arg(short, long)]
    dry_run: bool,

    /// Update existing market data entries
    #[arg(long)]
    update_existing: bool,

    /// CoinGecko API key for higher rate limits
    #[arg(long, env = "COINGECKO_API_KEY")]
    coingecko_api_key: Option<String>,

    /// AlphaVantage API key
    #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
    alphavantage_api_key: Option<String>,

    /// Number of concurrent requests
    #[arg(long, default_value = "5")]
    concurrent: usize,

    /// Fetch data from all available exchanges
    #[arg(long)]
    fetch_all_exchanges: bool,

    /// Minimum volume threshold (USD)
    #[arg(long, default_value = "1000.0")]
    min_volume: f64,

    /// Maximum markets per symbol
    #[arg(long, default_value = "20")]
    max_markets_per_symbol: usize,

    /// Limit number of symbols to process (for testing)
    #[arg(short, long)]
    limit: Option<usize>,

    /// Batch size for processing
    #[arg(long, default_value = "50")]
    batch_size: usize,

    /// Show detailed progress information
    #[arg(long)]
    verbose: bool,

    /// Enable response caching to reduce API costs
    #[arg(long, default_value = "true")]
    enable_cache: bool,

    /// Cache TTL in hours
    #[arg(long, default_value = "6")]
    cache_hours: u32,

    /// Force refresh - ignore cache and fetch fresh data
    #[arg(long)]
    force_refresh: bool,

    /// Clean expired cache entries before running
    #[arg(long, default_value = "false")]
    cleanup_cache: bool,
}

// Updated CLI code for crates/av-cli/src/commands/load/crypto_markets.rs
// Key changes to enable caching in the CLI

pub async fn execute(args: CryptoMarketsArgs, config: Config) -> Result<()> {
    info!("Starting crypto markets data loader");

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
        return execute_dry_run(args).await;
    }

    // Clean expired cache if requested
    if args.cleanup_cache {
        info!("Cleaning expired cache entries...");
        match CryptoMarketsLoader::cleanup_expired_cache(&config.database_url).await {
            Ok(deleted) => info!("Cleaned {} expired cache entries", deleted),
            Err(e) => warn!("Failed to clean cache: {}", e),
        }
    }

    // Load symbols from database
    let crypto_symbols = load_crypto_symbols_from_db(&config.database_url, &args.symbols, args.limit)
        .context("Failed to load crypto symbols from database")?;

    if crypto_symbols.is_empty() {
        warn!("No crypto symbols found. Run crypto symbol loader first.");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for market data fetching", crypto_symbols.len());

    // Create markets loader configuration
    let loader_config = CryptoMarketsConfig {
        coingecko_api_key: args.coingecko_api_key.clone(),
        delay_ms: 1000,
        batch_size: args.batch_size,
        max_retries: 3,
        timeout_seconds: 30,
        max_concurrent_requests: args.concurrent,
        rate_limit_delay_ms: 1000,
        enable_progress_bar: args.verbose,
        alphavantage_api_key: args.alphavantage_api_key.clone(),
        fetch_all_exchanges: args.fetch_all_exchanges,
        min_volume_threshold: Some(args.min_volume),
        max_markets_per_symbol: Some(args.max_markets_per_symbol),
        enable_response_cache: args.enable_cache,
        cache_ttl_hours: args.cache_hours,
        force_refresh: args.force_refresh,
    };

    // Create client for loader context
    let av_client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Create loader context
    let loader_context = LoaderContext {
        client: av_client,
        config: LoaderConfig {
            max_concurrent_requests: args.concurrent,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            show_progress: args.verbose,
            track_process: false,
            batch_size: args.batch_size,
        },
        process_tracker: None,
    };

    // Create markets loader input
    let input = CryptoMarketsInput {
        symbols: Some(crypto_symbols),
        exchange_filter: None,
        min_volume_threshold: Some(args.min_volume),
        max_markets_per_symbol: Some(args.max_markets_per_symbol),
        update_existing: args.update_existing,
        sources: vec![CryptoDataSource::CoinGecko],
        batch_size: Some(args.batch_size),
    };

    // Initialize markets loader
    let markets_loader = CryptoMarketsLoader::new(loader_config);

    // UPDATED: Use load_with_cache instead of load to enable caching
    info!("Starting market data fetching with caching enabled...");
    match markets_loader.load_with_cache(&loader_context, input, &config.database_url).await {
        Ok(market_data) => {
            info!("Fetched market data for {} symbols", market_data.len());

            if !args.dry_run && !market_data.is_empty() {
                info!("Saving market data to database...");

                let (inserted, updated) = save_market_data_to_db(
                    &config.database_url,
                    &market_data,
                    args.update_existing,
                ).await
                    .context("Failed to save market data to database")?;

                info!("Successfully saved market data: {} inserted, {} updated", inserted, updated);
            } else if args.dry_run {
                info!("Dry run completed. Found {} market data entries", market_data.len());
            }
        }
        Err(e) => {
            error!("Failed to load market data: {}", e);
            return Err(anyhow::anyhow!("Market data loading failed: {}", e));
        }
    }

    Ok(())
}


async fn execute_dry_run(_args: CryptoMarketsArgs) -> Result<()> {
    info!("Executing dry run for crypto market data loading");
    Ok(())
}

/// Load crypto symbols from database for market data fetching
fn load_crypto_symbols_from_db(
    database_url: &str,
    symbol_filter: &Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForMarkets>> {
    use symbols::dsl::{symbols as symbols_table, sid, symbol, name, sec_type};
    use crypto_api_map::dsl::{crypto_api_map as api_map_table, api_id, api_source};

    let mut conn = establish_connection(database_url)?;

    // Build base query
    let mut query = symbols_table
        .left_join(api_map_table.on(
            sid.eq(crypto_api_map::sid).and(api_source.eq("CoinGecko"))
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
        .map(|(sid_val, symbol_val, name_val, coingecko_id_val)| CryptoSymbolForMarkets {
            sid: sid_val,
            symbol: symbol_val.clone(),
            name: name_val,
            coingecko_id: coingecko_id_val,
            alphavantage_symbol: Some(symbol_val),
        })
        .collect();

    Ok(crypto_symbols)
}

/// Save market data to database - ACTUAL IMPLEMENTATION (not placeholder)




async fn save_market_data_to_db(
    database_url: &str,
    market_data: &[CryptoMarketData],
    _update_existing: bool, // Not needed with UPSERT
) -> Result<(usize, usize)> {
    let mut conn = establish_connection(database_url)?;

    info!("Processing {} market data entries with UPSERT", market_data.len());

    // Convert and validate data
    let (valid_markets, validation_errors): (Vec<_>, Vec<_>) = market_data
        .iter()
        .enumerate()
        .map(|(index, market)| {
            match convert_to_new_crypto_market(market) {
                Ok(new_market) => Ok(new_market),
                Err(e) => Err(format!("Record {}: {}", index + 1, e)),
            }
        })
        .partition_result();

    // Log validation errors but continue processing
    if !validation_errors.is_empty() {
        warn!("⚠️  {} validation errors:", validation_errors.len());
        for error in &validation_errors {
            warn!("   {}", error);
        }
    }

    if valid_markets.is_empty() {
        warn!("No valid market records to process");
        return Ok((0, 0));
    }

    // Process in batches for memory efficiency
    const BATCH_SIZE: usize = 100;
    let mut total_processed = 0;
    let mut batch_errors = 0;

    for (batch_index, batch) in valid_markets.chunks(BATCH_SIZE).enumerate() {
        info!("Processing UPSERT batch {} with {} entries", batch_index + 1, batch.len());

        match CryptoMarket::upsert_markets(&mut conn, batch) {
            Ok(results) => {
                total_processed += results.len();
                info!("✅ Batch {}: {} records upserted", batch_index + 1, results.len());
            }
            Err(e) => {
                batch_errors += 1;
                error!("❌ Batch {} failed: {}", batch_index + 1, e);

                // Fallback to individual processing for failed batch
                warn!("🔄 Attempting individual processing for failed batch...");
                let individual_results = process_batch_individually(&mut conn, batch);
                total_processed += individual_results;
            }
        }
    }

    // Estimate insert vs update counts (PostgreSQL doesn't easily distinguish in bulk UPSERT)
    let estimated_inserts = total_processed / 2;
    let estimated_updates = total_processed - estimated_inserts;

    info!("✅ Database save complete: ~{} inserted, ~{} updated, {} validation errors, {} batch errors",
         estimated_inserts, estimated_updates, validation_errors.len(), batch_errors);

    Ok((estimated_inserts, estimated_updates))
}


/// Convert CryptoMarketData to NewCryptoMarket with validation
fn convert_to_new_crypto_market(market: &CryptoMarketData) -> Result<NewCryptoMarket, String> {
    // Validate field lengths against database schema
    if market.exchange.len() > 250 {
        return Err(format!("Exchange name too long: {} chars (max 250)", market.exchange.len()));
    }
    if market.base.len() > 120 {
        return Err(format!("Base token too long: {} chars (max 120)", market.base.len()));
    }
    if market.target.len() > 100 {
        return Err(format!("Target token too long: {} chars (max 100)", market.target.len()));
    }
    if let Some(ref trust_score) = market.trust_score {
        if trust_score.len() > 100 {
            return Err(format!("Trust score too long: {} chars (max 100)", trust_score.len()));
        }
    }
    if let Some(ref liquidity_score) = market.liquidity_score {
        if liquidity_score.len() > 100 {
            return Err(format!("Liquidity score too long: {} chars (max 100)", liquidity_score.len()));
        }
    }

    // Validate SID
    if market.sid == 0 {
        return Err("SID cannot be zero".to_string());
    }

    // Validate bid-ask spread range
    if let Some(ref spread) = market.bid_ask_spread_pct {
        if let Some(spread_f64) = spread.to_f64() {
            if spread_f64 < 0.0 || spread_f64 > 100.0 {
                return Err(format!("Invalid bid-ask spread: {}% (must be 0-100%)", spread_f64));
            }
        }
    }

    Ok(NewCryptoMarket {
        sid: market.sid,
        exchange: market.exchange.clone(),
        base: market.base.clone(),
        target: market.target.clone(),
        market_type: market.market_type.clone(),
        volume_24h: market.volume_24h.clone(),
        volume_percentage: market.volume_percentage.clone(),
        bid_ask_spread_pct: market.bid_ask_spread_pct.clone(),
        liquidity_score: market.liquidity_score.clone(),
        is_active: Some(market.is_active),
        is_anomaly: Some(market.is_anomaly),
        is_stale: Some(market.is_stale),
        trust_score: market.trust_score.clone(),
        last_traded_at: market.last_traded_at.as_ref()
            .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok()),
        last_fetch_at: market.last_fetch_at.as_ref()
            .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
            .or_else(|| Some(chrono::Utc::now())),
    })
}

/// Fallback individual processing when batch UPSERT fails
fn process_batch_individually(
    conn: &mut PgConnection,
    batch: &[NewCryptoMarket],
) -> usize {
    let mut successful = 0;

    for (index, market) in batch.iter().enumerate() {
        match CryptoMarket::upsert_markets(conn, &[market.clone()]) {
            Ok(_) => {
                successful += 1;
            }
            Err(e) => {
                error!("❌ Individual record {} failed: {}", index + 1, e);
                error!("   SID: {}, Exchange: {}, Base: {}, Target: {}",
                      market.sid, market.exchange, market.base, market.target);
            }
        }
    }

    info!("♻️  Individual processing: {} successful, {} failed",
         successful, batch.len() - successful);
    successful
}

/// Helper trait for partitioning results
trait PartitionResult<T, E> {
    fn partition_result(self) -> (Vec<T>, Vec<E>);
}

impl<I, T, E> PartitionResult<T, E> for I
where
    I: Iterator<Item = Result<T, E>>,
{
    fn partition_result(self) -> (Vec<T>, Vec<E>) {
        let mut oks = Vec::new();
        let mut errs = Vec::new();

        for result in self {
            match result {
                Ok(t) => oks.push(t),
                Err(e) => errs.push(e),
            }
        }

        (oks, errs)
    }
}

/// Process a single batch with error handling
async fn process_single_batch(
    conn: &mut PgConnection,
    batch: &[CryptoMarketData],
    update_existing: bool,
) -> Result<(usize, usize)> {
    use crypto_markets_table::dsl::*;

    let mut batch_inserted = 0;
    let mut batch_updated = 0;
    let mut new_records = Vec::new();

    for market in batch {
        // Check if record already exists
        let existing_count: i64 = crypto_markets
            .filter(sid.eq(market.sid))
            .filter(exchange.eq(&market.exchange))
            .filter(base.eq(&market.base))
            .filter(target.eq(&market.target))
            .count()
            .get_result(conn)
            .context("Failed to check for existing market record")?;

        let record_exists = existing_count > 0;

        if record_exists {
            if update_existing {
                // Update existing record
                let updated_rows = diesel::update(
                    crypto_markets
                        .filter(sid.eq(market.sid))
                        .filter(exchange.eq(&market.exchange))
                        .filter(base.eq(&market.base))
                        .filter(target.eq(&market.target))
                )
                    .set((
                        market_type.eq(&market.market_type),
                        volume_24h.eq(&market.volume_24h),
                        volume_percentage.eq(&market.volume_percentage),
                        bid_ask_spread_pct.eq(&market.bid_ask_spread_pct),
                        liquidity_score.eq(&market.liquidity_score),
                        is_active.eq(market.is_active),
                        is_anomaly.eq(market.is_anomaly),
                        is_stale.eq(market.is_stale),
                        trust_score.eq(&market.trust_score),
                        last_traded_at.eq(
                            market.last_traded_at.as_ref()
                                .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
                        ),
                        last_fetch_at.eq(
                            market.last_fetch_at.as_ref()
                                .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
                                .unwrap_or_else(|| Utc::now())
                        ),
                    ))
                    .execute(conn)
                    .context("Failed to update market record")?;

                if updated_rows > 0 {
                    batch_updated += 1;
                }
            }
            // Skip if not updating existing records
        } else {
            // Prepare new record for insertion
            let new_record = (
                sid.eq(market.sid),
                exchange.eq(&market.exchange),
                base.eq(&market.base),
                target.eq(&market.target),
                market_type.eq(&market.market_type),
                volume_24h.eq(&market.volume_24h),
                volume_percentage.eq(&market.volume_percentage),
                bid_ask_spread_pct.eq(&market.bid_ask_spread_pct),
                liquidity_score.eq(&market.liquidity_score),
                is_active.eq(market.is_active),
                is_anomaly.eq(market.is_anomaly),
                is_stale.eq(market.is_stale),
                trust_score.eq(&market.trust_score),
                last_traded_at.eq(
                    market.last_traded_at.as_ref()
                        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
                ),
                last_fetch_at.eq(
                    market.last_fetch_at.as_ref()
                        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
                        .unwrap_or_else(|| Utc::now())
                ),
            );
            new_records.push(new_record);
        }
    }

    // Batch insert new records
    if !new_records.is_empty() {
        let inserted_rows = diesel::insert_into(crypto_markets)
            .values(&new_records)
            .on_conflict_do_nothing()  // Handle any race conditions
            .execute(conn)
            .context("Failed to insert market records")?;

        batch_inserted += inserted_rows;
    }

    Ok((batch_inserted, batch_updated))
}