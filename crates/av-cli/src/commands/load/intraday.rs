/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::Parser;
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{error, info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
  establish_connection,
  models::price::NewIntradayPrice,
  schema::{intradayprices, symbols},
};
use av_loaders::{
  DataLoader, IntradayInterval, IntradayPriceConfig, IntradayPriceData, IntradayPriceLoader,
  IntradayPriceLoaderInput, IntradaySymbolInfo, LoaderConfig, LoaderContext, ProcessTracker,
};

use crate::config::Config;

/// Symbol info for the loader
#[derive(Debug, Clone)]
struct LoaderSymbolInfo {
  pub sid: i64,
  pub symbol: String,
}

/// Arguments for the intraday price loader
#[derive(Parser, Debug)]
#[clap(about = "Load intraday price data from AlphaVantage")]
pub struct IntradayArgs {
  /// Specific symbol to load (loads all equity symbols if not specified)
  #[clap(short, long)]
  symbol: Option<String>,

  /// Time interval between data points (1min, 5min, 15min, 30min, 60min)
  #[clap(short, long, default_value = "1min")]
  interval: String,

  /// Specific month for historical data (format: YYYY-MM)
  #[clap(long)]
  month: Option<String>,

  /// Include extended trading hours
  #[clap(long, default_value = "true")]
  extended_hours: bool,

  /// Include split/dividend adjustments
  #[clap(long, default_value = "true")]
  adjusted: bool,

  /// Number of concurrent API requests
  #[clap(long, default_value = "5")]
  concurrent: usize,

  /// Output size (compact or full)
  #[clap(long, default_value = "full")]
  outputsize: String,

  /// Delay between API calls in milliseconds
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

  /// Continue on error
  #[clap(long)]
  continue_on_error: bool,

  /// Update symbols table to mark intraday data as loaded
  #[clap(long, default_value = "true")]
  update_symbols: bool,

  /// Maximum number of symbols to process
  #[clap(long)]
  limit: Option<usize>,

  /// Check each record individually for duplicates (use for historical/backfill data)
  #[clap(long)]
  check_each_record: bool,
}

/// Get symbols to load based on command arguments
async fn get_symbols_to_load(
  args: &IntradayArgs,
  config: &Config,
) -> Result<Vec<LoaderSymbolInfo>> {
  let mut conn = establish_connection(&config.database_url)?;

  let symbols = if let Some(ref symbol) = args.symbol {
    // Load specific symbol
    symbols::table
      .filter(symbols::symbol.eq(symbol))
      .filter(symbols::sec_type.eq("Equity"))
      .select((symbols::sid, symbols::symbol))
      .load::<(i64, String)>(&mut conn)?
  } else {
    // Load all equity symbols with overview data
    let mut query = symbols::table
      .filter(symbols::sec_type.eq("Equity"))
      .filter(symbols::overview.eq(true))
      .select((symbols::sid, symbols::symbol))
      .into_boxed();

    // Apply limit if specified
    if let Some(limit) = args.limit {
      query = query.limit(limit as i64);
    }

    query.load::<(i64, String)>(&mut conn)?
  };

  Ok(symbols.into_iter().map(|(sid, symbol)| LoaderSymbolInfo { sid, symbol }).collect())
}

/// Get the maximum event ID from the database
async fn get_max_eventid(config: &Config) -> Result<i64> {
  let mut conn = establish_connection(&config.database_url)?;

  let max_id: Option<i64> =
    intradayprices::table.select(diesel::dsl::max(intradayprices::eventid)).first(&mut conn)?;

  Ok(max_id.unwrap_or(0))
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

      // Get the maximum timestamp for each sid
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

/// Save intraday prices with optimized timestamp-based deduplication
async fn save_intraday_prices_optimized(
  config: &Config,
  prices: Vec<IntradayPriceData>,
  _update_existing: bool, //todo: FIX THIS!!
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

      // Set up progress bar for database insert/update operations
      let progress = ProgressBar::new(prices.len() as u64);
      progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
      );
      progress.set_message("Saving to database");

      let mut saved_count = 0;
      let mut skipped_count = 0;
      let mut unique_sids = HashSet::new();

      // Group prices by symbol for efficient processing
      let mut prices_by_symbol: HashMap<i64, Vec<IntradayPriceData>> = HashMap::new();
      for price in prices {
        prices_by_symbol.entry(price.sid).or_default().push(price);
      }

      for (sid, symbol_prices) in prices_by_symbol {
        unique_sids.insert(sid);
        let original_count = symbol_prices.len();
        let latest_existing = latest_timestamps.get(&sid);

        // Get the symbol string from the first price record
        let symbol_str = symbol_prices.first().map(|p| p.symbol.clone()).unwrap_or_default();

        // Filter prices based on timestamp
        let new_prices: Vec<IntradayPriceData> = if check_each_record {
          // For historical data, check each record individually
          info!("Checking {} records for {} (historical mode)", original_count, symbol_str);

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
              symbol: &symbol_str,
              open: &p.open,
              high: &p.high,
              low: &p.low,
              close: &p.close,
              volume: &p.volume,
              price_source_id: &1, //todo!! have to correct this!!
            })
            .collect();

          // Batch insert new records
          for chunk in new_records.chunks(500) {
            let inserted = diesel::insert_into(intradayprices::table)
              .values(chunk)
              .on_conflict_do_nothing() // Safety net
              .execute(&mut conn)?;

            progress.inc(chunk.len() as u64);
            saved_count += inserted;
          }

          info!(
            "Symbol {} ({}): saved {} new records, skipped {} existing",
            symbol_str,
            sid,
            filtered_count,
            original_count - filtered_count
          );
        } else if original_count > 0 {
          info!(
            "Symbol {} ({}): all {} records already exist (latest: {:?})",
            symbol_str, sid, original_count, latest_existing
          );
        }
      }

      // Update symbols table to mark intraday data as loaded
      if update_symbols && !unique_sids.is_empty() {
        let sids: Vec<i64> = unique_sids.into_iter().collect();
        diesel::update(symbols::table)
          .filter(symbols::sid.eq_any(&sids))
          .set((symbols::intraday.eq(true), symbols::m_time.eq(diesel::dsl::now)))
          .execute(&mut conn)?;

        info!("Updated symbols table for {} symbols", sids.len());
      }

      progress.finish_with_message(format!(
        "Saved {} new, , skipped {} already existing records",
        saved_count, skipped_count
      ));

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

