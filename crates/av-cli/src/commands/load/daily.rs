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

use anyhow::Result;
use clap::Args;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
  models::price::NewSummaryPriceOwned,
  schema::{summaryprices, symbols},
};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, ProcessState, ProcessTracker, SummaryPriceConfig,
  SummaryPriceData, SummaryPriceLoader, SummaryPriceLoaderInput,
};

use crate::config::Config;

#[derive(Args, Debug)]
pub struct DailyArgs {
  /// Single symbol to load
  #[arg(short, long, conflicts_with = "symbols_file")]
  pub symbol: Option<String>,

  /// Load symbols from file (one per line)
  #[arg(long, conflicts_with = "symbol")]
  pub symbols_file: Option<String>,

  /// Output size: "compact" (100 days) or "full" (20+ years)
  #[arg(long, default_value = "compact")]
  pub outputsize: String,

  /// Number of concurrent API requests
  #[arg(short, long, default_value = "5")]
  pub concurrent: usize,

  /// Delay between API calls in milliseconds (800 for premium = 75 calls/minute)
  #[arg(long, default_value = "800")]
  pub api_delay: u64,

  /// Update existing data (re-fetch latest values)
  #[arg(long)]
  pub update: bool,

  /// Force refresh (bypass cache)
  #[arg(long)]
  pub force_refresh: bool,

  /// Dry run - don't save to database
  #[arg(long)]
  pub dry_run: bool,

  /// Continue on errors
  #[arg(short = 'k', long)]
  pub continue_on_error: bool,

  /// Show verbose output
  #[arg(short, long)]
  pub verbose: bool,
}

/// Get symbols to load based on command arguments
async fn get_symbols_to_load(args: &DailyArgs, config: &Config) -> Result<Vec<(i64, String)>> {
  if let Some(symbol) = &args.symbol {
    // Single symbol - need to look up SID
    let sid = get_or_create_sid(symbol, config).await?;
    Ok(vec![(sid, symbol.clone())])
  } else if let Some(file_path) = &args.symbols_file {
    // Load symbols from file
    load_symbols_from_file(file_path, config).await
  } else {
    // Default: Load all active equity symbols with overview data
    load_active_symbols(config).await
  }
}

/// Get or create SID for a symbol
async fn get_or_create_sid(symbol: &str, config: &Config) -> Result<i64> {
  use tokio::task;

  let db_url = config.database_url.clone();
  let symbol = symbol.to_uppercase();

  task::spawn_blocking(move || -> Result<i64> {
        let mut conn = PgConnection::establish(&db_url)?;

        // Try to find existing symbol that is an Equity with overview data
        let existing: Option<(i64, String, bool)> = symbols::table
            .filter(symbols::symbol.eq(&symbol))
            .filter(symbols::sec_type.eq("Equity"))
            .select((symbols::sid, symbols::sec_type, symbols::overview))
            .first(&mut conn)
            .optional()?;

        if let Some((sid, _sec_type, has_overview)) = existing {
            if !has_overview {
                return Err(anyhow::anyhow!(
                    "Symbol {} exists but does not have overview data. Please run 'av load overviews' first",
                    symbol
                ));
            }
            Ok(sid)
        } else {
            // Symbol doesn't exist or is not an Equity
            Err(anyhow::anyhow!(
                "Symbol {} not found as an Equity in database. Please run 'av load securities' and 'av load overviews' first",
                symbol
            ))
        }
    })
        .await?
}

/// Load symbols from a file
async fn load_symbols_from_file(file_path: &str, config: &Config) -> Result<Vec<(i64, String)>> {
  let content = fs::read_to_string(file_path)?;
  let mut symbols = Vec::new();

  for line in content.lines() {
    let symbol = line.trim();
    if !symbol.is_empty() && !symbol.starts_with('#') {
      match get_or_create_sid(symbol, config).await {
        Ok(sid) => symbols.push((sid, symbol.to_string())),
        Err(e) => warn!("Skipping symbol {}: {}", symbol, e),
      }
    }
  }

  Ok(symbols)
}

/// Load all active symbols from database
async fn load_active_symbols(config: &Config) -> Result<Vec<(i64, String)>> {
  use tokio::task;

  let db_url = config.database_url.clone();

  task::spawn_blocking(move || -> Result<Vec<(i64, String)>> {
    let mut conn = PgConnection::establish(&db_url)?;

    let symbols: Vec<(i64, String)> = symbols::table
      .filter(symbols::sec_type.eq("Equity"))
      .filter(symbols::overview.eq(true))
      .select((symbols::sid, symbols::symbol))
      .order(symbols::priority.asc())
      .load(&mut conn)?;

    Ok(symbols)
  })
  .await?
}

