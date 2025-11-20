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

// Bit allocation based on typical universe sizes
// Total: 64 bits (1 bit sign, 63 bits usable)
// Format: [SecurityType Identifier (4-8 bits)] [Unique ID (remaining bits)]

// Security type identifiers (using prefix codes for variable length)
const TYPE_COMMON_STOCK: u8 = 0b0000; // 4 bits - millions of stocks
const TYPE_PREFERRED: u8 = 0b0001; // 4 bits - tens of thousands
const TYPE_ETF: u8 = 0b0010; // 4 bits - tens of thousands
const TYPE_MUTUAL_FUND: u8 = 0b0011; // 4 bits - tens of thousands
const TYPE_OPTION: u8 = 0b0100; // 4 bits - hundreds of millions
const TYPE_FUTURE: u8 = 0b0101; // 4 bits - tens of millions
const TYPE_WARRANT: u8 = 0b0110; // 4 bits - hundreds of thousands
const TYPE_ADR: u8 = 0b0111; // 4 bits - thousands

// Less common types get longer prefixes
const TYPE_BOND: u8 = 0b10000; // 5 bits - millions
const TYPE_GOVT_BOND: u8 = 0b10001; // 5 bits - tens of thousands
const TYPE_CORP_BOND: u8 = 0b10010; // 5 bits - hundreds of thousands
const TYPE_MUNI_BOND: u8 = 0b10011; // 5 bits - hundreds of thousands
const TYPE_CRYPTO: u8 = 0b10100; // 5 bits - tens of thousands
const TYPE_REIT: u8 = 0b10101; // 5 bits - thousands

// Smallest universes get longest prefixes
const TYPE_CURRENCY: u8 = 0b110000; // 6 bits - ~200 pairs
const TYPE_INDEX: u8 = 0b110001; // 6 bits - thousands
const TYPE_COMMODITY: u8 = 0b110010; // 6 bits - hundreds
const TYPE_CD: u8 = 0b110011; // 6 bits - tens of thousands
const TYPE_T_BILL: u8 = 0b110100; // 6 bits - hundreds
const TYPE_OTHER: u8 = 0b111111; // 6 bits - catch-all

// Bit shifts based on type prefix length
const SHIFT_4BIT: u8 = 60; // 64 - 4 = 60 bits for ID (1.15 x 10^18)
const SHIFT_5BIT: u8 = 59; // 64 - 5 = 59 bits for ID (5.76 x 10^17)
const SHIFT_6BIT: u8 = 58; // 64 - 6 = 58 bits for ID (2.88 x 10^17)

/// Type of security
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
  Equity,
  PreferredStock,
  ETF,
  MutualFund,
  REIT,
  ADR,
  CD,
  Bond,
  GovernmentBond,
  CorporateBond,
  MunicipalBond,
  TreasuryBill,
  Option,
  Future,
  Warrant,
  Index,
  Currency,
  Commodity,
  Cryptocurrency,
  Other,
}

/// A struct that represents a Security identifier with bitmap encoding
#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash, Deserialize)]
pub struct SecurityIdentifier {
  pub security_type: SecurityType,
  pub raw_id: u32,
}

impl SecurityIdentifier {
  /// Decode a full SecurityIdentifier from an encoded i64 SID
  pub fn decode(encoded_id: i64) -> Option<SecurityIdentifier> {
    let security_type = SecurityType::decode_type(encoded_id);
    let shift = SecurityType::get_shift(security_type);
    let mask = (1i64 << shift) - 1;
    let raw_id = (encoded_id & mask) as u32;

    Some(SecurityIdentifier { security_type, raw_id })
  }
}

impl std::fmt::Display for SecurityType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SecurityType::Equity => write!(f, "Common Stock"),
      SecurityType::PreferredStock => write!(f, "Preferred Stock"),
      SecurityType::ETF => write!(f, "ETF"),
      SecurityType::MutualFund => write!(f, "Mutual Fund"),
      SecurityType::REIT => write!(f, "REIT"),
      SecurityType::ADR => write!(f, "ADR"),
      SecurityType::CD => write!(f, "Certificate of Deposit"),
      SecurityType::Bond => write!(f, "Bond"),
      SecurityType::GovernmentBond => write!(f, "Government Bond"),
      SecurityType::CorporateBond => write!(f, "Corporate Bond"),
      SecurityType::MunicipalBond => write!(f, "Municipal Bond"),
      SecurityType::TreasuryBill => write!(f, "Treasury Bill"),
      SecurityType::Option => write!(f, "Option"),
      SecurityType::Future => write!(f, "Future"),
      SecurityType::Warrant => write!(f, "Warrant"),
      SecurityType::Index => write!(f, "Index"),
      SecurityType::Currency => write!(f, "Currency"),
      SecurityType::Commodity => write!(f, "Commodity"),
      SecurityType::Cryptocurrency => write!(f, "Cryptocurrency"),
      SecurityType::Other => write!(f, "Other"),
    }
  }
}

