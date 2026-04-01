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

//! Common types used across the API

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Data output format for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
  Json,
  Csv,
}

impl std::fmt::Display for DataType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DataType::Json => write!(f, "json"),
      DataType::Csv => write!(f, "csv"),
    }
  }
}

/// Time interval for intraday data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Interval {
  Min1,
  Min5,
  Min15,
  Min30,
  Min60,
}

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
  /// Get interval duration in minutes
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

/// Output size for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputSize {
  /// Compact output (latest 100 data points)
  Compact,
  /// Full output (up to 20 years of data)
  Full,
}

impl std::fmt::Display for OutputSize {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OutputSize::Compact => write!(f, "compact"),
      OutputSize::Full => write!(f, "full"),
    }
  }
}

/// Sort order for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
  Latest,
  Earliest,
  Relevance,
}

impl std::fmt::Display for SortOrder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SortOrder::Latest => write!(f, "LATEST"),
      SortOrder::Earliest => write!(f, "EARLIEST"),
      SortOrder::Relevance => write!(f, "RELEVANCE"),
    }
  }
}

/// Time horizon for calendar data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeHorizon {
  ThreeMonth,
  SixMonth,
  TwelveMonth,
}

impl std::fmt::Display for TimeHorizon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TimeHorizon::ThreeMonth => write!(f, "3month"),
      TimeHorizon::SixMonth => write!(f, "6month"),
      TimeHorizon::TwelveMonth => write!(f, "12month"),
    }
  }
}

/// Listing state for securities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListingState {
  Active,
  Delisted,
}

impl std::fmt::Display for ListingState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ListingState::Active => write!(f, "active"),
      ListingState::Delisted => write!(f, "delisted"),
    }
  }
}

/// Sentiment label
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SentimentLabel {
  Bullish,
  Neutral,
  Bearish,
}

impl std::fmt::Display for SentimentLabel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SentimentLabel::Bullish => write!(f, "Bullish"),
      SentimentLabel::Neutral => write!(f, "Neutral"),
      SentimentLabel::Bearish => write!(f, "Bearish"),
    }
  }
}

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
  /// Get sentiment score range
  pub fn score_range(&self) -> (f64, f64) {
    match self {
      SentimentLabel::Bearish => (-1.0, -0.35),
      SentimentLabel::Neutral => (-0.35, 0.35),
      SentimentLabel::Bullish => (0.35, 1.0),
    }
  }
}

/// Currency codes commonly used in financial APIs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurrencyCode {
  USD,
  EUR,
  GBP,
  JPY,
  CHF,
  CAD,
  AUD,
  NZD,
  CNY,
  HKD,
  SGD,
  SEK,
  NOK,
  DKK,
  PLN,
  CZK,
  HUF,
  RUB,
  ZAR,
  BRL,
  MXN,
  INR,
  KRW,
  TRY,
  ILS,
  THB,
  MYR,
  PHP,
  IDR,
}

impl std::fmt::Display for CurrencyCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

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
  /// Check if this is a major currency
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

  /// Get decimal places typically used for this currency
  pub fn decimal_places(&self) -> u8 {
    match self {
      CurrencyCode::JPY | CurrencyCode::KRW | CurrencyCode::HUF => 0,
      _ => 2,
    }
  }
}

/// Cryptocurrency symbols commonly used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoSymbol {
  BTC,
  ETH,
  BNB,
  ADA,
  SOL,
  XRP,
  DOT,
  DOGE,
  AVAX,
  MATIC,
  LINK,
  LTC,
  BCH,
  XLM,
  VET,
  ICP,
  FIL,
  TRX,
  ETC,
  XMR,
}

impl std::fmt::Display for CryptoSymbol {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

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
  /// Check if this is a major cryptocurrency
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

  /// Get the full name of the cryptocurrency
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
