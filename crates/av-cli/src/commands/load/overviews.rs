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

//! Equity company overview loader for `av-cli load overviews`.
//!
//! Fetches comprehensive company overview data (financials, ratios,
//! identifiers, and corporate metadata) from the AlphaVantage `OVERVIEW`
//! endpoint and persists it across two database tables: `overviews`
//! (core fields) and `overviewext` (extended financial metrics).
//!
//! ## Data Captured
//!
//! ### `overviews` (core fields)
//!
//! - Identification: `symbol`, `name`, `cik`, `exchange`, `currency`, `country`
//! - Classification: `sector`, `industry`, `address`, `fiscal_year_end`
//! - Description and `latest_quarter` date
//! - Headline financials: `market_capitalization`, `ebitda`, `pe_ratio`,
//!   `peg_ratio`, `book_value`, `dividend_per_share`, `dividend_yield`, `eps`
//!
//! ### `overviewext` (extended metrics)
//!
//! - Profitability: `profit_margin`, `operating_margin_ttm`,
//!   `return_on_assets_ttm`, `return_on_equity_ttm`
//! - Revenue: `revenue_per_share_ttm`, `revenue_ttm`, `gross_profit_ttm`
//! - Growth: `quarterly_earnings_growth_yoy`, `quarterly_revenue_growth_yoy`
//! - Valuation: `analyst_target_price`, `trailing_pe`, `forward_pe`,
//!   `price_to_sales_ratio_ttm`, `price_to_book_ratio`, `ev_to_revenue`, `ev_to_ebitda`
//! - Risk and price: `beta`, `week_high_52`, `week_low_52`,
//!   `day_moving_average_50`, `day_moving_average_200`
//! - Capital structure: `shares_outstanding`
//! - Dividends: `dividend_date`, `ex_dividend_date`
//!
//! ## Data Flow
//!
//! ```text
//! symbols (sec_type = "Equity", region = "USA", overview = false)
//!   │  via OverviewRepository::get_symbols_to_load(filter)
//!   ▼
//! Vec<SymbolInfo>
//!   │
//!   ▼
//! OverviewLoader::load()  ── AlphaVantage OVERVIEW API + cache
//!   │
//!   ▼
//! save_overviews_to_db()
//!   ├── parse strings → typed values (clean_string, parse_date, parse_i64, parse_f32)
//!   ├── build (NewOverviewOwned, NewOverviewextOwned) pairs
//!   └── OverviewRepository::batch_save_overviews()
//! ```
//!
//! ## Symbol Selection
//!
//! - **`--symbols` or `--symbols-file`** — Explicit list (one symbol per line
//!   when reading from file). Filters to `missing_overviews_only = true` so
//!   already-loaded symbols are skipped.
//! - **No flag** — Defaults to all US equities (`sec_type = "Equity"`,
//!   `region = "USA"`) where `overview = false`.
//!
//! Both modes support `--limit` for testing.
//!
//! ## String Parsing
//!
//! AlphaVantage returns numeric fields as strings (e.g., `"123.45"`) and uses
//! sentinel values for missing data (`""`, `"None"`, `"-"`). This module
//! provides four small helpers to handle these conversions:
//!
//! - [`clean_string`] — Returns empty string for sentinels, otherwise the value.
//! - [`parse_date`] — Parses `YYYY-MM-DD`; returns `None` for sentinels.
//! - [`parse_i64`] — Parses integer; returns `None` for sentinels.
//! - [`parse_f32`] — Parses float; returns `None` for sentinels.
//!
//! Numeric fields use `unwrap_or(0)` / `unwrap_or(0.0)` defaults rather than
//! `Option<T>` because the database schema requires non-null values for these
//! columns. Date fields with `None` are stored as `NULL` (where the column
//! permits) or fall back to [`default_date`] (`2000-01-01`) for the required
//! `latest_quarter` field.
//!
//! ## Usage
//!
//! ```bash
//! # Load overviews for all US equities that don't have one yet
//! av-cli load overviews
//!
//! # Specific symbols
//! av-cli load overviews --symbols AAPL,MSFT,GOOGL
//!
//! # From a file (one symbol per line)
//! av-cli load overviews --symbols-file sp500.txt
//!
//! # Test with first 10 symbols, dry run
//! av-cli load overviews --limit 10 --dry-run
//!
//! # Higher concurrency
//! av-cli load overviews --concurrent 10 --continue-on-error
//! ```