impl SecurityType {
  /// Encode with variable bit allocation based on expected universe size
  pub fn encode(st: SecurityType, id: u32) -> i64 {
    let unsigned_result = match st {
      // High-volume types (4-bit prefix, 60 bits for ID)
      SecurityType::Equity => (TYPE_COMMON_STOCK as i64) << SHIFT_4BIT | id as i64,
      SecurityType::PreferredStock => (TYPE_PREFERRED as i64) << SHIFT_4BIT | id as i64,
      SecurityType::ETF => (TYPE_ETF as i64) << SHIFT_4BIT | id as i64,
      SecurityType::MutualFund => (TYPE_MUTUAL_FUND as i64) << SHIFT_4BIT | id as i64,
      SecurityType::Option => (TYPE_OPTION as i64) << SHIFT_4BIT | id as i64,
      SecurityType::Future => (TYPE_FUTURE as i64) << SHIFT_4BIT | id as i64,
      SecurityType::Warrant => (TYPE_WARRANT as i64) << SHIFT_4BIT | id as i64,
      SecurityType::ADR => (TYPE_ADR as i64) << SHIFT_4BIT | id as i64,

      // Medium-volume types (5-bit prefix, 59 bits for ID)
      SecurityType::Bond => (TYPE_BOND as i64) << SHIFT_5BIT | id as i64,
      SecurityType::GovernmentBond => (TYPE_GOVT_BOND as i64) << SHIFT_5BIT | id as i64,
      SecurityType::CorporateBond => (TYPE_CORP_BOND as i64) << SHIFT_5BIT | id as i64,
      SecurityType::MunicipalBond => (TYPE_MUNI_BOND as i64) << SHIFT_5BIT | id as i64,
      SecurityType::Cryptocurrency => (TYPE_CRYPTO as i64) << SHIFT_5BIT | id as i64,
      SecurityType::REIT => (TYPE_REIT as i64) << SHIFT_5BIT | id as i64,

      // Low-volume types (6-bit prefix, 58 bits for ID)
      SecurityType::Currency => (TYPE_CURRENCY as i64) << SHIFT_6BIT | id as i64,
      SecurityType::Index => (TYPE_INDEX as i64) << SHIFT_6BIT | id as i64,
      SecurityType::Commodity => (TYPE_COMMODITY as i64) << SHIFT_6BIT | id as i64,
      SecurityType::CD => (TYPE_CD as i64) << SHIFT_6BIT | id as i64,
      SecurityType::TreasuryBill => (TYPE_T_BILL as i64) << SHIFT_6BIT | id as i64,
      SecurityType::Other => (TYPE_OTHER as i64) << SHIFT_6BIT | id as i64,
    };
    unsigned_result
  }

  /// Decode SecurityType from an encoded SID
  pub fn decode_type(sid: i64) -> SecurityType {
    // Check 4-bit types first (most common)
    let type_4bit = (sid >> SHIFT_4BIT) & 0b1111;
    match type_4bit {
      x if x == TYPE_COMMON_STOCK as i64 => return SecurityType::Equity,
      x if x == TYPE_PREFERRED as i64 => return SecurityType::PreferredStock,
      x if x == TYPE_ETF as i64 => return SecurityType::ETF,
      x if x == TYPE_MUTUAL_FUND as i64 => return SecurityType::MutualFund,
      x if x == TYPE_OPTION as i64 => return SecurityType::Option,
      x if x == TYPE_FUTURE as i64 => return SecurityType::Future,
      x if x == TYPE_WARRANT as i64 => return SecurityType::Warrant,
      x if x == TYPE_ADR as i64 => return SecurityType::ADR,
      _ => {}
    }

    // Check 5-bit types
    let type_5bit = (sid >> SHIFT_5BIT) & 0b11111;
    match type_5bit {
      x if x == TYPE_BOND as i64 => return SecurityType::Bond,
      x if x == TYPE_GOVT_BOND as i64 => return SecurityType::GovernmentBond,
      x if x == TYPE_CORP_BOND as i64 => return SecurityType::CorporateBond,
      x if x == TYPE_MUNI_BOND as i64 => return SecurityType::MunicipalBond,
      x if x == TYPE_CRYPTO as i64 => return SecurityType::Cryptocurrency,
      x if x == TYPE_REIT as i64 => return SecurityType::REIT,
      _ => {}
    }

    // Check 6-bit types
    let type_6bit = (sid >> SHIFT_6BIT) & 0b111111;
    match type_6bit {
      x if x == TYPE_CURRENCY as i64 => SecurityType::Currency,
      x if x == TYPE_INDEX as i64 => SecurityType::Index,
      x if x == TYPE_COMMODITY as i64 => SecurityType::Commodity,
      x if x == TYPE_CD as i64 => SecurityType::CD,
      x if x == TYPE_T_BILL as i64 => SecurityType::TreasuryBill,
      x if x == TYPE_OTHER as i64 => SecurityType::Other,
      _ => SecurityType::Other,
    }
  }

  /// Get the bit shift for encoding based on security type
  fn get_shift(st: SecurityType) -> u8 {
    match st {
      SecurityType::Equity
      | SecurityType::PreferredStock
      | SecurityType::ETF
      | SecurityType::MutualFund
      | SecurityType::Option
      | SecurityType::Future
      | SecurityType::Warrant
      | SecurityType::ADR => SHIFT_4BIT,

      SecurityType::Bond
      | SecurityType::GovernmentBond
      | SecurityType::CorporateBond
      | SecurityType::MunicipalBond
      | SecurityType::Cryptocurrency
      | SecurityType::REIT => SHIFT_5BIT,

      _ => SHIFT_6BIT,
    }
  }

