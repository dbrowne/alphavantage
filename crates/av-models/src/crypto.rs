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

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cryptocurrency exchange rate response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchangeRate {
  /// Realtime currency exchange rate
  #[serde(rename = "Realtime Currency Exchange Rate")]
  pub realtime_currency_exchange_rate: CryptoExchangeRateData,
}

/// Cryptocurrency exchange rate data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchangeRateData {
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

  /// Last refreshed
  #[serde(rename = "6. Last Refreshed")]
  pub last_refreshed: String,

  /// Time zone
  #[serde(rename = "7. Time Zone")]
  pub time_zone: String,

  /// Bid price
  #[serde(rename = "8. Bid Price")]
  pub bid_price: String,

  /// Ask price
  #[serde(rename = "9. Ask Price")]
  pub ask_price: String,
}

/// Cryptocurrency intraday time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoIntraday {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: CryptoMetadata,

  /// Time series data
  #[serde(flatten)]
  pub time_series: BTreeMap<String, CryptoOhlcvData>,
}

/// Cryptocurrency daily time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoDaily {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: CryptoMetadata,

  /// Time series data
  #[serde(rename = "Time Series (Digital Currency Daily)")]
  pub time_series: BTreeMap<String, CryptoOhlcvData>,
}

/// Cryptocurrency weekly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoWeekly {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: CryptoMetadata,

  /// Time series data
  #[serde(rename = "Time Series (Digital Currency Weekly)")]
  pub time_series: BTreeMap<String, CryptoOhlcvData>,
}

/// Cryptocurrency monthly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoMonthly {
  /// Metadata
  #[serde(rename = "Meta Data")]
  pub meta_data: CryptoMetadata,

  /// Time series data
  #[serde(rename = "Time Series (Digital Currency Monthly)")]
  pub time_series: BTreeMap<String, CryptoOhlcvData>,
}

/// Cryptocurrency OHLCV data with USD values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoOhlcvData {
  /// Open price in USD
  #[serde(rename = "1a. open (USD)")]
  pub open_usd: String,

  /// High price in USD
  #[serde(rename = "2a. high (USD)")]
  pub high_usd: String,

  /// Low price in USD
  #[serde(rename = "3a. low (USD)")]
  pub low_usd: String,

  /// Close price in USD
  #[serde(rename = "4a. close (USD)")]
  pub close_usd: String,

  /// Trading volume
  #[serde(rename = "5. volume")]
  pub volume: String,

  /// Market capitalization in USD
  #[serde(rename = "6. market cap (USD)")]
  pub market_cap_usd: String,
}

/// Cryptocurrency metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoMetadata {
  /// Information
  #[serde(rename = "1. Information")]
  pub information: String,

  /// Digital currency code
  #[serde(rename = "2. Digital Currency Code")]
  pub digital_currency_code: String,

  /// Digital currency name
  #[serde(rename = "3. Digital Currency Name")]
  pub digital_currency_name: String,

  /// Market code
  #[serde(rename = "4. Market Code")]
  pub market_code: String,

  /// Market name
  #[serde(rename = "5. Market Name")]
  pub market_name: String,

  /// Last refreshed
  #[serde(rename = "6. Last Refreshed")]
  pub last_refreshed: String,

  /// Interval (for intraday data)
  #[serde(rename = "7. Interval", skip_serializing_if = "Option::is_none")]
  pub interval: Option<String>,

  /// Output size (for intraday data)
  #[serde(rename = "8. Output Size", skip_serializing_if = "Option::is_none")]
  pub output_size: Option<String>,

  /// Time zone
  #[serde(rename = "9. Time Zone")]
  pub time_zone: String,
}