/// Get the maximum event ID from the database
async fn get_max_eventid(config: &Config) -> Result<i64> {
  use tokio::task;

  let db_url = config.database_url.clone();

  task::spawn_blocking(move || -> Result<i64> {
    let mut conn = PgConnection::establish(&db_url)?;

    let max_id: Option<i64> =
      summaryprices::table.select(diesel::dsl::max(summaryprices::eventid)).first(&mut conn)?;

    Ok(max_id.unwrap_or(0))
  })
  .await?
}

/// Save summary prices to database
async fn save_summary_prices(
  data: Vec<SummaryPriceData>,
  config: &Config,
  update_symbols: bool,
) -> Result<usize> {
  use tokio::task;

  if data.is_empty() {
    return Ok(0);
  }

  let db_url = config.database_url.clone();

  task::spawn_blocking(move || -> Result<usize> {
    let mut conn = PgConnection::establish(&db_url)?;

    // First, check for existing records to avoid duplicates
    let mut records_to_insert = Vec::new();
    let mut records_to_update = Vec::new();
    let mut skipped_count = 0;
    
    // Set up progress bar for database retrieval operations
    let progress = ProgressBar::new(data.len() as u64);
    progress.set_style(
      ProgressStyle::default_bar()
          .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
          .unwrap()
          .progress_chars("##-"),
    );
    progress.set_message("Querying database for existing records to avoid duplicates");

    for price_data in data {
      // Check if record already exists for this (sid, date)
      // Get the FIRST eventid for this combination (should be unique)
      let existing: Option<(i64, f32, f32, f32, f32, i64)> = summaryprices::table
        .filter(summaryprices::sid.eq(price_data.sid))
        .filter(summaryprices::date.eq(price_data.date))
        .select((
          summaryprices::eventid,
          summaryprices::open,
          summaryprices::high,
          summaryprices::low,
          summaryprices::close,
          summaryprices::volume,
        ))
        .first(&mut conn)
        .optional()?;

      if let Some((
        existing_eventid,
        existing_open,
        existing_high,
        existing_low,
        existing_close,
        existing_volume,
      )) = existing
      {
        // Check if data actually changed
        if existing_open != price_data.open
          || existing_high != price_data.high
          || existing_low != price_data.low
          || existing_close != price_data.close
          || existing_volume != price_data.volume
        {
          // Data changed, update the record
          records_to_update.push(NewSummaryPriceOwned {
            eventid: existing_eventid, // Use existing eventid
            tstamp: price_data.tstamp,
            date: price_data.date,
            sid: price_data.sid,
            symbol: price_data.symbol,
            open: price_data.open,
            high: price_data.high,
            low: price_data.low,
            close: price_data.close,
            volume: price_data.volume,
            price_source_id: 1,
          });
        } else {
          // Data unchanged, skip
          skipped_count += 1;
          debug!("Skipping unchanged record for {} on {}", price_data.symbol, price_data.date);
        }
      } else {
        // New record - use generated eventid
        records_to_insert.push(NewSummaryPriceOwned {
          eventid: price_data.eventid,
          tstamp: price_data.tstamp,
          date: price_data.date,
          sid: price_data.sid,
          symbol: price_data.symbol,
          open: price_data.open,
          high: price_data.high,
          low: price_data.low,
          close: price_data.close,
          volume: price_data.volume,
          price_source_id: 1,
        });
      }
      progress.inc(1);
    }

    // Collect unique SIDs for updating symbols table
    let mut unique_sids = std::collections::HashSet::new();
    for record in &records_to_insert {
      unique_sids.insert(record.sid);
    }
    for record in &records_to_update {
      unique_sids.insert(record.sid);
    }
    let unique_sids: Vec<i64> = unique_sids.into_iter().collect();

    let total_records = records_to_insert.len() + records_to_update.len();

    // Set up progress bar for database insert/update operations
    let progress = ProgressBar::new(total_records as u64);
    progress.set_style(
      ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("##-"),
    );
    progress.set_message("Saving to database");

    let mut total_inserted = 0;
    let mut total_updated = 0;

    // Insert new records in batches
    const BATCH_SIZE: usize = 10_000;
    if !records_to_insert.is_empty() {
      info!("Inserting {} new price records", records_to_insert.len());
      for chunk in records_to_insert.chunks(BATCH_SIZE) {
        let inserted =
          diesel::insert_into(summaryprices::table).values(chunk).execute(&mut conn)?;

        total_inserted += inserted;
        progress.inc(chunk.len() as u64);
      }
    }

    // Update existing records - use the composite primary key for precise updates
    if !records_to_update.is_empty() {
      info!("Updating {} existing price records", records_to_update.len());
      for record in records_to_update {
        // Use the composite primary key (tstamp, sid, eventid) for precise update
        let updated = diesel::update(summaryprices::table)
          .filter(summaryprices::tstamp.eq(record.tstamp))
          .filter(summaryprices::sid.eq(record.sid))
          .filter(summaryprices::eventid.eq(record.eventid))
          .set((
            summaryprices::open.eq(record.open),
            summaryprices::high.eq(record.high),
            summaryprices::low.eq(record.low),
            summaryprices::close.eq(record.close),
            summaryprices::volume.eq(record.volume),
          ))
          .execute(&mut conn)?;

        if updated > 1 {
          warn!("Updated {} records for single price point - this shouldn't happen!", updated);
        }
        total_updated += updated;
        progress.inc(1);
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

    // Update symbols table to mark summary data as loaded
    if update_symbols && !unique_sids.is_empty() {
      info!("Updating {} symbols to mark summary data as loaded", unique_sids.len());

      diesel::update(symbols::table)
        .filter(symbols::sid.eq_any(&unique_sids))
        .set((symbols::summary.eq(true), symbols::m_time.eq(diesel::dsl::now)))
        .execute(&mut conn)?;
    }

    Ok(total_inserted + total_updated)
  })
  .await?
}

/// Main execute function
pub async fn execute(args: DailyArgs, config: Config) -> Result<()> {
  info!("Starting daily price loader");

  // Clean up expired cache entries periodically
  if !args.dry_run {
    match SummaryPriceLoader::cleanup_expired_cache(&config.database_url).await {
      Ok(deleted) if deleted > 0 => info!("Cleaned up {} expired cache entries", deleted),
      Err(e) => warn!("Failed to cleanup expired cache: {}", e),
      _ => {}
    }
  }

  if args.dry_run {
    info!("DRY RUN MODE - No database updates will be performed");
  }

  // Validate output size
  if args.outputsize != "compact" && args.outputsize != "full" {
    return Err(anyhow::anyhow!(
      "Invalid output size '{}'. Must be 'compact' or 'full'",
      args.outputsize
    ));
  }

  // Get symbols to load
  let symbols = get_symbols_to_load(&args, &config).await?;

  if symbols.is_empty() {
    warn!("No symbols to load. Ensure symbols have sec_type='Equity' and overview=true");
    return Ok(());
  }

  info!("Loading daily prices for {} equity symbols with overview data", symbols.len());
  info!(
    "Configuration: output_size={}, concurrent={}, api_delay={}ms",
    args.outputsize, args.concurrent, args.api_delay
  );

  // Calculate estimated time
  if args.api_delay > 0 {
    let estimated_seconds =
      (symbols.len() as u64 * args.api_delay) / 1000 / args.concurrent.max(1) as u64;
    let hours = estimated_seconds / 3600;
    let minutes = (estimated_seconds % 3600) / 60;
    info!("Estimated time: {}h {}m (assuming no failures)", hours, minutes);
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
  let loader_cfg = SummaryPriceConfig {
    max_concurrent: args.concurrent,
    update_existing: args.update,
    skip_non_trading_days: true,
    api_delay_ms: args.api_delay,
    enable_cache: !args.force_refresh,
    cache_ttl_hours: 24,
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
  };

  let loader = SummaryPriceLoader::new(args.concurrent)
    .with_config(loader_cfg)
    .with_starting_eventid(max_eventid + 1);

  // Prepare input
  let input = SummaryPriceLoaderInput { symbols, outputsize: args.outputsize };

  // Execute the loader
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load daily prices: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      // Return empty output if continuing on error
      return Ok(());
    }
  };

  // Display summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║          DAILY PRICE LOADING SUMMARY       ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Symbols Loaded:     {:<22} ║", output.symbols_loaded);
  println!("║ Symbols Failed:     {:<22} ║", output.symbols_failed);
  println!("║ Symbols Skipped:    {:<22} ║", output.symbols_skipped);
  println!("║ Total Records:      {:<22} ║", output.data.len());
  println!("╚════════════════════════════════════════════╝");

  // Save to database unless dry run
  if !args.dry_run && !output.data.is_empty() {
    info!("Saving {} price records to database", output.data.len());

    let saved = save_summary_prices(output.data, &config, true).await?;

    info!("Successfully saved {} price records", saved);
  } else if args.dry_run {
    info!("Dry run complete - would have saved {} records", output.data.len());
  }

  // Complete process tracking
  if let Some(tracker) = context.process_tracker {
    let state = if output.symbols_failed > 0 {
      ProcessState::CompletedWithErrors
    } else {
      ProcessState::Success
    };
    tracker.complete(state).await?;
  }

  Ok(())
}

#[cfg(test)]
mod tests {

  #[test]
  fn test_outputsize_validation() {
    assert!(["compact", "full"].contains(&"compact"));
    assert!(["compact", "full"].contains(&"full"));
    assert!(!["compact", "full"].contains(&"invalid"));
  }
}
