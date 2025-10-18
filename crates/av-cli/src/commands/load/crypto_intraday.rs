// Optimized version using timestamp-based deduplication

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
  establish_connection,
  models::price::NewIntradayPrice,
  schema::{intradayprices, symbols},
};
use av_loaders::{
  CryptoIntradayConfig, CryptoIntradayLoader, CryptoIntradayLoaderInput, CryptoIntradayPriceData,
  CryptoIntradaySymbolInfo, DataLoader, IntradayInterval, LoaderConfig, LoaderContext,
  ProcessTracker,
};

use crate::config::Config;

/// Arguments for the crypto intraday price loader
#[derive(Parser, Debug)]
#[clap(about = "Load crypto intraday price data from AlphaVantage")]
pub struct CryptoIntradayArgs {
  /// Specific symbol to load (defaults to top cryptocurrencies if not specified)
  #[clap(short, long)]
  symbol: Option<String>,

  /// Market/currency for pricing (e.g., USD, EUR, GBP)
  #[clap(short, long, default_value = "USD")]
  market: String,

  /// Time interval between data points (1min, 5min, 15min, 30min, 60min)
  #[clap(short, long, default_value = "1min")]
  interval: String,

  /// Output size (compact=100 data points, full=full available history)
  #[clap(long, default_value = "compact")]
  outputsize: String,

  /// Skip symbols that already have intraday data (symbols.intraday = true)
  #[clap(long)]
  skip_existing: bool,

  /// Only load symbols that already have intraday data (useful for updates/refreshing)
  #[clap(long, conflicts_with = "skip_existing")]
  only_existing: bool,

  /// Number of concurrent API requests
  #[clap(long, default_value = "5")]
  concurrent: usize,

  /// Delay between API calls in milliseconds (800ms = 75 calls/minute for premium)
  #[clap(long, default_value = "800")]
  api_delay: u64,

  /// Dry run - don't save to database
  #[clap(long)]
  dry_run: bool,

  /// Force refresh - bypass cache and timestamp checks
  #[clap(long)]
  force_refresh: bool,

  /// Update existing records
  #[clap(long)]
  update: bool,

  /// Continue on error (don't stop if one symbol fails)
  #[clap(long)]
  continue_on_error: bool,

  /// Update symbols table to mark intraday data as loaded
  #[clap(long, default_value = "true")]
  update_symbols: bool,

  /// Maximum number of symbols to process (default: 100, use for smaller batches)
  #[clap(long)]
  limit: Option<usize>,

  /// Check each record individually for duplicates (use for historical/backfill data)
  #[clap(long)]
  check_each_record: bool,

  /// Show verbose output
  #[clap(short, long)]
  verbose: bool,
}

/// Get the latest timestamp for each symbol from the database
async fn get_latest_timestamps(
  config: &Config,
  sids: &[i64],
) -> Result<HashMap<i64, DateTime<Utc>>> {
  tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let sids = sids.to_vec();

    move || -> Result<HashMap<i64, DateTime<Utc>>> {
      use diesel::prelude::*;

      let mut conn = establish_connection(&database_url)?;

      // Get the maximum timestamp for each sid - need to do this individually per group
      let mut timestamp_map = HashMap::new();

      for sid in sids {
        let latest: Option<DateTime<Utc>> = intradayprices::table
          .select(diesel::dsl::max(intradayprices::tstamp))
          .filter(intradayprices::sid.eq(sid))
          .first(&mut conn)?;

        if let Some(ts) = latest {
          timestamp_map.insert(sid, ts);
        }
      }

      info!("Retrieved latest timestamps for {} symbols", timestamp_map.len());

      Ok(timestamp_map)
    }
  })
  .await?
  .map_err(|e| anyhow::anyhow!(e))
}

