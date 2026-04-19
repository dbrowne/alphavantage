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

//! Common types shared across Alpha Vantage API endpoint categories.
//!
//! This module defines lightweight enum types that parameterize API requests and
//! describe response metadata. These types are **not** specific to any single
//! Alpha Vantage endpoint — they appear in time-series queries, news sentiment,
//! listing status, and other API families.
//!
//! # Type summary
//!
//! | Type              | Variants | Purpose                                      |
//! |-------------------|----------|----------------------------------------------|
//! | [`DataType`]      | 2        | Response format: JSON or CSV                 |
//! | [`Interval`]      | 5        | Intraday bar width (1–60 minutes)            |
//! | [`OutputSize`]    | 2        | Result set size: compact (100) or full (20y) |
//! | [`SortOrder`]     | 3        | News/search result ordering                  |
//! | [`TimeHorizon`]   | 3        | Calendar data look-ahead period              |
//! | [`ListingState`]  | 2        | Active vs. delisted security status          |
//! | [`SentimentLabel`]| 3        | News sentiment: bullish/neutral/bearish      |
//! | [`CurrencyCode`]  | 29       | ISO 4217 fiat currency codes                 |
//! | [`CryptoSymbol`]  | 20       | Major cryptocurrency ticker symbols          |
//!
//! # Common trait implementations
//!
//! All types derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
//! `Serialize`, and `Deserialize`. Most also implement `Display` (for API
//! query-string formatting) and `FromStr` (for parsing API responses).
//!
//! # Examples
//!
//! ```rust
//! use av_core::types::common::{Interval, OutputSize, DataType};
//!
//! // Build query parameters
//! let interval = Interval::Min5;
//! assert_eq!(interval.to_string(), "5min");
//! assert_eq!(interval.minutes(), 5);
//!
//! let size = OutputSize::Compact;
//! assert_eq!(size.to_string(), "compact");
//!
//! let fmt = DataType::Json;
//! assert_eq!(fmt.to_string(), "json");
//! ```

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Specifies the response format for Alpha Vantage API requests.
///
/// Most Alpha Vantage endpoints accept a `datatype` query parameter that controls
/// whether the response body is JSON or CSV.
///
/// # Display output
///
/// The `Display` implementation produces the lowercase string expected by the API:
/// - `DataType::Json` → `"json"`
/// - `DataType::Csv`  → `"csv"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
  /// JSON response format (default for most endpoints).
  Json,
  /// CSV response format — useful for bulk data ingestion or spreadsheet import.
  Csv,
}

/// Formats as the API query-string value (`"json"` or `"csv"`).
impl std::fmt::Display for DataType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DataType::Json => write!(f, "json"),
      DataType::Csv => write!(f, "csv"),
    }
  }
}

/// Intraday bar interval for time-series queries.
///
/// Used with the `TIME_SERIES_INTRADAY` Alpha Vantage endpoint to specify
/// the width of each OHLCV bar. The five supported intervals match those
/// offered by the Alpha Vantage API.
///
/// # Display output
///
/// `Display` produces the API-expected format: `"1min"`, `"5min"`, `"15min"`,
/// `"30min"`, `"60min"`.
///
/// # Parsing
///
/// `FromStr` expects the exact API strings above. Unlike some other types in
/// this crate, parsing is **case-sensitive** and **not** infallible — an
/// unrecognized string returns `Err`.
///
/// # Metadata
///
/// [`Interval::minutes()`] returns the bar width as a `u32` for arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Interval {
  /// 1-minute bars — highest granularity available.
  Min1,
  /// 5-minute bars — common default for intraday analysis.
  Min5,
  /// 15-minute bars.
  Min15,
  /// 30-minute bars.
  Min30,
  /// 60-minute bars (hourly).
  Min60,
}

/// Formats as the API query-string value (e.g., `"5min"`).
impl std::fmt::Display for Interval {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Interval::Min1 => write!(f, "1min"),
      Interval::Min5 => write!(f, "5min"),
      Interval::Min15 => write!(f, "15min"),
      Interval::Min30 => write!(f, "30min"),
      Interval::Min60 => write!(f, "60min"),
    }
  }
}