use anyhow::{Result, anyhow};
use av_client::AlphaVantageClient;
use av_database_postgres::models::security::{NewOverviewOwned, NewOverviewextOwned};
use av_database_postgres::repository::{DatabaseContext, OverviewRepository, OverviewSymbolFilter};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext,
  overview_loader::{OverviewLoader, OverviewLoaderInput},
};
use chrono::{NaiveDate, Utc};
use clap::Args;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;

/// Returns the default date `2000-01-01` for use when date parsing fails.
///
/// Used as a fallback for required date columns (e.g., `latest_quarter`) when
/// the AlphaVantage response contains an unparseable, empty, or sentinel
/// date value. The function is guaranteed not to panic — `2000-01-01` is
/// always a valid date — and falls through to [`NaiveDate::default`] (Unix
/// epoch) only as a defensive measure that's effectively unreachable.
fn default_date() -> NaiveDate {
  // 2000-01-01 is always valid; from_ymd_opt returns Some for valid dates
  match NaiveDate::from_ymd_opt(2000, 1, 1) {
    Some(date) => date,
    None => {
      // This branch is unreachable for 2000-01-01, but we handle it
      // by returning the Unix epoch start as an absolute fallback
      NaiveDate::default()
    }
  }
}

/// Command-line arguments for `av-cli load overviews`.
///
/// Controls symbol selection (explicit list, file, or default US-equity pool),
/// concurrency, and error/dry-run behavior.
#[derive(Args, Clone, Debug)]
pub struct OverviewsArgs {
  /// Comma-separated list of symbols to load.
  ///
  /// When neither this nor `--symbols-file` is set, defaults to all US equities
  /// (`sec_type = "Equity"`, `region = "USA"`) without an existing overview.
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Path to a text file containing symbols, one per line.
  ///
  /// Mutually convenient with `--symbols`. When both are set, the file takes
  /// precedence. Empty lines and surrounding whitespace are ignored.
  #[arg(short = 'f', long)]
  symbols_file: Option<String>,

  /// Cap the number of symbols to process (useful for testing).
  #[arg(short, long)]
  limit: Option<usize>,

  /// Maximum number of concurrent API requests. Defaults to 5.
  #[arg(short, long, default_value = "5")]
  concurrent: usize,

  /// Continue processing remaining symbols when one fails at the loader stage.
  #[arg(long)]
  continue_on_error: bool,

  /// Fetch data from the API but skip database writes.
  #[arg(long)]
  dry_run: bool,
}

/// Main entry point for `av-cli load overviews`.
///
/// Orchestrates the equity overview loading pipeline:
///
/// 1. **Database setup** — Creates [`DatabaseContext`] and an
///    [`OverviewRepository`] handle.
/// 2. **Symbol selection** — [`get_symbols_to_load`] queries the database
///    using the appropriate filter:
///    - Explicit `--symbols` or `--symbols-file` → no type/region filter
///    - Default → US equities only
///    Both modes filter to `missing_overviews_only = true`.
/// 3. **Loader setup** — Creates [`AlphaVantageClient`], [`LoaderContext`]
///    with cache repository attached, and [`OverviewLoader`].
/// 4. **API loading** — Calls [`DataLoader::load`] which fetches the
///    `OVERVIEW` endpoint for each symbol concurrently with caching.
/// 5. **Statistics report** — Logs counts: loaded, no-data, errors,
///    cache-hits, api-calls.
/// 6. **Persistence** — Unless `--dry-run`, calls [`save_overviews_to_db`]
///    which parses string fields, builds `(NewOverviewOwned, NewOverviewextOwned)`
///    pairs, and batch-saves via the repository.
///
/// # Errors
///
/// Returns errors from: database context creation, symbol query, API client
/// creation, loader execution (unless `--continue-on-error`), or database save.
pub async fn execute(args: OverviewsArgs, config: Config) -> Result<()> {
  info!("Starting overview loader");

  // Create database context and overview repository
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow!("Failed to create database context: {}", e))?;
  let overview_repo = db_context.overview_repository();

  // Get symbols to load from database
  let symbols_to_load = {
    let symbols = args.symbols.clone();
    let symbols_file = args.symbols_file.clone();
    let limit = args.limit;

    get_symbols_to_load(&overview_repo, symbols, symbols_file, limit).await?
  };

  if symbols_to_load.is_empty() {
    info!("No symbols to load");
    return Ok(());
  }

  info!("Found {} symbols to load", symbols_to_load.len());

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
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
    track_process: false,
    batch_size: 100,
  };

  // Create loader context with cache repository
  let cache_repo = Arc::new(db_context.cache_repository());
  let context = LoaderContext::new(client, loader_config).with_cache_repository(cache_repo);

  // Create overview loader
  let loader = OverviewLoader::new(args.concurrent);

  // Prepare input
  let input = OverviewLoaderInput { symbols: symbols_to_load };

  // Load data from API
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load overviews: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      return Ok(());
    }
  };

  info!(
    "API loading complete: {} loaded, {} no data, {} errors, {} cache hits, {} API calls",
    output.loaded_count, output.no_data_count, output.errors, output.cache_hits, output.api_calls
  );

  // Save to database unless dry run
  let saved_count = if !args.dry_run && !output.data.is_empty() {
    save_overviews_to_db(&overview_repo, output.data).await?
  } else {
    0
  };

  if !args.dry_run {
    info!(
      "Saved {} overviews to database (saved {} API calls via caching)",
      saved_count, output.cache_hits
    );
  } else {
    info!("Dry run complete - would have saved {} overviews", output.loaded_count);
  }

  Ok(())
}

