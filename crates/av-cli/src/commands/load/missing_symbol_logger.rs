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

//! Utility helpers for logging unrecognized symbols to `missing_symbols`.
//!
//! When ingesting news feeds, ticker sentiments, or top-movers data from
//! AlphaVantage, the response often references symbols that don't exist in
//! the local `symbols` table — newly listed equities, foreign tickers, or
//! crypto symbols that haven't been loaded yet. This module provides two
//! helper functions to record those symbols for later resolution.
//!
//! ## Data Flow
//!
//! ```text
//! news/top-movers loaders
//!   │  encounters unknown symbol
//!   ▼
//! log_missing_symbol() / log_missing_symbols_batch()
//!   │
//!   ▼
//! MissingSymbol::record_or_increment()
//!   │
//!   ▼
//! missing_symbols table
//!   │  (deduplicated; existing rows have seen_count incremented)
//!   ▼
//! av-cli load missing-symbols   ← later resolution pass
//!   │  via AlphaVantage symbol search
//!   ▼
//! creates symbols + crypto_api_map entries, or marks as not-found
//! ```
//!
//! ## Deduplication
//!
//! The underlying [`MissingSymbol::record_or_increment`] uses
//! `INSERT ... ON CONFLICT DO UPDATE` to either insert a new row (with
//! `seen_count = 1`) or atomically increment `seen_count` on the existing row.
//! This makes the helpers idempotent — calling them repeatedly for the same
//! `(symbol, source)` pair just bumps the counter.
//!
//! ## Consumed By
//!
//! These helpers are currently marked `#[allow(dead_code)]` because they're
//! retained for direct CLI use and equivalent functionality is provided via
//! [`crate::commands::load::news_utils::save_news_to_database`], which
//! handles missing-symbol logging as part of news persistence.
//!
//! See also [`crate::commands::load::missing_symbols`] for the resolution
//! command that consumes the entries logged here.

use av_database_postgres::models::MissingSymbol;
use diesel::prelude::*;
use tracing::{debug, warn};

/// Records an unknown symbol in the `missing_symbols` table.
///
/// Wraps [`MissingSymbol::record_or_increment`] which handles the
/// insert-or-increment semantics atomically. On the first occurrence, a new
/// row is created with `seen_count = 1`. On subsequent occurrences for the
/// same `(symbol, source)` pair, the existing row's `seen_count` is incremented.
///
/// Logs at `debug` level on success (distinguishing first-time vs. repeat
/// occurrences) and `warn` level on failure.
///
/// # Arguments
///
/// * `conn` — Diesel PostgreSQL connection (TODO: decouple from Postgres).
/// * `symbol` — The unknown symbol (e.g., `"NVDA"`, `"BTC"`).
/// * `source` — Origin context, used for filtering in the resolution pass
///   (e.g., `"news_feed"`, `"ticker_sentiment"`, `"top_movers"`).
///
/// # Errors
///
/// Returns the underlying [`diesel::result::Error`] on database failure
/// (connection lost, constraint violation, etc.).
#[allow(dead_code)] // Available for CLI commands that need direct DB access
pub fn log_missing_symbol(
  conn: &mut PgConnection, //todo: decouple from postgres
  symbol: &str,
  source: &str,
) -> Result<(), diesel::result::Error> {
  match MissingSymbol::record_or_increment(conn, symbol, source) {
    Ok(record) => {
      if record.seen_count == 1 {
        debug!("Logged new missing symbol: {} (source: {})", symbol, source);
      } else {
        debug!(
          "Incremented missing symbol count: {} (seen {} times from {})",
          symbol, record.seen_count, source
        );
      }
      Ok(())
    }
    Err(e) => {
      warn!("Failed to log missing symbol {}: {}", symbol, e);
      Err(e)
    }
  }
}

/// Records multiple unknown symbols in the `missing_symbols` table.
///
/// Iterates over the symbol list and calls [`log_missing_symbol`] for each.
/// Unlike calling [`log_missing_symbol`] in a loop directly, this function
/// **does not stop on the first error** — failed inserts are logged as
/// warnings (via the underlying call) and the loop continues with the next
/// symbol. This makes it suitable for best-effort batch logging where you
/// want to record as many symbols as possible.
///
/// # Note
///
/// Despite the name, this function performs one round-trip per symbol — it's
/// not a true bulk insert. The "batch" terminology refers to the error-handling
/// behavior (graceful continuation), not query batching. For true batched
/// inserts, use the underlying [`MissingSymbol`] model methods directly.
///
/// # Arguments
///
/// * `conn` — Diesel PostgreSQL connection.
/// * `symbols` — List of unknown symbols to record.
/// * `source` — Origin context applied to all entries (see [`log_missing_symbol`]).
///
/// # Returns
///
/// The count of symbols that were successfully logged. Logs a debug summary
/// line when the count is non-zero.
#[allow(dead_code)] // Available for CLI commands that need direct DB access
pub fn log_missing_symbols_batch(
  conn: &mut PgConnection,
  symbols: &[String],
  source: &str,
) -> usize {
  let mut logged_count = 0;

  for symbol in symbols {
    if let Ok(_) = log_missing_symbol(conn, symbol, source) {
      logged_count += 1;
    }
  }

  if logged_count > 0 {
    debug!(
      "Logged {} missing symbols from {} (out of {} total)",
      logged_count,
      source,
      symbols.len()
    );
  }

  logged_count
}
