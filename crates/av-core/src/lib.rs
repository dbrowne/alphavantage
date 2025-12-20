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

//! # av-core
//!
//! Core types, configuration, and error handling for the AlphaVantage Rust client.
//!
//! This crate provides the foundational components shared across all AlphaVantage crates:
//!
//! - [`Config`] - API configuration (key, rate limits, timeouts)
//! - [`Error`] and [`Result`] - Unified error handling
//! - [`FuncType`] - Type-safe API function identifiers
//!
//! ## Example
//!
//! ```
//! use av_core::{Config, FuncType};
//!
//! let config = Config::default_with_key("your_api_key".to_string());
//! let function = FuncType::TimeSeriesDaily;
//! ```

pub mod config;
pub mod error;
pub mod types;
pub use config::Config;
pub use error::{Error, Result};

/// The current supported AlphaVantage API functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FuncType {
  // Time Series functions
  TimeSeriesIntraday,
  TimeSeriesDaily,
  TimeSeriesDailyAdjusted,
  TimeSeriesWeekly,
  TimeSeriesWeeklyAdjusted,
  TimeSeriesMonthly,
  TimeSeriesMonthlyAdjusted,

  // Fundamentals functions
  Overview,
  IncomeStatement,
  BalanceSheet,
  CashFlow,
  Earnings,
  TopGainersLosers,
  ListingStatus,
  EarningsCalendar,
  IpoCalendar,

  // News functions
  NewsSentiment,

  // Forex functions
  CurrencyExchangeRate,
  FxIntraday,
  FxDaily,
  FxWeekly,
  FxMonthly,

  // Crypto functions
  CryptoExchangeRate,
  CryptoIntraday, // Note: this was CryptoIntraDay in the original, fixing the typo
  CryptoDaily,
  CryptoWeekly,
  CryptoMonthly,

  // Market status and search
  MarketStatus,
  SymbolSearch,

  // Legacy support
  TsIntra,
  TsDaily,
  SymSearch,
  TopQuery,
  NewsQuery,
  CryptoIntraDay, // Keep for backward compatibility
}

// Implement Display trait for FuncType
impl std::fmt::Display for FuncType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      // Time Series functions
      FuncType::TimeSeriesIntraday => write!(f, "TIME_SERIES_INTRADAY"),
      FuncType::TimeSeriesDaily => write!(f, "TIME_SERIES_DAILY"),
      FuncType::TimeSeriesDailyAdjusted => write!(f, "TIME_SERIES_DAILY_ADJUSTED"),
      FuncType::TimeSeriesWeekly => write!(f, "TIME_SERIES_WEEKLY"),
      FuncType::TimeSeriesWeeklyAdjusted => write!(f, "TIME_SERIES_WEEKLY_ADJUSTED"),
      FuncType::TimeSeriesMonthly => write!(f, "TIME_SERIES_MONTHLY"),
      FuncType::TimeSeriesMonthlyAdjusted => write!(f, "TIME_SERIES_MONTHLY_ADJUSTED"),

      // Fundamentals functions
      FuncType::Overview => write!(f, "OVERVIEW"),
      FuncType::IncomeStatement => write!(f, "INCOME_STATEMENT"),
      FuncType::BalanceSheet => write!(f, "BALANCE_SHEET"),
      FuncType::CashFlow => write!(f, "CASH_FLOW"),
      FuncType::Earnings => write!(f, "EARNINGS"),
      FuncType::TopGainersLosers => write!(f, "TOP_GAINERS_LOSERS"),
      FuncType::ListingStatus => write!(f, "LISTING_STATUS"),
      FuncType::EarningsCalendar => write!(f, "EARNINGS_CALENDAR"),
      FuncType::IpoCalendar => write!(f, "IPO_CALENDAR"),

      // News functions
      FuncType::NewsSentiment => write!(f, "NEWS_SENTIMENT"),

      // Forex and Crypto exchange rates (both use the same AlphaVantage endpoint)
      FuncType::CurrencyExchangeRate | FuncType::CryptoExchangeRate => {
        write!(f, "CURRENCY_EXCHANGE_RATE")
      }

      // Forex functions
      FuncType::FxIntraday => write!(f, "FX_INTRADAY"),
      FuncType::FxDaily => write!(f, "FX_DAILY"),
      FuncType::FxWeekly => write!(f, "FX_WEEKLY"),
      FuncType::FxMonthly => write!(f, "FX_MONTHLY"),

      // Crypto functions
      FuncType::CryptoIntraday | FuncType::CryptoIntraDay => write!(f, "CRYPTO_INTRADAY"),
      FuncType::CryptoDaily => write!(f, "DIGITAL_CURRENCY_DAILY"),
      FuncType::CryptoWeekly => write!(f, "DIGITAL_CURRENCY_WEEKLY"),
      FuncType::CryptoMonthly => write!(f, "DIGITAL_CURRENCY_MONTHLY"),

      // Market status and search
      FuncType::MarketStatus => write!(f, "MARKET_STATUS"),
      FuncType::SymbolSearch => write!(f, "SYMBOL_SEARCH"),

      // Legacy support
      FuncType::TsIntra => write!(f, "TIME_SERIES_INTRADAY"),
      FuncType::TsDaily => write!(f, "TIME_SERIES_DAILY"),
      FuncType::SymSearch => write!(f, "SYMBOL_SEARCH"),
      FuncType::TopQuery => write!(f, "TOP_GAINERS_LOSERS"),
      FuncType::NewsQuery => write!(f, "NEWS_SENTIMENT"),
    }
  }
}