/// Save crypto intraday prices with optimized timestamp-based deduplication
async fn save_crypto_intraday_prices_optimized(
  config: &Config,
  prices: Vec<CryptoIntradayPriceData>,
  update_existing: bool,
  update_symbols: bool,
  check_each_record: bool,
  latest_timestamps: HashMap<i64, DateTime<Utc>>,
) -> Result<usize> {
  if prices.is_empty() {
    info!("No prices to save");
    return Ok(0);
  }

  tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();

    move || -> Result<usize> {
      use diesel::prelude::*;
      use std::collections::HashSet;

      let mut conn = establish_connection(&database_url)?;

      info!("💾 Processing {} crypto intraday price records", prices.len());

      let mut saved_count = 0;
      let mut skipped_count = 0;
      let mut symbols_updated = HashSet::new();

      // Group prices by symbol for efficient processing
      let mut prices_by_symbol: HashMap<i64, Vec<CryptoIntradayPriceData>> = HashMap::new();
      for price in prices {
        prices_by_symbol.entry(price.sid).or_insert_with(Vec::new).push(price);
      }

      for (sid, mut symbol_prices) in prices_by_symbol {
        let original_count = symbol_prices.len();
        let latest_existing = latest_timestamps.get(&sid);

        // Get the symbol string from the first price record (all records for a sid have the same symbol)
        let symbol_str = symbol_prices.first().map(|p| p.symbol.clone()).unwrap_or_default();

        // Filter prices based on timestamp
        let new_prices: Vec<CryptoIntradayPriceData> = if check_each_record {
          // For historical data, check each record individually
          let timestamps: Vec<DateTime<Utc>> = symbol_prices.iter().map(|p| p.tstamp).collect();

          let existing: Vec<DateTime<Utc>> = intradayprices::table
            .select(intradayprices::tstamp)
            .filter(intradayprices::sid.eq(sid))
            .filter(intradayprices::tstamp.eq_any(&timestamps))
            .load::<DateTime<Utc>>(&mut conn)?;

          let existing_set: HashSet<DateTime<Utc>> = existing.into_iter().collect();

          symbol_prices.into_iter().filter(|p| !existing_set.contains(&p.tstamp)).collect()
        } else if let Some(&latest_ts) = latest_existing {
          // For real-time data, only keep records newer than the latest we have
          symbol_prices.into_iter().filter(|p| p.tstamp > latest_ts).collect()
        } else {
          // No existing data for this symbol, all records are new
          symbol_prices
        };

        let filtered_count = new_prices.len();
        skipped_count += original_count - filtered_count;

        if !new_prices.is_empty() {
          // Sort by timestamp to maintain order
          let mut sorted_prices = new_prices;
          sorted_prices.sort_by_key(|p| p.tstamp);

          // Convert to insert format
          let new_records: Vec<NewIntradayPrice> = sorted_prices
            .iter()
            .map(|p| NewIntradayPrice {
              eventid: &p.eventid,
              tstamp: &p.tstamp,
              sid: &p.sid,
              symbol: &symbol_str, // Use the actual symbol string
              open: &p.open,
              high: &p.high,
              low: &p.low,
              close: &p.close,
              volume: &p.volume,
              price_source_id: &p.price_source_id,
            })
            .collect();

          // Batch insert new records
          for chunk in new_records.chunks(500) {
            let inserted = diesel::insert_into(intradayprices::table)
              .values(chunk)
              .on_conflict_do_nothing() // Safety net
              .execute(&mut conn)?;

            saved_count += inserted;
          }

          symbols_updated.insert(sid);

          info!(
            "Symbol {}: saved {} new records, skipped {} existing",
            sid,
            filtered_count,
            original_count - filtered_count
          );
        } else if original_count > 0 {
          info!(
            "Symbol {}: all {} records already exist (latest: {:?})",
            sid, original_count, latest_existing
          );
        }
      }

      // Update symbols table to mark intraday data as loaded
      if update_symbols && !symbols_updated.is_empty() {
        let sids: Vec<i64> = symbols_updated.into_iter().collect();
        diesel::update(symbols::table)
          .filter(symbols::sid.eq_any(&sids))
          .set(symbols::intraday.eq(true))
          .execute(&mut conn)?;

        info!("Updated symbols table for {} symbols", sids.len());
      }

      info!(
        "✅ Database operation complete: {} new records saved, {} skipped (already existed)",
        saved_count, skipped_count
      );

      Ok(saved_count)
    }
  })
  .await?
  .map_err(|e| anyhow::anyhow!(e))
}

