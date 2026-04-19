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

//! Foreign exchange (forex) data models.
//!
//! This module provides serde-deserializable structs for the Alpha Vantage
//! forex endpoints, plus utility types for currency-pair analysis.
//!
//! # Endpoint mapping
//!
//! | Endpoint                 | Model              | Time-series JSON key                  |
//! |--------------------------|--------------------|---------------------------------------|
//! | `CURRENCY_EXCHANGE_RATE` | [`ExchangeRate`]   | N/A (single-value response)           |
//! | `FX_INTRADAY`            | [`FxIntraday`]     | Flattened via `#[serde(flatten)]`     |
//! | `FX_DAILY`               | [`FxDaily`]        | `"Time Series FX (Daily)"`            |
//! | `FX_WEEKLY`              | [`FxWeekly`]       | `"Time Series FX (Weekly)"`           |
//! | `FX_MONTHLY`             | [`FxMonthly`]      | `"Time Series FX (Monthly)"`          |
//!
//! # Key differences from equity time-series
//!
//! - Forex bars use [`OhlcData`] (no volume) instead of [`OhlcvData`](super::common::OhlcvData).
//! - Metadata uses `From Symbol` / `To Symbol` instead of a single `Symbol` field.
//! - Exchange rate responses include bid/ask prices with spread helpers.
//!
//! # Utility types
//!
//! | Type                  | Purpose                                           |
//! |-----------------------|---------------------------------------------------|
//! | [`CurrencyPair`]      | Currency pair metadata, major/cross classification |
//! | [`ForexSession`]      | Trading session hours by geographic region         |
//! | [`CrossRate`]         | Computed cross-currency rate via intermediate      |
//! | [`CurrencyVolatility`]| Historical/implied volatility for a pair           |
//! | [`EconomicImpact`]    | Economic indicator release with currency impact    |

use crate::common::{OhlcData, TimeSeriesData};
use serde::{Deserialize, Serialize};

// ─── Exchange rate ──────────────────────────────────────────────────────────

/// Top-level response from the `CURRENCY_EXCHANGE_RATE` endpoint (forex variant).
///
/// Wraps a single [`ExchangeRateData`] under the JSON key
/// `"Realtime Currency Exchange Rate"`. Use [`rate()`](ExchangeRate::rate)
/// for convenient access to the inner data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRate {
  /// Realtime currency exchange rate data
  #[serde(rename = "Realtime Currency Exchange Rate")]
  pub realtime_currency_exchange_rate: ExchangeRateData,
}

/// Real-time exchange rate data for a fiat currency pair.
///
/// Contains from/to currency codes and names, the rate itself,
/// bid/ask prices, and a timestamp. All numeric values are strings.
///
/// # Helper methods
///
/// - [`rate_as_f64`](ExchangeRateData::rate_as_f64) — parse the exchange rate.
/// - [`bid_as_f64`](ExchangeRateData::bid_as_f64) / [`ask_as_f64`](ExchangeRateData::ask_as_f64) — parse bid/ask.
/// - [`spread`](ExchangeRateData::spread) — absolute bid-ask spread.
/// - [`spread_percentage`](ExchangeRateData::spread_percentage) — spread as % of mid price.
/// - [`pair_symbol`](ExchangeRateData::pair_symbol) — e.g., `"EURUSD"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateData {
  /// From currency code
  #[serde(rename = "1. From_Currency Code")]
  pub from_currency_code: String,

  /// From currency name
  #[serde(rename = "2. From_Currency Name")]
  pub from_currency_name: String,

  /// To currency code
  #[serde(rename = "3. To_Currency Code")]
  pub to_currency_code: String,

  /// To currency name
  #[serde(rename = "4. To_Currency Name")]
  pub to_currency_name: String,

  /// Exchange rate
  #[serde(rename = "5. Exchange Rate")]
  pub exchange_rate: String,

  /// Last refreshed timestamp
  #[serde(rename = "6. Last Refreshed")]
  pub last_refreshed: String,

  /// Timezone
  #[serde(rename = "7. Time Zone")]
  pub time_zone: String,

  /// Bid price
  #[serde(rename = "8. Bid Price")]
  pub bid_price: String,

  /// Ask price
  #[serde(rename = "9. Ask Price")]
  pub ask_price: String,
}

// ─── Time series responses ──────────────────────────────────────────────────