  /// Map AlphaVantage asset type string to SecurityType
  pub fn from_alpha_vantage(asset_type: &str) -> Self {
    match asset_type.to_uppercase().replace([' ', '_', '-'], "").as_str() {
      "EQUITY" | "CS" | "COMMONSTOCK" => SecurityType::Equity,
      "PREFERREDSTOCK" | "PS" => SecurityType::PreferredStock,
      "EXCHANGETRADEDFUND" | "ETF" => SecurityType::ETF,
      "MUTUALFUND" | "MF" => SecurityType::MutualFund,
      "AMERICANDEPOSITARYRECEIPT" | "ADR" => SecurityType::ADR,
      "REALESTATEINVESTMENTTRUST" | "REIT" => SecurityType::REIT,
      "WARRANT" | "WT" => SecurityType::Warrant,
      "BOND" => SecurityType::Bond,
      "GOVERNMENTBOND" => SecurityType::GovernmentBond,
      "CORPORATEBOND" => SecurityType::CorporateBond,
      "MUNICIPALBOND" => SecurityType::MunicipalBond,
      "TREASURYBILL" | "TBILL" => SecurityType::TreasuryBill,
      "OPTION" => SecurityType::Option,
      "FUTURE" | "FUTURES" => SecurityType::Future,
      "CRYPTOCURRENCY" | "CRYPTO" | "DIGITALCURRENCY" => SecurityType::Cryptocurrency,
      "CURRENCY" | "FX" | "FOREX" => SecurityType::Currency,
      "INDEX" => SecurityType::Index,
      "COMMODITY" => SecurityType::Commodity,
      "CERTIFICATEOFDEPOSIT" | "CD" => SecurityType::CD,
      _ => SecurityType::Other,
    }
  }

  /// Convert SecurityType to AlphaVantage asset type string
  pub fn to_alpha_vantage(&self) -> &'static str {
    match self {
      SecurityType::Equity => "Common Stock",
      SecurityType::PreferredStock => "Preferred Stock",
      SecurityType::ETF => "Exchange Traded Fund",
      SecurityType::MutualFund => "Mutual Fund",
      SecurityType::ADR => "American Depositary Receipt",
      SecurityType::REIT => "Real Estate Investment Trust",
      SecurityType::Warrant => "Warrant",
      SecurityType::Bond => "Bond",
      SecurityType::GovernmentBond => "Government Bond",
      SecurityType::CorporateBond => "Corporate Bond",
      SecurityType::MunicipalBond => "Municipal Bond",
      SecurityType::TreasuryBill => "Treasury Bill",
      SecurityType::Option => "Option",
      SecurityType::Future => "Future",
      SecurityType::Currency => "Currency",
      SecurityType::Index => "Index",
      SecurityType::Commodity => "Commodity",
      SecurityType::Cryptocurrency => "Cryptocurrency",
      SecurityType::CD => "Certificate of Deposit",
      SecurityType::Other => "Other",
    }
  }

  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
      "COMMONSTOCK" | "EQUITY" | "STOCK" => Some(SecurityType::Equity),
      "PREFERREDSTOCK" | "PREFERRED" => Some(SecurityType::PreferredStock),
      "ETF" | "EXCHANGETRADEDFUND" => Some(SecurityType::ETF),
      "MUTUALFUND" | "FUND" => Some(SecurityType::MutualFund),
      "REIT" | "REALESTATEINVESTMENTTRUST" => Some(SecurityType::REIT),
      "ADR" | "AMERICANDEPOSITARYRECEIPT" => Some(SecurityType::ADR),
      "CD" | "CERTIFICATEOFDEPOSIT" => Some(SecurityType::CD),
      "BOND" => Some(SecurityType::Bond),
      "GOVERNMENTBOND" | "GOVBOND" => Some(SecurityType::GovernmentBond),
      "CORPORATEBOND" | "CORPBOND" => Some(SecurityType::CorporateBond),
      "MUNICIPALBOND" | "MUNIBOND" => Some(SecurityType::MunicipalBond),
      "TREASURYBILL" | "TBILL" => Some(SecurityType::TreasuryBill),
      "OPTION" => Some(SecurityType::Option),
      "FUTURE" | "FUTURES" => Some(SecurityType::Future),
      "WARRANT" => Some(SecurityType::Warrant),
      "INDEX" => Some(SecurityType::Index),
      "CURRENCY" | "FX" | "FOREX" => Some(SecurityType::Currency),
      "COMMODITY" => Some(SecurityType::Commodity),
      "CRYPTOCURRENCY" | "CRYPTO" => Some(SecurityType::Cryptocurrency),
      _ => Some(SecurityType::Other),
    }
  }

  /// Check if this security type represents equity
  pub fn is_equity(&self) -> bool {
    matches!(
      self,
      SecurityType::Equity
        | SecurityType::PreferredStock
        | SecurityType::ETF
        | SecurityType::REIT
        | SecurityType::ADR
    )
  }

  /// Check if this security type represents fixed income
  pub fn is_fixed_income(&self) -> bool {
    matches!(
      self,
      SecurityType::Bond
        | SecurityType::GovernmentBond
        | SecurityType::CorporateBond
        | SecurityType::MunicipalBond
        | SecurityType::TreasuryBill
        | SecurityType::CD
    )
  }

  /// Check if this security type represents derivatives
  pub fn is_derivative(&self) -> bool {
    matches!(self, SecurityType::Option | SecurityType::Future | SecurityType::Warrant)
  }

  /// Get the typical settlement period in days
  pub fn settlement_days(&self) -> u8 {
    match self {
      SecurityType::Equity
      | SecurityType::PreferredStock
      | SecurityType::ETF
      | SecurityType::REIT
      | SecurityType::ADR => 2, // T+2
      SecurityType::MutualFund => 1, // T+1
      SecurityType::Bond
      | SecurityType::GovernmentBond
      | SecurityType::CorporateBond
      | SecurityType::MunicipalBond => 1, // T+1
      SecurityType::TreasuryBill => 1, // T+1
      SecurityType::Option => 1,     // T+1
      SecurityType::Future => 0,     // Daily mark-to-market
      SecurityType::Currency => 2,   // T+2
      SecurityType::Commodity => 0,  // Immediate
      SecurityType::Cryptocurrency => 0, // Immediate
      _ => 2,                        // Default T+2
    }
  }
}

