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
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::sid_generator::SidGenerator;
use crate::config::Config;
use av_client::AlphaVantageClient;
use av_core::types::market::SecurityType;
use av_database_postgres::models::{MissingSymbol, NewSymbolOwned};
use av_database_postgres::schema::{missing_symbols, symbols};

const NO_PRIORITY: i32 = 9_999_999;
#[derive(Args, Debug)]
pub struct MissingSymbolsArgs {
  /// Source filter (e.g., 'news_feed')
  #[arg(long)]
  source: Option<String>,

  /// Limit number of symbols to process
  #[arg(short, long)]
  limit: Option<usize>,

  /// Delay between API requests in milliseconds (default: 800ms = 75 calls/minute)
  #[arg(long, default_value = "800")]
  delay_ms: u64,

  /// Continue on errors
  #[arg(short = 'k', long, default_value = "true")]
  continue_on_error: bool,

  /// Dry run - don't make API calls or update database
  #[arg(short, long)]
  dry_run: bool,

  /// Mark symbols as 'skipped' if they don't meet criteria
  #[arg(long)]
  auto_skip: bool,
}

pub async fn execute(args: MissingSymbolsArgs, config: Config) -> Result<()> {
  info!("Starting missing symbol resolver");

  // Get pending missing symbols
  let pending_symbols = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let source_filter = args.source.clone();
    let limit = args.limit;

    move || -> Result<Vec<MissingSymbol>> {
      let mut conn = diesel::PgConnection::establish(&database_url)?;

      let mut query = missing_symbols::table
        .filter(missing_symbols::resolution_status.eq("pending"))
        .order_by(missing_symbols::seen_count.desc())
        .into_boxed();

      if let Some(source) = source_filter {
        query = query.filter(missing_symbols::source.eq(source));
      }

      if let Some(lim) = limit {
        query = query.limit(lim as i64);
      }

      let results = query.load::<MissingSymbol>(&mut conn)?;
      Ok(results)
    }
  })
  .await??;

  if pending_symbols.is_empty() {
    info!("No pending missing symbols to process");
    return Ok(());
  }

  info!("Found {} pending symbols to resolve", pending_symbols.len());

  if args.dry_run {
    info!("Dry run mode - no API calls or database updates");
    for symbol in &pending_symbols {
      info!(
        "Would process: {} (source: {}, seen {} times)",
        symbol.symbol, symbol.source, symbol.seen_count
      );
    }
    return Ok(());
  }

  // Create API client
  let client = AlphaVantageClient::new(config.api_config.clone());

  // Initialize SID generator
  let mut sid_generator = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    move || -> Result<SidGenerator> {
      let mut conn = diesel::PgConnection::establish(&database_url)?;
      SidGenerator::new(&mut conn)
    }
  })
  .await??;

  let progress = ProgressBar::new(pending_symbols.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .expect("Invalid progress bar template")
      .progress_chars("##-"),
  );

  let mut found_count = 0;
  let mut not_found_count = 0;
  let mut skipped_count = 0;
  let mut error_count = 0;

  for missing_symbol in pending_symbols {
    progress.set_message(format!("Processing {}", missing_symbol.symbol));

    // Apply rate limiting
    sleep(Duration::from_millis(args.delay_ms)).await;

    // Try to fetch overview from AlphaVantage
    let result = fetch_and_store_overview(
      &client,
      &config.database_url,
      &missing_symbol,
      args.auto_skip,
      &mut sid_generator,
    )
    .await;

    match result {
      Ok(ResolutionResult::Found(sid)) => {
        found_count += 1;
        info!("✓ Found and loaded: {} (SID: {})", missing_symbol.symbol, sid);
      }
      Ok(ResolutionResult::NotFound(reason)) => {
        not_found_count += 1;
        warn!("✗ Not found: {} - {}", missing_symbol.symbol, reason);
      }
      Ok(ResolutionResult::Skipped(reason)) => {
        skipped_count += 1;
        info!("⊘ Skipped: {} - {}", missing_symbol.symbol, reason);
      }
      Err(e) => {
        error_count += 1;
        error!("Error processing {}: {}", missing_symbol.symbol, e);

        if !args.continue_on_error {
          progress.finish_with_message("Stopped due to error");
          return Err(e);
        }
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message("Symbol resolution complete");

  info!("📊 Resolution statistics:");
  info!("  - Found and loaded: {}", found_count);
  info!("  - Not found: {}", not_found_count);
  info!("  - Skipped: {}", skipped_count);
  info!("  - Errors: {}", error_count);

  Ok(())
}

enum ResolutionResult {
  Found(i64),       // SID of loaded symbol
  NotFound(String), // Reason
  Skipped(String),  // Reason
}

/// Normalize symbol for querying AlphaVantage API
/// Returns (query_symbol, skip_reason)
fn normalize_symbol_for_query(symbol: &str) -> (String, Option<String>) {
  // Handle prefixed symbols from news feeds
  if let Some(stripped) = symbol.strip_prefix("CRYPTO:") {
    return (stripped.to_string(), None);
  }

  // Skip FOREX, INDEX, COMMODITY - these aren't equities/ETFs/mutual funds
  if symbol.starts_with("FOREX:") {
    return (symbol.to_string(), Some("Skipped: FOREX symbols not supported".to_string()));
  }
  if symbol.starts_with("INDEX:") {
    return (symbol.to_string(), Some("Skipped: INDEX symbols not supported".to_string()));
  }
  if symbol.starts_with("COMMODITY:") {
    return (symbol.to_string(), Some("Skipped: COMMODITY symbols not supported".to_string()));
  }

  (symbol.to_string(), None)
}

async fn fetch_and_store_overview(
  client: &AlphaVantageClient,
  database_url: &str,
  missing_symbol: &MissingSymbol,
  auto_skip: bool,
  sid_generator: &mut SidGenerator,
) -> Result<ResolutionResult> {
  let symbol = &missing_symbol.symbol;

  // Normalize the symbol for API query
  let (query_symbol, skip_reason) = normalize_symbol_for_query(symbol);

  if let Some(reason) = skip_reason {
    // Skip this symbol type
    tokio::task::spawn_blocking({
      let database_url = database_url.to_string();
      let id = missing_symbol.id;
      let details = Some(reason.clone());

      move || -> Result<()> {
        let mut conn = diesel::PgConnection::establish(&database_url)?;
        MissingSymbol::mark_skipped(&mut conn, id, details)?;
        Ok(())
      }
    })
    .await??;

    return Ok(ResolutionResult::Skipped(reason));
  }

  // Fetch overview from AlphaVantage
  debug!("Fetching overview for {} (query: {})", symbol, query_symbol);

  let overview_result = client.fundamentals().company_overview(&query_symbol).await;

  match overview_result {
    Ok(overview) => {
      // Check if overview has meaningful data
      if overview.symbol.is_empty() {
        // Symbol not found in AlphaVantage
        tokio::task::spawn_blocking({
          let database_url = database_url.to_string();
          let id = missing_symbol.id;
          let details = Some("Not found in AlphaVantage API".to_string());

          move || -> Result<()> {
            let mut conn = diesel::PgConnection::establish(&database_url)?;
            MissingSymbol::mark_not_found(&mut conn, id, details)?;
            Ok(())
          }
        })
        .await??;

        return Ok(ResolutionResult::NotFound("Symbol not found in AlphaVantage".to_string()));
      }

      // Use av-core's SecurityType mapping
      let security_type = SecurityType::from_alpha_vantage(&overview.asset_type);

      // Skip "Other" types as we don't know how to handle them
      if security_type == SecurityType::Other {
        tokio::task::spawn_blocking({
          let database_url = database_url.to_string();
          let id = missing_symbol.id;
          let asset_type = overview.asset_type.clone();
          let details = Some(format!("Skipped: Unknown asset_type '{}'", asset_type));

          move || -> Result<()> {
            let mut conn = diesel::PgConnection::establish(&database_url)?;
            MissingSymbol::mark_skipped(&mut conn, id, details)?;
            Ok(())
          }
        })
        .await??;

        return Ok(ResolutionResult::Skipped(format!(
          "Unknown asset_type: '{}'",
          overview.asset_type
        )));
      }

      // Symbol found - insert into symbols table
      let sid = tokio::task::spawn_blocking({
        let database_url = database_url.to_string();
        let symbol_str = overview.symbol.clone();
        let name =
          if overview.name.is_empty() { symbol_str.clone() } else { overview.name.clone() };
        let asset_type = overview.asset_type.clone();
        let new_sid = sid_generator.next_sid(security_type);

        move || -> Result<i64> {
          let mut conn = diesel::PgConnection::establish(&database_url)?;

          // Check if symbol already exists
          let existing_sid: Option<i64> = symbols::table
            .filter(symbols::symbol.eq(&symbol_str))
            .select(symbols::sid)
            .first(&mut conn)
            .optional()?;

          if let Some(sid) = existing_sid {
            info!("Symbol {} already exists with SID: {}", symbol_str, sid);
            return Ok(sid);
          }

          // Insert new symbol using the pattern from securities.rs
          let new_symbol = NewSymbolOwned {
            sid: new_sid,
            symbol: symbol_str.clone(),
            priority: NO_PRIORITY,
            name: name.clone(),
            sec_type: format!("{:?}", security_type), // Matches securities.rs pattern
            region: "US".to_string(),
            currency: "USD".to_string(),
            overview: true,
            intraday: false,
            summary: false,
            c_time: chrono::Utc::now().naive_utc(),
            m_time: chrono::Utc::now().naive_utc(),
          };

          diesel::insert_into(symbols::table).values(&new_symbol).execute(&mut conn)?;

          info!(
            "Inserted new symbol: {} with SID: {} (type: {}, asset_type: {})",
            symbol_str, new_sid, security_type, asset_type
          );
          Ok(new_sid)
        }
      })
      .await??;

      // Mark as found in missing_symbols table
      tokio::task::spawn_blocking({
        let database_url = database_url.to_string();
        let id = missing_symbol.id;
        let asset_type = overview.asset_type.clone();
        let details = Some(format!("Loaded from AlphaVantage as {}", asset_type));

        move || -> Result<()> {
          let mut conn = diesel::PgConnection::establish(&database_url)?;
          MissingSymbol::mark_found(&mut conn, id, sid, details)?;
          Ok(())
        }
      })
      .await??;

      Ok(ResolutionResult::Found(sid))
    }
    Err(e) => {
      // Check if it's a rate limit or API error
      let error_msg = e.to_string();

      if error_msg.contains("rate limit") || error_msg.contains("429") {
        return Err(anyhow::anyhow!("Rate limit exceeded"));
      }

      if error_msg.contains("Invalid API") {
        return Err(anyhow::anyhow!("API key invalid or missing"));
      }

      // Unknown error
      if auto_skip {
        tokio::task::spawn_blocking({
          let database_url = database_url.to_string();
          let id = missing_symbol.id;
          let reason = Some(format!("API error: {}", error_msg));

          move || -> Result<()> {
            let mut conn = diesel::PgConnection::establish(&database_url)?;
            MissingSymbol::mark_skipped(&mut conn, id, reason)?;
            Ok(())
          }
        })
        .await??;

        Ok(ResolutionResult::Skipped(format!("API error: {}", error_msg)))
      } else {
        Err(e.into())
      }
    }
  }
}
