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

//! Equity securities loader for `av-cli load securities`.
//!
//! This is the **bootstrap loader** for the equity portion of the database.
//! It reads NASDAQ and NYSE symbol listings from CSV files (downloaded from
//! the official exchange feeds), enriches each symbol via the AlphaVantage
//! `SYMBOL_SEARCH` endpoint, generates Security IDs (SIDs) via [`SidGenerator`],
//! and persists the results to `symbols` and `equity_details`.
//!
//! ## Data Flow
//!
//! ```text
//! NASDAQ CSV (nasdaqlisted.txt) ──┐
//!                                  │
//! NYSE CSV (otherlisted.txt) ─────┤
//!                                  ▼
//! SecurityLoader::load()  ── AlphaVantage SYMBOL_SEARCH (per symbol) + cache
//!   │  applies match_mode (Exact / All / Top N)
//!   ▼
//! Vec<SecurityData>
//!   │  combined from both exchanges, processed in a single transaction
//!   ▼
//! save_symbols_to_db()
//!   ├── normalize_alpha_region()  ── e.g., "United States" → "USA"
//!   ├── parse market hours / timezone (with defaults: 09:30 / 16:00 / US/Eastern)
//!   ├── SidGenerator::next_sid(security_type)
//!   ├── INSERT INTO symbols (or UPDATE if same symbol+region)
//!   └── INSERT INTO equity_details (skipped for Cryptocurrency type)
//! ```
//!
//! ## Symbol Matching Modes
//!
//! AlphaVantage's `SYMBOL_SEARCH` endpoint may return multiple matches per
//! query. The [`MatchMode`] enum controls which matches are accepted:
//!
//! | Mode    | Description                                              |
//! |---------|----------------------------------------------------------|
//! | `Exact` | Only the result whose `symbol` exactly equals the query  |
//! | `All`   | Every result returned by the API                         |
//! | `Top`   | The top N results by `match_score` (`--top-matches=3`)   |
//!
//! `All` is the default and produces the broadest coverage.
//!
//! ## Region Normalization
//!
//! AlphaVantage returns full region names (e.g., `"United States"`,
//! `"Toronto Venture"`) but the database `region` column is `VARCHAR(10)`.
//! [`normalize_alpha_region`] maps the most common full names to short codes
//! (e.g., `"USA"`, `"TOR"`, `"Bomb"`) and truncates anything longer than 10
//! characters with a warning.
//!
//! This function is also re-exported through [`super::missing_symbols`] and
//! used during the missing-symbol resolution pass.
//!
//! ## Insert vs. Update Semantics
//!
//! For each symbol from the loader, [`save_symbols_to_db`] checks for an
//! existing row by `symbol` and decides:
//!
//! - **Existing, same region** — Updates `name`, `currency`, and `m_time`.
//!   Diesel returns `0` rows affected if nothing actually changed, in which
//!   case the symbol is counted as "skipped" rather than "updated".
//! - **Existing, different region** — Logs a warning and skips (we don't
//!   want to overwrite a symbol's region).
//! - **New** — Generates a new SID, validates field lengths against the
//!   schema (`symbol ≤ 20`, `region ≤ 10`, `currency ≤ 10`), inserts into
//!   `symbols`, then inserts into `equity_details` (unless the type is
//!   `Cryptocurrency`).
//!
//! ## Equity Details
//!
//! Non-cryptocurrency securities also get an `equity_details` row capturing
//! exchange, market open/close times, and timezone. Market hours default to
//! `09:30`-`16:00` and timezone defaults to `US/Eastern` (or the [`Exchange`]
//! enum's known timezone) if the API doesn't supply them.
//!
//! ## Caching
//!
//! Symbol search responses are cached via [`CacheRepository`] with a default
//! TTL of **168 hours** (1 week) — the symbol metadata changes infrequently
//! enough that aggressive caching is safe and dramatically reduces API costs
//! on repeated runs.
//!
//! ## Usage
//!
//! ```bash
//! # Bootstrap with default CSV paths from config
//! av-cli load securities
//!
//! # Custom CSV paths and exact-match mode
//! av-cli load securities --nasdaq-csv ./nasdaq.txt --nyse-csv ./nyse.txt \
//!   --match-mode exact
//!
//! # Top 3 matches per query, higher concurrency
//! av-cli load securities --match-mode top --top-matches 3 --concurrent 10
//!
//! # Dry run to verify CSVs and API connectivity
//! av-cli load securities --dry-run
//!
//! # Force refresh, ignoring the 1-week cache
//! av-cli load securities --force-refresh
//! ```

