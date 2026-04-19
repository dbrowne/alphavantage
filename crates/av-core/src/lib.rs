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
//! Core types, configuration, and error handling for the Alpha Vantage Rust client.
//!
//! `av-core` is the foundational crate in the `alphavantage` workspace. It owns the
//! types and infrastructure that every other crate in the workspace depends on,
//! but contains **no HTTP or networking logic** itself — that lives in sibling crates
//! that consume `av-core`.
//!
//! ## Crate contents
//!
//! | Export                            | Description                                                            |
//! |-----------------------------------|------------------------------------------------------------------------|
//! | [`Config`]                        | API configuration: key, rate limits, timeouts, base URL. Loadable from environment variables or constructed directly. |
//! | [`Error`] / [`Result`]            | Unified error enum covering env-var, config, serde, date-parse, rate-limit, and API response errors. |
//! | [`FuncType`]                      | Type-safe enum of all supported Alpha Vantage API function names (time series, fundamentals, news, forex, crypto, search). |
//! | [`types`]                         | Shared domain types: exchanges, security types, intervals, currencies, sentiment labels, and more. |
//! | [`ALPHA_VANTAGE_BASE_URL`]        | The canonical API endpoint (`https://www.alphavantage.co/query`).      |
//! | [`DEFAULT_RATE_LIMIT`] / [`PREMIUM_RATE_LIMIT`] | Request-per-minute caps for free (75) and premium (600) API tiers. |
//!
//! ## Feature flags
//!
//! | Flag          | Effect                                                            |
//! |---------------|-------------------------------------------------------------------|
//! | `test-utils`  | Enables the [`test_utils`] module with shared test helpers.       |
//!
//! ## Example
//!
//! ```rust
//! use av_core::{Config, FuncType};
//!
//! let config = Config::default_with_key("your_api_key".to_string());
//! let function = FuncType::TimeSeriesDaily;
//!
//! // The Display impl produces the API query-string value
//! assert_eq!(function.to_string(), "TIME_SERIES_DAILY");
//! ```
//!
//! ## Module layout
//!
//! ```text
//! av-core/src/
//! ├── lib.rs          ← this file: FuncType, constants, re-exports
//! ├── config.rs       → Config (API key, rate limit, timeout, retries, base URL)
//! ├── error.rs        → Error enum, Result type alias
//! ├── types/
//! │   ├── mod.rs      → re-export façade
//! │   ├── common.rs   → DataType, Interval, OutputSize, SortOrder, TimeHorizon,
//! │   │                  ListingState, SentimentLabel, CurrencyCode, CryptoSymbol
//! │   └── market/
//! │       ├── mod.rs            → re-export façade
//! │       ├── exchange.rs       → Exchange (25 global exchanges)
//! │       ├── security_type.rs  → SecurityType, SecurityIdentifier (bitmap encoding)
//! │       └── classifications.rs → TopType, Sector, MarketCap
//! └── test_utils.rs   → shared test helpers (feature-gated)
//! ```

/// API client configuration: key management, rate limits, timeouts, and retries.
///
/// See [`Config`] for the primary export.
pub mod config;

/// Unified error types for the crate.
///
/// See [`Error`] for the error enum and [`Result`] for the convenience alias.
pub mod error;

/// Shared domain types for exchanges, securities, intervals, currencies, and more.
///
/// The [`types`] module re-exports the most common types at its top level for
/// convenience. See [`types::common`] and [`types::market`] for the full inventory.
pub mod types;

// ─── Convenience re-exports ─────────────────────────────────────────────────
//
// These bring the three most-used items to the crate root so callers can write
// `use av_core::{Config, Error, Result}` without navigating sub-modules.

/// Re-exported from [`config`].
pub use config::Config;

/// Re-exported from [`error`].
pub use error::{Error, Result};