/// Get crypto symbols to load based on command arguments
async fn get_crypto_symbols_to_load(
  args: &CryptoIntradayArgs,
  config: &Config,
) -> Result<Vec<CryptoIntradaySymbolInfo>> {
  let mut conn = establish_connection(&config.database_url)?;

  let symbols = if let Some(ref symbol) = args.symbol {
    // Load specific symbol if provided - get the one with the BEST priority (lowest number)
    symbols::table
      .filter(symbols::symbol.eq(symbol))
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .order(symbols::priority.asc())
      .limit(1) // Get only the top priority one
      .select((symbols::sid, symbols::symbol, symbols::priority))
      .load::<(i64, String, i32)>(&mut conn)?
  } else {
    // Build base query for cryptocurrencies
    // IMPORTANT: For crypto, we only want symbols with valid priorities (not 9999999)
    let mut query = symbols::table
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(symbols::priority.lt(9999999)) // Use less than instead of not equal
      .into_boxed();

    // Apply intraday data filters
    if args.skip_existing {
      query = query.filter(symbols::intraday.eq(false).or(symbols::intraday.is_null()));
      info!("Filtering to symbols WITHOUT existing intraday data");
    } else if args.only_existing {
      query = query.filter(symbols::intraday.eq(true));
      info!("Filtering to symbols WITH existing intraday data (refresh mode)");
    }

    // Order by priority - MOST IMPORTANT: get top priority first
    query = query.order(symbols::priority.asc());

    // Apply limit - default to 500 for batch processing
    let limit = args.limit.unwrap_or(500); // Back to original default of 500
    query = query.limit(limit as i64);

    let results =
      query
        .select((symbols::sid, symbols::symbol, symbols::priority))
        .load::<(i64, String, i32)>(&mut conn)?;

    // Log what we actually got
    if results.is_empty() {
      warn!("No symbols found with priority < 9999999");
    } else {
      info!(
        "Found {} symbols with valid priorities: {:?}",
        results.len(),
        results
          .iter()
          .map(|(_, sym, pri)| format!("{} (priority: {})", sym, pri))
          .collect::<Vec<_>>()
      );
    }

    results
  };

  // Final logging
  info!(
    "Retrieved {} symbols to load: {:?}",
    symbols.len(),
    symbols
      .iter()
      .map(|(sid, sym, pri)| format!("{} (sid: {}, priority: {})", sym, sid, pri))
      .collect::<Vec<_>>()
  );

  // Convert to CryptoSymbolInfo
  Ok(
    symbols
      .into_iter()
      .map(|(sid, symbol, priority)| CryptoIntradaySymbolInfo { sid, symbol, priority })
      .collect(),
  )
}

/// Get the maximum event ID from the database
async fn get_max_eventid(config: &Config) -> Result<i64> {
  let mut conn = establish_connection(&config.database_url)?;

  let max_id: Option<i64> =
    intradayprices::table.select(diesel::dsl::max(intradayprices::eventid)).first(&mut conn)?;

  Ok(max_id.unwrap_or(0))
}

/// Clean up expired cache entries
async fn cleanup_expired_cache(config: &Config) -> Result<()> {
  tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();

    move || -> Result<()> {
      use av_database_postgres::schema::api_response_cache;
      use diesel::prelude::*;

      let mut conn = establish_connection(&database_url)?;

      let deleted = diesel::delete(
        api_response_cache::table.filter(api_response_cache::expires_at.lt(diesel::dsl::now)),
      )
      .execute(&mut conn)?;

      if deleted > 0 {
        info!("Cleaned up {} expired cache entries", deleted);
      }

      Ok(())
    }
  })
  .await?
  .map_err(|e| anyhow::anyhow!(e))
}