/// Main execute function
pub async fn execute(args: IntradayArgs, config: Config) -> Result<()> {
  info!("Starting intraday price loader");

  // Clean up expired cache entries periodically
  if !args.dry_run {
    match IntradayPriceLoader::cleanup_expired_cache(&config.database_url).await {
      Ok(deleted) if deleted > 0 => info!("Cleaned up {} expired cache entries", deleted),
      Err(e) => warn!("Failed to cleanup expired cache: {}", e),
      _ => {}
    }
  }

  if args.dry_run {
    info!("DRY RUN MODE - No database updates will be performed");
  }

  // Validate interval
  let valid_intervals = ["1min", "5min", "15min", "30min", "60min"];
  if !valid_intervals.contains(&args.interval.as_str()) {
    return Err(anyhow::anyhow!(
      "Invalid interval '{}'. Must be one of: {:?}",
      args.interval,
      valid_intervals
    ));
  }

  // Get symbols to load
  let symbols = get_symbols_to_load(&args, &config).await?;

  if symbols.is_empty() {
    warn!("No symbols found to load");
    return Ok(());
  }

  info!("Found {} symbols to load", symbols.len());

  // Get latest timestamps for all symbols upfront (unless force refresh or checking each record)
  let latest_timestamps = if !args.force_refresh && !args.dry_run && !args.check_each_record {
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

  if args.check_each_record {
    warn!("Running in historical mode - will check each record individually for duplicates");
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

  // Configure the loader
  let loader_cfg = IntradayPriceConfig {
    interval: IntradayInterval::from_str(args.interval.as_str()).ok_or_else(|| {
      anyhow!("Invalid interval: {}. Must be 1min, 5min, 15min, 30min, or 60min", args.interval)
    })?,
    extended_hours: args.extended_hours,
    adjusted: args.adjusted,
    month: args.month.clone(),
    max_concurrent: args.concurrent,
    update_existing: args.update,
    api_delay_ms: args.api_delay,
    enable_cache: !args.force_refresh,
    cache_ttl_hours: 24, // Longer cache for equity data
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
  };

  let loader = IntradayPriceLoader::new(args.concurrent)
    .with_config(loader_cfg)
    .with_starting_eventid(max_eventid + 1);

  // Convert to loader format
  let loader_symbols: Vec<IntradaySymbolInfo> =
    symbols.iter().map(|s| IntradaySymbolInfo { sid: s.sid, symbol: s.symbol.clone() }).collect();

  // Prepare input
  let input = IntradayPriceLoaderInput {
    symbols: loader_symbols,
    interval: args.interval.clone(),
    extended_hours: args.extended_hours,
    adjusted: args.adjusted,
    month: args.month.clone(),
    output_size: args.outputsize.clone(),
  };

  // Execute the loader
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load intraday prices: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      return Ok(());
    }
  };

  // Display summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║      INTRADAY PRICE LOADING SUMMARY         ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Interval:           {:<24} ║", args.interval);
  println!("║ Extended Hours:     {:<24} ║", args.extended_hours);
  println!("║ Adjusted:           {:<24} ║", args.adjusted);
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
    info!("Saving {} intraday price records to database", output.data.len());

    let saved = save_intraday_prices_optimized(
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

  info!("Intraday price loader completed");

  Ok(())
}