/// Response from the `FX_INTRADAY` endpoint.
///
/// Uses `#[serde(flatten)]` for the time-series data because the JSON key
/// varies by interval (e.g., `"Time Series FX (Intraday) (5min)"`).
/// The `time_series` field uses [`OhlcData`] (no volume for forex).
///
/// # Methods
///
/// - [`latest`](FxIntraday::latest) — most recent bar.
/// - [`len`](FxIntraday::len) / [`is_empty`](FxIntraday::is_empty) — count.
/// - [`calculate_volatility`](FxIntraday::calculate_volatility) — standard
///   deviation of period returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxIntraday {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: FxMetadata,

  /// Time series data
  #[serde(flatten)]
  pub time_series: TimeSeriesData<OhlcData>,
}

/// Response from the `FX_DAILY` endpoint.
///
/// # Methods
///
/// - [`latest`](FxDaily::latest), [`len`](FxDaily::len), [`is_empty`](FxDaily::is_empty).
/// - [`simple_moving_average`](FxDaily::simple_moving_average) — compute SMA
///   of closing prices over `periods` bars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxDaily {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: FxMetadata,

  /// Time series data
  #[serde(rename = "Time Series FX (Daily)")]
  pub time_series: TimeSeriesData<OhlcData>,
}

/// Response from the `FX_WEEKLY` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxWeekly {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: FxMetadata,

  /// Time series data
  #[serde(rename = "Time Series FX (Weekly)")]
  pub time_series: TimeSeriesData<OhlcData>,
}

/// Response from the `FX_MONTHLY` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxMonthly {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: FxMetadata,

  /// Time series data
  #[serde(rename = "Time Series FX (Monthly)")]
  pub time_series: TimeSeriesData<OhlcData>,
}

// ─── Metadata ───────────────────────────────────────────────────────────────

/// Metadata block for forex time-series responses.
///
/// Uses `From Symbol` / `To Symbol` instead of the equity `Symbol` field.
/// `interval` and `output_size` are only present for intraday responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxMetadata {
  /// Information
  #[serde(rename = "1. Information")]
  pub information: String,

  /// From symbol
  #[serde(rename = "2. From Symbol")]
  pub from_symbol: String,

  /// To symbol
  #[serde(rename = "3. To Symbol")]
  pub to_symbol: String,

  /// Last refreshed
  #[serde(rename = "4. Last Refreshed")]
  pub last_refreshed: String,

  /// Interval (for intraday data)
  #[serde(rename = "5. Interval", skip_serializing_if = "Option::is_none")]
  pub interval: Option<String>,

  /// Output size (for intraday data)
  #[serde(rename = "6. Output Size", skip_serializing_if = "Option::is_none")]
  pub output_size: Option<String>,

  /// Time zone
  #[serde(rename = "7. Time Zone", skip_serializing_if = "Option::is_none")]
  pub time_zone: Option<String>,
}

// ─── Utility types ──────────────────────────────────────────────────────────

/// A currency pair with metadata for forex analysis.
///
/// Constructed via [`CurrencyPair::new`], which auto-generates the `symbol`
/// (e.g., `"EURUSD"`) and `display_name` (e.g., `"EUR/USD"`), and sets
/// `decimal_places` based on whether JPY is involved (3 for JPY pairs, 5
/// for all others).
///
/// # Methods
///
/// - [`is_major`](CurrencyPair::is_major) — checks against the 7 major pairs.
/// - [`is_cross`](CurrencyPair::is_cross) — `true` if neither currency is USD.
/// - [`inverse`](CurrencyPair::inverse) — returns the reciprocal pair (e.g., EUR/USD → USD/EUR).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyPair {
  /// Base currency
  pub base_currency: String,

  /// Quote currency
  pub quote_currency: String,

  /// Currency pair symbol (e.g., "EURUSD")
  pub symbol: String,

  /// Display name
  pub display_name: String,

  /// Decimal places for pricing
  pub decimal_places: u8,
}

/// Forex trading session information (London, New York, Tokyo, Sydney).
///
/// Tracks session hours in UTC, whether the session is currently active,
/// and which major pairs are primarily traded during the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForexSession {
  /// Session name (e.g., "London", "New York", "Tokyo", "Sydney")
  pub name: String,

  /// Session timezone
  pub timezone: String,

  /// Session open time (UTC)
  pub open_time_utc: String,

  /// Session close time (UTC)
  pub close_time_utc: String,

  /// Whether session is currently active
  pub is_active: bool,

  /// Major currency pairs traded in this session
  pub major_pairs: Vec<String>,
}

/// A computed cross-currency rate, optionally via an intermediate currency.
///
/// When a direct pair is unavailable, the rate is calculated through an
/// intermediate (typically USD). The `calculation_method` field records
/// how the rate was derived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRate {
  /// Base currency
  pub base_currency: String,

  /// Quote currency
  pub quote_currency: String,

  /// Intermediate currency (if used for calculation)
  pub intermediate_currency: Option<String>,

  /// Calculated rate
  pub rate: f64,

  /// Calculation method
  pub calculation_method: String,

  /// Timestamp of calculation
  pub calculated_at: String,
}

