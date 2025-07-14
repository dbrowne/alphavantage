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