use super::sid_generator::SidGenerator;
use anyhow::{Result, anyhow};
use av_client::AlphaVantageClient;
use av_core::types::market::{Exchange, SecurityType};
use av_database_postgres::repository::DatabaseContext;
use av_loaders::SecurityLoaderConfig;
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, SecurityLoader, SecurityLoaderInput, SymbolMatchMode,
  process_tracker::ProcessTracker,
};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

// Import diesel types
use diesel::PgConnection;
use diesel::prelude::*;

use crate::config::Config;

/// Command-line arguments for `av-cli load securities`.
///
/// Controls CSV file paths, concurrency, symbol-matching strategy, caching,
/// and dry-run/error behavior.
#[derive(Args, Debug)]
pub struct SecuritiesArgs {
  /// Path to the NASDAQ-listed symbols CSV file (`nasdaqlisted.txt`).
  ///
  /// When omitted, falls back to `config.nasdaq_csv_path`. Can also be set
  /// via the `NASDAQ_LISTED` environment variable. The file is silently
  /// skipped (with a warning) if it doesn't exist.
  #[arg(long, env = "NASDAQ_LISTED")]
  nasdaq_csv: Option<String>,

  /// Path to the NYSE / other-listed symbols CSV file (`otherlisted.txt`).
  ///
  /// When omitted, falls back to `config.nyse_csv_path`. Can also be set
  /// via the `OTHER_LISTED` environment variable.
  #[arg(long, env = "OTHER_LISTED")]
  nyse_csv: Option<String>,

  /// Maximum number of concurrent API requests. Defaults to 5.
  #[arg(short, long, default_value = "5")]
  concurrent: usize,

  /// Fetch from the API but skip database writes.
  ///
  /// Routes to [`execute_dry_run`] which uses an empty cache and reports
  /// per-exchange counts.
  #[arg(short, long)]
  dry_run: bool,

  /// Continue processing remaining CSVs/symbols when one fails.
  #[arg(short = 'k', long)]
  continue_on_error: bool,

  /// Symbol matching strategy. Defaults to `all`.
  ///
  /// See the [`MatchMode`] enum for the three options.
  #[arg(long, value_enum, default_value = "all")]
  match_mode: MatchMode,

  /// Number of top matches to accept when `--match-mode=top` is set. Defaults to 3.
  #[arg(long, default_value = "3")]
  top_matches: usize,

  /// Disable response caching entirely.
  #[arg(long)]
  no_cache: bool,

  /// Bypass the cache and fetch fresh data, but continue to write new
  /// responses into the cache.
  #[arg(long)]
  force_refresh: bool,

  /// Cache TTL in hours. Defaults to 168 (1 week).
  ///
  /// Symbol metadata changes infrequently so aggressive caching is safe.
  #[arg(long, default_value = "168")]
  cache_ttl_hours: i64,
}

/// CLI-level enum for symbol-matching strategy.
///
/// Controls how the loader handles multiple results from AlphaVantage's
/// `SYMBOL_SEARCH` endpoint. Maps to [`SymbolMatchMode`] in `av_loaders`.
#[derive(Debug, Clone, clap::ValueEnum)]
enum MatchMode {
  /// Only accept the result whose `symbol` field exactly matches the query.
  Exact,
  /// Accept every result returned by the API (broadest coverage).
  All,
  /// Accept the top N results sorted by `match_score` (N from `--top-matches`).
  Top,
}

