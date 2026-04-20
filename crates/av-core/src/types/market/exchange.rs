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

//! Stock exchange identifiers and related metadata.
//!
//! This module provides the [`Exchange`] enum, a comprehensive representation of major
//! global stock and derivatives exchanges. It is designed for use throughout the
//! `alphavantage` crate to normalize exchange identifiers returned by the Alpha Vantage API
//! into a strongly-typed, ergonomic Rust enum.
//!
//! # Features
//!
//! - **25 exchange variants** covering the Americas, Europe, Asia-Pacific, Middle East, and Africa.
//! - **Case-insensitive parsing** via [`FromStr`], accepting both abbreviations (e.g., `"NYSE"`)
//!   and full names (e.g., `"New York Stock Exchange"`). Unrecognized strings map to [`Exchange::OTHER`].
//! - **Display** formatting that returns the canonical abbreviation.
//! - **Rich metadata** methods: [`Exchange::full_name`], [`Exchange::timezone`],
//!   [`Exchange::primary_currency`], and [`Exchange::is_major`].
//! - **Serde support** for seamless JSON serialization/deserialization.
//!
//! # Examples
//!
//! ```rust
//! use std::str::FromStr;
//! use av_core::types::market::Exchange;
//!
//! // Parse from string (case-insensitive)
//! let nyse = Exchange::from_str("nyse").unwrap();
//! assert_eq!(nyse, Exchange::NYSE);
//!
//! // Also accepts full names
//! let nyse2 = Exchange::from_str("New York Stock Exchange").unwrap();
//! assert_eq!(nyse2, Exchange::NYSE);
//!
//! // Unknown exchanges map to OTHER
//! let unknown = Exchange::from_str("UNKNOWN").unwrap();
//! assert_eq!(unknown, Exchange::OTHER);
//!
//! // Access metadata
//! assert_eq!(nyse.full_name(), "New York Stock Exchange");
//! assert_eq!(nyse.timezone(), "America/New_York");
//! assert_eq!(nyse.primary_currency(), "USD");
//! assert!(nyse.is_major());
//! ```

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Represents a stock or derivatives exchange.
///
/// Each variant corresponds to a well-known global exchange. The [`OTHER`](Exchange::OTHER)
/// variant serves as a catch-all for exchanges not explicitly enumerated.
///
/// # Derives
///
/// | Trait          | Purpose                                                    |
/// |----------------|------------------------------------------------------------|
/// | `Debug`        | Enables `{:?}` formatting for logging and diagnostics      |
/// | `Clone`, `Copy`| Value-type semantics — cheap to pass by value               |
/// | `PartialEq`, `Eq` | Equality comparisons (e.g., filtering by exchange)     |
/// | `Hash`         | Usable as a key in `HashMap` / `HashSet`                   |
/// | `Serialize`, `Deserialize` | Serde JSON support for API payloads          |
///
/// # Variant naming conventions
///
/// Variants use the widely-recognized abbreviation for each exchange (e.g., `NYSE`, `LSE`).
/// Where an abbreviation could be ambiguous (e.g., `TSE` for Tokyo vs. Toronto), the
/// doc-comment on each variant clarifies the intended exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
  /// New York Stock Exchange — the world's largest equities exchange by market
  /// capitalization, located at 11 Wall Street, New York City.
  /// Timezone: `America/New_York` | Currency: `USD`
  NYSE,
  /// NASDAQ — an American electronic stock exchange and the second-largest
  /// stock exchange in the world by market capitalization.
  /// Timezone: `America/New_York` | Currency: `USD`
  NASDAQ,
  /// American Stock Exchange (now NYSE American) — historically the third-largest
  /// U.S. stock exchange, known for listing smaller-cap companies and ETFs.
  /// Timezone: `America/New_York` | Currency: `USD`
  AMEX,
  /// Chicago Board of Trade — one of the oldest futures and options exchanges,
  /// now part of the CME Group. Trades agricultural and financial derivatives.
  /// Timezone: `America/Chicago` | Currency: `USD`
  CBOT,
  /// Chicago Mercantile Exchange — the world's largest financial derivatives
  /// exchange, trading futures and options on interest rates, equity indexes,
  /// foreign exchange, and commodities.
  /// Timezone: `America/Chicago` | Currency: `USD`
  CME,
  /// London Stock Exchange — one of the oldest and largest exchanges in Europe,
  /// and a major global listing venue.
  /// Timezone: `Europe/London` | Currency: `GBP`
  LSE,
  /// Toronto Stock Exchange — Canada's largest stock exchange, listing many
  /// natural resources and mining companies.
  /// Timezone: `America/Toronto` | Currency: `CAD`
  TSX,
  /// Tokyo Stock Exchange — Japan's largest stock exchange and the third-largest
  /// in the world by market capitalization. Also known as "Tosho" (東証).
  /// Timezone: `Asia/Tokyo` | Currency: `JPY`
  TSE,
  /// Hong Kong Stock Exchange — the primary securities exchange in Hong Kong,
  /// and a major gateway for international investment into mainland China.
  /// Timezone: `Asia/Hong_Kong` | Currency: `HKD`
  HKSE,
  /// Shanghai Stock Exchange — one of the two main stock exchanges in mainland
  /// China, primarily listing large state-owned enterprises.
  /// Timezone: `Asia/Shanghai` | Currency: `CNY`
  SSE,
  /// Shenzhen Stock Exchange — the second of China's two mainland exchanges,
  /// known for listing technology and growth-oriented companies.
  /// Timezone: `Asia/Shanghai` | Currency: `CNY`
  SZSE,
  /// Euronext — a pan-European exchange operating markets in Amsterdam, Brussels,
  /// Dublin, Lisbon, Milan, Oslo, and Paris.
  /// Timezone: `Europe/Paris` | Currency: `EUR`
  EURONEXT,
  /// Frankfurt Stock Exchange (Frankfurter Wertpapierbörse) — Germany's largest
  /// stock exchange, operated by Deutsche Börse.
  /// Timezone: `Europe/Berlin` | Currency: `EUR`
  FRA,
  /// SIX Swiss Exchange — Switzerland's principal stock exchange, headquartered
  /// in Zurich.
  /// Timezone: `Europe/Zurich` | Currency: `CHF`
  SIX,
  /// Australian Securities Exchange — Australia's primary securities exchange,
  /// based in Sydney.
  /// Timezone: `Australia/Sydney` | Currency: `AUD`
  ASX,
  /// Bombay Stock Exchange — Asia's oldest stock exchange, established in 1875,
  /// located in Mumbai, India.
  /// Timezone: `Asia/Kolkata` | Currency: `INR`
  BSE,
  /// National Stock Exchange of India — India's largest stock exchange by
  /// trading volume, also located in Mumbai.
  /// Timezone: `Asia/Kolkata` | Currency: `INR`
  NSE,
  /// B3 (formerly BM&F Bovespa / São Paulo Stock Exchange) — the main exchange
  /// in Brazil and the largest in Latin America.
  /// Timezone: `America/Sao_Paulo` | Currency: `BRL`
  BOVESPA,
  /// Moscow Exchange (MOEX) — Russia's largest exchange, formed from the merger
  /// of MICEX and RTS.
  /// Timezone: `Europe/Moscow` | Currency: `RUB`
  MOEX,
  /// Korea Exchange — South Korea's sole securities exchange operator,
  /// headquartered in Busan.
  /// Timezone: `Asia/Seoul` | Currency: `KRW`
  KRX,
  /// Taiwan Stock Exchange — the primary securities exchange in Taiwan,
  /// located in Taipei.
  /// Timezone: `Asia/Taipei` | Currency: `TWD`
  TWSE,
  /// Singapore Exchange — a multi-asset exchange in Singapore offering equities,
  /// fixed income, derivatives, and commodities trading.
  /// Timezone: `Asia/Singapore` | Currency: `SGD`
  SGX,
  /// Johannesburg Stock Exchange — Africa's largest stock exchange, located in
  /// Sandton, South Africa.
  /// Timezone: `Africa/Johannesburg` | Currency: `ZAR`
  JSE,
  /// Tel Aviv Stock Exchange — Israel's only public securities exchange.
  /// Timezone: `Asia/Jerusalem` | Currency: `ILS`
  TASE,
  /// Catch-all variant for exchanges not explicitly listed in this enum.
  ///
  /// The [`FromStr`] implementation maps any unrecognized string to this variant
  /// rather than returning an error, ensuring parsing is infallible for valid UTF-8 input.
  /// Timezone defaults to `UTC` | Currency defaults to `USD`.
  OTHER,
}

