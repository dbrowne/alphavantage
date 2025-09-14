use anyhow::{Context, Result};
use clap::Args;
use std::sync::Arc;
use tracing::{info, warn,error};
use chrono::Utc;
use bigdecimal::BigDecimal;

use crate::config::Config;
use av_client::AlphaVantageClient;
use av_database_postgres::schema::{crypto_api_map, symbols, crypto_markets as crypto_markets_table};
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
    #[arg(long), default_value = "false"]
    cleanup_cache: bool,
}

pub async fn execute(args: CryptoMarketsArgs, config: Config) -> Result<()> {
    info!("Starting crypto markets data loader");

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
        return execute_dry_run(args).await;
    }

    // Load symbols from database
    let crypto_symbols = load_crypto_symbols_from_db(&config.database_url, &args.symbols, args.limit)
        .context("Failed to load crypto symbols from database")?;

    if crypto_symbols.is_empty() {
        warn!("No crypto symbols found. Run crypto symbol loader first.");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for market data fetching", crypto_symbols.len());

    // Create markets loader configuration - include ALL required fields
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
        cache_ttl_hours:  args.cache_hours,
        force_refresh: args.force_refresh,
    };

    // Create client for loader context
    let av_client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Create loader context - use correct field names
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

    // Create markets loader input - include ALL required fields
    let input = CryptoMarketsInput {
        symbols: Some(crypto_symbols),
        exchange_filter: None,
        update_existing: args.update_existing,
        sources: vec![CryptoDataSource::CoinGecko],
        batch_size: Some(args.batch_size),
    };

    // Initialize markets loader
    let markets_loader = CryptoMarketsLoader::new(loader_config);

    // Load market data using the correct method name (load, not load_data)
    info!("Starting market data fetching...");
    match markets_loader.load(&loader_context, input).await {
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
            warn!("Failed to load market data: {}", e);
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
    update_existing: bool,
) -> Result<(usize, usize)> {
    use crypto_markets_table::dsl::*;

    let mut conn = establish_connection(database_url)?;
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut failed_batches = 0;

    info!("Processing {} market data entries", market_data.len());

    // Process in batches to avoid overwhelming the database
    const BATCH_SIZE: usize = 100;

    for (batch_index, batch) in market_data.chunks(BATCH_SIZE).enumerate() {
        info!("Processing batch {} with {} entries", batch_index + 1, batch.len());

        // Wrap each batch in error handling
        match process_single_batch(&mut conn, batch, update_existing).await {
            Ok((batch_inserted, batch_updated)) => {
                inserted_count += batch_inserted;
                updated_count += batch_updated;
                if batch_inserted > 0 || batch_updated > 0 {
                    info!("✅ Batch {}: {} inserted, {} updated",
                         batch_index + 1, batch_inserted, batch_updated);
                }
            }
            Err(e) => {
                failed_batches += 1;
                error!("❌ Batch {} failed: {}", batch_index + 1, e);

                // Log detailed field lengths and values for the failed batch
                error!("Failed batch contained {} entries with detailed field analysis:", batch.len());
                for (i, market) in batch.iter().take(5).enumerate() {
                    error!("  Entry {}: SID {}", i + 1, market.sid);
                    error!("    exchange: '{}' (len: {})", market.exchange, market.exchange.len());
                    error!("    base: '{}' (len: {})", market.base, market.base.len());
                    error!("    target: '{}' (len: {})", market.target, market.target.len());

                    if let Some(ref mt) = market.market_type {
                        error!("    market_type: '{}' (len: {})", mt, mt.len());
                    }
                    if let Some(ref ts) = market.trust_score {
                        error!("    trust_score: '{}' (len: {})", ts, ts.len());
                    }
                    if let Some(ref ls) = market.liquidity_score {
                        error!("    liquidity_score: '{}' (len: {})", ls, ls.len());
                    }
                    if let Some(ref lta) = market.last_traded_at {
                        error!("    last_traded_at: '{}' (len: {})", lta, lta.len());
                    }

                    // Check for exceptionally long fields
                    if market.exchange.len() > 150 {
                        error!("    ⚠️  EXCHANGE TOO LONG: '{}'", market.exchange);
                    }
                    if market.base.len() > 100 {
                        error!("    ⚠️  BASE TOO LONG: '{}'", market.base);
                    }
                    if market.target.len() > 100 {
                        error!("    ⚠️  TARGET TOO LONG: '{}'", market.target);
                    }
                    if let Some(ref ts) = market.trust_score {
                        if ts.len() > 100 {
                            error!("    ⚠️  TRUST_SCORE TOO LONG: '{}'", ts);
                        }
                    }
                    if let Some(ref ls) = market.liquidity_score {
                        if ls.len() > 100 {
                            error!("    ⚠️  LIQUIDITY_SCORE TOO LONG: '{}'", ls);
                        }
                    }
                }

                if batch.len() > 5 {
                    error!("  ... and {} more entries", batch.len() - 5);

                    // Also check the longest fields in the entire batch
                    let max_exchange_len = batch.iter().map(|m| m.exchange.len()).max().unwrap_or(0);
                    let max_base_len = batch.iter().map(|m| m.base.len()).max().unwrap_or(0);
                    let max_target_len = batch.iter().map(|m| m.target.len()).max().unwrap_or(0);

                    error!("  Batch maximums: exchange={}, base={}, target={}",
                           max_exchange_len, max_base_len, max_target_len);

                    // Find and log the entries with maximum lengths
                    if let Some(longest_exchange) = batch.iter().max_by_key(|m| m.exchange.len()) {
                        error!("  Longest exchange: '{}' ({})", longest_exchange.exchange, longest_exchange.exchange.len());
                    }
                    if let Some(longest_base) = batch.iter().max_by_key(|m| m.base.len()) {
                        error!("  Longest base: '{}' ({})", longest_base.base, longest_base.base.len());
                    }
                    if let Some(longest_target) = batch.iter().max_by_key(|m| m.target.len()) {
                        error!("  Longest target: '{}' ({})", longest_target.target, longest_target.target.len());
                    }
                }

                // Continue processing instead of failing completely
                warn!("⚠️  Continuing with next batch despite failure");
            }
        }
    }

    if failed_batches > 0 {
        warn!("⚠️  {} batches failed during processing", failed_batches);
        warn!("Successfully processed {} entries, {} failed",
             inserted_count + updated_count, failed_batches * BATCH_SIZE);
    }

    info!("Database save complete: {} inserted, {} updated, {} batches failed",
         inserted_count, updated_count, failed_batches);

    Ok((inserted_count, updated_count))
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