/// Normalizes a region name from AlphaVantage to the abbreviated form used
/// in the database `region` column.
///
/// AlphaVantage's `SYMBOL_SEARCH` endpoint returns full region names (e.g.,
/// `"United States"`, `"Toronto Venture"`, `"India/Bombay"`) which are too
/// long for the `VARCHAR(10)` `region` column. This function maps the most
/// common full names to short codes:
///
/// | Input                                  | Output  |
/// |----------------------------------------|---------|
/// | `"United States"`                       | `"USA"`   |
/// | `"United Kingdom"`                      | `"UK"`    |
/// | `"Toronto"`, `"Toronto Venture"`        | `"TOR"`   |
/// | `"India"`, `"India/Bombay"`, `"Bombay"` | `"Bomb"`  |
/// | `"Brazil"`, `"Sao Paolo"`, etc.        | `"SaoP"`  |
/// | (many more...)                          |         |
///
/// Unknown regions are returned unchanged. Any result longer than 10
/// characters is truncated with a warning so it fits the database column.
///
/// This function is also used by [`super::missing_symbols`] when resolving
/// pending symbols via `SYMBOL_SEARCH`, so changes here affect both bootstrap
/// and resolution paths.
pub fn normalize_alpha_region(region: &str) -> String {
  let normalized = match region {
    "United States" => "USA",
    "United Kingdom" => "UK",
    "Frankfurt" => "Frank",
    "Toronto" | "Toronto Venture" => "TOR",
    "India/Bombay" | "India" | "Bombay" => "Bomb",
    "Brazil/Sao Paolo" | "Brazil" | "Sao Paolo" => "SaoP",
    "Amsterdam" => "AMS",
    "XETRA" => "XETRA",
    "Shanghai" => "SH",
    "Hong Kong" => "HK",
    "Tokyo" => "TYO",
    "London" => "LON",
    "Paris" => "PAR",
    "Singapore" => "SG",
    "Sydney" => "SYD",
    "Mexico" => "MEX",
    "Canada" => "CAN",
    "Germany" => "DE",
    "Switzerland" => "CH",
    "Japan" => "JP",
    "Australia" => "AU",
    "Netherlands" => "NL",
    _ => region,
  };

  // Ensure the result fits in VARCHAR(10)
  if normalized.len() > 10 {
    warn!("Region '{}' exceeds 10 characters, truncating to '{}'", normalized, &normalized[..10]);
    normalized[..10].to_string()
  } else {
    normalized.to_string()
  }
}