/// Formats the exchange as its canonical abbreviation (e.g., `"NYSE"`, `"NASDAQ"`).
///
/// This is the inverse of [`FromStr`] when using the short-form input. For the
/// human-readable full name, use [`Exchange::full_name`] instead.
impl std::fmt::Display for Exchange {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Exchange::NYSE => write!(f, "NYSE"),
      Exchange::NASDAQ => write!(f, "NASDAQ"),
      Exchange::AMEX => write!(f, "AMEX"),
      Exchange::CBOT => write!(f, "CBOT"),
      Exchange::CME => write!(f, "CME"),
      Exchange::LSE => write!(f, "LSE"),
      Exchange::TSX => write!(f, "TSX"),
      Exchange::TSE => write!(f, "TSE"),
      Exchange::HKSE => write!(f, "HKSE"),
      Exchange::SSE => write!(f, "SSE"),
      Exchange::SZSE => write!(f, "SZSE"),
      Exchange::EURONEXT => write!(f, "EURONEXT"),
      Exchange::FRA => write!(f, "FRA"),
      Exchange::SIX => write!(f, "SIX"),
      Exchange::ASX => write!(f, "ASX"),
      Exchange::BSE => write!(f, "BSE"),
      Exchange::NSE => write!(f, "NSE"),
      Exchange::BOVESPA => write!(f, "BOVESPA"),
      Exchange::MOEX => write!(f, "MOEX"),
      Exchange::KRX => write!(f, "KRX"),
      Exchange::TWSE => write!(f, "TWSE"),
      Exchange::SGX => write!(f, "SGX"),
      Exchange::JSE => write!(f, "JSE"),
      Exchange::TASE => write!(f, "TASE"),
      Exchange::OTHER => write!(f, "OTHER"),
    }
  }
}

