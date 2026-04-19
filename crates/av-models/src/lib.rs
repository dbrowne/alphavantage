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

//! # av-models
//!
//! Strongly-typed Rust data models for Alpha Vantage API responses.
//!
//! This crate sits between the HTTP client layer and the database layer in the
//! `alphavantage` workspace. It defines the serde-deserializable structs that
//! map directly to the JSON payloads returned by the Alpha Vantage REST API,
//! and also includes enrichment types for external data sources (CoinGecko).
//!
//! # Design principles
//!
//! - **Type safety:** Every API response is represented by a concrete Rust struct
//!   with named fields — no untyped `serde_json::Value` maps.
//! - **Serde integration:** All types derive `Deserialize` (and usually `Serialize`)
//!   with field-level `#[serde(rename = "...")]` attributes matching the exact
//!   Alpha Vantage JSON keys.
//! - **Financial precision:** Monetary values use [`rust_decimal::Decimal`] to avoid
//!   floating-point rounding errors.
//! - **Timezone-aware dates:** Timestamps use [`chrono`] types with appropriate
//!   timezone handling.
//!
//! # Module overview
//!
//! | Module            | Endpoint family              | Key types                                              |
//! |-------------------|------------------------------|--------------------------------------------------------|
//! | [`common`]        | Shared across endpoints      | `Metadata`, `OhlcvData`, `SymbolMatch`, `ApiResponse`  |
//! | [`time_series`]   | `TIME_SERIES_*`, `SYMBOL_SEARCH`, `MARKET_STATUS`, `GLOBAL_QUOTE` | `IntradayTimeSeries`, `DailyTimeSeries`, `DailyAdjustedTimeSeries`, `SymbolSearch`, `GlobalQuote`, technical indicators |
//! | [`fundamentals`]  | `OVERVIEW`, `INCOME_STATEMENT`, `BALANCE_SHEET`, `CASH_FLOW`, `EARNINGS`, `TOP_GAINERS_LOSERS`, `LISTING_STATUS`, calendars | `CompanyOverview`, `IncomeStatement`, `BalanceSheet`, `CashFlow`, `Earnings`, `TopGainersLosers`, `ListingStatus` |
//! | [`news`]          | `NEWS_SENTIMENT`             | `NewsSentiment`, `NewsArticle`, `TickerSentiment`, `SentimentTrend` |
//! | [`forex`]         | `CURRENCY_EXCHANGE_RATE`, `FX_*` | `ExchangeRate`, `FxIntraday`, `FxDaily`, `CurrencyPair` |
//! | [`crypto`]        | `CRYPTO_*`, `DIGITAL_CURRENCY_*` | `CryptoExchangeRate`, `CryptoIntraday`, `CryptoDaily`  |
//! | [`crypto_social`] | CoinGecko social/developer API | `CoinGeckoSocialResponse`, `ProcessedSocialData`       |
//!
//! # Usage
//!
//! ```rust,no_run
//! use av_models::time_series::DailyTimeSeries;
//! use av_models::fundamentals::CompanyOverview;
//!
//! // Deserialize API responses directly
//! # let response_json = "{}";
//! # let overview_json = "{}";
//! let daily_data: DailyTimeSeries = serde_json::from_str(response_json).unwrap();
//! let overview: CompanyOverview = serde_json::from_str(overview_json).unwrap();
//! ```
//!
//! All types are also glob-re-exported at the crate root, so you can write
//! `use av_models::DailyTimeSeries` without the module prefix.
//!
//! # Relationship to other crates
//!
//! ```text
//! av-client (HTTP)
//!   └──► av-models (this crate)  ← deserialize JSON responses
//!          └──► av-core::types   ← shared enums (Exchange, SecurityType, etc.)
//!
//! av-loaders (ingestion)
//!   └──► av-models              ← read API structs
//!          └──► av-database-postgres::models  ← convert to DB insertable structs
//! ```

#![warn(clippy::all)]

/// Common types shared across all API response families.
///
/// Includes [`Metadata`], OHLCV data structs ([`OhlcvData`],
/// [`OhlcvAdjustedData`], [`OhlcData`]), symbol search results
/// ([`SymbolMatch`]), market info, and the generic [`ApiResponse<T>`]
/// wrapper. Also defines the [`TimeSeriesData<T>`] type alias
/// (`BTreeMap<String, T>`) used by all time-series modules.
pub mod common;

/// Cryptocurrency OHLCV and exchange-rate models.
///
/// Covers `CRYPTO_INTRADAY`, `DIGITAL_CURRENCY_DAILY/WEEKLY/MONTHLY`,
/// and `CURRENCY_EXCHANGE_RATE` (crypto variant). Key types:
/// [`CryptoExchangeRate`], [`CryptoIntraday`], [`CryptoDaily`],
/// [`CryptoOhlcvData`], [`CryptoMetadata`].
pub mod crypto;

/// CoinGecko social and developer data models.
///
/// Structs for deserializing CoinGecko API responses (community data,
/// developer stats, GitHub repos) and a [`ProcessedSocialData`] struct
/// that normalizes this data for database storage.
pub mod crypto_social;

/// Foreign exchange (forex) data models.
///
/// Covers `CURRENCY_EXCHANGE_RATE` and `FX_INTRADAY/DAILY/WEEKLY/MONTHLY`.
/// Key types: [`ExchangeRate`], [`FxIntraday`], [`FxDaily`],
/// [`CurrencyPair`], [`ForexSession`].
pub mod forex;

/// Company fundamental analysis models.
///
/// Covers `OVERVIEW`, `INCOME_STATEMENT`, `BALANCE_SHEET`, `CASH_FLOW`,
/// `EARNINGS`, `TOP_GAINERS_LOSERS`, `LISTING_STATUS`, `EARNINGS_CALENDAR`,
/// and `IPO_CALENDAR`. Key types: [`CompanyOverview`], [`IncomeStatement`],
/// [`BalanceSheet`], [`CashFlow`], [`Earnings`], [`TopGainersLosers`].
pub mod fundamentals;

/// News sentiment analysis models.
///
/// Covers the `NEWS_SENTIMENT` endpoint. Key types: [`NewsSentiment`],
/// [`NewsArticle`], [`TickerSentiment`], [`SentimentTrend`],
/// [`SentimentDistribution`].
pub mod news;

/// Stock time-series, search, market status, and technical indicator models.
///
/// Covers `TIME_SERIES_INTRADAY/DAILY/WEEKLY/MONTHLY` (with adjusted
/// variants), `SYMBOL_SEARCH`, `MARKET_STATUS`, `GLOBAL_QUOTE`, and
/// technical indicators (SMA, RSI, MACD, Bollinger Bands). Key types:
/// [`IntradayTimeSeries`], [`DailyTimeSeries`], [`DailyAdjustedTimeSeries`],
/// [`SymbolSearch`], [`GlobalQuote`], [`TechnicalIndicator`].
pub mod time_series;

// ─── Glob re-exports ────────────────────────────────────────────────────────
//
// All public types from every sub-module are re-exported at the crate root
// so consumers can write `use av_models::DailyTimeSeries` without module
// qualification. The sub-modules remain available for explicit imports.

pub use common::*;
pub use crypto::*;
pub use crypto_social::*;
pub use forex::*;
pub use fundamentals::*;
pub use news::*;
pub use time_series::*;