/// Parses an interval from the API string format (`"1min"`, `"5min"`, etc.).
///
/// Returns `Err` for unrecognized strings. Parsing is case-sensitive.
impl FromStr for Interval {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "1min" => Ok(Interval::Min1),
      "5min" => Ok(Interval::Min5),
      "15min" => Ok(Interval::Min15),
      "30min" => Ok(Interval::Min30),
      "60min" => Ok(Interval::Min60),
      _ => Err(format!("Invalid interval: {}", s)),
    }
  }
}

impl Interval {
  /// Returns the bar width in minutes as a `u32`.
  ///
  /// Useful for computing the number of bars in a trading session or converting
  /// between intervals.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::common::Interval;
  ///
  /// assert_eq!(Interval::Min1.minutes(), 1);
  /// assert_eq!(Interval::Min60.minutes(), 60);
  ///
  /// // Bars in a 6.5-hour U.S. trading session
  /// let bars = (6 * 60 + 30) / Interval::Min5.minutes();
  /// assert_eq!(bars, 78);
  /// ```
  pub fn minutes(&self) -> u32 {
    match self {
      Interval::Min1 => 1,
      Interval::Min5 => 5,
      Interval::Min15 => 15,
      Interval::Min30 => 30,
      Interval::Min60 => 60,
    }
  }
}

/// Controls the size of the result set returned by time-series endpoints.
///
/// Alpha Vantage supports two modes:
/// - **Compact** — returns the latest 100 data points. Suitable for dashboards
///   and real-time displays where only recent data is needed.
/// - **Full** — returns up to 20 years of historical data. Suitable for
///   backtesting, research, and one-time data ingestion.
///
/// # Display output
///
/// Produces the API query-string value: `"compact"` or `"full"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputSize {
  /// Latest 100 data points — lightweight and fast.
  Compact,
  /// Full historical data (up to 20 years) — larger payload, higher latency.
  Full,
}

/// Formats as the API query-string value (`"compact"` or `"full"`).
impl std::fmt::Display for OutputSize {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OutputSize::Compact => write!(f, "compact"),
      OutputSize::Full => write!(f, "full"),
    }
  }
}

/// Ordering for news and search result endpoints.
///
/// Used with the `NEWS_SENTIMENT` and similar Alpha Vantage endpoints to
/// control the sort order of returned items.
///
/// # Display output
///
/// Produces the uppercase API value: `"LATEST"`, `"EARLIEST"`, or `"RELEVANCE"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
  /// Most recent items first (reverse chronological).
  Latest,
  /// Oldest items first (chronological).
  Earliest,
  /// Items ranked by relevance to the query (default for search endpoints).
  Relevance,
}

/// Formats as the API query-string value (e.g., `"LATEST"`).
impl std::fmt::Display for SortOrder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SortOrder::Latest => write!(f, "LATEST"),
      SortOrder::Earliest => write!(f, "EARLIEST"),
      SortOrder::Relevance => write!(f, "RELEVANCE"),
    }
  }
}

/// Look-ahead period for calendar-based endpoints (earnings, IPO, dividends).
///
/// Controls how far into the future the API returns scheduled events.
///
/// # Display output
///
/// Produces the API value: `"3month"`, `"6month"`, or `"12month"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeHorizon {
  /// 3-month look-ahead window.
  ThreeMonth,
  /// 6-month look-ahead window.
  SixMonth,
  /// 12-month (1-year) look-ahead window.
  TwelveMonth,
}

/// Formats as the API query-string value (e.g., `"3month"`).
impl std::fmt::Display for TimeHorizon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TimeHorizon::ThreeMonth => write!(f, "3month"),
      TimeHorizon::SixMonth => write!(f, "6month"),
      TimeHorizon::TwelveMonth => write!(f, "12month"),
    }
  }
}

/// Indicates whether a security is currently listed or has been removed from trading.
///
/// Used with the `LISTING_STATUS` Alpha Vantage endpoint to filter results
/// by active or delisted securities.
///
/// # Display output
///
/// Produces the API value: `"active"` or `"delisted"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListingState {
  /// The security is currently listed and actively traded.
  Active,
  /// The security has been removed from the exchange (acquired, bankrupt,
  /// voluntarily delisted, etc.).
  Delisted,
}

