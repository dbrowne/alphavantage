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

//! Common types and structures shared across Alpha Vantage API responses.
//!
//! This module defines the building blocks that the endpoint-specific modules
//! ([`time_series`](super::time_series), [`forex`](super::forex),
//! [`crypto`](super::crypto), etc.) compose into their response structs.
//!
//! # Type inventory
//!
//! ## API response primitives
//!
//! | Type              | Description                                                        |
//! |-------------------|--------------------------------------------------------------------|
//! | [`Metadata`]      | Standard numbered-key metadata block from time-series responses    |
//! | [`OhlcvData`]     | Open/High/Low/Close/Volume bar (strings, parse via helper methods) |
//! | [`OhlcvAdjustedData`] | OHLCV plus adjusted close, dividend, and split coefficient     |
//! | [`OhlcData`]      | Open/High/Low/Close without volume (used by forex endpoints)       |
//! | [`SymbolMatch`]   | A single result from the `SYMBOL_SEARCH` endpoint                  |
//! | [`MarketInfo`]    | Exchange status from the `MARKET_STATUS` endpoint                  |
//! | [`ApiError`]      | Error JSON shape: `{"Error Message": "..."}`                       |
//! | [`ApiNote`]       | Rate-limit note shape: `{"Note": "..."}`                           |
//!
//! ## Utility / wrapper types
//!
//! | Type                | Description                                                    |
//! |---------------------|----------------------------------------------------------------|
//! | [`TimeSeriesData<T>`] | Type alias for `BTreeMap<String, T>` — sorted by timestamp key |
//! | [`ApiResponse<T>`]  | Generic response wrapper with optional metadata and pagination |
//! | [`FinancialMetric`] | Name/value/unit triple for financial ratios                    |
//! | [`DateRange`]       | Start/end date pair                                            |
//! | [`Pagination`]      | Page/total pagination metadata                                 |
//!
//! # String-based price fields
//!
//! Alpha Vantage returns all numeric values as JSON strings (e.g., `"182.63"`).
//! The OHLCV structs preserve this representation for lossless round-tripping.
//! Use the `*_as_f64()` and `*_as_u64()` helper methods on [`OhlcvData`] and
//! [`OhlcData`] for numeric access.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─── Metadata ───────────────────────────────────────────────────────────────

/// Standard metadata block returned by Alpha Vantage time-series responses.
///
/// Fields are keyed by numbered prefixes in the JSON (e.g., `"1. Information"`).
/// The `serde(rename)` attributes map them to ergonomic Rust field names.
///
/// # Example JSON
///
/// ```json
/// {
///   "1. Information": "Intraday (5min) open, high, low, close prices and volume",
///   "2. Symbol": "AAPL",
///   "3. Last Refreshed": "2025-04-18 16:00:00",
///   "4. Output Size": "Compact",
///   "5. Time Zone": "US/Eastern"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
  /// Information about the data
  #[serde(rename = "1. Information")]
  pub information: String,

  /// Symbol for the security
  #[serde(rename = "2. Symbol")]
  pub symbol: String,

  /// Last refreshed timestamp
  #[serde(rename = "3. Last Refreshed")]
  pub last_refreshed: String,

  /// Output size (Compact or Full)
  #[serde(rename = "4. Output Size", skip_serializing_if = "Option::is_none")]
  pub output_size: Option<String>,

  /// Time zone
  #[serde(rename = "5. Time Zone", skip_serializing_if = "Option::is_none")]
  pub time_zone: Option<String>,
}

// ─── OHLCV data points ──────────────────────────────────────────────────────

/// A single Open/High/Low/Close/Volume bar from a time-series response.
///
/// All price and volume fields are stored as `String` to preserve the exact
/// representation from the API. Use the helper methods ([`open_as_f64`],
/// [`close_as_f64`], [`volume_as_u64`], [`price_change`],
/// [`percentage_change`]) for numeric access.
///
/// [`open_as_f64`]: OhlcvData::open_as_f64
/// [`close_as_f64`]: OhlcvData::close_as_f64
/// [`volume_as_u64`]: OhlcvData::volume_as_u64
/// [`price_change`]: OhlcvData::price_change
/// [`percentage_change`]: OhlcvData::percentage_change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OhlcvData {
  /// Opening price
  #[serde(rename = "1. open")]
  pub open: String,

  /// Highest price
  #[serde(rename = "2. high")]
  pub high: String,

  /// Lowest price
  #[serde(rename = "3. low")]
  pub low: String,

  /// Closing price
  #[serde(rename = "4. close")]
  pub close: String,

  /// Trading volume
  #[serde(rename = "5. volume")]
  pub volume: String,
}