/// Main entry point for `av-cli load securities`.
///
/// Orchestrates the bootstrap pipeline for loading equity securities from
/// NASDAQ and NYSE CSV files:
///
/// 1. **Dry-run check** — If `--dry-run`, delegates to [`execute_dry_run`]
///    and returns.
/// 2. **Infrastructure setup** — Creates [`AlphaVantageClient`],
///    [`DatabaseContext`], cache repository, and [`LoaderContext`] with
///    process tracking enabled.
/// 3. **Loader configuration** — Builds [`SecurityLoader`] with the selected
///    [`SymbolMatchMode`] (Exact / All / TopMatches) and a
///    [`SecurityLoaderConfig`] for caching (default TTL 168h = 1 week).
/// 4. **NASDAQ processing** — If the NASDAQ CSV exists, calls
///    [`SecurityLoader::load`] for `exchange = "NASDAQ"` and collects results.
/// 5. **NYSE processing** — Same as above for `exchange = "NYSE"`.
/// 6. **Persistence** — All collected securities are saved in a single
///    `spawn_blocking` task that initializes a [`SidGenerator`] and calls
///    [`save_symbols_to_db`].
/// 7. **Process tracker completion** — Marks the run as `Success` or
///    `CompletedWithErrors` based on whether any symbols were saved.
///
/// # Errors
///
/// Returns errors from: API client creation, database context creation,
/// loader execution (unless `--continue-on-error`), SID generator init, or
/// the save operation.
pub async fn execute(args: SecuritiesArgs, config: Config) -> Result<()> {
  info!("Starting security symbol loader");

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    return execute_dry_run(args, config).await;
  }

  // Create API client
  let client = Arc::new(
    AlphaVantageClient::new(config.api_config)
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: true,
    batch_size: 100,
  };

  // Create database context and cache repository
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;
  let cache_repo = Arc::new(db_context.cache_repository());

  // Create loader context with cache repository
  let mut context = LoaderContext::new(client, loader_config);
  context = context.with_cache_repository(cache_repo);

  // Set up process tracking
  let tracker = ProcessTracker::new();
  context = context.with_process_tracker(tracker);

  // Create security loader with match mode
  let match_mode = match args.match_mode {
    MatchMode::Exact => SymbolMatchMode::ExactMatch,
    MatchMode::All => SymbolMatchMode::AllMatches,
    MatchMode::Top => SymbolMatchMode::TopMatches(args.top_matches),
  };
  let security_config = SecurityLoaderConfig {
    enable_cache: !args.no_cache,
    cache_ttl_hours: args.cache_ttl_hours,
    force_refresh: args.force_refresh,
  };

  let loader =
    SecurityLoader::new(args.concurrent).with_match_mode(match_mode).with_config(security_config);

  // Collect all securities first, then save in one transaction
  let mut all_securities = Vec::new();

  // Process NASDAQ file
  let nasdaq_path = args.nasdaq_csv.unwrap_or(config.nasdaq_csv_path);
  if std::path::Path::new(&nasdaq_path).exists() {
    info!("Loading NASDAQ symbols from: {}", nasdaq_path);

    let input = SecurityLoaderInput { file_path: nasdaq_path, exchange: "NASDAQ".to_string() };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
          "NASDAQ API calls complete: {} loaded, {} errors, {} skipped, {} cache hits, {} API calls",
          output.loaded_count,
          output.errors,
          output.skipped_count,
          output.cache_hits,
          output.api_calls
        );

        // Collect securities for later saving
        all_securities.extend(output.data);
      }
      Err(e) => {
        error!("Failed to load NASDAQ securities: {}", e);
        if !args.continue_on_error {
          return Err(e.into());
        }
      }
    }
  } else {
    warn!("NASDAQ CSV file not found: {}", nasdaq_path);
  }

  // Process NYSE file
  let nyse_path = args.nyse_csv.unwrap_or(config.nyse_csv_path);
  if std::path::Path::new(&nyse_path).exists() {
    info!("Loading NYSE symbols from: {}", nyse_path);

    let input = SecurityLoaderInput { file_path: nyse_path, exchange: "NYSE".to_string() };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
          "NYSE API calls complete: {} loaded, {} errors, {} skipped, {} cache hits, {} API calls",
          output.loaded_count,
          output.errors,
          output.skipped_count,
          output.cache_hits,
          output.api_calls
        );
        // Collect securities for later saving
        all_securities.extend(output.data);
      }
      Err(e) => {
        error!("Failed to load NYSE securities: {}", e);
        if !args.continue_on_error {
          return Err(e.into());
        }
      }
    }
  } else {
    warn!("NYSE CSV file not found: {}", nyse_path);
  }

  // Save all securities in one blocking operation
  let total_loaded = if !all_securities.is_empty() {
    let db_url = config.database_url.clone();

    tokio::task::spawn_blocking(move || -> Result<usize> {
      // Establish connection in the blocking context
      let mut conn = PgConnection::establish(&db_url)
        .map_err(|e| anyhow::anyhow!("Error connecting to database: {}", e))?;

      // Initialize SID generator
      let mut sid_generator = SidGenerator::new(&mut conn)?;

      // Save all symbols
      save_symbols_to_db(&mut conn, &all_securities, &mut sid_generator)
    })
    .await??
  } else {
    0
  };

  // Complete process tracking
  if let Some(tracker) = &context.process_tracker {
    let state = if total_loaded == 0 {
      av_loaders::process_tracker::ProcessState::CompletedWithErrors
    } else {
      av_loaders::process_tracker::ProcessState::Success
    };
    tracker.complete(state).await?;
  }

  info!("Symbol loading completed: {} symbols saved", total_loaded);
  Ok(())
}