/// Formats as the API query-string value (`"active"` or `"delisted"`).
impl std::fmt::Display for ListingState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ListingState::Active => write!(f, "active"),
      ListingState::Delisted => write!(f, "delisted"),
    }
  }
}

/// Qualitative sentiment classification for news articles and ticker mentions.
///
/// Alpha Vantage's `NEWS_SENTIMENT` endpoint returns a numeric sentiment score
/// in the range `[-1.0, 1.0]`. This enum discretizes that score into three
/// buckets using the thresholds defined in [`SentimentLabel::score_range`]:
///
/// | Label     | Score range         |
/// |-----------|---------------------|
/// | `Bearish` | `[-1.00, -0.35)`    |
/// | `Neutral` | `[-0.35,  0.35)`    |
/// | `Bullish` | `[ 0.35,  1.00]`    |
///
/// # Parsing
///
/// `FromStr` is case-insensitive (e.g., `"bullish"`, `"BULLISH"`, `"Bullish"`
/// all work). Unrecognized strings return `Err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SentimentLabel {
  /// Positive market outlook — score ≥ 0.35.
  Bullish,
  /// Mixed or indeterminate outlook — score in `[-0.35, 0.35)`.
  Neutral,
  /// Negative market outlook — score < -0.35.
  Bearish,
}

/// Formats as the capitalized label (`"Bullish"`, `"Neutral"`, `"Bearish"`).
impl std::fmt::Display for SentimentLabel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SentimentLabel::Bullish => write!(f, "Bullish"),
      SentimentLabel::Neutral => write!(f, "Neutral"),
      SentimentLabel::Bearish => write!(f, "Bearish"),
    }
  }
}

/// Parses a sentiment label from a case-insensitive string.
///
/// Returns `Err` for unrecognized strings.
impl FromStr for SentimentLabel {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "bullish" => Ok(SentimentLabel::Bullish),
      "neutral" => Ok(SentimentLabel::Neutral),
      "bearish" => Ok(SentimentLabel::Bearish),
      _ => Err(format!("Invalid sentiment label: {}", s)),
    }
  }
}

impl SentimentLabel {
  /// Returns the `(min, max)` bounds of the numeric sentiment score for this label.
  ///
  /// The boundaries partition the `[-1.0, 1.0]` score space into three
  /// non-overlapping intervals. A score at exactly `±0.35` falls on the
  /// boundary between buckets.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::common::SentimentLabel;
  ///
  /// let (lo, hi) = SentimentLabel::Neutral.score_range();
  /// assert_eq!((lo, hi), (-0.35, 0.35));
  /// ```
  pub fn score_range(&self) -> (f64, f64) {
    match self {
      SentimentLabel::Bearish => (-1.0, -0.35),
      SentimentLabel::Neutral => (-0.35, 0.35),
      SentimentLabel::Bullish => (0.35, 1.0),
    }
  }
}

/// ISO 4217 fiat currency codes commonly used in financial APIs.
///
/// Covers the 8 major currencies (the "G8" forex group) plus 21 additional
/// widely-traded emerging-market and regional currencies — 29 total.
///
/// # Parsing
///
/// `FromStr` is case-insensitive (e.g., `"usd"` → `CurrencyCode::USD`).
/// Unrecognized codes return `Err`.
///
/// # Metadata methods
///
/// - [`CurrencyCode::is_major()`] — `true` for the 8 major currencies
///   (USD, EUR, GBP, JPY, CHF, CAD, AUD, NZD).
/// - [`CurrencyCode::decimal_places()`] — standard display precision
///   (0 for JPY/KRW/HUF, 2 for all others).
///
/// # Display output
///
/// `Display` uses `Debug` formatting, producing the 3-letter ISO code (e.g., `"USD"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurrencyCode {
  /// United States Dollar
  USD,
  /// Euro
  EUR,
  /// British Pound Sterling
  GBP,
  /// Japanese Yen
  JPY,
  /// Swiss Franc
  CHF,
  /// Canadian Dollar
  CAD,
  /// Australian Dollar
  AUD,
  /// New Zealand Dollar
  NZD,
  /// Chinese Yuan (Renminbi)
  CNY,
  /// Hong Kong Dollar
  HKD,
  /// Singapore Dollar
  SGD,
  /// Swedish Krona
  SEK,
  /// Norwegian Krone
  NOK,
  /// Danish Krone
  DKK,
  /// Polish Zloty
  PLN,
  /// Czech Koruna
  CZK,
  /// Hungarian Forint
  HUF,
  /// Russian Ruble
  RUB,
  /// South African Rand
  ZAR,
  /// Brazilian Real
  BRL,
  /// Mexican Peso
  MXN,
  /// Indian Rupee
  INR,
  /// South Korean Won
  KRW,
  /// Turkish Lira
  TRY,
  /// Israeli New Shekel
  ILS,
  /// Thai Baht
  THB,
  /// Malaysian Ringgit
  MYR,
  /// Philippine Peso
  PHP,
  /// Indonesian Rupiah
  IDR,
}

