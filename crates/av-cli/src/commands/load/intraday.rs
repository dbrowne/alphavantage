//! Intraday price loading command

use anyhow::Result;
use clap::Parser;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tracing::{error, info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
  establish_connection,
  models::price::NewIntradayPrice,
  schema::{intradayprices, symbols},
};
use av_loaders::{
  DataLoader,
  IntradayPriceConfig,
  IntradayPriceData,
  IntradayPriceLoader,
  IntradayPriceLoaderInput,
  IntradaySymbolInfo, // Use the correct type alias
  LoaderConfig,
  LoaderContext,
  ProcessTracker,
};

// Use the CLI's own Config type
use crate::config::Config;

/// Symbol info for the loader - local type without exchange field
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

  /// Force refresh - bypass cache
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

  /// Maximum number of symbols to process (for testing)
  #[clap(long)]
  limit: Option<usize>,
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

/// Check for existing data to avoid duplicates
async fn check_existing_data(
  config: &Config,
  sid: i64,
  start_time: chrono::DateTime<chrono::Utc>,
  end_time: chrono::DateTime<chrono::Utc>,
) -> Result<bool> {
  let mut conn = establish_connection(&config.database_url)?;

  let count: i64 = intradayprices::table
    .filter(intradayprices::sid.eq(sid))
    .filter(intradayprices::tstamp.ge(start_time))
    .filter(intradayprices::tstamp.le(end_time))
    .count()
    .get_result(&mut conn)?;

  Ok(count > 0)
}