impl CryptoOhlcvData {
  /// Parse close price as f64
  pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.close_usd.parse()
  }

  /// Parse volume as f64
  pub fn volume_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.volume.parse()
  }

  /// Parse market cap as f64
  pub fn market_cap_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.market_cap_usd.parse()
  }

  /// Calculate price change percentage from open to close
  pub fn price_change_percent(&self) -> Result<f64, std::num::ParseFloatError> {
    let open: f64 = self.open_usd.parse()?;
    let close: f64 = self.close_usd.parse()?;

    if open == 0.0 { Ok(0.0) } else { Ok(((close - open) / open) * 100.0) }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_exchange_rate_json() -> &'static str {
    r#"{
      "Realtime Currency Exchange Rate": {
        "1. From_Currency Code": "BTC",
        "2. From_Currency Name": "Bitcoin",
        "3. To_Currency Code": "USD",
        "4. To_Currency Name": "United States Dollar",
        "5. Exchange Rate": "43250.50000000",
        "6. Last Refreshed": "2025-01-15 12:30:00",
        "7. Time Zone": "UTC",
        "8. Bid Price": "43248.00000000",
        "9. Ask Price": "43253.00000000"
      }
    }"#
  }

  fn sample_crypto_daily_json() -> &'static str {
    r#"{
      "Meta Data": {
        "1. Information": "Daily Prices and Volumes for Digital Currency",
        "2. Digital Currency Code": "BTC",
        "3. Digital Currency Name": "Bitcoin",
        "4. Market Code": "USD",
        "5. Market Name": "United States Dollar",
        "6. Last Refreshed": "2025-01-15 00:00:00",
        "9. Time Zone": "UTC"
      },
      "Time Series (Digital Currency Daily)": {
        "2025-01-15": {
          "1a. open (USD)": "42500.00000000",
          "2a. high (USD)": "44000.00000000",
          "3a. low (USD)": "42000.00000000",
          "4a. close (USD)": "43250.50000000",
          "5. volume": "15234.56789000",
          "6. market cap (USD)": "850000000000.00"
        },
        "2025-01-14": {
          "1a. open (USD)": "41000.00000000",
          "2a. high (USD)": "43000.00000000",
          "3a. low (USD)": "40500.00000000",
          "4a. close (USD)": "42500.00000000",
          "5. volume": "14000.00000000",
          "6. market cap (USD)": "830000000000.00"
        }
      }
    }"#
  }

  fn sample_ohlcv_data() -> CryptoOhlcvData {
    CryptoOhlcvData {
      open_usd: "42500.00".to_string(),
      high_usd: "44000.00".to_string(),
      low_usd: "42000.00".to_string(),
      close_usd: "43250.50".to_string(),
      volume: "15234.56789".to_string(),
      market_cap_usd: "850000000000.00".to_string(),
    }
  }

  // CryptoExchangeRate tests
  #[test]
  fn test_crypto_exchange_rate_deserialize() {
    let json = sample_exchange_rate_json();
    let rate: CryptoExchangeRate = serde_json::from_str(json).unwrap();

    assert_eq!(rate.realtime_currency_exchange_rate.from_currency_code, "BTC");
    assert_eq!(rate.realtime_currency_exchange_rate.from_currency_name, "Bitcoin");
    assert_eq!(rate.realtime_currency_exchange_rate.to_currency_code, "USD");
    assert_eq!(rate.realtime_currency_exchange_rate.to_currency_name, "United States Dollar");
    assert_eq!(rate.realtime_currency_exchange_rate.exchange_rate, "43250.50000000");
    assert_eq!(rate.realtime_currency_exchange_rate.time_zone, "UTC");
    assert_eq!(rate.realtime_currency_exchange_rate.bid_price, "43248.00000000");
    assert_eq!(rate.realtime_currency_exchange_rate.ask_price, "43253.00000000");
  }

  #[test]
  fn test_crypto_exchange_rate_serialize_roundtrip() {
    let json = sample_exchange_rate_json();
    let rate: CryptoExchangeRate = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&rate).unwrap();
    let deserialized: CryptoExchangeRate = serde_json::from_str(&serialized).unwrap();

    assert_eq!(rate, deserialized);
  }

  #[test]
  fn test_crypto_exchange_rate_clone() {
    let json = sample_exchange_rate_json();
    let rate: CryptoExchangeRate = serde_json::from_str(json).unwrap();
    let cloned = rate.clone();

    assert_eq!(rate, cloned);
  }

  #[test]
  fn test_crypto_exchange_rate_debug() {
    let json = sample_exchange_rate_json();
    let rate: CryptoExchangeRate = serde_json::from_str(json).unwrap();
    let debug_str = format!("{:?}", rate);

    assert!(debug_str.contains("CryptoExchangeRate"));
    assert!(debug_str.contains("BTC"));
  }

  // CryptoDaily tests
  #[test]
  fn test_crypto_daily_deserialize() {
    let json = sample_crypto_daily_json();
    let daily: CryptoDaily = serde_json::from_str(json).unwrap();

    assert_eq!(daily.meta_data.digital_currency_code, "BTC");
    assert_eq!(daily.meta_data.digital_currency_name, "Bitcoin");
    assert_eq!(daily.meta_data.market_code, "USD");
    assert_eq!(daily.time_series.len(), 2);

    let jan_15 = daily.time_series.get("2025-01-15").unwrap();
    assert_eq!(jan_15.open_usd, "42500.00000000");
    assert_eq!(jan_15.close_usd, "43250.50000000");
  }

  #[test]
  fn test_crypto_daily_serialize_roundtrip() {
    let json = sample_crypto_daily_json();
    let daily: CryptoDaily = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&daily).unwrap();
    let deserialized: CryptoDaily = serde_json::from_str(&serialized).unwrap();

    assert_eq!(daily, deserialized);
  }

  #[test]
  fn test_crypto_daily_time_series_ordering() {
    let json = sample_crypto_daily_json();
    let daily: CryptoDaily = serde_json::from_str(json).unwrap();

    // BTreeMap maintains sorted order
    let dates: Vec<&String> = daily.time_series.keys().collect();
    assert_eq!(dates[0], "2025-01-14");
    assert_eq!(dates[1], "2025-01-15");
  }

  // CryptoMetadata tests
  #[test]
  fn test_crypto_metadata_deserialize() {
    let json = r#"{
      "1. Information": "Daily Prices",
      "2. Digital Currency Code": "ETH",
      "3. Digital Currency Name": "Ethereum",
      "4. Market Code": "USD",
      "5. Market Name": "United States Dollar",
      "6. Last Refreshed": "2025-01-15",
      "9. Time Zone": "UTC"
    }"#;

    let metadata: CryptoMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(metadata.digital_currency_code, "ETH");
    assert_eq!(metadata.digital_currency_name, "Ethereum");
    assert!(metadata.interval.is_none());
    assert!(metadata.output_size.is_none());
  }

  #[test]
  fn test_crypto_metadata_with_optional_fields() {
    let json = r#"{
      "1. Information": "Intraday Prices",
      "2. Digital Currency Code": "BTC",
      "3. Digital Currency Name": "Bitcoin",
      "4. Market Code": "USD",
      "5. Market Name": "United States Dollar",
      "6. Last Refreshed": "2025-01-15 12:00:00",
      "7. Interval": "5min",
      "8. Output Size": "Compact",
      "9. Time Zone": "UTC"
    }"#;

    let metadata: CryptoMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(metadata.interval, Some("5min".to_string()));
    assert_eq!(metadata.output_size, Some("Compact".to_string()));
  }

  // CryptoOhlcvData tests
  #[test]
  fn test_crypto_ohlcv_data_deserialize() {
    let json = r#"{
      "1a. open (USD)": "42500.00",
      "2a. high (USD)": "44000.00",
      "3a. low (USD)": "42000.00",
      "4a. close (USD)": "43250.50",
      "5. volume": "15234.56789",
      "6. market cap (USD)": "850000000000.00"
    }"#;

    let data: CryptoOhlcvData = serde_json::from_str(json).unwrap();
    assert_eq!(data.open_usd, "42500.00");
    assert_eq!(data.high_usd, "44000.00");
    assert_eq!(data.low_usd, "42000.00");
    assert_eq!(data.close_usd, "43250.50");
    assert_eq!(data.volume, "15234.56789");
    assert_eq!(data.market_cap_usd, "850000000000.00");
  }

  #[test]
  fn test_crypto_ohlcv_close_as_f64() {
    let data = sample_ohlcv_data();
    let close = data.close_as_f64().unwrap();
    assert!((close - 43250.50).abs() < 0.01);
  }

  #[test]
  fn test_crypto_ohlcv_volume_as_f64() {
    let data = sample_ohlcv_data();
    let volume = data.volume_as_f64().unwrap();
    assert!((volume - 15234.56789).abs() < 0.00001);
  }

  #[test]
  fn test_crypto_ohlcv_market_cap_as_f64() {
    let data = sample_ohlcv_data();
    let market_cap = data.market_cap_as_f64().unwrap();
    assert!((market_cap - 850000000000.00).abs() < 1.0);
  }

  #[test]
  fn test_crypto_ohlcv_price_change_percent_positive() {
    let data = sample_ohlcv_data();
    let change = data.price_change_percent().unwrap();
    // (43250.50 - 42500.00) / 42500.00 * 100 = 1.766%
    assert!((change - 1.766).abs() < 0.01);
  }

  #[test]
  fn test_crypto_ohlcv_price_change_percent_negative() {
    let data = CryptoOhlcvData {
      open_usd: "44000.00".to_string(),
      high_usd: "44500.00".to_string(),
      low_usd: "42000.00".to_string(),
      close_usd: "42500.00".to_string(),
      volume: "10000.00".to_string(),
      market_cap_usd: "800000000000.00".to_string(),
    };
    let change = data.price_change_percent().unwrap();
    // (42500 - 44000) / 44000 * 100 = -3.409%
    assert!(change < 0.0);
    assert!((change - (-3.409)).abs() < 0.01);
  }

  #[test]
  fn test_crypto_ohlcv_price_change_percent_zero_open() {
    let data = CryptoOhlcvData {
      open_usd: "0.0".to_string(),
      high_usd: "100.00".to_string(),
      low_usd: "0.00".to_string(),
      close_usd: "50.00".to_string(),
      volume: "1000.00".to_string(),
      market_cap_usd: "1000000.00".to_string(),
    };
    let change = data.price_change_percent().unwrap();
    assert_eq!(change, 0.0);
  }

  #[test]
  fn test_crypto_ohlcv_parse_error() {
    let data = CryptoOhlcvData {
      open_usd: "not_a_number".to_string(),
      high_usd: "100.00".to_string(),
      low_usd: "50.00".to_string(),
      close_usd: "75.00".to_string(),
      volume: "1000.00".to_string(),
      market_cap_usd: "1000000.00".to_string(),
    };
    let result = data.price_change_percent();
    assert!(result.is_err());
  }

  #[test]
  fn test_crypto_ohlcv_clone() {
    let data = sample_ohlcv_data();
    let cloned = data.clone();
    assert_eq!(data, cloned);
  }

  #[test]
  fn test_crypto_ohlcv_debug() {
    let data = sample_ohlcv_data();
    let debug_str = format!("{:?}", data);
    assert!(debug_str.contains("CryptoOhlcvData"));
    assert!(debug_str.contains("42500.00"));
  }

  // CryptoWeekly tests
  #[test]
  fn test_crypto_weekly_structure() {
    let json = r#"{
      "Meta Data": {
        "1. Information": "Weekly Prices",
        "2. Digital Currency Code": "BTC",
        "3. Digital Currency Name": "Bitcoin",
        "4. Market Code": "USD",
        "5. Market Name": "United States Dollar",
        "6. Last Refreshed": "2025-01-15",
        "9. Time Zone": "UTC"
      },
      "Time Series (Digital Currency Weekly)": {
        "2025-01-12": {
          "1a. open (USD)": "40000.00",
          "2a. high (USD)": "45000.00",
          "3a. low (USD)": "39000.00",
          "4a. close (USD)": "43000.00",
          "5. volume": "50000.00",
          "6. market cap (USD)": "840000000000.00"
        }
      }
    }"#;

    let weekly: CryptoWeekly = serde_json::from_str(json).unwrap();
    assert_eq!(weekly.meta_data.digital_currency_code, "BTC");
    assert_eq!(weekly.time_series.len(), 1);
  }

  // CryptoMonthly tests
  #[test]
  fn test_crypto_monthly_structure() {
    let json = r#"{
      "Meta Data": {
        "1. Information": "Monthly Prices",
        "2. Digital Currency Code": "ETH",
        "3. Digital Currency Name": "Ethereum",
        "4. Market Code": "USD",
        "5. Market Name": "United States Dollar",
        "6. Last Refreshed": "2025-01-01",
        "9. Time Zone": "UTC"
      },
      "Time Series (Digital Currency Monthly)": {
        "2025-01-01": {
          "1a. open (USD)": "2500.00",
          "2a. high (USD)": "3000.00",
          "3a. low (USD)": "2400.00",
          "4a. close (USD)": "2800.00",
          "5. volume": "100000.00",
          "6. market cap (USD)": "340000000000.00"
        }
      }
    }"#;

    let monthly: CryptoMonthly = serde_json::from_str(json).unwrap();
    assert_eq!(monthly.meta_data.digital_currency_code, "ETH");
    assert_eq!(monthly.meta_data.digital_currency_name, "Ethereum");
    assert_eq!(monthly.time_series.len(), 1);
  }

  // PartialEq tests
  #[test]
  fn test_crypto_ohlcv_partial_eq() {
    let data1 = sample_ohlcv_data();
    let data2 = sample_ohlcv_data();
    let data3 = CryptoOhlcvData { open_usd: "99999.00".to_string(), ..sample_ohlcv_data() };

    assert_eq!(data1, data2);
    assert_ne!(data1, data3);
  }

  #[test]
  fn test_crypto_metadata_partial_eq() {
    let meta1 = CryptoMetadata {
      information: "Daily".to_string(),
      digital_currency_code: "BTC".to_string(),
      digital_currency_name: "Bitcoin".to_string(),
      market_code: "USD".to_string(),
      market_name: "United States Dollar".to_string(),
      last_refreshed: "2025-01-15".to_string(),
      interval: None,
      output_size: None,
      time_zone: "UTC".to_string(),
    };
    let meta2 = meta1.clone();

    assert_eq!(meta1, meta2);
  }
}