/// Formats as the 3-letter ISO 4217 code (e.g., `"USD"`).
impl std::fmt::Display for CurrencyCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

/// Parses a 3-letter currency code (case-insensitive).
///
/// Returns `Err` for unrecognized codes.
impl FromStr for CurrencyCode {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().as_str() {
      "USD" => Ok(CurrencyCode::USD),
      "EUR" => Ok(CurrencyCode::EUR),
      "GBP" => Ok(CurrencyCode::GBP),
      "JPY" => Ok(CurrencyCode::JPY),
      "CHF" => Ok(CurrencyCode::CHF),
      "CAD" => Ok(CurrencyCode::CAD),
      "AUD" => Ok(CurrencyCode::AUD),
      "NZD" => Ok(CurrencyCode::NZD),
      "CNY" => Ok(CurrencyCode::CNY),
      "HKD" => Ok(CurrencyCode::HKD),
      "SGD" => Ok(CurrencyCode::SGD),
      "SEK" => Ok(CurrencyCode::SEK),
      "NOK" => Ok(CurrencyCode::NOK),
      "DKK" => Ok(CurrencyCode::DKK),
      "PLN" => Ok(CurrencyCode::PLN),
      "CZK" => Ok(CurrencyCode::CZK),
      "HUF" => Ok(CurrencyCode::HUF),
      "RUB" => Ok(CurrencyCode::RUB),
      "ZAR" => Ok(CurrencyCode::ZAR),
      "BRL" => Ok(CurrencyCode::BRL),
      "MXN" => Ok(CurrencyCode::MXN),
      "INR" => Ok(CurrencyCode::INR),
      "KRW" => Ok(CurrencyCode::KRW),
      "TRY" => Ok(CurrencyCode::TRY),
      "ILS" => Ok(CurrencyCode::ILS),
      "THB" => Ok(CurrencyCode::THB),
      "MYR" => Ok(CurrencyCode::MYR),
      "PHP" => Ok(CurrencyCode::PHP),
      "IDR" => Ok(CurrencyCode::IDR),
      _ => Err(format!("Invalid currency code: {}", s)),
    }
  }
}

impl CurrencyCode {
  /// Returns `true` if this is one of the 8 major ("G8") forex currencies.
  ///
  /// Major currencies are: USD, EUR, GBP, JPY, CHF, CAD, AUD, NZD.
  /// These account for the vast majority of global forex volume and are
  /// typically quoted with tighter spreads.
  pub fn is_major(&self) -> bool {
    matches!(
      self,
      CurrencyCode::USD
        | CurrencyCode::EUR
        | CurrencyCode::GBP
        | CurrencyCode::JPY
        | CurrencyCode::CHF
        | CurrencyCode::CAD
        | CurrencyCode::AUD
        | CurrencyCode::NZD
    )
  }

  /// Returns the standard number of decimal places used when displaying amounts
  /// in this currency.
  ///
  /// Most currencies use 2 decimal places (cents). Zero-decimal currencies are
  /// JPY (Yen), KRW (Won), and HUF (Forint) — their smallest unit is the
  /// whole currency unit itself.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::common::CurrencyCode;
  ///
  /// assert_eq!(CurrencyCode::USD.decimal_places(), 2);
  /// assert_eq!(CurrencyCode::JPY.decimal_places(), 0);
  /// ```
  pub fn decimal_places(&self) -> u8 {
    match self {
      CurrencyCode::JPY | CurrencyCode::KRW | CurrencyCode::HUF => 0,
      _ => 2,
    }
  }
}

