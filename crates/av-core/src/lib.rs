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
//