/// Execute the crypto intraday price loading command
pub async fn execute(args: CryptoIntradayArgs, config: Config) -> Result<()> {
  // Clean up expired cache entries before starting
  info!("Cleaning up expired cache entries...");
  if let Err(e) = cleanup_expired_cache(&config).await {
    warn!("Failed to clean up cache: {}", e);
  }

  // Get symbols to load - already filtered by priority
  let symbols = get_crypto_symbols_to_load(&args, &config).await?;

  if symbols.is_empty() {
    warn!("No crypto symbols found matching the criteria");
    return Ok(());
  }

  // Log which symbols we're about to process
  info!(
    "Found {} symbols to process: {:?}",
    symbols.len(),
    symbols.iter().map(|s| &s.symbol).collect::<Vec<_>>()
  );

  info!("Loading crypto intraday prices for {} symbols", symbols.len());

  // Get latest timestamps for all symbols upfront (unless force refresh)
  let latest_timestamps = if !args.force_refresh && !args.dry_run {
    let sids: Vec<i64> = symbols.iter().map(|s| s.sid).collect();
    get_latest_timestamps(&config, &sids).await?
  } else {
    HashMap::new()
  };

  // Log symbols that already have data
  if !latest_timestamps.is_empty() {
    info!(
      "{} symbols already have intraday data and will be updated incrementally",
      latest_timestamps.len()
    );
  }

  info!(
    "Configuration: market={}, interval={}, outputsize={}, concurrent={}, api_delay={}ms",
    args.market, args.interval, args.outputsize, args.concurrent, args.api_delay
  );

  if args.check_each_record {
    warn!("Running in historical mode - will check each record individually for duplicates");
  }

  // Calculate estimated time for all symbols
  if !args.dry_run && symbols.len() > 1 {
    let delay_seconds = args.api_delay as f64 / 1000.0;
    let total_time = symbols.len() as f64 * delay_seconds;
    let hours = (total_time / 3600.0) as u64;
    let minutes = ((total_time % 3600.0) / 60.0) as u64;

    info!("Estimated minimum time: {}h {}m (based on API rate limiting)", hours, minutes);

    if symbols.len() > 50 {
      warn!(
        "⚠️  Loading {} symbols will take significant time due to API rate limits",
        symbols.len()
      );
      warn!("Consider using --limit flag to process in smaller batches");
    }
  }

  // Create API client
  let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: !args.dry_run,
    batch_size: 100,
  };

  // Create loader context
  let mut context = LoaderContext::new(client, loader_config);

  // Set up process tracking
  if !args.dry_run {
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Get the current max event ID
  let max_eventid = if args.dry_run { 0 } else { get_max_eventid(&config).await? };

  // Create and configure the loader
  // IMPORTANT: primary_only is always false here because filtering
  // already happened at the CLI level when getting symbols to load
  let loader_cfg = CryptoIntradayConfig {
    interval: IntradayInterval::from_str(&args.interval)
      .ok_or_else(|| anyhow::anyhow!("Invalid interval"))?,
    market: args.market.clone(),
    outputsize: args.outputsize.clone(),
    max_concurrent: args.concurrent,
    update_existing: args.update,
    api_delay_ms: args.api_delay,
    enable_cache: !args.force_refresh,
    cache_ttl_hours: 2,
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
    primary_only: false, // Always false - filtering done at CLI level
  };

  let loader = CryptoIntradayLoader::new(args.concurrent)
    .with_config(loader_cfg)
    .with_starting_eventid(max_eventid + 1);

  // Prepare input
  let input = CryptoIntradayLoaderInput {
    symbols,
    market: args.market.clone(),
    interval: args.interval.clone(),
    outputsize: args.outputsize.clone(),
  };

  // Execute the loader
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load crypto intraday prices: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      return Ok(());
    }
  };

  // Display summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║     CRYPTO INTRADAY PRICE LOADING SUMMARY   ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Market:             {:<24} ║", args.market);
  println!("║ Interval:           {:<24} ║", args.interval);
  println!("║ Symbols Loaded:     {:<24} ║", output.symbols_loaded);
  println!("║ Symbols Failed:     {:<24} ║", output.symbols_failed);
  println!("║ Symbols Skipped:    {:<24} ║", output.symbols_skipped);
  println!("║ Total Records:      {:<24} ║", output.data.len());
  println!("╚════════════════════════════════════════════╝");

  // Show failed symbols if any
  if !output.failed_symbols.is_empty() {
    warn!("Failed symbols: {:?}", output.failed_symbols);
  }

  // Save to database unless dry run
  if !args.dry_run && !output.data.is_empty() {
    info!("Saving {} crypto intraday price records to database", output.data.len());

    let saved = save_crypto_intraday_prices_optimized(
      &config,
      output.data,
      args.update,
      args.update_symbols,
      args.check_each_record,
      latest_timestamps,
    )
    .await?;

    info!("Successfully processed {} records", saved);
  } else if args.dry_run {
    info!("Dry run - would have saved {} records", output.data.len());
  }

  info!("Crypto intraday price loader completed");

  Ok(())
}