/// Top movers type (gainers, losers, most active)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TopType {
  Gainers,
  Losers,
  MostActive,
}

impl std::fmt::Display for TopType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TopType::Gainers => write!(f, "gainers"),
      TopType::Losers => write!(f, "losers"),
      TopType::MostActive => write!(f, "most_active"),
    }
  }
}

impl TopType {
  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().replace([' ', '-', '_'], "").as_str() {
      "gainers" | "topgainers" | "winners" => Some(TopType::Gainers),
      "losers" | "toplosers" | "decliners" => Some(TopType::Losers),
      "mostactive" | "active" | "volume" => Some(TopType::MostActive),
      _ => None,
    }
  }
}

/// Market sector classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sector {
  Technology,
  Healthcare,
  FinancialServices,
  ConsumerDiscretionary,
  ConsumerStaples,
  Industrials,
  Energy,
  Materials,
  RealEstate,
  Utilities,
  CommunicationServices,
  Other,
}

impl std::fmt::Display for Sector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Sector::Technology => write!(f, "Technology"),
      Sector::Healthcare => write!(f, "Healthcare"),
      Sector::FinancialServices => write!(f, "Financial Services"),
      Sector::ConsumerDiscretionary => write!(f, "Consumer Discretionary"),
      Sector::ConsumerStaples => write!(f, "Consumer Staples"),
      Sector::Industrials => write!(f, "Industrials"),
      Sector::Energy => write!(f, "Energy"),
      Sector::Materials => write!(f, "Materials"),
      Sector::RealEstate => write!(f, "Real Estate"),
      Sector::Utilities => write!(f, "Utilities"),
      Sector::CommunicationServices => write!(f, "Communication Services"),
      Sector::Other => write!(f, "Other"),
    }
  }
}

impl Sector {
  /// Parse sector from string
  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
      "TECHNOLOGY" | "TECH" | "IT" | "INFORMATIONTECHNOLOGY" => Some(Sector::Technology),
      "HEALTHCARE" | "HEALTH" | "MEDICAL" | "PHARMA" | "PHARMACEUTICAL" => Some(Sector::Healthcare),
      "FINANCIALSERVICES" | "FINANCIAL" | "FINANCE" | "BANKING" | "FINTECH" => {
        Some(Sector::FinancialServices)
      }
      "CONSUMERDISCRETIONARY" | "CONSUMER" | "RETAIL" | "DISCRETIONARY" => {
        Some(Sector::ConsumerDiscretionary)
      }
      "CONSUMERSTAPLES" | "STAPLES" | "DEFENSIVE" => Some(Sector::ConsumerStaples),
      "INDUSTRIALS" | "INDUSTRIAL" | "MANUFACTURING" => Some(Sector::Industrials),
      "ENERGY" | "OIL" | "GAS" | "PETROLEUM" => Some(Sector::Energy),
      "MATERIALS" | "BASIC" | "BASICMATERIALS" | "MINING" => Some(Sector::Materials),
      "REALESTATE" | "PROPERTY" | "REIT" => Some(Sector::RealEstate),
      "UTILITIES" | "UTILITY" | "POWER" | "ELECTRIC" => Some(Sector::Utilities),
      "COMMUNICATIONSERVICES" | "COMMUNICATION" | "TELECOM" | "MEDIA" => {
        Some(Sector::CommunicationServices)
      }
      _ => Some(Sector::Other),
    }
  }

  /// Check if this is a cyclical sector
  pub fn is_cyclical(&self) -> bool {
    matches!(
      self,
      Sector::Technology
        | Sector::ConsumerDiscretionary
        | Sector::Industrials
        | Sector::Energy
        | Sector::Materials
        | Sector::FinancialServices
    )
  }

  /// Check if this is a defensive sector
  pub fn is_defensive(&self) -> bool {
    matches!(self, Sector::Healthcare | Sector::ConsumerStaples | Sector::Utilities)
  }

  /// Get typical P/E ratio range for the sector
  pub fn typical_pe_range(&self) -> (f64, f64) {
    match self {
      Sector::Technology => (15.0, 35.0),
      Sector::Healthcare => (12.0, 25.0),
      Sector::FinancialServices => (8.0, 15.0),
      Sector::ConsumerDiscretionary => (12.0, 25.0),
      Sector::ConsumerStaples => (15.0, 25.0),
      Sector::Industrials => (12.0, 20.0),
      Sector::Energy => (8.0, 15.0),
      Sector::Materials => (10.0, 18.0),
      Sector::RealEstate => (15.0, 30.0),
      Sector::Utilities => (12.0, 20.0),
      Sector::CommunicationServices => (10.0, 25.0),
      Sector::Other => (10.0, 25.0),
    }
  }
}