/// Ticker symbols for the 20 most widely-traded cryptocurrencies.
///
/// Used with Alpha Vantage's `DIGITAL_CURRENCY_DAILY`, `DIGITAL_CURRENCY_WEEKLY`,
/// and `CRYPTO_RATING` endpoints.
///
/// # Parsing
///
/// `FromStr` is case-insensitive (e.g., `"btc"` → `CryptoSymbol::BTC`).
/// Unrecognized symbols return `Err`.
///
/// # Metadata methods
///
/// - [`CryptoSymbol::is_major()`] — `true` for the top-8 by historical market cap
///   (BTC, ETH, BNB, ADA, SOL, XRP, DOT, LINK).
/// - [`CryptoSymbol::full_name()`] — returns the project/coin name
///   (e.g., `"Bitcoin"`, `"Ethereum"`).
///
/// # Display output
///
/// `Display` uses `Debug` formatting, producing the uppercase ticker (e.g., `"BTC"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoSymbol {
  /// Bitcoin — the original and largest cryptocurrency by market cap.
  BTC,
  /// Ethereum — smart contract platform; second-largest by market cap.
  ETH,
  /// Binance Coin — native token of the Binance exchange.
  BNB,
  /// Cardano — proof-of-stake smart contract platform.
  ADA,
  /// Solana — high-throughput layer-1 blockchain.
  SOL,
  /// XRP — digital payment protocol (formerly Ripple).
  XRP,
  /// Polkadot — multi-chain interoperability protocol.
  DOT,
  /// Dogecoin — meme-originated cryptocurrency.
  DOGE,
  /// Avalanche — layer-1 blockchain with sub-second finality.
  AVAX,
  /// Polygon (MATIC) — Ethereum layer-2 scaling solution.
  MATIC,
  /// Chainlink — decentralized oracle network.
  LINK,
  /// Litecoin — early Bitcoin fork optimized for faster transactions.
  LTC,
  /// Bitcoin Cash — Bitcoin fork with larger block size.
  BCH,
  /// Stellar — open-source payment network.
  XLM,
  /// VeChain — supply chain and enterprise blockchain.
  VET,
  /// Internet Computer — decentralized cloud computing platform.
  ICP,
  /// Filecoin — decentralized storage network.
  FIL,
  /// TRON — entertainment and content-sharing blockchain.
  TRX,
  /// Ethereum Classic — original Ethereum chain (pre-DAO-fork).
  ETC,
  /// Monero — privacy-focused cryptocurrency.
  XMR,
}

/// Formats as the uppercase ticker symbol (e.g., `"BTC"`).
impl std::fmt::Display for CryptoSymbol {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

/// Parses a cryptocurrency ticker (case-insensitive).
///
/// Returns `Err` for unrecognized symbols.
impl FromStr for CryptoSymbol {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().as_str() {
      "BTC" => Ok(CryptoSymbol::BTC),
      "ETH" => Ok(CryptoSymbol::ETH),
      "BNB" => Ok(CryptoSymbol::BNB),
      "ADA" => Ok(CryptoSymbol::ADA),
      "SOL" => Ok(CryptoSymbol::SOL),
      "XRP" => Ok(CryptoSymbol::XRP),
      "DOT" => Ok(CryptoSymbol::DOT),
      "DOGE" => Ok(CryptoSymbol::DOGE),
      "AVAX" => Ok(CryptoSymbol::AVAX),
      "MATIC" => Ok(CryptoSymbol::MATIC),
      "LINK" => Ok(CryptoSymbol::LINK),
      "LTC" => Ok(CryptoSymbol::LTC),
      "BCH" => Ok(CryptoSymbol::BCH),
      "XLM" => Ok(CryptoSymbol::XLM),
      "VET" => Ok(CryptoSymbol::VET),
      "ICP" => Ok(CryptoSymbol::ICP),
      "FIL" => Ok(CryptoSymbol::FIL),
      "TRX" => Ok(CryptoSymbol::TRX),
      "ETC" => Ok(CryptoSymbol::ETC),
      "XMR" => Ok(CryptoSymbol::XMR),
      _ => Err(format!("Invalid crypto symbol: {}", s)),
    }
  }
}