/// Dry-run pipeline that fetches from the API but doesn't touch the database.
///
/// Differences from [`execute`]:
///
/// - **No database context** — Skips `DatabaseContext`, cache repository, and
///   process tracker creation.
/// - **Cache disabled** — `enable_cache = false` because no cache repository
///   is attached to the loader context.
/// - **Per-exchange counters** — Tracks `total_loaded`, `total_errors`, and
///   `total_skipped` across both files and prints a final summary.
///
/// Useful for verifying CSV file paths and AlphaVantage API connectivity
/// before running a full bootstrap.
async fn execute_dry_run(args: SecuritiesArgs, config: Config) -> Result<()> {
  let client = Arc::new(
    AlphaVantageClient::new(config.api_config)
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );

  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: false,
    batch_size: 100,
  };

  let context = LoaderContext::new(client, loader_config);

  let match_mode = match args.match_mode {
    MatchMode::Exact => SymbolMatchMode::ExactMatch,
    MatchMode::All => SymbolMatchMode::AllMatches,
    MatchMode::Top => SymbolMatchMode::TopMatches(args.top_matches),
  };
  let security_config = SecurityLoaderConfig {
    enable_cache: false, // Disable for dry run (no cache repository provided)
    cache_ttl_hours: 168,
    force_refresh: false,
  };

  let loader =
    SecurityLoader::new(args.concurrent).with_match_mode(match_mode).with_config(security_config);
  let mut total_loaded = 0;
  let mut total_errors = 0;
  let mut total_skipped = 0;

  // Process files
  for (path, exchange) in [
    (args.nasdaq_csv.unwrap_or(config.nasdaq_csv_path), "NASDAQ"),
    (args.nyse_csv.unwrap_or(config.nyse_csv_path), "NYSE"),
  ] {
    if std::path::Path::new(&path).exists() {
      info!("Loading {} symbols from: {}", exchange, path);

      let input = SecurityLoaderInput { file_path: path, exchange: exchange.to_string() };

      match loader.load(&context, input).await {
        Ok(output) => {
          info!(
            "API calls complete (DRY RUN): {} loaded, {} errors, {} skipped, {} cache hits, {} API calls",
            output.loaded_count,
            output.errors,
            output.skipped_count,
            output.cache_hits,
            output.api_calls
          );

          total_loaded += output.loaded_count;
          total_errors += output.errors;
          total_skipped += output.skipped_count;
        }
        Err(e) => {
          error!("Failed to load {} securities: {}", exchange, e);
          if !args.continue_on_error {
            return Err(e.into());
          }
        }
      }
    }
  }

  info!(
    "Dry run completed: {} symbols found, {} errors, {} skipped",
    total_loaded, total_errors, total_skipped
  );
  Ok(())
}