/// Type-safe identifiers for all supported Alpha Vantage API functions.
///
/// Each variant maps to a specific `function=` query-string value accepted by the
/// Alpha Vantage REST API. The [`Display`](std::fmt::Display) implementation
/// produces that exact string, so you can interpolate a `FuncType` directly into
/// a URL query.
///
/// # Variant groups
///
/// ## Time series (OHLCV price data)
///
/// | Variant                     | API function string                 | Description                              |
/// |-----------------------------|-------------------------------------|------------------------------------------|
/// | `TimeSeriesIntraday`        | `TIME_SERIES_INTRADAY`              | Intraday bars (requires [`Interval`](types::Interval)) |
/// | `TimeSeriesDaily`           | `TIME_SERIES_DAILY`                 | Daily unadjusted OHLCV                   |
/// | `TimeSeriesDailyAdjusted`   | `TIME_SERIES_DAILY_ADJUSTED`        | Daily split/dividend-adjusted data       |
/// | `TimeSeriesWeekly`          | `TIME_SERIES_WEEKLY`                | Weekly aggregated bars                   |
/// | `TimeSeriesWeeklyAdjusted`  | `TIME_SERIES_WEEKLY_ADJUSTED`       | Weekly adjusted bars                     |
/// | `TimeSeriesMonthly`         | `TIME_SERIES_MONTHLY`               | Monthly aggregated bars                  |
/// | `TimeSeriesMonthlyAdjusted` | `TIME_SERIES_MONTHLY_ADJUSTED`      | Monthly adjusted bars                    |
///
/// ## Fundamentals (company data & calendars)
///
/// | Variant              | API function string      | Description                              |
/// |----------------------|--------------------------|------------------------------------------|
/// | `Overview`           | `OVERVIEW`               | Company profile, key metrics, description|
/// | `IncomeStatement`    | `INCOME_STATEMENT`       | Annual & quarterly income statements     |
/// | `BalanceSheet`       | `BALANCE_SHEET`          | Annual & quarterly balance sheets        |
/// | `CashFlow`           | `CASH_FLOW`              | Annual & quarterly cash flow statements  |
/// | `Earnings`           | `EARNINGS`               | Annual & quarterly earnings (EPS)        |
/// | `TopGainersLosers`   | `TOP_GAINERS_LOSERS`     | Top movers by percent change             |
/// | `ListingStatus`      | `LISTING_STATUS`         | Active/delisted securities listing       |
/// | `EarningsCalendar`   | `EARNINGS_CALENDAR`      | Upcoming earnings dates                  |
/// | `IpoCalendar`        | `IPO_CALENDAR`           | Upcoming IPO dates                       |
///
/// ## News
///
/// | Variant         | API function string | Description                              |
/// |-----------------|---------------------|------------------------------------------|
/// | `NewsSentiment`  | `NEWS_SENTIMENT`   | News articles with sentiment scores      |
///
/// ## Forex
///
/// | Variant                | API function string        | Description                           |
/// |------------------------|----------------------------|---------------------------------------|
/// | `CurrencyExchangeRate` | `CURRENCY_EXCHANGE_RATE`   | Real-time currency pair exchange rate |
/// | `FxIntraday`           | `FX_INTRADAY`              | Intraday forex bars                   |
/// | `FxDaily`              | `FX_DAILY`                 | Daily forex bars                      |
/// | `FxWeekly`             | `FX_WEEKLY`                | Weekly forex bars                     |
/// | `FxMonthly`            | `FX_MONTHLY`               | Monthly forex bars                    |
///
/// ## Cryptocurrency
///
/// | Variant              | API function string          | Description                          |
/// |----------------------|------------------------------|--------------------------------------|
/// | `CryptoExchangeRate` | `CURRENCY_EXCHANGE_RATE`     | Real-time crypto exchange rate (shares endpoint with forex) |
/// | `CryptoIntraday`     | `CRYPTO_INTRADAY`            | Intraday crypto bars                 |
/// | `CryptoDaily`        | `DIGITAL_CURRENCY_DAILY`     | Daily crypto OHLCV                   |
/// | `CryptoWeekly`       | `DIGITAL_CURRENCY_WEEKLY`    | Weekly crypto OHLCV                  |
/// | `CryptoMonthly`      | `DIGITAL_CURRENCY_MONTHLY`   | Monthly crypto OHLCV                 |
///
/// ## Market status & search
///
/// | Variant        | API function string | Description                              |
/// |----------------|---------------------|------------------------------------------|
/// | `MarketStatus` | `MARKET_STATUS`     | Current open/closed state of global exchanges |
/// | `SymbolSearch`  | `SYMBOL_SEARCH`    | Search for securities by keyword         |
///
/// ## Legacy aliases
///
/// The following variants exist for backward compatibility with earlier versions of
/// the crate. They produce the same `Display` output as their modern equivalents
/// and should be avoided in new code:
///
/// | Legacy variant   | Equivalent to         |
/// |------------------|-----------------------|
/// | `TsIntra`        | `TimeSeriesIntraday`  |
/// | `TsDaily`        | `TimeSeriesDaily`     |
/// | `SymSearch`      | `SymbolSearch`        |
/// | `TopQuery`       | `TopGainersLosers`    |
/// | `NewsQuery`      | `NewsSentiment`       |
/// | `CryptoIntraDay` | `CryptoIntraday`      |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FuncType {
  // ── Time Series ───────────────────────────────────────────────────────
  /// Intraday OHLCV bars at a configurable [`Interval`](types::Interval).
  TimeSeriesIntraday,
  /// Daily unadjusted OHLCV.
  TimeSeriesDaily,
  /// Daily OHLCV adjusted for splits and dividends.
  TimeSeriesDailyAdjusted,
  /// Weekly aggregated OHLCV.
  TimeSeriesWeekly,
  /// Weekly OHLCV adjusted for splits and dividends.
  TimeSeriesWeeklyAdjusted,
  /// Monthly aggregated OHLCV.
  TimeSeriesMonthly,
  /// Monthly OHLCV adjusted for splits and dividends.
  TimeSeriesMonthlyAdjusted,

  // ── Fundamentals ──────────────────────────────────────────────────────
  /// Company overview: profile, key metrics, and description.
  Overview,
  /// Annual and quarterly income statements.
  IncomeStatement,
  /// Annual and quarterly balance sheets.
  BalanceSheet,
  /// Annual and quarterly cash flow statements.
  CashFlow,
  /// Annual and quarterly EPS (earnings per share).
  Earnings,
  /// Top gainers, losers, and most actively traded tickers.
  TopGainersLosers,
  /// Active and delisted securities listing.
  ListingStatus,
  /// Upcoming earnings announcement dates.
  EarningsCalendar,
  /// Upcoming IPO dates.
  IpoCalendar,

  // ── News ──────────────────────────────────────────────────────────────
  /// News articles with ticker-level and topic-level sentiment scores.
  NewsSentiment,

  // ── Forex ─────────────────────────────────────────────────────────────
  /// Real-time exchange rate for a fiat currency pair.
  CurrencyExchangeRate,
  /// Intraday forex OHLCV bars.
  FxIntraday,
  /// Daily forex OHLCV bars.
  FxDaily,
  /// Weekly forex OHLCV bars.
  FxWeekly,
  /// Monthly forex OHLCV bars.
  FxMonthly,

  // ── Cryptocurrency ────────────────────────────────────────────────────
  /// Real-time exchange rate for a cryptocurrency pair.
  /// Shares the `CURRENCY_EXCHANGE_RATE` API endpoint with [`CurrencyExchangeRate`](FuncType::CurrencyExchangeRate).
  CryptoExchangeRate,
  /// Intraday cryptocurrency OHLCV bars.
  CryptoIntraday,
  /// Daily cryptocurrency OHLCV (uses `DIGITAL_CURRENCY_DAILY` endpoint).
  CryptoDaily,
  /// Weekly cryptocurrency OHLCV (uses `DIGITAL_CURRENCY_WEEKLY` endpoint).
  CryptoWeekly,
  /// Monthly cryptocurrency OHLCV (uses `DIGITAL_CURRENCY_MONTHLY` endpoint).
  CryptoMonthly,

  // ── Market status & search ────────────────────────────────────────────
  /// Current open/closed state of global exchanges.
  MarketStatus,
  /// Keyword search for securities (returns matching tickers, names, regions).
  SymbolSearch,

  // ── Legacy aliases (backward compatibility) ───────────────────────────
  /// **Deprecated** — use [`TimeSeriesIntraday`](FuncType::TimeSeriesIntraday).
  TsIntra,
  /// **Deprecated** — use [`TimeSeriesDaily`](FuncType::TimeSeriesDaily).
  TsDaily,
  /// **Deprecated** — use [`SymbolSearch`](FuncType::SymbolSearch).
  SymSearch,
  /// **Deprecated** — use [`TopGainersLosers`](FuncType::TopGainersLosers).
  TopQuery,
  /// **Deprecated** — use [`NewsSentiment`](FuncType::NewsSentiment).
  NewsQuery,
  /// **Deprecated** — use [`CryptoIntraday`](FuncType::CryptoIntraday).
  /// Retained for backward compatibility with the original `CryptoIntraDay` casing.
  CryptoIntraDay,
}