impl CryptoSymbol {
  /// Returns `true` if this is one of the top-8 cryptocurrencies by historical
  /// market capitalization.
  ///
  /// Major coins: BTC, ETH, BNB, ADA, SOL, XRP, DOT, LINK.
  pub fn is_major(&self) -> bool {
    matches!(
      self,
      CryptoSymbol::BTC
        | CryptoSymbol::ETH
        | CryptoSymbol::BNB
        | CryptoSymbol::ADA
        | CryptoSymbol::SOL
        | CryptoSymbol::XRP
        | CryptoSymbol::DOT
        | CryptoSymbol::LINK
    )
  }

  /// Returns the full project/coin name for this cryptocurrency.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::common::CryptoSymbol;
  ///
  /// assert_eq!(CryptoSymbol::BTC.full_name(), "Bitcoin");
  /// assert_eq!(CryptoSymbol::ETH.full_name(), "Ethereum");
  /// assert_eq!(CryptoSymbol::MATIC.full_name(), "Polygon");
  /// ```
  pub fn full_name(&self) -> &'static str {
    match self {
      CryptoSymbol::BTC => "Bitcoin",
      CryptoSymbol::ETH => "Ethereum",
      CryptoSymbol::BNB => "Binance Coin",
      CryptoSymbol::ADA => "Cardano",
      CryptoSymbol::SOL => "Solana",
      CryptoSymbol::XRP => "XRP",
      CryptoSymbol::DOT => "Polkadot",
      CryptoSymbol::DOGE => "Dogecoin",
      CryptoSymbol::AVAX => "Avalanche",
      CryptoSymbol::MATIC => "Polygon",
      CryptoSymbol::LINK => "Chainlink",
      CryptoSymbol::LTC => "Litecoin",
      CryptoSymbol::BCH => "Bitcoin Cash",
      CryptoSymbol::XLM => "Stellar",
      CryptoSymbol::VET => "VeChain",
      CryptoSymbol::ICP => "Internet Computer",
      CryptoSymbol::FIL => "Filecoin",
      CryptoSymbol::TRX => "TRON",
      CryptoSymbol::ETC => "Ethereum Classic",
      CryptoSymbol::XMR => "Monero",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_interval_parsing() {
    assert_eq!("5min".parse::<Interval>(), Ok(Interval::Min5));
    assert!("invalid".parse::<Interval>().is_err());
    assert_eq!(Interval::Min15.minutes(), 15);
  }

  #[test]
  fn test_currency_code_parsing() {
    assert_eq!("USD".parse::<CurrencyCode>(), Ok(CurrencyCode::USD));
    assert_eq!("usd".parse::<CurrencyCode>(), Ok(CurrencyCode::USD));
    assert!(CurrencyCode::USD.is_major());
    assert_eq!(CurrencyCode::USD.decimal_places(), 2);
    assert_eq!(CurrencyCode::JPY.decimal_places(), 0);
  }

  #[test]
  fn test_crypto_symbol_parsing() {
    assert_eq!("BTC".parse::<CryptoSymbol>(), Ok(CryptoSymbol::BTC));
    assert_eq!("btc".parse::<CryptoSymbol>(), Ok(CryptoSymbol::BTC));
    assert!(CryptoSymbol::BTC.is_major());
    assert_eq!(CryptoSymbol::BTC.full_name(), "Bitcoin");
  }

  #[test]
  fn test_sentiment_label() {
    assert_eq!("Bullish".parse::<SentimentLabel>(), Ok(SentimentLabel::Bullish));
    assert_eq!("bullish".parse::<SentimentLabel>(), Ok(SentimentLabel::Bullish));

    let (min, max) = SentimentLabel::Bullish.score_range();
    assert_eq!(min, 0.35);
    assert_eq!(max, 1.0);
  }
}