/// Persists fetched security data to the `symbols` and `equity_details` tables.
///
/// Iterates over each [`SecurityData`](av_loaders::SecurityData) and processes
/// it through the following decision flow:
///
/// 1. **Skip empty/invalid** — Empty or `"None"` symbols are skipped immediately.
/// 2. **In-batch deduplication** — Tracks uppercased symbols seen in this run
///    via a [`HashMap`] and skips intra-batch duplicates.
/// 3. **Type mapping** — Converts AlphaVantage's `stock_type` string to
///    [`SecurityType`] via [`SecurityType::from_alpha_vantage`]. Unknown
///    types map to `Other` with a warning.
/// 4. **Market hours parsing** — Parses `market_open` / `market_close` as
///    `HH:MM`, falling back to `09:30` / `16:00` if parsing fails.
/// 5. **Timezone resolution** — Uses the API-provided timezone, or falls
///    back to the [`Exchange`] enum's known timezone, or `US/Eastern`.
/// 6. **Region normalization** — Calls [`normalize_alpha_region`].
/// 7. **Existence check** — Looks up the symbol in `symbols`. Three branches:
///    - **Same region** — `UPDATE` name/currency/m_time (counts as `updated` if
///      Diesel reports >0 rows affected, otherwise `skipped`).
///    - **Different region** — Logs a warning and skips (preserves existing
///      region rather than overwriting).
///    - **Not found** — Generates a new SID via `sid_generator.next_sid()`,
///      truncates `name` to 255 chars, validates field lengths against
///      `VARCHAR(20)/(10)/(10)` constraints, inserts into `symbols`, and
///      (for non-cryptocurrency types) inserts into `equity_details` with
///      market hours and timezone.
/// 8. **Unique violation handling** — If the insert hits a unique constraint
///    (concurrent insert), it's logged and counted as skipped, not failed.
///
/// Tracks four counters: `saved` (new inserts), `updated` (existing rows
/// modified), `skipped` (no-op updates, duplicates, region conflicts,
/// unique violations), and `failed` (validation errors, database errors).
/// Returns `saved + updated` as the total successful operation count.
fn save_symbols_to_db(
  conn: &mut PgConnection,
  securities: &[av_loaders::SecurityData],
  sid_generator: &mut SidGenerator,
) -> Result<usize> {
  use av_database_postgres::models::security::{NewEquityDetailOwned, NewSymbol};
  use av_database_postgres::schema::{equity_details, symbols};
  use diesel::result::DatabaseErrorKind;

  let progress = ProgressBar::new(securities.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .expect("Invalid progress bar template - this is a programming error")
      .progress_chars("##-"),
  );
  progress.set_message("Saving symbols to database");

  let mut saved_count = 0;
  let mut updated_count = 0;
  let mut failed_count = 0;
  let mut skipped_count = 0;
  let mut symbol_map = HashMap::new();

  // Process each symbol individually
  for security_data in securities {
    // Only save if we got valid data from the API
    if security_data.symbol.is_empty() || security_data.symbol == "None" {
      skipped_count += 1;
      continue;
    }

    // Check for duplicates within this batch
    if symbol_map.contains_key(&security_data.symbol.to_uppercase()) {
      debug!("Duplicate symbol {} found in batch, skipping", security_data.symbol);
      skipped_count += 1;
      continue;
    }
    symbol_map.insert(security_data.symbol.to_uppercase(), true);

    // Log if the matched symbol differs from original query
    if let Some(original) = &security_data.original_query {
      if !original.eq_ignore_ascii_case(&security_data.symbol) {
        info!(
          "Processing symbol {} (from search: {}, score: {:?})",
          security_data.symbol, original, security_data.match_score
        );
      }
    }

    // Use the av-core mapping function for security type
    let security_type = SecurityType::from_alpha_vantage(&security_data.stock_type);

    if security_type == SecurityType::Other {
      warn!(
        "Unknown asset type '{}' for symbol {}, mapping to Other",
        security_data.stock_type, security_data.symbol
      );
    }

    // Parse market hours from the security data
    let market_open = chrono::NaiveTime::parse_from_str(&security_data.market_open, "%H:%M")
      .unwrap_or_else(|_| {
        chrono::NaiveTime::parse_from_str("09:30", "%H:%M")
          .expect("Default market open time '09:30' should always parse")
      });
    let market_close = chrono::NaiveTime::parse_from_str(&security_data.market_close, "%H:%M")
      .unwrap_or_else(|_| {
        chrono::NaiveTime::parse_from_str("16:00", "%H:%M")
          .expect("Default market close time '16:00' should always parse")
      });

    // Use the timezone from the security data or fall back to Exchange lookup
    let timezone = if !security_data.timezone.is_empty() {
      security_data.timezone.clone()
    } else {
      security_data
        .exchange
        .parse::<Exchange>()
        .map(|ex| ex.timezone().to_string())
        .unwrap_or_else(|_| "US/Eastern".to_string())
    };

    // Normalize the region before saving with enhanced mapping
    let normalized_region = normalize_alpha_region(&security_data.region);

    // Check if THIS EXACT symbol already exists
    let existing_result = symbols::table
      .filter(symbols::symbol.eq(&security_data.symbol))
      .select((symbols::sid, symbols::region))
      .first::<(i64, String)>(conn)
      .optional();

    match existing_result {
      Ok(Some((sid_val, existing_region))) => {
        // Symbol already exists in database
        debug!(
          "Symbol {} already exists with SID {} in region {}",
          security_data.symbol, sid_val, existing_region
        );

        // Only update if it's the same region, otherwise skip
        if existing_region == normalized_region {
          // Use Diesel's built-in change detection by comparing all fields
          // This will only execute if at least one field is different
          match diesel::update(symbols::table.find(sid_val))
            .set((
              symbols::name.eq(&security_data.name),
              symbols::currency.eq(&security_data.currency),
              symbols::m_time.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
          {
            Ok(rows_affected) => {
              if rows_affected > 0 {
                updated_count += 1;
                info!("Updated symbol {} (SID {}) - data changed", security_data.symbol, sid_val);
              } else {
                // No rows affected means no changes
                debug!(
                  "No changes for symbol {} (SID {}), skipped update",
                  security_data.symbol, sid_val
                );
                skipped_count += 1;
              }
            }
            Err(e) => {
              error!("Failed to update symbol {}: {}", security_data.symbol, e);
              failed_count += 1;
            }
          }
        } else {
          // Different region for same symbol - this is a problem
          warn!(
            "Symbol {} already exists with different region (existing: {}, new: {}). Skipping.",
            security_data.symbol, existing_region, normalized_region
          );
          skipped_count += 1;
        }
      }
      Ok(None) => {
        // New symbol, generate SID and insert
        let new_sid = sid_generator.next_sid(security_type);
        let now_t = chrono::Utc::now().naive_utc();

        // Truncate name if needed
        let truncated_name = if security_data.name.len() > 255 {
          warn!("Truncating name for {}: '{}'", security_data.symbol, security_data.name);
          security_data.name.chars().take(255).collect()
        } else {
          security_data.name.clone()
        };

        let new_symbol = NewSymbol {
          sid: &new_sid,
          symbol: &security_data.symbol,
          priority: &9999999,
          name: &truncated_name,
          sec_type: &format!("{:?}", security_type),
          region: &normalized_region,
          currency: &security_data.currency,
          overview: &false,
          intraday: &false,
          summary: &false,
          c_time: &now_t,
          m_time: &now_t,
        };

        // Validate before insert
        let mut validation_failed = false;
        if security_data.symbol.len() > 20 {
          error!("Symbol '{}' exceeds VARCHAR(20) limit!", security_data.symbol);
          validation_failed = true;
        }
        if normalized_region.len() > 10 {
          error!("Region '{}' exceeds VARCHAR(10) limit!", normalized_region);
          validation_failed = true;
        }
        if security_data.currency.len() > 10 {
          error!("Currency '{}' exceeds VARCHAR(10) limit!", security_data.currency);
          validation_failed = true;
        }

        if validation_failed {
          failed_count += 1;
          continue;
        }

        // Try to insert
        match diesel::insert_into(symbols::table).values(&new_symbol).execute(conn) {
          Ok(_) => {
            saved_count += 1;
            info!(
              "Saved new symbol {} with SID {} in region {}",
              security_data.symbol, new_sid, normalized_region
            );
          }
          Err(e) => {
            // Check if it's a unique constraint violation
            if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = e {
              warn!(
                "Symbol {} already exists (concurrent insert?), skipping",
                security_data.symbol
              );
              skipped_count += 1;
            } else {
              error!("Failed to insert symbol {}: {}", security_data.symbol, e);
              failed_count += 1;
            }
          }
        }
        if security_type != SecurityType::Cryptocurrency {
          let new_equity_detail = NewEquityDetailOwned {
            sid: new_sid,
            exchange: security_data.exchange.clone(),
            market_open,
            market_close,
            timezone: timezone.clone(),
            c_time: now_t,
            m_time: now_t,
          };

          match diesel::insert_into(equity_details::table).values(&new_equity_detail).execute(conn)
          {
            Ok(_) => {
              debug!("Created equity details for {} (SID {})", security_data.symbol, new_sid);
            }
            Err(e) => {
              error!("Failed to create equity details for {}: {}", security_data.symbol, e);
              // Optionally handle this error - maybe roll back the symbol insert?
            }
          }
        }
      }
      Err(e) => {
        error!("Database error checking symbol {}: {}", security_data.symbol, e);
        failed_count += 1;
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message(format!(
    "Completed: {} saved, {} updated, {} skipped, {} failed",
    saved_count, updated_count, skipped_count, failed_count
  ));

  if failed_count > 0 {
    warn!("Failed to process {} symbols", failed_count);
  }
  if skipped_count > 0 {
    info!("Skipped {} symbols (duplicates or no changes)", skipped_count);
  }

  // Return total successful operations
  Ok(saved_count + updated_count)
}
