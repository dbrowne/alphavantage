// crates/av-cli/src/commands/load/overviews.rs

use anyhow::{Result, anyhow};
use av_client::AlphaVantageClient;
use av_database_postgres::models::Symbol;
use av_database_postgres::models::security::{NewOverviewOwned, NewOverviewextOwned};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext,
  overview_loader::{OverviewLoader, OverviewLoaderInput, SymbolInfo},
};
use chrono::{NaiveDate, Utc};
use clap::Args;
use diesel::prelude::*;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;

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

  // Get symbols to load from database
  let symbols_to_load = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let symbols = args.symbols.clone();
    let symbols_file = args.symbols_file.clone();
    let limit = args.limit;
    move || get_symbols_to_load(&database_url, symbols, symbols_file, limit)
  })
  .await??;

  if symbols_to_load.is_empty() {
    info!("No symbols to load");
    return Ok(());
  }

  info!("Found {} symbols to load", symbols_to_load.len());

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
  }

  // Create API client
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: false,
    batch_size: 100,
  };

  // Create loader context
  let context = LoaderContext::new(client, loader_config);

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
    "API loading complete: {} loaded, {} no data, {} errors",
    output.loaded_count, output.no_data_count, output.errors
  );

  // Save to database unless dry run
  let saved_count = if !args.dry_run && !output.data.is_empty() {
    tokio::task::spawn_blocking({
      let database_url = config.database_url.clone();
      let data = output.data;
      move || save_overviews_to_db(&database_url, data)
    })
    .await??
  } else {
    0
  };

  if !args.dry_run {
    info!("Saved {} overviews to database", saved_count);
  } else {
    info!("Dry run complete - would have saved {} overviews", output.loaded_count);
  }

  Ok(())
}

/// Get symbols to load based on command arguments
fn get_symbols_to_load(
  database_url: &str,
  symbols_arg: Option<Vec<String>>,
  symbols_file: Option<String>,
  limit: Option<usize>,
) -> Result<Vec<SymbolInfo>> {
  use av_database_postgres::schema::symbols::dsl::*;
  use diesel::PgConnection;

  let mut conn = PgConnection::establish(database_url)
    .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

  // Check if specific symbols or file was provided
  if let Some(ref symbol_list) = symbols_arg {
    // Load specific symbols that don't have overviews
    let symbol_records: Vec<Symbol> = symbols
      .filter(symbol.eq_any(symbol_list))
      .filter(overview.eq(false))
      .load::<Symbol>(&mut conn)?;

    if symbol_records.is_empty() {
      warn!("No symbols found that need overviews");
    } else if symbol_records.len() < symbol_list.len() {
      warn!(
        "Only {} of {} requested symbols need overviews",
        symbol_records.len(),
        symbol_list.len()
      );
    }

    Ok(symbol_records.into_iter().map(|s| SymbolInfo { sid: s.sid, symbol: s.symbol }).collect())
  } else if let Some(ref file) = symbols_file {
    // Load symbols from file
    let content = std::fs::read_to_string(file)?;
    let file_symbols: Vec<String> =
      content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    let symbol_records: Vec<Symbol> = symbols
      .filter(symbol.eq_any(&file_symbols))
      .filter(overview.eq(false))
      .load::<Symbol>(&mut conn)?;

    Ok(symbol_records.into_iter().map(|s| SymbolInfo { sid: s.sid, symbol: s.symbol }).collect())
  } else {
    // Default: Load all US equities without overviews
    let mut query = av_database_postgres::schema::symbols::table
      .filter(sec_type.eq("Equity"))
      .filter(region.eq("USA"))
      .filter(overview.eq(false))
      .order(symbol.asc())
      .into_boxed();

    // Apply limit if specified
    if let Some(limit_val) = limit {
      query = query.limit(limit_val as i64);
    }

    let symbol_records: Vec<Symbol> = query.load::<Symbol>(&mut conn)?;

    if let Some(limit_val) = limit {
      info!(
        "Found {} US equity symbols without overviews (limited to {})",
        symbol_records.len(),
        limit_val
      );
    } else {
      info!("Found {} US equity symbols without overviews", symbol_records.len());
    }

    Ok(symbol_records.into_iter().map(|s| SymbolInfo { sid: s.sid, symbol: s.symbol }).collect())
  }
}

/// Save overview data to database
fn save_overviews_to_db(
  database_url: &str,
  data: Vec<av_loaders::overview_loader::OverviewData>,
) -> Result<usize> {
  use av_database_postgres::schema::{overviewexts, overviews, symbols};
  use diesel::PgConnection;

  let mut conn = PgConnection::establish(database_url)
    .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

  let mut saved_count = 0;
  let now = Utc::now().naive_utc();

  // Process in a transaction
  conn.transaction::<_, diesel::result::Error, _>(|conn| {
    for overview_data in data {
      // Parse dates
      let latest_quarter_date = parse_date(&overview_data.overview.latest_quarter)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
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

      // Save main overview
      diesel::insert_into(overviews::table)
        .values(&new_overview)
        .on_conflict(overviews::sid)
        .do_update()
        .set(&new_overview)
        .execute(conn)?;

      // Save extended overview
      diesel::insert_into(overviewexts::table)
        .values(&new_overview_ext)
        .on_conflict(overviewexts::sid)
        .do_update()
        .set(&new_overview_ext)
        .execute(conn)?;

      // Update symbols table
      diesel::update(symbols::table.filter(symbols::sid.eq(overview_data.sid)))
        .set(symbols::overview.eq(true))
        .execute(conn)?;

      saved_count += 1;
    }

    Ok(())
  })?;

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