/// Selects equity symbols to load overviews for, based on the CLI arguments.
///
/// Two modes:
///
/// - **Explicit list** (when `--symbols` or `--symbols-file` is provided) —
///   Reads the file (if specified, one symbol per line, trimmed, empty lines
///   skipped) or uses the comma-separated list. Builds an
///   [`OverviewSymbolFilter`] with `symbols = Some(list)`, no type/region
///   filter, and `missing_overviews_only = true`. Logs warnings if some
///   requested symbols don't need overviews.
///
/// - **Default mode** (no flags) — Builds a filter for `sec_type = "Equity"`
///   and `region = "USA"` with `missing_overviews_only = true`. Logs the
///   number of US equities without overviews.
///
/// Both modes apply `--limit` if set. The filter is passed to
/// [`OverviewRepository::get_symbols_to_load`] for execution.
async fn get_symbols_to_load(
  repo: &impl OverviewRepository,
  symbols_arg: Option<Vec<String>>,
  symbols_file: Option<String>,
  limit: Option<usize>,
) -> Result<Vec<av_loaders::overview_loader::SymbolInfo>> {
  // Handle symbols from file if provided
  let symbols_list = if let Some(file) = symbols_file {
    let content = std::fs::read_to_string(file)?;
    let file_symbols: Vec<String> =
      content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    Some(file_symbols)
  } else {
    symbols_arg
  };

  // Build filter
  let filter = if symbols_list.is_some() {
    OverviewSymbolFilter {
      symbols: symbols_list.clone(),
      sec_type: None, // Don't filter by type when specific symbols provided
      region: None,   // Don't filter by region when specific symbols provided
      missing_overviews_only: true,
      limit,
    }
  } else {
    OverviewSymbolFilter {
      symbols: None,
      sec_type: Some("Equity".to_string()),
      region: Some("USA".to_string()),
      missing_overviews_only: true,
      limit,
    }
  };

  // Get symbols from repository
  let symbol_infos = repo
    .get_symbols_to_load(&filter)
    .await
    .map_err(|e| anyhow!("Failed to query symbols: {}", e))?;

  // Log results
  if let Some(symbol_list) = &symbols_list {
    if symbol_infos.is_empty() {
      warn!("No symbols found that need overviews");
    } else if symbol_infos.len() < symbol_list.len() {
      warn!(
        "Only {} of {} requested symbols need overviews",
        symbol_infos.len(),
        symbol_list.len()
      );
    }
  } else if let Some(limit_val) = limit {
    info!(
      "Found {} US equity symbols without overviews (limited to {})",
      symbol_infos.len(),
      limit_val
    );
  } else {
    info!("Found {} US equity symbols without overviews", symbol_infos.len());
  }

  // Convert to loader's SymbolInfo type
  Ok(
    symbol_infos
      .into_iter()
      .map(|s| av_loaders::overview_loader::SymbolInfo { sid: s.sid, symbol: s.symbol })
      .collect(),
  )
}

