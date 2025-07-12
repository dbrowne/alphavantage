//! # av-core
//!
//! Core types and traits for the AlphaVantage Rust client ecosystem.
//!
//! This crate provides the fundamental building blocks used across all av-* crates,
//! including error types, configuration, and common data structures.

pub mod config;
pub mod error;
pub mod types;

pub use config::Config;
pub use error::{Error, Result};

/// The current supported AlphaVantage API functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FuncType {
  /// Time Series Intraday
  TsIntra,
  /// Time Series Daily
  TsDaily,
  /// Company Overview
  Overview,
  /// Symbol Search
  SymSearch,
  /// Top Gainers/Losers
  TopQuery,
  /// News & Sentiment
  NewsQuery,

  CryptoIntraDay,
}

/// Base URL for AlphaVantage API
pub const ALPHA_VANTAGE_BASE_URL: &str = "https://www.alphavantage.co/query";

/// API rate limits
pub const DEFAULT_RATE_LIMIT: u32 = 75; // requests per minute
pub const PREMIUM_RATE_LIMIT: u32 = 600; // requests per minute

/// `create_url!` is a macro used for constructing request URLs to various endpoints of the
/// AlphaVantage API. It is necessary because macros run before name resolution
/// see https://github.com/rust-lang/rust/issues/69133 for more details
///
/// This macro takes a `FuncType`, which denotes the AlphaVantage API endpoint to construct a URL
/// for, and two expression parameters representing the symbol and API key. The order of the
/// expression parameters is always: symbol then API key.
///
/// The available `FuncType`s are:
///
/// `TsIntraExt`: Constructs a URL for the TIME_SERIES_INTRADAY_EXTENDED endpoint.
/// `TsDaily`: Constructs a URL for the TIME_SERIES_DAILY endpoint.
/// `Overview`: Constructs a URL for the OVERVIEW endpoint.
/// `SymSearch`: Constructs a URL for the SYMBOL_SEARCH endpoint.
///
/// # Example
///
///
/// let url = create_url!(FuncType:TsDaily, "AAPL", "demo");
/// assert_eq!(url, "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&datatype=json&symbol=AAPL&apikey=demo");
///
///
/// If an unrecognized `FuncType` is passed, it returns a string saying "Unknown function type
/// received".
///
/// # Panics
///
/// This macro does not panic.
#[macro_export]
macro_rules! create_url {
  (FuncType::TsIntra, $symbol:expr, $api_key:expr) => {
    format!(
      "{}?function=TIME_SERIES_INTRADAY&datatype=csv&symbol={}&interval=1min&apikey={}",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $symbol,
      $api_key
    )
  };
  (FuncType::TsDaily, $symbol:expr, $api_key:expr) => {
    format!(
      "{}?function=TIME_SERIES_DAILY&datatype=json&symbol={}&apikey={}",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $symbol,
      $api_key
    )
  };
  (FuncType::Overview, $symbol:expr, $api_key:expr) => {
    format!(
      "{}?function=OVERVIEW&symbol={}&apikey={}",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $symbol,
      $api_key
    )
  };
  (FuncType::SymSearch, $keywords:expr, $api_key:expr) => {
    format!(
      "{}?function=SYMBOL_SEARCH&keywords={}&apikey={}&datatype=csv",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $keywords,
      $api_key
    )
  };
  (FuncType::TopQuery, $_:expr, $api_key:expr) => {
    format!("{}?function=TOP_GAINERS_LOSERS&apikey={}", $crate::ALPHA_VANTAGE_BASE_URL, $api_key)
  };
  (FuncType::NewsQuery, $symbol:expr, $api_key:expr) => {
    format!(
      "{}?function=NEWS_SENTIMENT&tickers={}&apikey={}",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $symbol,
      $api_key
    )
  };
  (FuncType::CryptoIntraDay, $symbol:expr, $api_key:expr) => {
    format!(
      "{}?function=CRYPTO_INTRADAY&symbol={}&market=USD&interval=1min&apikey={}&datatype=csv",
      $crate::ALPHA_VANTAGE_BASE_URL,
      $symbol,
      $api_key
    )
  };
  ($other:expr,$string1:expr, $string2:expr) => {
    format!("Unknown function type received {:?}", $other)
  };
}

#[cfg(test)]
mod test {
  #[test]
  fn t_01() {
    let (sym, api_key) = ("AAPL", "123456789");
    let url = create_url!(FuncType::TsIntra, sym, api_key);
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=TIME_SERIES_INTRADAY&datatype=csv&symbol=AAPL&interval=1min&apikey=123456789"
    );
  }

  #[test]
  fn t_02() {
    let (sym, api_key) = ("AAPL", "123456789");
    let url = create_url!(FuncType::TsDaily, sym, api_key);
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&datatype=json&symbol=AAPL&apikey=123456789"
    );
  }

  #[test]
  fn t_03() {
    let (sym, api_key) = ("AAPL", "123456789");
    let url = create_url!(FuncType::Overview, sym, api_key);
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=OVERVIEW&symbol=AAPL&apikey=123456789"
    );
  }

  #[test]
  fn t_04() {
    let (sym, api_key) = ("AAPL", "123456789");
    let url = create_url!(FuncType::SymSearch, sym, api_key);
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=SYMBOL_SEARCH&keywords=AAPL&apikey=123456789&datatype=csv"
    );
  }

  #[test]
  fn t_05() {
    let url = create_url!(55, "AAPL", "123456789");
    assert_eq!(url, "Unknown function type received 55");
  }

  #[test]
  fn t_06() {
    let url = create_url!(FuncType::TopQuery, "NONE", "12345678");
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=TOP_GAINERS_LOSERS&apikey=12345678"
    );
  }

  #[test]
  fn t_09() {
    let url = create_url!(FuncType::NewsQuery, "AAPL", "12345678");
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=NEWS_SENTIMENT&tickers=AAPL&apikey=12345678"
    );
  }
  #[test]
  fn t_10() {
    let url = create_url!(FuncType::CryptoIntraDay, "BTC", "12345678");
    assert_eq!(
      url,
      "https://www.alphavantage.co/query?function=CRYPTO_INTRADAY&symbol=BTC&market=USD&interval=1min&apikey=12345678&datatype=csv"
    );
  }
}