/// Historical and implied volatility for a currency pair over a time period.
///
/// `historical_volatility` is annualized from the specified `period`.
/// `implied_volatility` is available only when options data is provided.
/// `average_true_range` (ATR) measures typical bar range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyVolatility {
  /// Currency pair
  pub currency_pair: String,

  /// Time period for calculation
  pub period: String,

  /// Historical volatility (annualized)
  pub historical_volatility: f64,

  /// Implied volatility (if available)
  pub implied_volatility: Option<f64>,

  /// Average true range
  pub average_true_range: f64,

  /// Calculation date
  pub calculated_on: String,
}

/// An economic indicator release and its potential impact on a currency.
///
/// Tracks expected vs. actual values and the observed market reaction
/// direction. `impact_level` is `"High"`, `"Medium"`, or `"Low"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomicImpact {
  /// Currency affected
  pub currency: String,

  /// Economic indicator name
  pub indicator_name: String,

  /// Release date/time
  pub release_time: String,

  /// Expected value
  pub expected_value: Option<String>,

  /// Actual value
  pub actual_value: Option<String>,

  /// Previous value
  pub previous_value: Option<String>,

  /// Impact level (High/Medium/Low)
  pub impact_level: String,

  /// Currency reaction direction
  pub currency_reaction: Option<String>,
}

// ─── Helper methods ─────────────────────────────────────────────────────────

impl ExchangeRate {
  /// Returns a reference to the inner [`ExchangeRateData`].
  pub fn rate(&self) -> &ExchangeRateData {
    &self.realtime_currency_exchange_rate
  }
}

/// Numeric parsing and spread calculation helpers for [`ExchangeRateData`].
impl ExchangeRateData {
  /// Parses the exchange rate as `f64`.
  pub fn rate_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.exchange_rate.parse()
  }

  /// Parses the bid price as `f64`.
  pub fn bid_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.bid_price.parse()
  }

  /// Parses the ask price as `f64`.
  pub fn ask_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.ask_price.parse()
  }

  /// Computes the absolute bid-ask spread (`ask - bid`).
  pub fn spread(&self) -> Result<f64, std::num::ParseFloatError> {
    let bid = self.bid_as_f64()?;
    let ask = self.ask_as_f64()?;
    Ok(ask - bid)
  }

  /// Computes the spread as a percentage of the mid price.
  ///
  /// Formula: `((ask - bid) / ((bid + ask) / 2)) * 100`. Returns `0.0`
  /// if the mid price is zero.
  pub fn spread_percentage(&self) -> Result<f64, std::num::ParseFloatError> {
    let bid = self.bid_as_f64()?;
    let ask = self.ask_as_f64()?;
    let spread = ask - bid;
    let mid = (bid + ask) / 2.0;

    if mid == 0.0 { Ok(0.0) } else { Ok((spread / mid) * 100.0) }
  }

  /// Returns the concatenated currency pair symbol (e.g., `"EURUSD"`).
  pub fn pair_symbol(&self) -> String {
    format!("{}{}", self.from_currency_code, self.to_currency_code)
  }
}

/// Time-series access and analysis methods for [`FxIntraday`].
impl FxIntraday {
  /// Returns the first (earliest) data point from the sorted BTreeMap.
  pub fn latest(&self) -> Option<(&String, &OhlcData)> {
    self.time_series.iter().next()
  }

  /// Get the number of data points
  pub fn len(&self) -> usize {
    self.time_series.len()
  }

  /// Check if the time series is empty
  pub fn is_empty(&self) -> bool {
    self.time_series.is_empty()
  }

  /// Computes the standard deviation of period-over-period returns.
  ///
  /// Returns `0.0` if fewer than 2 data points are available. Returns are
  /// calculated as `(close[i-1] / close[i] - 1) * 100`.
  pub fn calculate_volatility(&self) -> Result<f64, std::num::ParseFloatError> {
    let closes: Result<Vec<f64>, _> =
      self.time_series.values().map(|data| data.close.parse::<f64>()).collect();

    let closes = closes?;
    if closes.len() < 2 {
      return Ok(0.0);
    }

    // Calculate returns
    let mut returns = Vec::new();
    for i in 1..closes.len() {
      let return_rate = (closes[i - 1] / closes[i] - 1.0) * 100.0;
      returns.push(return_rate);
    }

    // Calculate standard deviation
    let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance: f64 =
      returns.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / returns.len() as f64;

    Ok(variance.sqrt())
  }
}

/// Time-series access and analysis methods for [`FxDaily`].
impl FxDaily {
  /// Returns the first (earliest) data point from the sorted BTreeMap.
  pub fn latest(&self) -> Option<(&String, &OhlcData)> {
    self.time_series.iter().next()
  }