/// Base URL for AlphaVantage API
pub const ALPHA_VANTAGE_BASE_URL: &str = "https://www.alphavantage.co/query";

/// API rate limits
pub const DEFAULT_RATE_LIMIT: u32 = 75; // requests per minute
pub const PREMIUM_RATE_LIMIT: u32 = 600; // requests per minute

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod tests {
  use super::*;

  // Time Series function tests
  #[test]
  fn test_func_type_time_series_intraday() {
    assert_eq!(FuncType::TimeSeriesIntraday.to_string(), "TIME_SERIES_INTRADAY");
  }

  #[test]
  fn test_func_type_time_series_daily() {
    assert_eq!(FuncType::TimeSeriesDaily.to_string(), "TIME_SERIES_DAILY");
  }

  #[test]
  fn test_func_type_time_series_daily_adjusted() {
    assert_eq!(FuncType::TimeSeriesDailyAdjusted.to_string(), "TIME_SERIES_DAILY_ADJUSTED");
  }

  #[test]
  fn test_func_type_time_series_weekly() {
    assert_eq!(FuncType::TimeSeriesWeekly.to_string(), "TIME_SERIES_WEEKLY");
  }

  #[test]
  fn test_func_type_time_series_weekly_adjusted() {
    assert_eq!(FuncType::TimeSeriesWeeklyAdjusted.to_string(), "TIME_SERIES_WEEKLY_ADJUSTED");
  }

  #[test]
  fn test_func_type_time_series_monthly() {
    assert_eq!(FuncType::TimeSeriesMonthly.to_string(), "TIME_SERIES_MONTHLY");
  }

  #[test]
  fn test_func_type_time_series_monthly_adjusted() {
    assert_eq!(FuncType::TimeSeriesMonthlyAdjusted.to_string(), "TIME_SERIES_MONTHLY_ADJUSTED");
  }

  // Fundamentals function tests
  #[test]
  fn test_func_type_overview() {
    assert_eq!(FuncType::Overview.to_string(), "OVERVIEW");
  }

  #[test]
  fn test_func_type_income_statement() {
    assert_eq!(FuncType::IncomeStatement.to_string(), "INCOME_STATEMENT");
  }

  #[test]
  fn test_func_type_balance_sheet() {
    assert_eq!(FuncType::BalanceSheet.to_string(), "BALANCE_SHEET");
  }

  #[test]
  fn test_func_type_cash_flow() {
    assert_eq!(FuncType::CashFlow.to_string(), "CASH_FLOW");
  }

  #[test]
  fn test_func_type_earnings() {
    assert_eq!(FuncType::Earnings.to_string(), "EARNINGS");
  }

  #[test]
  fn test_func_type_top_gainers_losers() {
    assert_eq!(FuncType::TopGainersLosers.to_string(), "TOP_GAINERS_LOSERS");
  }

  #[test]
  fn test_func_type_listing_status() {
    assert_eq!(FuncType::ListingStatus.to_string(), "LISTING_STATUS");
  }

  #[test]
  fn test_func_type_earnings_calendar() {
    assert_eq!(FuncType::EarningsCalendar.to_string(), "EARNINGS_CALENDAR");
  }

  #[test]
  fn test_func_type_ipo_calendar() {
    assert_eq!(FuncType::IpoCalendar.to_string(), "IPO_CALENDAR");
  }

  // News function tests
  #[test]
  fn test_func_type_news_sentiment() {
    assert_eq!(FuncType::NewsSentiment.to_string(), "NEWS_SENTIMENT");
  }

  // Forex function tests
  #[test]
  fn test_func_type_currency_exchange_rate() {
    assert_eq!(FuncType::CurrencyExchangeRate.to_string(), "CURRENCY_EXCHANGE_RATE");
  }

  #[test]
  fn test_func_type_fx_intraday() {
    assert_eq!(FuncType::FxIntraday.to_string(), "FX_INTRADAY");
  }

  #[test]
  fn test_func_type_fx_daily() {
    assert_eq!(FuncType::FxDaily.to_string(), "FX_DAILY");
  }

  #[test]
  fn test_func_type_fx_weekly() {
    assert_eq!(FuncType::FxWeekly.to_string(), "FX_WEEKLY");
  }

  #[test]
  fn test_func_type_fx_monthly() {
    assert_eq!(FuncType::FxMonthly.to_string(), "FX_MONTHLY");
  }

  // Crypto function tests
  #[test]
  fn test_func_type_crypto_exchange_rate() {
    assert_eq!(FuncType::CryptoExchangeRate.to_string(), "CURRENCY_EXCHANGE_RATE");
  }

  #[test]
  fn test_func_type_crypto_intraday() {
    assert_eq!(FuncType::CryptoIntraday.to_string(), "CRYPTO_INTRADAY");
  }

  #[test]
  fn test_func_type_crypto_intraday_legacy() {
    // Test backward compatibility variant
    assert_eq!(FuncType::CryptoIntraDay.to_string(), "CRYPTO_INTRADAY");
  }

  #[test]
  fn test_func_type_crypto_daily() {
    assert_eq!(FuncType::CryptoDaily.to_string(), "DIGITAL_CURRENCY_DAILY");
  }

  #[test]
  fn test_func_type_crypto_weekly() {
    assert_eq!(FuncType::CryptoWeekly.to_string(), "DIGITAL_CURRENCY_WEEKLY");
  }

  #[test]
  fn test_func_type_crypto_monthly() {
    assert_eq!(FuncType::CryptoMonthly.to_string(), "DIGITAL_CURRENCY_MONTHLY");
  }

  // Market status and search tests
  #[test]
  fn test_func_type_market_status() {
    assert_eq!(FuncType::MarketStatus.to_string(), "MARKET_STATUS");
  }

  #[test]
  fn test_func_type_symbol_search() {
    assert_eq!(FuncType::SymbolSearch.to_string(), "SYMBOL_SEARCH");
  }

  // Legacy support tests
  #[test]
  fn test_func_type_legacy_ts_intra() {
    assert_eq!(FuncType::TsIntra.to_string(), "TIME_SERIES_INTRADAY");
  }

  #[test]
  fn test_func_type_legacy_ts_daily() {
    assert_eq!(FuncType::TsDaily.to_string(), "TIME_SERIES_DAILY");
  }

  #[test]
  fn test_func_type_legacy_sym_search() {
    assert_eq!(FuncType::SymSearch.to_string(), "SYMBOL_SEARCH");
  }

  #[test]
  fn test_func_type_legacy_top_query() {
    assert_eq!(FuncType::TopQuery.to_string(), "TOP_GAINERS_LOSERS");
  }

  #[test]
  fn test_func_type_legacy_news_query() {
    assert_eq!(FuncType::NewsQuery.to_string(), "NEWS_SENTIMENT");
  }

  // FuncType trait tests
  #[test]
  fn test_func_type_clone() {
    let original = FuncType::TimeSeriesDaily;
    let cloned = original.clone();
    assert_eq!(original, cloned);
  }

  #[test]
  fn test_func_type_copy() {
    let original = FuncType::TimeSeriesDaily;
    let copied = original;
    assert_eq!(original, copied);
  }

  #[test]
  fn test_func_type_debug() {
    let func = FuncType::Overview;
    let debug_str = format!("{:?}", func);
    assert_eq!(debug_str, "Overview");
  }

  #[test]
  fn test_func_type_eq() {
    assert_eq!(FuncType::Overview, FuncType::Overview);
    assert_ne!(FuncType::Overview, FuncType::Earnings);
  }

  #[test]
  fn test_func_type_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(FuncType::Overview);
    set.insert(FuncType::Earnings);
    set.insert(FuncType::Overview); // duplicate
    assert_eq!(set.len(), 2);
  }

  // Constants tests
  #[test]
  fn test_alpha_vantage_base_url() {
    assert_eq!(ALPHA_VANTAGE_BASE_URL, "https://www.alphavantage.co/query");
    assert!(ALPHA_VANTAGE_BASE_URL.starts_with("https://"));
  }

  #[test]
  fn test_default_rate_limit() {
    assert_eq!(DEFAULT_RATE_LIMIT, 75);
  }

  #[test]
  fn test_premium_rate_limit() {
    assert_eq!(PREMIUM_RATE_LIMIT, 600);
    assert!(PREMIUM_RATE_LIMIT > DEFAULT_RATE_LIMIT);
  }
}
