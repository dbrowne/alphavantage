//! Crypto intraday price loading command

use anyhow::Result;
use clap::Parser;
use diesel::prelude::*;
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
  /// Specific symbol to load (defaults to top 500 cryptocurrencies if not specified)
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

  /// Only load primary crypto symbols (priority != 9999999)
  /// This is always true unless explicitly set to false
  #[clap(long, default_value = "true")]
  primary_only: bool,

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

  /// Force refresh - bypass cache
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

  /// Maximum number of symbols to process (default: 500, use for smaller batches)
  #[clap(long)]
  limit: Option<usize>,

  /// Show verbose output
  #[clap(short, long)]
  verbose: bool,
}

/// Get crypto symbols to load based on command arguments
async fn get_crypto_symbols_to_load(
  args: &CryptoIntradayArgs,
  config: &Config,
) -> Result<Vec<CryptoIntradaySymbolInfo>> {
  let mut conn = establish_connection(&config.database_url)?;

  let symbols = if let Some(ref symbol) = args.symbol {
    // Load specific symbol if provided
    symbols::table
      .filter(symbols::symbol.eq(symbol))
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .select((symbols::sid, symbols::symbol, symbols::priority))
      .load::<(i64, String, i32)>(&mut conn)?
  } else {
    // Build base query for cryptocurrencies
    let mut query = symbols::table
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(symbols::priority.ne(9999999)) // Exclude unranked
      .into_boxed();

    // Apply intraday data filters
    if args.skip_existing {
      // Skip symbols that already have intraday data
      query = query.filter(symbols::intraday.eq(false).or(symbols::intraday.is_null()));
      info!("Filtering to symbols WITHOUT existing intraday data");
    } else if args.only_existing {
      // Only load symbols that already have intraday data (for updates/refreshing)
      query = query.filter(symbols::intraday.eq(true));
      info!("Filtering to symbols WITH existing intraday data (refresh mode)");
    }

    // Order by priority and apply limit
    query = query.order(symbols::priority.asc());

    let limit = args.limit.unwrap_or(500);
    query = query.limit(limit as i64);

    query
      .select((symbols::sid, symbols::symbol, symbols::priority))
      .load::<(i64, String, i32)>(&mut conn)?
  };

  // Log filtering results
  if args.skip_existing || args.only_existing {
    info!("Found {} symbols matching intraday data filter", symbols.len());
  }

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

/// Save crypto intraday prices to the database
async fn save_crypto_intraday_prices(
  config: &Config,
  prices: Vec<CryptoIntradayPriceData>,
  update_existing: bool,
  update_symbols: bool,
) -> Result<usize> {
  if prices.is_empty() {
    info!("No prices to save");
    return Ok(0);
  }

  tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let prices = prices;

    move || -> Result<usize> {
      use diesel::prelude::*;
      use std::collections::HashSet;

      let mut conn = establish_connection(&database_url)?;

      info!("💾 Saving {} crypto intraday price records to database", prices.len());

      // Convert to NewIntradayPrice records
      let new_prices: Vec<NewIntradayPrice> = prices
        .iter()
        .map(|p| NewIntradayPrice {
          eventid: &p.eventid,
          tstamp: &p.tstamp,
          sid: &p.sid,
          symbol: &p.symbol,
          open: &p.open,
          high: &p.high,
          low: &p.low,
          close: &p.close,
          volume: &p.volume,
        })
        .collect();

      // Insert in batches of 1000
      let mut total_inserted = 0;
      for chunk in new_prices.chunks(1000) {
        let result = if update_existing {
          diesel::insert_into(intradayprices::table)
            .values(chunk)
            .on_conflict((intradayprices::sid, intradayprices::tstamp))
            .do_update()
            .set((
              intradayprices::open.eq(diesel::pg::upsert::excluded(intradayprices::open)),
              intradayprices::high.eq(diesel::pg::upsert::excluded(intradayprices::high)),
              intradayprices::low.eq(diesel::pg::upsert::excluded(intradayprices::low)),
              intradayprices::close.eq(diesel::pg::upsert::excluded(intradayprices::close)),
              intradayprices::volume.eq(diesel::pg::upsert::excluded(intradayprices::volume)),
            ))
            .execute(&mut conn)
        } else {
          diesel::insert_into(intradayprices::table)
            .values(chunk)
            .on_conflict_do_nothing()
            .execute(&mut conn)
        };

        match result {
          Ok(count) => total_inserted += count,
          Err(e) => {
            error!("Failed to insert batch: {}", e);
            return Err(anyhow::anyhow!("Database insert failed: {}", e));
          }
        }
      }

      // Update symbols table if requested
      if update_symbols && total_inserted > 0 {
        let unique_sids: HashSet<i64> = prices.iter().map(|p| p.sid).collect();
        let sids_count = unique_sids.len();
        for sid in &unique_sids {
          diesel::update(symbols::table.filter(symbols::sid.eq(sid)))
            .set((symbols::intraday.eq(true), symbols::m_time.eq(diesel::dsl::now)))
            .execute(&mut conn)?;
        }
        info!("Updated symbols table for {} symbols", sids_count);
      }

      info!("✅ Successfully saved {} records", total_inserted);
      Ok(total_inserted)
    }
  })
  .await?
}