/// OHLCV bar with split/dividend-adjusted close, dividend amount, and
/// split coefficient.
///
/// Used by the `*_ADJUSTED` time-series endpoints (daily, weekly, monthly).
/// Extends [`OhlcvData`] with three additional fields for corporate-action
/// adjustments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OhlcvAdjustedData {
  /// Opening price
  #[serde(rename = "1. open")]
  pub open: String,

  /// Highest price
  #[serde(rename = "2. high")]
  pub high: String,

  /// Lowest price
  #[serde(rename = "3. low")]
  pub low: String,

  /// Closing price
  #[serde(rename = "4. close")]
  pub close: String,

  /// Adjusted closing price
  #[serde(rename = "5. adjusted close")]
  pub adjusted_close: String,

  /// Trading volume
  #[serde(rename = "6. volume")]
  pub volume: String,

  /// Dividend amount
  #[serde(rename = "7. dividend amount")]
  pub dividend_amount: String,

  /// Split coefficient
  #[serde(rename = "8. split coefficient")]
  pub split_coefficient: String,
}

/// Open/High/Low/Close bar **without** volume.
///
/// Used by forex (`FX_*`) endpoints, which do not report volume.
/// Provides [`open_as_f64`](OhlcData::open_as_f64),
/// [`close_as_f64`](OhlcData::close_as_f64), and
/// [`price_change`](OhlcData::price_change) helpers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OhlcData {
  /// Opening price
  #[serde(rename = "1. open")]
  pub open: String,

  /// Highest price
  #[serde(rename = "2. high")]
  pub high: String,

  /// Lowest price
  #[serde(rename = "3. low")]
  pub low: String,

  /// Closing price
  #[serde(rename = "4. close")]
  pub close: String,
}

// ─── Time series container ──────────────────────────────────────────────────

/// Ordered map of timestamp-string → price-data, sorted chronologically.
///
/// Alpha Vantage returns time-series data as a JSON object with
/// date/datetime string keys (e.g., `"2025-04-18 16:00:00"`) and OHLCV
/// objects as values. `BTreeMap` preserves the natural sort order of these
/// string keys.
///
/// The generic parameter `T` is typically [`OhlcvData`], [`OhlcvAdjustedData`],
/// or [`OhlcData`].
pub type TimeSeriesData<T> = BTreeMap<String, T>;

// ─── Symbol search ──────────────────────────────────────────────────────────

/// A single match from the `SYMBOL_SEARCH` endpoint.
///
/// Contains the ticker symbol, company name, security type, region,
/// trading hours, timezone, currency, and a relevance `match_score`.
/// Fields are renamed from the API's numbered-key format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolMatch {
  /// Stock symbol
  #[serde(rename = "1. symbol")]
  pub symbol: String,

  /// Company name
  #[serde(rename = "2. name")]
  pub name: String,

  /// Stock type (e.g., "Equity")
  #[serde(rename = "3. type")]
  pub stock_type: String,

  /// Region
  #[serde(rename = "4. region")]
  pub region: String,

  /// Market open time
  #[serde(rename = "5. marketOpen")]
  pub market_open: String,

  /// Market close time
  #[serde(rename = "6. marketClose")]
  pub market_close: String,

  /// Timezone
  #[serde(rename = "7. timezone")]
  pub timezone: String,

  /// Currency
  #[serde(rename = "8. currency")]
  pub currency: String,

  /// Match score
  #[serde(rename = "9. matchScore")]
  pub match_score: String,
}

// ─── Market status ──────────────────────────────────────────────────────────

/// Status information for a single market/exchange from the `MARKET_STATUS` endpoint.
///
/// Reports the exchange's current open/closed state, trading hours,
/// and optional notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketInfo {
  /// Market type (e.g., "Equity")
  pub market_type: String,

  /// Region
  pub region: String,

  /// Primary exchanges
  pub primary_exchanges: String,

  /// Local open time
  pub local_open: String,

  /// Local close time
  pub local_close: String,

  /// Current status
  pub current_status: String,

  /// Notes (optional)
  pub notes: Option<String>,
}

// ─── Utility types ──────────────────────────────────────────────────────────

