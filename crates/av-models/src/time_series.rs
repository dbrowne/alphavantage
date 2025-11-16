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

//! Time series data models for stock prices and market data

use crate::common::{
  MarketInfo, Metadata, OhlcvAdjustedData, OhlcvData, SymbolMatch, TimeSeriesData,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Intraday time series response
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IntradayTimeSeries {
  pub meta_data: IntradayMetadata,

  pub time_series: TimeSeriesData<OhlcvData>,
}
impl<'de> Deserialize<'de> for IntradayTimeSeries {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let mut map = serde_json::Map::deserialize(deserializer)?;

    // Extract metadata
    let meta_data: IntradayMetadata = map
      .remove("Meta Data")
      .ok_or_else(|| serde::de::Error::missing_field("Meta Data"))
      .and_then(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))?;

    // Find the time series field - it could be any interval
    let time_series_key = map
      .keys()
      .find(|k| k.starts_with("Time Series"))
      .cloned()
      .ok_or_else(|| serde::de::Error::custom("No time series data found in response"))?;

    // Extract time series data
    let time_series: TimeSeriesData<OhlcvData> = map
      .remove(&time_series_key)
      .ok_or_else(|| serde::de::Error::missing_field("Time Series"))
      .and_then(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))?;

    Ok(IntradayTimeSeries { meta_data, time_series })
  }
}
/// Metadata for intraday time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntradayMetadata {
  /// Information about the data
  #[serde(rename = "1. Information")]
  pub information: String,

  /// Symbol
  #[serde(rename = "2. Symbol")]
  pub symbol: String,

  /// Last refreshed timestamp
  #[serde(rename = "3. Last Refreshed")]
  pub last_refreshed: String,

  /// Interval (e.g., "5min")
  #[serde(rename = "4. Interval")]
  pub interval: String,

  /// Output size
  #[serde(rename = "5. Output Size")]
  pub output_size: String,

  /// Time zone
  #[serde(rename = "6. Time Zone")]
  pub time_zone: String,
}

/// Daily time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Daily time series data
  #[serde(rename = "Time Series (Daily)")]
  pub time_series: TimeSeriesData<OhlcvData>,
}

/// Daily adjusted time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyAdjustedTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Daily adjusted time series data
  #[serde(rename = "Time Series (Daily)")]
  pub time_series: TimeSeriesData<OhlcvAdjustedData>,
}

/// Weekly time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeeklyTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Weekly time series data
  #[serde(rename = "Weekly Time Series")]
  pub time_series: TimeSeriesData<OhlcvData>,
}

/// Weekly adjusted time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeeklyAdjustedTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Weekly adjusted time series data
  #[serde(rename = "Weekly Adjusted Time Series")]
  pub time_series: TimeSeriesData<OhlcvAdjustedData>,
}

/// Monthly time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Monthly time series data
  #[serde(rename = "Monthly Time Series")]
  pub time_series: TimeSeriesData<OhlcvData>,
}

/// Monthly adjusted time series response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyAdjustedTimeSeries {
  /// Metadata about the time series
  #[serde(rename = "Meta Data")]
  pub meta_data: Metadata,

  /// Monthly adjusted time series data
  #[serde(rename = "Monthly Adjusted Time Series")]
  pub time_series: TimeSeriesData<OhlcvAdjustedData>,
}

/// Symbol search response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSearch {
  /// List of matching symbols
  #[serde(rename = "bestMatches")]
  pub best_matches: Vec<SymbolMatch>,
}

/// Market status response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketStatus {
  /// Endpoint information
  pub endpoint: String,

  /// List of markets
  pub markets: Vec<MarketInfo>,
}

/// Quote endpoint response (real-time price)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalQuote {
  /// Global quote data
  #[serde(rename = "Global Quote")]
  pub global_quote: QuoteData,
}

/// Quote data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteData {
  #[serde(rename = "01. symbol")]
  pub symbol: String,

  #[serde(rename = "02. open")]
  pub open: String,

  #[serde(rename = "03. high")]
  pub high: String,

  #[serde(rename = "04. low")]
  pub low: String,

  /// Current price
  #[serde(rename = "05. price")]
  pub price: String,

  #[serde(rename = "06. volume")]
  pub volume: String,

  #[serde(rename = "07. latest trading day")]
  pub latest_trading_day: String,

  #[serde(rename = "08. previous close")]
  pub previous_close: String,

  #[serde(rename = "09. change")]
  pub change: String,

  #[serde(rename = "10. change percent")]
  pub change_percent: String,
}