  /// Get the number of data points
  pub fn len(&self) -> usize {
    self.time_series.len()
  }

  /// Check if the time series is empty
  pub fn is_empty(&self) -> bool {
    self.time_series.is_empty()
  }

  /// Computes a simple moving average (SMA) of closing prices.
  ///
  /// Returns `Vec<(date_string, sma_value)>` for each bar that has enough
  /// preceding data to fill the window. Returns an empty vec if
  /// `data_points.len() < periods`.
  pub fn simple_moving_average(
    &self,
    periods: usize,
  ) -> Result<Vec<(String, f64)>, std::num::ParseFloatError> {
    let mut result = Vec::new();
    let data_points: Vec<_> = self.time_series.iter().collect();

    if data_points.len() < periods {
      return Ok(result);
    }

    for i in (periods - 1)..data_points.len() {
      let window = &data_points[i - (periods - 1)..=i];
      let sum: Result<f64, _> = window.iter().map(|(_, data)| data.close.parse::<f64>()).sum();

      let average = sum? / periods as f64;
      result.push((data_points[i].0.clone(), average));
    }

    Ok(result)
  }
}

/// Construction and classification methods for [`CurrencyPair`].
impl CurrencyPair {
  /// Creates a new currency pair, auto-generating the symbol and display name.
  ///
  /// `decimal_places` is set to 3 for JPY pairs, 5 for all others.
  pub fn new(base: &str, quote: &str) -> Self {
    let symbol = format!("{}{}", base, quote);
    let display_name = format!("{}/{}", base, quote);

    Self {
      base_currency: base.to_uppercase(),
      quote_currency: quote.to_uppercase(),
      symbol,
      display_name,
      decimal_places: if base == "JPY" || quote == "JPY" { 3 } else { 5 },
    }
  }

  /// Returns `true` if this is one of the 7 major forex pairs
  /// (EURUSD, USDJPY, GBPUSD, USDCHF, AUDUSD, USDCAD, NZDUSD).
  pub fn is_major(&self) -> bool {
    let majors = ["EURUSD", "USDJPY", "GBPUSD", "USDCHF", "AUDUSD", "USDCAD", "NZDUSD"];
    majors.contains(&self.symbol.as_str())
  }

  /// Returns `true` if neither currency in the pair is USD.
  pub fn is_cross(&self) -> bool {
    self.base_currency != "USD" && self.quote_currency != "USD"
  }

  /// Returns the reciprocal pair (e.g., EUR/USD → USD/EUR).
  pub fn inverse(&self) -> Self {
    CurrencyPair::new(&self.quote_currency, &self.base_currency)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use av_core::test_utils::{DEFAULT_TOLERANCE, assert_approx_eq};
  #[test]
  fn test_exchange_rate_calculations() {
    let rate_data = ExchangeRateData {
      from_currency_code: "EUR".to_string(),
      from_currency_name: "Euro".to_string(),
      to_currency_code: "USD".to_string(),
      to_currency_name: "United States Dollar".to_string(),
      exchange_rate: "1.0850".to_string(),
      last_refreshed: "2024-01-15 16:00:00".to_string(),
      time_zone: "UTC".to_string(),
      bid_price: "1.0849".to_string(),
      ask_price: "1.0851".to_string(),
    };

    assert_eq!(rate_data.rate_as_f64().unwrap(), 1.0850);
    assert_eq!(rate_data.bid_as_f64().unwrap(), 1.0849);
    assert_eq!(rate_data.ask_as_f64().unwrap(), 1.0851);
    assert_approx_eq(rate_data.spread().unwrap(), 0.0002, DEFAULT_TOLERANCE);
    assert_eq!(rate_data.pair_symbol(), "EURUSD");

    let spread_pct = rate_data.spread_percentage().unwrap();
    assert!((spread_pct - 0.0184).abs() < 0.001); // Approximately 0.0184%
  }

  #[test]
  fn test_currency_pair() {
    let pair = CurrencyPair::new("EUR", "USD");

    assert_eq!(pair.base_currency, "EUR");
    assert_eq!(pair.quote_currency, "USD");
    assert_eq!(pair.symbol, "EURUSD");
    assert_eq!(pair.display_name, "EUR/USD");
    assert_eq!(pair.decimal_places, 5);
    assert!(pair.is_major());
    assert!(!pair.is_cross());

    let inverse = pair.inverse();
    assert_eq!(inverse.symbol, "USDEUR");
  }

  #[test]
  fn test_jpy_pair_decimals() {
    let pair = CurrencyPair::new("USD", "JPY");
    assert_eq!(pair.decimal_places, 3);
  }
}