/// Main execution function
pub async fn execute(args: CryptoIntradayArgs, config: Config) -> Result<()> {
  // Clean up expired cache if not forcing refresh
  if !args.force_refresh && !args.dry_run {
    info!("Cleaning up expired cache entries...");
    // Cache cleanup could be implemented here
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

  // Validate market
  let valid_markets = ["USD", "EUR", "GBP", "JPY", "CNY", "CAD", "AUD"];
  if !valid_markets.contains(&args.market.as_str()) {
    warn!(
      "Market '{}' may not be supported for all cryptocurrencies. Common markets: {:?}",
      args.market, valid_markets
    );
  }

  // Get symbols to load
  let symbols = get_crypto_symbols_to_load(&args, &config).await?;

  if symbols.is_empty() {
    warn!("No cryptocurrency symbols found matching criteria");
    if args.skip_existing {
      info!("All symbols may already have intraday data. Use --force-refresh to reload.");
    } else if args.only_existing {
      info!("No symbols have been previously loaded. Run without --only-existing first.");
    }
    return Ok(());
  }

  let symbol_count = symbols.len();
  info!(
    "Loading crypto intraday prices for {} symbols (top {} by priority)",
    symbol_count,
    if args.symbol.is_some() {
      "single".to_string()
    } else if args.limit.is_some() {
      format!("{}", args.limit.unwrap())
    } else {
      "500".to_string()
    }
  );

  info!(
    "Configuration: market={}, interval={}, outputsize={}, concurrent={}, api_delay={}ms",
    args.market, args.interval, args.outputsize, args.concurrent, args.api_delay
  );

  // Calculate estimated time for rate limiting
  if args.api_delay > 0 {
    let estimated_seconds =
      (symbols.len() as u64 * args.api_delay) / 1000 / args.concurrent.max(1) as u64;
    let hours = estimated_seconds / 3600;
    let minutes = (estimated_seconds % 3600) / 60;
    info!("Estimated minimum time: {}h {}m (based on API rate limiting)", hours, minutes);

    // Warning for large batches
    if symbol_count > 100 {
      warn!(
        "⚠️  Loading {} symbols will take significant time due to API rate limits",
        symbol_count
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

  // Set up process tracking (unless dry run)
  if !args.dry_run {
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Get the current max event ID
  let max_eventid = if args.dry_run { 0 } else { get_max_eventid(&config).await? };

  // Create and configure the loader
  let loader_cfg = CryptoIntradayConfig {
    interval: IntradayInterval::from_str(&args.interval)
      .ok_or_else(|| anyhow::anyhow!("Invalid interval"))?,
    market: args.market.clone(),
    outputsize: args.outputsize.clone(),
    max_concurrent: args.concurrent,
    update_existing: args.update,
    api_delay_ms: args.api_delay,
    enable_cache: !args.force_refresh,
    cache_ttl_hours: 2, // Shorter cache for intraday data
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
    primary_only: args.primary_only,
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
      // Return empty output if continuing on error
      return Ok(());
    }
  };

  // Display summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║     CRYPTO INTRADAY PRICE LOADING SUMMARY   ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Market:             {:<24} ║", args.market);
  println!("║ Interval:           {:<24} ║", args.interval);
  println!("║ Primary Only:       {:<24} ║", args.primary_only);
  println!("║ Symbols Loaded:     {:<24} ║", output.symbols_loaded);
  println!("║ Symbols Failed:     {:<24} ║", output.symbols_failed);
  println!("║ Symbols Skipped:    {:<24} ║", output.symbols_skipped);
  println!("║ Total Records:      {:<24} ║", output.data.len());
  println!("╚════════════════════════════════════════════╝");

  if !output.failed_symbols.is_empty() && args.verbose {
    println!("\n❌ Failed symbols:");
    for symbol in &output.failed_symbols {
      println!("   - {}", symbol);
    }
  }

  // Save to database unless dry run
  if !args.dry_run && !output.data.is_empty() {
    info!("Saving {} crypto intraday price records to database", output.data.len());

    match save_crypto_intraday_prices(&config, output.data, args.update, args.update_symbols).await
    {
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

  info!("Crypto intraday price loader completed");
  Ok(())
}
