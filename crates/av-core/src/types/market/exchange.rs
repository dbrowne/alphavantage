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

use serde::{Deserialize, Serialize};

/// Stock exchange identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
  NYSE,
  NASDAQ,
  AMEX,
  CBOT,
  CME,
  LSE,
  /// Toronto Stock Exchange
  TSX,
  /// Tokyo Stock Exchange
  TSE,
  /// Hong Kong Stock Exchange
  HKSE,
  /// Shanghai Stock Exchange
  SSE,
  /// Shenzhen Stock Exchange
  SZSE,
  /// Euronext
  EURONEXT,
  /// Frankfurt Stock Exchange
  FRA,
  /// Swiss Exchange
  SIX,
  /// Australian Securities Exchange
  ASX,
  /// Bombay Stock Exchange
  BSE,
  /// National Stock Exchange of India
  NSE,
  /// São Paulo Stock Exchange
  BOVESPA,
  /// Moscow Exchange
  MOEX,
  /// Korea Exchange
  KRX,
  /// Taiwan Stock Exchange
  TWSE,
  /// Singapore Exchange
  SGX,
  /// Johannesburg Stock Exchange
  JSE,
  /// Tel Aviv Stock Exchange
  TASE,
  /// Other/Unknown exchange
  OTHER,
}

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

impl Exchange {
  /// Parse exchange from string
  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_uppercase().as_str() {
      "NYSE" | "NEW YORK STOCK EXCHANGE" => Some(Exchange::NYSE),
      "NASDAQ" => Some(Exchange::NASDAQ),
      "AMEX" | "AMERICAN STOCK EXCHANGE" => Some(Exchange::AMEX),
      "CBOT" => Some(Exchange::CBOT),
      "CME" => Some(Exchange::CME),
      "LSE" | "LONDON STOCK EXCHANGE" => Some(Exchange::LSE),
      "TSX" | "TORONTO STOCK EXCHANGE" => Some(Exchange::TSX),
      "TSE" | "TOKYO STOCK EXCHANGE" => Some(Exchange::TSE),
      "HKSE" | "HONG KONG STOCK EXCHANGE" => Some(Exchange::HKSE),
      "SSE" | "SHANGHAI STOCK EXCHANGE" => Some(Exchange::SSE),
      "SZSE" | "SHENZHEN STOCK EXCHANGE" => Some(Exchange::SZSE),
      "EURONEXT" => Some(Exchange::EURONEXT),
      "FRA" | "FRANKFURT STOCK EXCHANGE" => Some(Exchange::FRA),
      "SIX" | "SWISS EXCHANGE" => Some(Exchange::SIX),
      "ASX" | "AUSTRALIAN SECURITIES EXCHANGE" => Some(Exchange::ASX),
      "BSE" | "BOMBAY STOCK EXCHANGE" => Some(Exchange::BSE),
      "NSE" | "NATIONAL STOCK EXCHANGE OF INDIA" => Some(Exchange::NSE),
      "BOVESPA" => Some(Exchange::BOVESPA),
      "MOEX" | "MOSCOW EXCHANGE" => Some(Exchange::MOEX),
      "KRX" | "KOREA EXCHANGE" => Some(Exchange::KRX),
      "TWSE" | "TAIWAN STOCK EXCHANGE" => Some(Exchange::TWSE),
      "SGX" | "SINGAPORE EXCHANGE" => Some(Exchange::SGX),
      "JSE" | "JOHANNESBURG STOCK EXCHANGE" => Some(Exchange::JSE),
      "TASE" | "TEL AVIV STOCK EXCHANGE" => Some(Exchange::TASE),
      _ => Some(Exchange::OTHER),
    }
  }

  /// Get the full name of the exchange
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

  /// Get the timezone for the exchange
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

  /// Get the primary currency for the exchange
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

  /// Check if this is a major global exchange
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
    assert_eq!(Exchange::from_str("NYSE"), Some(Exchange::NYSE));
    assert_eq!(Exchange::from_str("nasdaq"), Some(Exchange::NASDAQ));
    assert_eq!(Exchange::from_str("new york stock exchange"), Some(Exchange::NYSE));
    assert_eq!(Exchange::from_str("UNKNOWN_EXCHANGE"), Some(Exchange::OTHER));

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