/// Transforms loaded overview data into database model structs and batch-saves them.
///
/// For each [`OverviewData`](av_loaders::overview_loader::OverviewData), builds
/// a pair of records:
///
/// - **[`NewOverviewOwned`]** — Core overview fields including identification,
///   classification, headline financials, and `latest_quarter` date.
/// - **[`NewOverviewextOwned`]** — Extended financial metrics covering
///   profitability, growth, valuation, risk, price ranges, and dividend dates.
///
/// All numeric string fields from the API response are parsed via [`parse_i64`]
/// or [`parse_f32`] with `unwrap_or(0)` / `unwrap_or(0.0)` defaults (the
/// columns are NOT NULL). String fields use [`clean_string`] to map sentinel
/// values to empty strings. The required `latest_quarter` date falls back to
/// [`default_date`] when parsing fails; nullable date fields (dividend dates)
/// remain `None` on parse failure.
///
/// The pairs are then passed to [`OverviewRepository::batch_save_overviews`]
/// which inserts both records atomically per symbol. Returns the count of
/// successfully saved records.
async fn save_overviews_to_db(
  repo: &impl OverviewRepository,
  data: Vec<av_loaders::overview_loader::OverviewData>,
) -> Result<usize> {
  let now = Utc::now().naive_utc();

  // Build overview records
  let overview_pairs: Vec<(NewOverviewOwned, NewOverviewextOwned)> = data
    .into_iter()
    .map(|overview_data| {
      // Parse dates - use default date (2000-01-01) if parsing fails
      let latest_quarter_date =
        parse_date(&overview_data.overview.latest_quarter).unwrap_or_else(default_date);
      let dividend_date_val = parse_date(&overview_data.overview.dividend_date);
      let ex_dividend_date_val = parse_date(&overview_data.overview.ex_dividend_date);

      // Create main overview record
      let new_overview = NewOverviewOwned {
        sid: overview_data.sid,
        symbol: overview_data.overview.symbol.clone(),
        name: clean_string(&overview_data.overview.name),
        description: clean_string(&overview_data.overview.description),
        cik: clean_string(&overview_data.overview.cik),
        exchange: clean_string(&overview_data.overview.exchange),
        currency: clean_string(&overview_data.overview.currency),
        country: clean_string(&overview_data.overview.country),
        sector: clean_string(&overview_data.overview.sector),
        industry: clean_string(&overview_data.overview.industry),
        address: clean_string(&overview_data.overview.address),
        fiscal_year_end: clean_string(&overview_data.overview.fiscal_year_end),
        latest_quarter: latest_quarter_date,
        market_capitalization: parse_i64(&overview_data.overview.market_capitalization)
          .unwrap_or(0),
        ebitda: parse_i64(&overview_data.overview.ebitda).unwrap_or(0),
        pe_ratio: parse_f32(&overview_data.overview.pe_ratio).unwrap_or(0.0),
        peg_ratio: parse_f32(&overview_data.overview.peg_ratio).unwrap_or(0.0),
        book_value: parse_f32(&overview_data.overview.book_value).unwrap_or(0.0),
        dividend_per_share: parse_f32(&overview_data.overview.dividend_per_share).unwrap_or(0.0),
        dividend_yield: parse_f32(&overview_data.overview.dividend_yield).unwrap_or(0.0),
        eps: parse_f32(&overview_data.overview.eps).unwrap_or(0.0),
        c_time: now,
        m_time: now,
      };

      // Create extended overview record
      let new_overview_ext = NewOverviewextOwned {
        sid: overview_data.sid,
        revenue_per_share_ttm: parse_f32(&overview_data.overview.revenue_per_share_ttm)
          .unwrap_or(0.0),
        profit_margin: parse_f32(&overview_data.overview.profit_margin).unwrap_or(0.0),
        operating_margin_ttm: parse_f32(&overview_data.overview.operating_margin_ttm)
          .unwrap_or(0.0),
        return_on_assets_ttm: parse_f32(&overview_data.overview.return_on_assets_ttm)
          .unwrap_or(0.0),
        return_on_equity_ttm: parse_f32(&overview_data.overview.return_on_equity_ttm)
          .unwrap_or(0.0),
        revenue_ttm: parse_i64(&overview_data.overview.revenue_ttm).unwrap_or(0),
        gross_profit_ttm: parse_i64(&overview_data.overview.gross_profit_ttm).unwrap_or(0),
        diluted_eps_ttm: parse_f32(&overview_data.overview.diluted_eps_ttm).unwrap_or(0.0),
        quarterly_earnings_growth_yoy: parse_f32(
          &overview_data.overview.quarterly_earnings_growth_yoy,
        )
        .unwrap_or(0.0),
        quarterly_revenue_growth_yoy: parse_f32(
          &overview_data.overview.quarterly_revenue_growth_yoy,
        )
        .unwrap_or(0.0),
        analyst_target_price: parse_f32(&overview_data.overview.analyst_target_price)
          .unwrap_or(0.0),
        trailing_pe: parse_f32(&overview_data.overview.trailing_pe).unwrap_or(0.0),
        forward_pe: parse_f32(&overview_data.overview.forward_pe).unwrap_or(0.0),
        price_to_sales_ratio_ttm: parse_f32(&overview_data.overview.price_to_sales_ratio_ttm)
          .unwrap_or(0.0),
        price_to_book_ratio: parse_f32(&overview_data.overview.price_to_book_ratio).unwrap_or(0.0),
        ev_to_revenue: parse_f32(&overview_data.overview.ev_to_revenue).unwrap_or(0.0),
        ev_to_ebitda: parse_f32(&overview_data.overview.ev_to_ebitda).unwrap_or(0.0),
        beta: parse_f32(&overview_data.overview.beta).unwrap_or(0.0),
        week_high_52: parse_f32(&overview_data.overview.week_52_high).unwrap_or(0.0),
        week_low_52: parse_f32(&overview_data.overview.week_52_low).unwrap_or(0.0),
        day_moving_average_50: parse_f32(&overview_data.overview.day_50_moving_average)
          .unwrap_or(0.0),
        day_moving_average_200: parse_f32(&overview_data.overview.day_200_moving_average)
          .unwrap_or(0.0),
        shares_outstanding: parse_i64(&overview_data.overview.shares_outstanding).unwrap_or(0),
        dividend_date: dividend_date_val,
        ex_dividend_date: ex_dividend_date_val,
        c_time: now,
        m_time: now,
      };

      (new_overview, new_overview_ext)
    })
    .collect();

  // Use repository to batch save
  let saved_count = repo
    .batch_save_overviews(&overview_pairs)
    .await
    .map_err(|e| anyhow!("Failed to save overviews: {}", e))?;

  Ok(saved_count)
}