/// Produces the Alpha Vantage `function=` query-string value for each variant.
///
/// Legacy aliases emit the same string as their modern counterparts:
/// - `TsIntra` → `"TIME_SERIES_INTRADAY"` (same as `TimeSeriesIntraday`)
/// - `CryptoIntraDay` → `"CRYPTO_INTRADAY"` (same as `CryptoIntraday`)
/// - `CryptoExchangeRate` → `"CURRENCY_EXCHANGE_RATE"` (same as `CurrencyExchangeRate`)
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

/// The canonical Alpha Vantage REST API endpoint.
///
/// All API requests are sent as `GET` requests to this URL with function-specific
/// query parameters appended. The [`Config`] struct stores this as `base_url` and
/// defaults to this constant.
pub const ALPHA_VANTAGE_BASE_URL: &str = "https://www.alphavantage.co/query";

/// Maximum requests per minute for the **free** Alpha Vantage API tier (75 RPM).
///
/// Free-tier keys are limited to 25 requests per day in addition to this per-minute cap.
/// See [`Config::rate_limit`] for runtime configuration.
pub const DEFAULT_RATE_LIMIT: u32 = 75;

/// Maximum requests per minute for **premium** Alpha Vantage API plans (600 RPM).
///
/// Premium plans remove the daily request cap and raise the per-minute limit.
pub const PREMIUM_RATE_LIMIT: u32 = 600;

/// Shared test helpers (available only when the `test-utils` feature is enabled).
///
/// Contains mock data, fixture builders, and assertion helpers used by tests
/// across the workspace.
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
