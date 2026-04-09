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

//! Top market movers loader for `av-cli load top-movers`.
//!
//! Fetches the daily top gainers, top losers, and most actively traded
//! securities from the AlphaVantage `TOP_GAINERS_LOSERS` endpoint and
//! persists them to the database. Also tracks any tickers in the response
//! that don't yet exist in the local `symbols` table by recording them in
//! `missing_symbols` for later resolution by [`super::missing_symbols`].
//!
//! ## What This Loads
//!
//! AlphaVantage's `TOP_GAINERS_LOSERS` returns three lists for the most
//! recent trading day (or a specified date):
//!
//! - **Top gainers** — Securities with the largest positive price change
//!   percentage.
//! - **Top losers** — Securities with the largest negative price change
//!   percentage.
//! - **Most actively traded** — Securities with the highest trading volume.
//!
//! Each entry includes the ticker, current price, change percentage, and
//! volume. All three lists are returned in a single API call.
//!
//! ## Data Flow
//!
//! ```text
//! AlphaVantage TOP_GAINERS_LOSERS endpoint
//!   │  (single API call, optionally cached for 24h)
//!   ▼
//! TopMoversLoader::load()
//!   │
//!   ▼
//! TopMoversLoaderOutput
//!   ├── raw_data.top_gainers          → top movers tables
//!   ├── raw_data.top_losers           → top movers tables
//!   ├── raw_data.most_actively_traded → top movers tables
//!   └── missing_symbols (tickers not in symbols table)
//!                                     → missing_symbols table
//! ```
//!
//! ## Single API Call
//!
//! Unlike most other loaders, this one makes a **single API call** that
//! returns all three lists at once. As a result:
//!
//! - `max_concurrent_requests = 1` (no parallelism needed)
//! - `show_progress = false` (no progress bar for one call)
//! - The output displays a formatted ASCII summary with optional verbose
//!   mode showing the top 5 in each category.
//!
//! ## Caching
//!
//! The response is cached with a default TTL of **24 hours**, which is
//! appropriate because the daily top-movers report doesn't change after
//! market close. The `--cache-ttl` flag overrides this; `--no-cache` disables
//! caching entirely; `--force-refresh` bypasses the cache for a single call
//! while still writing the new response into the cache.
//!
//! ## Missing Symbol Tracking
//!
//! When a ticker in the API response doesn't exist in the local `symbols`
//! table, it's recorded in `missing_symbols` (via the loader's
//! `track_missing_symbols = true` flag) so that [`super::missing_symbols`]
//! can resolve it later via `SYMBOL_SEARCH`. The output reports both the
//! count of missing tickers and how many were successfully recorded.
//!
//! ## Usage
//!
//! ```bash
//! # Fetch today's top movers
//! av-cli load top-movers
//!
//! # Specific date with verbose output
//! av-cli load top-movers --date 2026-04-08 --verbose
//!
//! # Force refresh, bypassing the 24h cache
//! av-cli load top-movers --force-refresh
//!
//! # Dry run (no database writes, no missing-symbol tracking)
//! av-cli load top-movers --dry-run --verbose
//! ```

use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use clap::Args;
use std::sync::Arc;

use av_client::AlphaVantageClient;
use av_database_postgres::repository::DatabaseContext;
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, ProcessTracker,
  top_movers_loader::{TopMoversConfig, TopMoversLoader, TopMoversLoaderInput},
};

use crate::config::Config;

/// Command-line arguments for `av-cli load top-movers`.
///
/// Controls date selection, caching, dry-run, and verbose output behavior.
#[derive(Args, Debug)]
pub struct TopMoversArgs {
  /// Specific date to fetch top movers for, in `YYYY-MM-DD` format.
  ///
  /// When omitted, AlphaVantage returns the most recent trading day's data.
  /// Invalid dates (failed parse) silently fall back to "most recent" rather
  /// than erroring.
  #[arg(short, long)]
  date: Option<String>,

  /// Fetch the data but skip database writes.
  ///
  /// In dry-run mode, missing-symbol tracking and process tracking are also
  /// disabled (no `news_repository` or `process_tracker` is attached to the
  /// loader context). The cache is still consulted for reads.
  #[arg(long)]
  dry_run: bool,

  /// Print the top 5 entries from each list (gainers/losers/active) and
  /// the full missing-symbol list.
  ///
  /// Without this flag, only counts are shown in the summary box.
  #[arg(short = 'v', long)]
  verbose: bool,

  /// Disable response caching entirely.
  #[arg(long)]
  no_cache: bool,

  /// Bypass the cache and fetch fresh data, but continue to write the new
  /// response into the cache.
  #[arg(long)]
  force_refresh: bool,

  /// Cache TTL in hours. Defaults to 24.
  ///
  /// Top-movers data is computed daily after market close, so a 24-hour TTL
  /// is appropriate.
  #[arg(long, default_value = "24")]
  cache_ttl: i64,
}

