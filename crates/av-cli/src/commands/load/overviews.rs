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

/// Returns a default date (2000-01-01) for use when date parsing fails.
/// This function is guaranteed not to panic as 2000-01-01 is always a valid date.
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

#[derive(Args, Clone, Debug)]
pub struct OverviewsArgs {
  /// Symbols to load (comma-separated)
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// File containing symbols (one per line)
  #[arg(short = 'f', long)]
  symbols_file: Option<String>,

  /// Limit number of symbols to load (for debugging)
  #[arg(short, long)]
  limit: Option<usize>,

  /// Number of concurrent requests
  #[arg(short, long, default_value = "5")]
  concurrent: usize,

  /// Continue on error instead of stopping
  #[arg(long)]
  continue_on_error: bool,

  /// Dry run - fetch data but don't save to database
  #[arg(long)]
  dry_run: bool,
}

/// Main execute function
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

/// Get symbols to load based on command arguments using OverviewRepository
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

/// Save overview data to database using OverviewRepository
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

// Helper functions
fn clean_string(value: &str) -> String {
  if value.is_empty() || value == "None" || value == "-" {
    String::new()
  } else {
    value.to_string()
  }
}

fn parse_date(value: &str) -> Option<NaiveDate> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn parse_i64(value: &str) -> Option<i64> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  value.parse::<i64>().ok()
}

fn parse_f32(value: &str) -> Option<f32> {
  if value.is_empty() || value == "None" || value == "-" {
    return None;
  }
  value.parse::<f32>().ok()
}
