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

//! Common types and structures used across different AlphaVantage API responses

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Common metadata returned by AlphaVantage API responses
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

/// OHLCV data point for price data
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

/// OHLCV data with adjusted closing price
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

/// Basic OHLC data without volume (used for forex)
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

/// Time series data structure
pub type TimeSeriesData<T> = BTreeMap<String, T>;

/// Symbol search result
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

/// Market status information
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

/// Financial ratio or metric
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinancialMetric {
  /// Metric name
  pub name: String,

  /// Current value
  pub value: Option<String>,

  /// Unit (e.g., "USD", "Percentage")
  pub unit: Option<String>,
}

/// Date range for financial data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateRange {
  /// Start date
  pub start_date: NaiveDate,

  /// End date
  pub end_date: NaiveDate,
}

/// Pagination information
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

/// Generic API response wrapper
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

/// Error response from AlphaVantage API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
  /// Error message
  #[serde(rename = "Error Message")]
  pub error_message: String,
}

/// Note response from AlphaVantage API (usually rate limit info)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiNote {
  /// Note message
  #[serde(rename = "Note")]
  pub note: String,
}

/// Utility functions for working with API data
impl OhlcvData {
  /// Parse opening price as f64
  pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.open.parse()
  }

  /// Parse closing price as f64
  pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.close.parse()
  }

  /// Parse volume as u64
  pub fn volume_as_u64(&self) -> Result<u64, std::num::ParseIntError> {
    self.volume.parse()
  }

  /// Calculate price change from open to close
  pub fn price_change(&self) -> Result<f64, std::num::ParseFloatError> {
    let open = self.open_as_f64()?;
    let close = self.close_as_f64()?;
    Ok(close - open)
  }

  /// Calculate percentage change from open to close
  pub fn percentage_change(&self) -> Result<f64, std::num::ParseFloatError> {
    let open = self.open_as_f64()?;
    let close = self.close_as_f64()?;
    if open == 0.0 { Ok(0.0) } else { Ok(((close - open) / open) * 100.0) }
  }
}

impl OhlcData {
  /// Parse opening price as f64
  pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.open.parse()
  }

  /// Parse closing price as f64
  pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.close.parse()
  }

  /// Calculate price change from open to close
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