/// Market cap classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketCap {
  /// Nano cap (< $50M)
  NanoCap,
  /// Micro cap ($50M - $300M)
  MicroCap,
  /// Small cap ($300M - $2B)
  SmallCap,
  /// Mid cap ($2B - $10B)
  MidCap,
  /// Large cap ($10B - $200B)
  LargeCap,
  /// Mega cap (> $200B)
  MegaCap,
}

impl std::fmt::Display for MarketCap {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MarketCap::NanoCap => write!(f, "Nano Cap"),
      MarketCap::MicroCap => write!(f, "Micro Cap"),
      MarketCap::SmallCap => write!(f, "Small Cap"),
      MarketCap::MidCap => write!(f, "Mid Cap"),
      MarketCap::LargeCap => write!(f, "Large Cap"),
      MarketCap::MegaCap => write!(f, "Mega Cap"),
    }
  }
}

impl MarketCap {
  /// Classify market cap from value in USD
  pub fn from_value(market_cap_usd: f64) -> Self {
    if market_cap_usd < 50_000_000.0 {
      MarketCap::NanoCap
    } else if market_cap_usd < 300_000_000.0 {
      MarketCap::MicroCap
    } else if market_cap_usd < 2_000_000_000.0 {
      MarketCap::SmallCap
    } else if market_cap_usd < 10_000_000_000.0 {
      MarketCap::MidCap
    } else if market_cap_usd < 200_000_000_000.0 {
      MarketCap::LargeCap
    } else {
      MarketCap::MegaCap
    }
  }

  /// Get the range for this market cap category
  pub fn range(&self) -> (f64, Option<f64>) {
    match self {
      MarketCap::NanoCap => (0.0, Some(50_000_000.0)),
      MarketCap::MicroCap => (50_000_000.0, Some(300_000_000.0)),
      MarketCap::SmallCap => (300_000_000.0, Some(2_000_000_000.0)),
      MarketCap::MidCap => (2_000_000_000.0, Some(10_000_000_000.0)),
      MarketCap::LargeCap => (10_000_000_000.0, Some(200_000_000_000.0)),
      MarketCap::MegaCap => (200_000_000_000.0, None),
    }
  }

  /// Check if this is considered a large company
  pub fn is_large(&self) -> bool {
    matches!(self, MarketCap::LargeCap | MarketCap::MegaCap)
  }