// ============================================================================
// String parsing helpers
// ============================================================================
//
// AlphaVantage's OVERVIEW endpoint returns all values as strings, with three
// sentinel values for missing data: empty string, "None", or "-". These
// helpers normalize those sentinels and parse the meaningful values into
// the target Rust types.

/// Returns an empty string for AlphaVantage sentinel values, otherwise the input.
///
/// Sentinels: `""`, `"None"`, `"-"`. Used for text fields where empty string
/// is acceptable in the database.
fn clean_string(value: &str) -> String {
  if value.is_empty() || value == "None" || value == "-" {
    String::new()
  } else {
    value.to_string()
  }
}

/// Parses an `YYYY-MM-DD` date string, returning `None` for sentinels or
/// invalid dates.
///
/// Sentinels: `""`, `"None"`, `"-"`. Used for nullable date columns. The
/// caller can fall back to [`default_date`] for required (NOT NULL) date columns.
fn parse_date(value: &str) -> Option<NaiveDate> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Parses an integer string, returning `None` for sentinels or non-numeric input.
///
/// Sentinels: `""`, `"None"`, `"-"`. Used for `BIGINT` columns; the caller
/// typically applies `.unwrap_or(0)` since the columns are NOT NULL.
fn parse_i64(value: &str) -> Option<i64> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  value.parse::<i64>().ok()
}

/// Parses a `f32` string, returning `None` for sentinels or non-numeric input.
///
/// Sentinels: `""`, `"None"`, `"-"`. Used for `REAL` columns; the caller
/// typically applies `.unwrap_or(0.0)` since the columns are NOT NULL.
fn parse_f32(value: &str) -> Option<f32> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  value.parse::<f32>().ok()
}