/// A named financial ratio or metric with optional value and unit.
///
/// General-purpose struct for representing computed financial data
/// (e.g., `name = "P/E Ratio"`, `value = Some("23.5")`, `unit = Some("ratio")`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinancialMetric {
  /// Metric name
  pub name: String,

  /// Current value
  pub value: Option<String>,

  /// Unit (e.g., "USD", "Percentage")
  pub unit: Option<String>,
}

/// An inclusive date range for filtering financial data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateRange {
  /// Start date
  pub start_date: NaiveDate,

  /// End date
  pub end_date: NaiveDate,
}

/// Page-based pagination metadata for paginated API results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pagination {
  /// Current page
  pub page: u32,

  /// Total pages
  pub total_pages: u32,

  /// Items per page
  pub per_page: u32,

  /// Total items
  pub total_items: u32,
}

/// Generic wrapper for API responses, combining data with optional metadata,
/// pagination, and a request timestamp.
///
/// This is a **client-side convenience type** — Alpha Vantage does not return
/// this exact shape. It is intended for use in application code that wants to
/// bundle the parsed response with context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
  /// Response data
  pub data: T,

  /// Metadata
  pub metadata: Option<Metadata>,

  /// Pagination info
  pub pagination: Option<Pagination>,

  /// Request timestamp
  pub timestamp: Option<DateTime<Utc>>,
}

// ─── API error / note shapes ────────────────────────────────────────────────

/// Error response shape returned by Alpha Vantage on invalid requests.
///
/// The API returns `{"Error Message": "..."}` — this struct deserializes
/// that single-field JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
  /// Error message
  #[serde(rename = "Error Message")]
  pub error_message: String,
}

/// Rate-limit / informational note returned by Alpha Vantage.
///
/// The API returns `{"Note": "..."}` when a rate limit is approaching or
/// when it wants to communicate an operational message. Not an error — the
/// request may still have succeeded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiNote {
  /// Note message
  #[serde(rename = "Note")]
  pub note: String,
}

// ─── Numeric parsing helpers ────────────────────────────────────────────────

/// Numeric parsing and derived-value helpers for [`OhlcvData`].
///
/// Since Alpha Vantage returns all values as strings, these methods
/// provide convenient, fallible parsing to native numeric types.
impl OhlcvData {
  /// Parses the opening price as `f64`.
  pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.open.parse()
  }

  /// Parses the closing price as `f64`.
  pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.close.parse()
  }

  /// Parses the volume as `u64`.
  pub fn volume_as_u64(&self) -> Result<u64, std::num::ParseIntError> {
    self.volume.parse()
  }

  /// Computes `close - open` (absolute price change within the bar).
  pub fn price_change(&self) -> Result<f64, std::num::ParseFloatError> {
    let open = self.open_as_f64()?;
    let close = self.close_as_f64()?;
    Ok(close - open)
  }

  /// Computes `((close - open) / open) * 100.0` (percentage change).
  ///
  /// Returns `0.0` if `open` is zero (avoids division by zero).
  pub fn percentage_change(&self) -> Result<f64, std::num::ParseFloatError> {
    let open = self.open_as_f64()?;
    let close = self.close_as_f64()?;
    if open == 0.0 { Ok(0.0) } else { Ok(((close - open) / open) * 100.0) }
  }
}

/// Numeric parsing helpers for [`OhlcData`] (forex — no volume field).
impl OhlcData {
  /// Parses the opening price as `f64`.
  pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.open.parse()
  }

  /// Parses the closing price as `f64`.
  pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.close.parse()
  }

  /// Computes `close - open` (absolute price change within the bar).
  pub fn price_change(&self) -> Result<f64, std::num::ParseFloatError> {
    let open = self.open_as_f64()?;
    let close = self.close_as_f64()?;
    Ok(close - open)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ohlcv_calculations() {
    let data = OhlcvData {
      open: "100.0".to_string(),
      high: "105.0".to_string(),
      low: "99.0".to_string(),
      close: "102.0".to_string(),
      volume: "1000000".to_string(),
    };

    assert_eq!(data.open_as_f64().unwrap(), 100.0);
    assert_eq!(data.close_as_f64().unwrap(), 102.0);
    assert_eq!(data.volume_as_u64().unwrap(), 1_000_000);
    assert_eq!(data.price_change().unwrap(), 2.0);
    assert_eq!(data.percentage_change().unwrap(), 2.0);
  }
}