  /// Check if this is considered a small company
  pub fn is_small(&self) -> bool {
    matches!(self, MarketCap::NanoCap | MarketCap::MicroCap | MarketCap::SmallCap)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_security_type_parsing() {
    assert_eq!(SecurityType::from_str("Common Stock"), Some(SecurityType::Equity));
    assert_eq!(SecurityType::from_str("ETF"), Some(SecurityType::ETF));
    assert!(SecurityType::Equity.is_equity());
    assert!(SecurityType::Bond.is_fixed_income());
    assert!(SecurityType::Option.is_derivative());
    assert_eq!(SecurityType::Equity.settlement_days(), 2);
  }

  // ===== Exchange Tests =====
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

  // ===== SecurityType Encoding/Decoding Tests =====
  #[test]
  fn test_security_type_encode_decode_4bit() {
    // Test all 4-bit types
    let test_cases = vec![
      (SecurityType::Equity, 12345),
      (SecurityType::PreferredStock, 67890),
      (SecurityType::ETF, 11111),
      (SecurityType::MutualFund, 99999),
      (SecurityType::Option, 55555),
      (SecurityType::Future, 77777),
      (SecurityType::Warrant, 33333),
      (SecurityType::ADR, 44444),
    ];

    for (sec_type, id) in test_cases {
      let encoded = SecurityType::encode(sec_type, id);
      let decoded = SecurityType::decode_type(encoded);
      assert_eq!(decoded, sec_type, "Failed to decode {:?}", sec_type);

      // Also test SecurityIdentifier decode
      let identifier = SecurityIdentifier::decode(encoded).unwrap();
      assert_eq!(identifier.security_type, sec_type);
      assert_eq!(identifier.raw_id, id);
    }
  }

  #[test]
  fn test_security_type_encode_decode_5bit() {
    // Test all 5-bit types
    let test_cases = vec![
      (SecurityType::Bond, 12345),
      (SecurityType::GovernmentBond, 67890),
      (SecurityType::CorporateBond, 11111),
      (SecurityType::MunicipalBond, 99999),
      (SecurityType::Cryptocurrency, 55555),
      (SecurityType::REIT, 77777),
    ];

    for (sec_type, id) in test_cases {
      let encoded = SecurityType::encode(sec_type, id);
      let decoded = SecurityType::decode_type(encoded);
      assert_eq!(decoded, sec_type, "Failed to decode {:?}", sec_type);

      let identifier = SecurityIdentifier::decode(encoded).unwrap();
      assert_eq!(identifier.security_type, sec_type);
      assert_eq!(identifier.raw_id, id);
    }
  }

  #[test]
  fn test_security_type_encode_decode_6bit() {
    // Test all 6-bit types
    let test_cases = vec![
      (SecurityType::Currency, 12345),
      (SecurityType::Index, 67890),
      (SecurityType::Commodity, 11111),
      (SecurityType::CD, 99999),
      (SecurityType::TreasuryBill, 55555),
      (SecurityType::Other, 77777),
    ];

    for (sec_type, id) in test_cases {
      let encoded = SecurityType::encode(sec_type, id);
      let decoded = SecurityType::decode_type(encoded);
      assert_eq!(decoded, sec_type, "Failed to decode {:?}", sec_type);

      let identifier = SecurityIdentifier::decode(encoded).unwrap();
      assert_eq!(identifier.security_type, sec_type);
      assert_eq!(identifier.raw_id, id);
    }
  }

  #[test]
  fn test_security_type_encode_max_values() {
    // Test with maximum ID values for each bit size
    let max_4bit = u32::MAX >> 4; // 60 bits available for ID
    let max_5bit = u32::MAX >> 5; // 59 bits available for ID
    let max_6bit = u32::MAX >> 6; // 58 bits available for ID

    // Test 4-bit type with max ID that fits in u32
    let encoded = SecurityType::encode(SecurityType::Equity, u32::MAX);
    let decoded = SecurityType::decode_type(encoded);
    assert_eq!(decoded, SecurityType::Equity);

    let identifier = SecurityIdentifier::decode(encoded).unwrap();
    assert_eq!(identifier.raw_id, u32::MAX);
  }

  #[test]
  fn test_security_type_get_shift() {
    assert_eq!(SecurityType::get_shift(SecurityType::Equity), SHIFT_4BIT);
    assert_eq!(SecurityType::get_shift(SecurityType::Option), SHIFT_4BIT);
    assert_eq!(SecurityType::get_shift(SecurityType::Bond), SHIFT_5BIT);
    assert_eq!(SecurityType::get_shift(SecurityType::Cryptocurrency), SHIFT_5BIT);
    assert_eq!(SecurityType::get_shift(SecurityType::Currency), SHIFT_6BIT);
    assert_eq!(SecurityType::get_shift(SecurityType::Other), SHIFT_6BIT);
  }

  #[test]
  fn test_security_identifier_decode_edge_cases() {
    // Test with 0 ID
    let encoded = SecurityType::encode(SecurityType::Equity, 0);
    let identifier = SecurityIdentifier::decode(encoded).unwrap();
    assert_eq!(identifier.security_type, SecurityType::Equity);
    assert_eq!(identifier.raw_id, 0);

    // Test with 1 ID
    let encoded = SecurityType::encode(SecurityType::Bond, 1);
    let identifier = SecurityIdentifier::decode(encoded).unwrap();
    assert_eq!(identifier.security_type, SecurityType::Bond);
    assert_eq!(identifier.raw_id, 1);
  }

  #[test]
  fn test_bitmap_non_overlap() {
    // Ensure different security types don't produce overlapping encodings
    let id = 12345u32;
    let mut encoded_values = std::collections::HashSet::new();

    let types = vec![
      SecurityType::Equity,
      SecurityType::PreferredStock,
      SecurityType::ETF,
      SecurityType::Bond,
      SecurityType::Currency,
      SecurityType::Cryptocurrency,
    ];

    for sec_type in types {
      let encoded = SecurityType::encode(sec_type, id);
      assert!(encoded_values.insert(encoded), "Duplicate encoding found for {:?}", sec_type);
    }
  }

  // ===== SecurityType Parsing and Utility Tests =====
  #[test]
  fn test_security_type_from_str() {
    assert_eq!(SecurityType::from_str("Common Stock"), Some(SecurityType::Equity));
    assert_eq!(SecurityType::from_str("EQUITY"), Some(SecurityType::Equity));
    assert_eq!(SecurityType::from_str("stock"), Some(SecurityType::Equity));
    assert_eq!(SecurityType::from_str("ETF"), Some(SecurityType::ETF));
    assert_eq!(SecurityType::from_str("Exchange Traded Fund"), Some(SecurityType::ETF));
    assert_eq!(SecurityType::from_str("CRYPTO"), Some(SecurityType::Cryptocurrency));
    assert_eq!(SecurityType::from_str("fx"), Some(SecurityType::Currency));
    assert_eq!(SecurityType::from_str("UNKNOWN"), Some(SecurityType::Other));
  }

  #[test]
  fn test_security_type_display() {
    assert_eq!(format!("{}", SecurityType::Equity), "Common Stock");
    assert_eq!(format!("{}", SecurityType::ETF), "ETF");
    assert_eq!(format!("{}", SecurityType::Cryptocurrency), "Cryptocurrency");
  }

  #[test]
  fn test_security_type_categories() {
    // Test equity types
    assert!(SecurityType::Equity.is_equity());
    assert!(SecurityType::PreferredStock.is_equity());
    assert!(SecurityType::ETF.is_equity());
    assert!(SecurityType::REIT.is_equity());
    assert!(SecurityType::ADR.is_equity());
    assert!(!SecurityType::Bond.is_equity());
    assert!(!SecurityType::Option.is_equity());

    // Test fixed income types
    assert!(SecurityType::Bond.is_fixed_income());
    assert!(SecurityType::GovernmentBond.is_fixed_income());
    assert!(SecurityType::CorporateBond.is_fixed_income());
    assert!(SecurityType::MunicipalBond.is_fixed_income());
    assert!(SecurityType::TreasuryBill.is_fixed_income());
    assert!(SecurityType::CD.is_fixed_income());
    assert!(!SecurityType::Equity.is_fixed_income());
    assert!(!SecurityType::Option.is_fixed_income());

    // Test derivative types
    assert!(SecurityType::Option.is_derivative());
    assert!(SecurityType::Future.is_derivative());
    assert!(SecurityType::Warrant.is_derivative());
    assert!(!SecurityType::Equity.is_derivative());
    assert!(!SecurityType::Bond.is_derivative());
  }

  #[test]
  fn test_security_type_settlement_days() {
    assert_eq!(SecurityType::Equity.settlement_days(), 2);
    assert_eq!(SecurityType::MutualFund.settlement_days(), 1);
    assert_eq!(SecurityType::Bond.settlement_days(), 1);
    assert_eq!(SecurityType::Option.settlement_days(), 1);
    assert_eq!(SecurityType::Future.settlement_days(), 0);
    assert_eq!(SecurityType::Currency.settlement_days(), 2);
    assert_eq!(SecurityType::Cryptocurrency.settlement_days(), 0);
  }

  // ===== TopType Tests =====
  #[test]
  fn test_top_type_parsing() {
    assert_eq!(TopType::from_str("gainers"), Some(TopType::Gainers));
    assert_eq!(TopType::from_str("TOP_GAINERS"), Some(TopType::Gainers));
    assert_eq!(TopType::from_str("winners"), Some(TopType::Gainers));
    assert_eq!(TopType::from_str("losers"), Some(TopType::Losers));
    assert_eq!(TopType::from_str("decliners"), Some(TopType::Losers));
    assert_eq!(TopType::from_str("most_active"), Some(TopType::MostActive));
    assert_eq!(TopType::from_str("volume"), Some(TopType::MostActive));
    assert_eq!(TopType::from_str("invalid"), None);
  }

  #[test]
  fn test_top_type_display() {
    assert_eq!(format!("{}", TopType::Gainers), "gainers");
    assert_eq!(format!("{}", TopType::Losers), "losers");
    assert_eq!(format!("{}", TopType::MostActive), "most_active");
  }

  // ===== Sector Tests =====
  #[test]
  fn test_sector_parsing() {
    assert_eq!(Sector::from_str("Technology"), Some(Sector::Technology));
    assert_eq!(Sector::from_str("TECH"), Some(Sector::Technology));
    assert_eq!(Sector::from_str("Information Technology"), Some(Sector::Technology));
    assert_eq!(Sector::from_str("Healthcare"), Some(Sector::Healthcare));
    assert_eq!(Sector::from_str("PHARMA"), Some(Sector::Healthcare));
    assert_eq!(Sector::from_str("Financial Services"), Some(Sector::FinancialServices));
    assert_eq!(Sector::from_str("BANKING"), Some(Sector::FinancialServices));
    assert_eq!(Sector::from_str("UNKNOWN_SECTOR"), Some(Sector::Other));
  }

  #[test]
  fn test_sector_display() {
    assert_eq!(format!("{}", Sector::Technology), "Technology");
    assert_eq!(format!("{}", Sector::FinancialServices), "Financial Services");
    assert_eq!(format!("{}", Sector::Other), "Other");
  }

  #[test]
  fn test_sector_classification() {
    // Test cyclical sectors
    assert!(Sector::Technology.is_cyclical());
    assert!(Sector::ConsumerDiscretionary.is_cyclical());
    assert!(Sector::Industrials.is_cyclical());
    assert!(Sector::Energy.is_cyclical());
    assert!(Sector::Materials.is_cyclical());
    assert!(Sector::FinancialServices.is_cyclical());
    assert!(!Sector::Healthcare.is_cyclical());
    assert!(!Sector::Utilities.is_cyclical());

    // Test defensive sectors
    assert!(Sector::Healthcare.is_defensive());
    assert!(Sector::ConsumerStaples.is_defensive());
    assert!(Sector::Utilities.is_defensive());
    assert!(!Sector::Technology.is_defensive());
    assert!(!Sector::Energy.is_defensive());
  }

  #[test]
  fn test_sector_pe_ranges() {
    let (min, max) = Sector::Technology.typical_pe_range();
    assert_eq!(min, 15.0);
    assert_eq!(max, 35.0);

    let (min, max) = Sector::FinancialServices.typical_pe_range();
    assert_eq!(min, 8.0);
    assert_eq!(max, 15.0);

    let (min, max) = Sector::Utilities.typical_pe_range();
    assert_eq!(min, 12.0);
    assert_eq!(max, 20.0);
  }

  // ===== MarketCap Tests =====
  #[test]
  fn test_market_cap_classification() {
    assert_eq!(MarketCap::from_value(10_000_000.0), MarketCap::NanoCap);
    assert_eq!(MarketCap::from_value(100_000_000.0), MarketCap::MicroCap);
    assert_eq!(MarketCap::from_value(1_000_000_000.0), MarketCap::SmallCap);
    assert_eq!(MarketCap::from_value(5_000_000_000.0), MarketCap::MidCap);
    assert_eq!(MarketCap::from_value(50_000_000_000.0), MarketCap::LargeCap);
    assert_eq!(MarketCap::from_value(500_000_000_000.0), MarketCap::MegaCap);
  }

  #[test]
  fn test_market_cap_boundaries() {
    // Test exact boundaries
    assert_eq!(MarketCap::from_value(49_999_999.99), MarketCap::NanoCap);
    assert_eq!(MarketCap::from_value(50_000_000.0), MarketCap::MicroCap);
    assert_eq!(MarketCap::from_value(299_999_999.99), MarketCap::MicroCap);
    assert_eq!(MarketCap::from_value(300_000_000.0), MarketCap::SmallCap);
    assert_eq!(MarketCap::from_value(1_999_999_999.99), MarketCap::SmallCap);
    assert_eq!(MarketCap::from_value(2_000_000_000.0), MarketCap::MidCap);
  }

  #[test]
  fn test_market_cap_ranges() {
    let (min, max) = MarketCap::NanoCap.range();
    assert_eq!(min, 0.0);
    assert_eq!(max, Some(50_000_000.0));

    let (min, max) = MarketCap::MidCap.range();
    assert_eq!(min, 2_000_000_000.0);
    assert_eq!(max, Some(10_000_000_000.0));

    let (min, max) = MarketCap::MegaCap.range();
    assert_eq!(min, 200_000_000_000.0);
    assert_eq!(max, None);
  }

  #[test]
  fn test_market_cap_classification_helpers() {
    assert!(MarketCap::LargeCap.is_large());
    assert!(MarketCap::MegaCap.is_large());
    assert!(!MarketCap::SmallCap.is_large());
    assert!(!MarketCap::MidCap.is_large());

    assert!(MarketCap::NanoCap.is_small());
    assert!(MarketCap::MicroCap.is_small());
    assert!(MarketCap::SmallCap.is_small());
    assert!(!MarketCap::MidCap.is_small());
    assert!(!MarketCap::LargeCap.is_small());
  }

  #[test]
  fn test_market_cap_display() {
    assert_eq!(format!("{}", MarketCap::NanoCap), "Nano Cap");
    assert_eq!(format!("{}", MarketCap::MicroCap), "Micro Cap");
    assert_eq!(format!("{}", MarketCap::SmallCap), "Small Cap");
    assert_eq!(format!("{}", MarketCap::MidCap), "Mid Cap");
    assert_eq!(format!("{}", MarketCap::LargeCap), "Large Cap");
    assert_eq!(format!("{}", MarketCap::MegaCap), "Mega Cap");
  }

  // ===== Integration Tests =====
  #[test]
  fn test_encoding_roundtrip_all_types() {
    // Test encoding/decoding for all security types with various IDs
    let test_ids = vec![0, 1, 42, 1000, 10000, 100000, 1000000, u32::MAX];

    for id in test_ids {
      for sec_type in [
        SecurityType::Equity,
        SecurityType::PreferredStock,
        SecurityType::ETF,
        SecurityType::MutualFund,
        SecurityType::REIT,
        SecurityType::ADR,
        SecurityType::CD,
        SecurityType::Bond,
        SecurityType::GovernmentBond,
        SecurityType::CorporateBond,
        SecurityType::MunicipalBond,
        SecurityType::TreasuryBill,
        SecurityType::Option,
        SecurityType::Future,
        SecurityType::Warrant,
        SecurityType::Index,
        SecurityType::Currency,
        SecurityType::Commodity,
        SecurityType::Cryptocurrency,
        SecurityType::Other,
      ] {
        let encoded = SecurityType::encode(sec_type, id);
        let decoded_type = SecurityType::decode_type(encoded);
        let identifier = SecurityIdentifier::decode(encoded).unwrap();

        assert_eq!(decoded_type, sec_type, "Type mismatch for {:?} with id {}", sec_type, id);
        assert_eq!(
          identifier.security_type, sec_type,
          "Identifier type mismatch for {:?} with id {}",
          sec_type, id
        );
        assert_eq!(identifier.raw_id, id, "ID mismatch for {:?} with id {}", sec_type, id);
      }
    }
  }
}