/// Parses a string into an [`Exchange`] variant.
///
/// Parsing is **case-insensitive** and accepts both the canonical abbreviation
/// (e.g., `"NYSE"`) and the full exchange name (e.g., `"New York Stock Exchange"`).
///
/// # Infallible by design
///
/// This implementation **never returns `Err`**. Any input that does not match a known
/// exchange is mapped to [`Exchange::OTHER`]. The `Err` type is `String` to satisfy
/// the [`FromStr`] trait contract, but callers can safely `.unwrap()` the result.
///
/// # Accepted aliases
///
/// | Variant    | Accepted inputs (case-insensitive)             |
/// |------------|------------------------------------------------|
/// | `NYSE`     | `"NYSE"`, `"New York Stock Exchange"`           |
/// | `AMEX`     | `"AMEX"`, `"American Stock Exchange"`           |
/// | `LSE`      | `"LSE"`, `"London Stock Exchange"`              |
/// | `TSX`      | `"TSX"`, `"Toronto Stock Exchange"`             |
/// | `TSE`      | `"TSE"`, `"Tokyo Stock Exchange"`               |
/// | `HKSE`     | `"HKSE"`, `"Hong Kong Stock Exchange"`          |
/// | `SSE`      | `"SSE"`, `"Shanghai Stock Exchange"`            |
/// | `SZSE`     | `"SZSE"`, `"Shenzhen Stock Exchange"`           |
/// | `FRA`      | `"FRA"`, `"Frankfurt Stock Exchange"`           |
/// | `SIX`      | `"SIX"`, `"Swiss Exchange"`                     |
/// | `ASX`      | `"ASX"`, `"Australian Securities Exchange"`     |
/// | `BSE`      | `"BSE"`, `"Bombay Stock Exchange"`              |
/// | `NSE`      | `"NSE"`, `"National Stock Exchange of India"`   |
/// | `MOEX`     | `"MOEX"`, `"Moscow Exchange"`                   |
/// | `KRX`      | `"KRX"`, `"Korea Exchange"`                     |
/// | `TWSE`     | `"TWSE"`, `"Taiwan Stock Exchange"`             |
/// | `SGX`      | `"SGX"`, `"Singapore Exchange"`                 |
/// | `JSE`      | `"JSE"`, `"Johannesburg Stock Exchange"`        |
/// | `TASE`     | `"TASE"`, `"Tel Aviv Stock Exchange"`           |
/// | Others     | Only the abbreviation (e.g., `"NASDAQ"`, `"CME"`, `"EURONEXT"`, `"BOVESPA"`) |
impl FromStr for Exchange {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().as_str() {
      "NYSE" | "NEW YORK STOCK EXCHANGE" => Ok(Exchange::NYSE),
      "NASDAQ" => Ok(Exchange::NASDAQ),
      "AMEX" | "AMERICAN STOCK EXCHANGE" => Ok(Exchange::AMEX),
      "CBOT" => Ok(Exchange::CBOT),
      "CME" => Ok(Exchange::CME),
      "LSE" | "LONDON STOCK EXCHANGE" => Ok(Exchange::LSE),
      "TSX" | "TORONTO STOCK EXCHANGE" => Ok(Exchange::TSX),
      "TSE" | "TOKYO STOCK EXCHANGE" => Ok(Exchange::TSE),
      "HKSE" | "HONG KONG STOCK EXCHANGE" => Ok(Exchange::HKSE),
      "SSE" | "SHANGHAI STOCK EXCHANGE" => Ok(Exchange::SSE),
      "SZSE" | "SHENZHEN STOCK EXCHANGE" => Ok(Exchange::SZSE),
      "EURONEXT" => Ok(Exchange::EURONEXT),
      "FRA" | "FRANKFURT STOCK EXCHANGE" => Ok(Exchange::FRA),
      "SIX" | "SWISS EXCHANGE" => Ok(Exchange::SIX),
      "ASX" | "AUSTRALIAN SECURITIES EXCHANGE" => Ok(Exchange::ASX),
      "BSE" | "BOMBAY STOCK EXCHANGE" => Ok(Exchange::BSE),
      "NSE" | "NATIONAL STOCK EXCHANGE OF INDIA" => Ok(Exchange::NSE),
      "BOVESPA" => Ok(Exchange::BOVESPA),
      "MOEX" | "MOSCOW EXCHANGE" => Ok(Exchange::MOEX),
      "KRX" | "KOREA EXCHANGE" => Ok(Exchange::KRX),
      "TWSE" | "TAIWAN STOCK EXCHANGE" => Ok(Exchange::TWSE),
      "SGX" | "SINGAPORE EXCHANGE" => Ok(Exchange::SGX),
      "JSE" | "JOHANNESBURG STOCK EXCHANGE" => Ok(Exchange::JSE),
      "TASE" | "TEL AVIV STOCK EXCHANGE" => Ok(Exchange::TASE),
      _ => Ok(Exchange::OTHER),
    }
  }
}

