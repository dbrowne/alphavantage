/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

use av_database_postgres::models::MissingSymbol;
use diesel::prelude::*;
use tracing::{debug, warn};

/// Log a missing symbol to the database
///
/// This function records symbols encountered in news feeds or other data sources
/// that are not yet in the symbols table. It handles deduplication automatically.
///
/// # Arguments
/// * `conn` - Database connection
/// * `symbol` - The symbol that was not found
/// * `source` - Where the symbol was encountered (e.g., "news_feed", "ticker_sentiment")
///
/// # Returns
/// * `Ok(())` - Symbol was logged successfully
/// * `Err(e)` - Database error occurred
pub fn log_missing_symbol(
  conn: &mut PgConnection,
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

/// Log multiple missing symbols in batch
///
/// More efficient than calling `log_missing_symbol` multiple times
/// as it handles errors gracefully and doesn't stop on first failure.
///
/// # Arguments
/// * `conn` - Database connection
/// * `symbols` - List of symbols to log
/// * `source` - Where the symbols were encountered
///
/// # Returns
/// Number of successfully logged symbols
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