/// Main entry point for `av-cli load top-movers`.
///
/// Orchestrates the top-movers loading pipeline:
///
/// 1. **API client setup** — Creates [`AlphaVantageClient`] from the
///    [`Config`].
/// 2. **Loader context** — Creates [`LoaderContext`] with
///    `max_concurrent_requests = 1` (single API call) and `show_progress = false`.
/// 3. **Repository attachment** — Unless `--dry-run`:
///    - News repository (used for missing-symbol tracking).
///    - Process tracker for monitoring.
///    Cache repository is attached unless `--no-cache`, even in dry-run mode
///    (so cached responses can be read).
/// 4. **Loader configuration** — Builds [`TopMoversConfig`] with
///    `track_missing_symbols = !dry_run` and the cache settings.
/// 5. **Date parsing** — Parses `--date` as `YYYY-MM-DD`; invalid dates
///    silently fall back to `None` (most recent).
/// 6. **API call** — Calls [`TopMoversLoader::load`] which fetches the data,
///    saves it to the top-movers tables (unless dry-run), and tracks missing
///    symbols.
/// 7. **Display** — Prints a formatted ASCII summary box with the date,
///    last-updated timestamp, counts for each list, optional verbose top-5
///    listings, cache vs. API source indicator, and (if not dry-run)
///    database update statistics.
///
/// # Errors
///
/// Returns errors from: API client creation, database context creation, or
/// loader execution.
pub async fn execute(args: TopMoversArgs, config: Config) -> Result<()> {
  // Create API client with the correct Config type
  let client = Arc::new(
    AlphaVantageClient::new(config.api_config)
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: 1, // Top movers is a single API call
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: false,         // Single call, no need for progress
    track_process: !args.dry_run, // Track process unless dry run
    batch_size: 1000,
  };

  // Create loader context
  let mut context = LoaderContext::new(client, loader_config);

  // Setup database context and repositories
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;

  if !args.dry_run {
    let news_repo: Arc<dyn av_database_postgres::repository::NewsRepository> =
      Arc::new(db_context.news_repository());
    context = context.with_news_repository(news_repo);

    // Setup process tracker
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Setup cache repository (even for dry run, to enable cache reads)
  if !args.no_cache {
    let cache_repo: Arc<dyn av_database_postgres::repository::CacheRepository> =
      Arc::new(db_context.cache_repository());
    context = context.with_cache_repository(cache_repo);
  }

  // Create loader configuration
  let loader_config = TopMoversConfig {
    track_missing_symbols: !args.dry_run,
    enable_cache: !args.no_cache,
    cache_ttl_hours: args.cache_ttl,
    force_refresh: args.force_refresh,
  };

  // Setup loader with database URL (None only for dry run)
  let database_url = if args.dry_run { None } else { Some(config.database_url.clone()) };
  let loader = TopMoversLoader::new(loader_config, database_url);

  // Parse date if provided
  let date = args.date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

  let input = TopMoversLoaderInput { date };

  let output = loader.load(&context, input).await?;

  // Display results
  println!("\n╔════════════════════════════════════════╗");
  println!("║       TOP MARKET MOVERS                ║");
  println!("╠════════════════════════════════════════╣");
  println!("║ Date: {:<33} ║", output.date);
  println!("║ Last Updated: {:<24} ║", output.last_updated);
  println!("╚════════════════════════════════════════╝\n");

  println!("📈 Top {} Gainers", output.gainers_count);
  if args.verbose {
    for gainer in output.raw_data.top_gainers.iter().take(5) {
      println!("   {} | ${} | {}% ↑", gainer.ticker, gainer.price, gainer.change_percentage);
    }
    println!();
  }

  println!("📉 Top {} Losers", output.losers_count);
  if args.verbose {
    for loser in output.raw_data.top_losers.iter().take(5) {
      println!("   {} | ${} | {}% ↓", loser.ticker, loser.price, loser.change_percentage);
    }
    println!();
  }

  println!("📊 Top {} Most Active", output.most_active_count);
  if args.verbose {
    for active in output.raw_data.most_actively_traded.iter().take(5) {
      println!("   {} | ${} | Vol: {}", active.ticker, active.price, active.volume);
    }
    println!();
  }

  // Show cache status
  if output.from_cache {
    println!("\n📦 Data Source: Cache (TTL: {} hours)", args.cache_ttl);
  } else {
    println!("\n🌐 Data Source: API (fresh data)");
  }

  if args.dry_run {
    println!("\n⚠️  Dry run mode - no data saved to database");
  } else {
    println!("\n✅ Database Update:");
    println!("   Records saved: {}", output.records_saved);
    if !output.missing_symbols.is_empty() {
      println!("   ⚠️  Missing symbols: {}", output.missing_symbols.len());
      println!("   📝 Missing symbols recorded: {}", output.missing_recorded);
      if args.verbose {
        println!("   Missing symbols:");
        for symbol in &output.missing_symbols {
          println!("      - {}", symbol);
        }
      }
    }
  }

  Ok(())
}