/// Inherent methods providing exchange metadata.
///
/// These methods expose static, compile-time-known attributes for each exchange,
/// enabling callers to look up contextual information without maintaining separate
/// mapping tables.
impl Exchange {
  /// Returns the full human-readable name of the exchange.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::market::Exchange;
  ///
  /// assert_eq!(Exchange::NYSE.full_name(), "New York Stock Exchange");
  /// assert_eq!(Exchange::BOVESPA.full_name(), "São Paulo Stock Exchange");
  /// assert_eq!(Exchange::OTHER.full_name(), "Other Exchange");
  /// ```
  pub fn full_name(&self) -> &'static str {
    match self {
      Exchange::NYSE => "New York Stock Exchange",
      Exchange::NASDAQ => "NASDAQ",
      Exchange::AMEX => "American Stock Exchange",
      Exchange::CBOT => "Chicago Board of Trade",
      Exchange::CME => "Chicago Mercantile Exchange",
      Exchange::LSE => "London Stock Exchange",
      Exchange::TSX => "Toronto Stock Exchange",
      Exchange::TSE => "Tokyo Stock Exchange",
      Exchange::HKSE => "Hong Kong Stock Exchange",
      Exchange::SSE => "Shanghai Stock Exchange",
      Exchange::SZSE => "Shenzhen Stock Exchange",
      Exchange::EURONEXT => "Euronext",
      Exchange::FRA => "Frankfurt Stock Exchange",
      Exchange::SIX => "Swiss Exchange",
      Exchange::ASX => "Australian Securities Exchange",
      Exchange::BSE => "Bombay Stock Exchange",
      Exchange::NSE => "National Stock Exchange of India",
      Exchange::BOVESPA => "São Paulo Stock Exchange",
      Exchange::MOEX => "Moscow Exchange",
      Exchange::KRX => "Korea Exchange",
      Exchange::TWSE => "Taiwan Stock Exchange",
      Exchange::SGX => "Singapore Exchange",
      Exchange::JSE => "Johannesburg Stock Exchange",
      Exchange::TASE => "Tel Aviv Stock Exchange",
      Exchange::OTHER => "Other Exchange",
    }
  }

  /// Returns the IANA timezone identifier for the exchange's primary trading location.
  ///
  /// The returned string is suitable for use with crates like `chrono-tz` to convert
  /// timestamps into the exchange's local time. [`Exchange::OTHER`] defaults to `"UTC"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::market::Exchange;
  ///
  /// assert_eq!(Exchange::TSE.timezone(), "Asia/Tokyo");
  /// assert_eq!(Exchange::LSE.timezone(), "Europe/London");
  /// assert_eq!(Exchange::OTHER.timezone(), "UTC");
  /// ```
  pub fn timezone(&self) -> &'static str {
    match self {
      Exchange::NYSE | Exchange::NASDAQ | Exchange::AMEX => "America/New_York",
      Exchange::CBOT | Exchange::CME => "America/Chicago",
      Exchange::LSE => "Europe/London",
      Exchange::TSX => "America/Toronto",
      Exchange::TSE => "Asia/Tokyo",
      Exchange::HKSE => "Asia/Hong_Kong",
      Exchange::SSE | Exchange::SZSE => "Asia/Shanghai",
      Exchange::EURONEXT => "Europe/Paris",
      Exchange::FRA => "Europe/Berlin",
      Exchange::SIX => "Europe/Zurich",
      Exchange::ASX => "Australia/Sydney",
      Exchange::BSE | Exchange::NSE => "Asia/Kolkata",
      Exchange::BOVESPA => "America/Sao_Paulo",
      Exchange::MOEX => "Europe/Moscow",
      Exchange::KRX => "Asia/Seoul",
      Exchange::TWSE => "Asia/Taipei",
      Exchange::SGX => "Asia/Singapore",
      Exchange::JSE => "Africa/Johannesburg",
      Exchange::TASE => "Asia/Jerusalem",
      Exchange::OTHER => "UTC",
    }
  }

  /// Returns the ISO 4217 currency code for the exchange's primary trading currency.
  ///
  /// Most securities on a given exchange are denominated in this currency, though
  /// individual listings may trade in other currencies (e.g., GDRs on the LSE
  /// denominated in USD). [`Exchange::OTHER`] defaults to `"USD"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::market::Exchange;
  ///
  /// assert_eq!(Exchange::HKSE.primary_currency(), "HKD");
  /// assert_eq!(Exchange::SIX.primary_currency(), "CHF");
  /// assert_eq!(Exchange::BSE.primary_currency(), "INR");
  /// ```
  pub fn primary_currency(&self) -> &'static str {
    match self {
      Exchange::NYSE | Exchange::NASDAQ | Exchange::AMEX | Exchange::CBOT | Exchange::CME => "USD",
      Exchange::LSE => "GBP",
      Exchange::TSX => "CAD",
      Exchange::TSE => "JPY",
      Exchange::HKSE => "HKD",
      Exchange::SSE | Exchange::SZSE => "CNY",
      Exchange::EURONEXT => "EUR",
      Exchange::FRA => "EUR",
      Exchange::SIX => "CHF",
      Exchange::ASX => "AUD",
      Exchange::BSE | Exchange::NSE => "INR",
      Exchange::BOVESPA => "BRL",
      Exchange::MOEX => "RUB",
      Exchange::KRX => "KRW",
      Exchange::TWSE => "TWD",
      Exchange::SGX => "SGD",
      Exchange::JSE => "ZAR",
      Exchange::TASE => "ILS",
      Exchange::OTHER => "USD",
    }
  }

  /// Returns `true` if this exchange is considered a major global exchange.
  ///
  /// "Major" here is defined as one of the top-tier exchanges by market capitalization
  /// and global influence. Currently includes:
  /// - **Americas:** NYSE, NASDAQ
  /// - **Europe:** LSE, Euronext, Frankfurt (FRA)
  /// - **Asia-Pacific:** TSE (Tokyo), HKSE (Hong Kong), SSE (Shanghai)
  ///
  /// This can be useful for filtering, prioritization, or tiered API query strategies.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::market::Exchange;
  ///
  /// assert!(Exchange::NYSE.is_major());
  /// assert!(Exchange::LSE.is_major());
  /// assert!(!Exchange::AMEX.is_major());
  /// assert!(!Exchange::OTHER.is_major());
  /// ```
  pub fn is_major(&self) -> bool {
    matches!(
      self,
      Exchange::NYSE
        | Exchange::NASDAQ
        | Exchange::LSE
        | Exchange::TSE
        | Exchange::HKSE
        | Exchange::EURONEXT
        | Exchange::SSE
        | Exchange::FRA
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_exchange_parsing() {
    assert_eq!("NYSE".parse::<Exchange>(), Ok(Exchange::NYSE));
    assert_eq!("nasdaq".parse::<Exchange>(), Ok(Exchange::NASDAQ));
    assert_eq!("new york stock exchange".parse::<Exchange>(), Ok(Exchange::NYSE));
    assert_eq!("UNKNOWN_EXCHANGE".parse::<Exchange>(), Ok(Exchange::OTHER));

    assert_eq!(Exchange::NYSE.full_name(), "New York Stock Exchange");
    assert_eq!(Exchange::NYSE.timezone(), "America/New_York");
    assert_eq!(Exchange::NYSE.primary_currency(), "USD");
    assert!(Exchange::NYSE.is_major());
    assert!(!Exchange::AMEX.is_major());
  }

  #[test]
  fn test_exchange_display() {
    assert_eq!(format!("{}", Exchange::NYSE), "NYSE");
    assert_eq!(format!("{}", Exchange::NASDAQ), "NASDAQ");
    assert_eq!(format!("{}", Exchange::OTHER), "OTHER");
  }
}