/// Save intraday prices to the database
async fn save_intraday_prices(
  config: &Config,
  prices: Vec<IntradayPriceData>,
  update_existing: bool,
  update_symbols: bool,
) -> Result<usize> {
  if prices.is_empty() {
    info!("No prices to save");
    return Ok(0);
  }

  tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let update_existing = update_existing;
    let update_symbols = update_symbols;
    let prices = prices;

    move || -> Result<usize> {
      use diesel::prelude::*;
      use std::collections::HashSet;

      let mut conn = establish_connection(&database_url)?;

      info!("Saving {} intraday price records to database", prices.len());

      // Set up progress bar
      let progress = ProgressBar::new(prices.len() as u64);
      progress.set_style(
        ProgressStyle::default_bar()
          .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
          .unwrap()
          .progress_chars("#>-"),
      );

      let mut total_inserted = 0;
      let mut total_updated = 0;
      let mut skipped_count = 0;
      let mut unique_sids = HashSet::new();

      // Process in batches for better performance
      const BATCH_SIZE: usize = 1000;

      for chunk in prices.chunks(BATCH_SIZE) {
        let mut batch_insert = Vec::new();
        let mut batch_update = Vec::new();

        for price in chunk {
          unique_sids.insert(price.sid);

          let new_price = NewIntradayPrice {
            eventid: &price.eventid,
            tstamp: &price.tstamp,
            sid: &price.sid,
            symbol: &price.symbol,
            open: &price.open,
            high: &price.high,
            low: &price.low,
            close: &price.close,
            volume: &price.volume,
          };

          // Check if record exists
          let exists = intradayprices::table
            .filter(intradayprices::tstamp.eq(&price.tstamp))
            .filter(intradayprices::sid.eq(&price.sid))
            .count()
            .get_result::<i64>(&mut conn)?
            > 0;

          if exists {
            if update_existing {
              batch_update.push(new_price);
            } else {
              skipped_count += 1;
            }
          } else {
            batch_insert.push(new_price);
          }

          progress.inc(1);
        }

        // Batch insert new records
        if !batch_insert.is_empty() {
          let inserted = diesel::insert_into(intradayprices::table)
            .values(&batch_insert)
            .on_conflict_do_nothing()
            .execute(&mut conn)?;
          total_inserted += inserted;
        }

        // Update existing records if requested
        if update_existing && !batch_update.is_empty() {
          for record in batch_update {
            let updated = diesel::update(intradayprices::table)
              .filter(intradayprices::tstamp.eq(record.tstamp))
              .filter(intradayprices::sid.eq(record.sid))
              .set((
                intradayprices::open.eq(record.open),
                intradayprices::high.eq(record.high),
                intradayprices::low.eq(record.low),
                intradayprices::close.eq(record.close),
                intradayprices::volume.eq(record.volume),
              ))
              .execute(&mut conn)?;
            total_updated += updated;
          }
        }
      }

      progress.finish_with_message(format!(
        "Saved {} new, updated {} existing, skipped {} unchanged price records",
        total_inserted, total_updated, skipped_count
      ));

      info!(
        "Database operation complete: {} inserted, {} updated, {} skipped",
        total_inserted, total_updated, skipped_count
      );

      // Update symbols table to mark intraday data as loaded
      if update_symbols && !unique_sids.is_empty() {
        info!("Updating {} symbols to mark intraday data as loaded", unique_sids.len());

        diesel::update(symbols::table)
          .filter(symbols::sid.eq_any(&unique_sids))
          .set((symbols::intraday.eq(true), symbols::m_time.eq(diesel::dsl::now)))
          .execute(&mut conn)?;
      }

      Ok(total_inserted + total_updated)
    }
  })
  .await?
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

  // Validate month format if provided
  if let Some(ref month) = args.month {
    if !month.chars().all(|c| c.is_ascii_digit() || c == '-') || month.len() != 7 {
      return Err(anyhow::anyhow!("Invalid month format '{}'. Must be in format YYYY-MM", month));
    }
  }

  // Get symbols to load
  let symbols = get_symbols_to_load(&args, &config).await?;

  if symbols.is_empty() {
    warn!("No symbols to load. Ensure symbols have sec_type='Equity' and overview=true");
    return Ok(());
  }

  info!("Loading intraday prices for {} equity symbols", symbols.len());
  info!(
    "Configuration: interval={}, extended_hours={}, adjusted={}, month={:?}, concurrent={}, api_delay={}ms",
    args.interval, args.extended_hours, args.adjusted, args.month, args.concurrent, args.api_delay
  );

  // Calculate estimated time
  if args.api_delay > 0 {
    let estimated_seconds =
      (symbols.len() as u64 * args.api_delay) / 1000 / args.concurrent.max(1) as u64;
    let hours = estimated_seconds / 3600;
    let minutes = (estimated_seconds % 3600) / 60;
    info!("Estimated time: {}h {}m (assuming no failures)", hours, minutes);
  }

  // Create API client with the correct configuration
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

  // Set up process tracking (unless dry run)
  if !args.dry_run {
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Get the current max event ID
  let max_eventid = if args.dry_run { 0 } else { get_max_eventid(&config).await? };

  // Create and configure the loader
  let loader_cfg = IntradayPriceConfig {
    interval: av_loaders::IntradayInterval::from_str(&args.interval)
      .ok_or_else(|| anyhow::anyhow!("Invalid interval"))?,
    extended_hours: args.extended_hours,
    adjusted: args.adjusted,
    month: args.month.clone(),
    max_concurrent: args.concurrent,
    update_existing: args.update,
    api_delay_ms: args.api_delay,
    enable_cache: !args.force_refresh,
    cache_ttl_hours: 2, // Shorter cache for intraday data
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
  };

  let loader = IntradayPriceLoader::new(args.concurrent)
    .with_config(loader_cfg)
    .with_starting_eventid(max_eventid + 1);

  // Convert symbols to the loader's expected type
  let loader_symbols: Vec<IntradaySymbolInfo> =
    symbols.into_iter().map(|s| IntradaySymbolInfo { sid: s.sid, symbol: s.symbol }).collect();

  // Prepare input
  let input = IntradayPriceLoaderInput {
    symbols: loader_symbols,
    interval: args.interval.clone(),
    extended_hours: args.extended_hours,
    adjusted: args.adjusted,
    month: args.month.clone(),
    outputsize: args.outputsize,
  };

  // Execute the loader
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load intraday prices: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      // Return empty output if continuing on error
      return Ok(());
    }
  };

  // Display summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║        INTRADAY PRICE LOADING SUMMARY       ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Interval:           {:<24} ║", args.interval);
  println!("║ Extended Hours:     {:<24} ║", args.extended_hours);
  println!("║ Symbols Loaded:     {:<24} ║", output.symbols_loaded);
  println!("║ Symbols Failed:     {:<24} ║", output.symbols_failed);
  println!("║ Symbols Skipped:    {:<24} ║", output.symbols_skipped);
  println!("║ Total Records:      {:<24} ║", output.data.len());
  println!("╚════════════════════════════════════════════╝");

  if !output.failed_symbols.is_empty() {
    println!("\n❌ Failed symbols:");
    for symbol in &output.failed_symbols {
      println!("   - {}", symbol);
    }
  }

  // Save to database unless dry run
  if !args.dry_run && !output.data.is_empty() {
    info!("Saving {} intraday price records to database", output.data.len());

    match save_intraday_prices(&config, output.data, args.update, args.update_symbols).await {
      Ok(saved) => info!("Successfully saved {} records", saved),
      Err(e) => {
        error!("Failed to save prices to database: {}", e);
        if !args.continue_on_error {
          return Err(e);
        }
      }
    }
  } else if args.dry_run {
    info!("Dry run - skipping database save");
  } else {
    info!("No data to save");
  }

  info!("Intraday price loader completed");
  Ok(())
}