/// Technical indicator response (generic)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalIndicator {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: TechnicalMetadata,

  /// Technical analysis data
  #[serde(flatten)]
  pub technical_analysis: BTreeMap<String, BTreeMap<String, String>>,
}

/// Technical indicator metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalMetadata {
  #[serde(rename = "1: Symbol")]
  pub symbol: String,

  #[serde(rename = "2: Indicator")]
  pub indicator: String,

  #[serde(rename = "3: Last Refreshed")]
  pub last_refreshed: String,

  #[serde(rename = "4: Interval")]
  pub interval: String,

  #[serde(rename = "5: Time Period", skip_serializing_if = "Option::is_none")]
  pub time_period: Option<String>,

  #[serde(rename = "6: Time Zone")]
  pub time_zone: String,
}

/// Moving average data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovingAverageData {
  /// Moving average value
  #[serde(rename = "MA")]
  pub ma: String,
}

/// RSI data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RsiData {
  /// RSI value
  #[serde(rename = "RSI")]
  pub rsi: String,
}

/// MACD data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacdData {
  /// MACD line
  #[serde(rename = "MACD")]
  pub macd: String,

  /// MACD histogram
  #[serde(rename = "MACD_Hist")]
  pub macd_hist: String,

  /// MACD signal line
  #[serde(rename = "MACD_Signal")]
  pub macd_signal: String,
}

/// Bollinger Bands data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BollingerBandsData {
  #[serde(rename = "Real Upper Band")]
  pub upper_band: String,

  #[serde(rename = "Real Middle Band")]
  pub middle_band: String,

  #[serde(rename = "Real Lower Band")]
  pub lower_band: String,
}

impl IntradayTimeSeries {
  pub fn latest(&self) -> Option<(&String, &OhlcvData)> {
    self.time_series.iter().next()
  }

  pub fn len(&self) -> usize {
    self.time_series.len()
  }

  pub fn is_empty(&self) -> bool {
    self.time_series.is_empty()
  }
}

impl DailyTimeSeries {
  pub fn latest(&self) -> Option<(&String, &OhlcvData)> {
    self.time_series.iter().next()
  }

  pub fn len(&self) -> usize {
    self.time_series.len()
  }

  /// Check if the time series is empty
  pub fn is_empty(&self) -> bool {
    self.time_series.is_empty()
  }

  /// Calculate average volume over the time series
  pub fn average_volume(&self) -> Result<f64, std::num::ParseFloatError> {
    let volumes: Result<Vec<f64>, _> =
      self.time_series.values().map(|data| data.volume.parse::<f64>()).collect();

    let volumes = volumes?;
    if volumes.is_empty() {
      Ok(0.0)
    } else {
      Ok(volumes.iter().sum::<f64>() / volumes.len() as f64)
    }
  }

  /// Calculate average closing price
  pub fn average_close(&self) -> Result<f64, std::num::ParseFloatError> {
    let closes: Result<Vec<f64>, _> =
      self.time_series.values().map(|data| data.close.parse::<f64>()).collect();

    let closes = closes?;
    if closes.is_empty() { Ok(0.0) } else { Ok(closes.iter().sum::<f64>() / closes.len() as f64) }
  }
}

impl QuoteData {
  /// Parse current price as f64
  pub fn price_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.price.parse()
  }

  /// Parse change as f64
  pub fn change_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.change.parse()
  }

  /// Parse change percent as f64 (removes % sign)
  pub fn change_percent_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.change_percent.trim_end_matches('%').parse()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_quote_data_parsing() {
    let quote = QuoteData {
      symbol: "AAPL".to_string(),
      open: "150.0".to_string(),
      high: "155.0".to_string(),
      low: "149.0".to_string(),
      price: "152.5".to_string(),
      volume: "1000000".to_string(),
      latest_trading_day: "2024-01-15".to_string(),
      previous_close: "151.0".to_string(),
      change: "1.5".to_string(),
      change_percent: "0.99%".to_string(),
    };

    assert_eq!(quote.price_as_f64().unwrap(), 152.5);
    assert_eq!(quote.change_as_f64().unwrap(), 1.5);
    assert_eq!(quote.change_percent_as_f64().unwrap(), 0.99);
  }
}
