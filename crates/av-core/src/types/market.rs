//! Market-related types

use serde::{Deserialize, Serialize};

/// Security type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
  CommonStock,
  PreferredStock,
  ETF,
  ETN,
  CEF, // Closed End Fund
  REIT,
  ADR,
  GDR,
  Unit,
  StructuredProduct,
  Warrant,
  Right,
  Bond,
  Cryptocurrency,
  Index,
  MutualFund,
  Unknown,
}

impl SecurityType {
  /// Get a human-readable name for the security type
  pub fn as_str(&self) -> &'static str {
    match self {
      SecurityType::CommonStock => "Common Stock",
      SecurityType::PreferredStock => "Preferred Stock",
      SecurityType::ETF => "ETF",
      SecurityType::ETN => "ETN",
      SecurityType::CEF => "Closed End Fund",
      SecurityType::REIT => "REIT",
      SecurityType::ADR => "ADR",
      SecurityType::GDR => "GDR",
      SecurityType::Unit => "Unit",
      SecurityType::StructuredProduct => "Structured Product",
      SecurityType::Warrant => "Warrant",
      SecurityType::Right => "Right",
      SecurityType::Bond => "Bond",
      SecurityType::Cryptocurrency => "Cryptocurrency",
      SecurityType::Index => "Index",
      SecurityType::MutualFund => "Mutual Fund",
      SecurityType::Unknown => "Unknown",
    }
  }

  /// Parse security type from string
  pub fn from_str(s: &str) -> Self {
    match s.to_uppercase().as_str() {
      "COMMON STOCK" | "CS" => SecurityType::CommonStock,
      "PREFERRED STOCK" | "PS" => SecurityType::PreferredStock,
      "ETF" => SecurityType::ETF,
      "ETN" => SecurityType::ETN,
      "CEF" | "CLOSED END FUND" => SecurityType::CEF,
      "REIT" => SecurityType::REIT,
      "ADR" => SecurityType::ADR,
      "GDR" => SecurityType::GDR,
      "UNIT" => SecurityType::Unit,
      "STRUCTURED PRODUCT" => SecurityType::StructuredProduct,
      "WARRANT" => SecurityType::Warrant,
      "RIGHT" => SecurityType::Right,
      "BOND" => SecurityType::Bond,
      "CRYPTO" | "CRYPTOCURRENCY" => SecurityType::Cryptocurrency,
      "INDEX" => SecurityType::Index,
      "MUTUAL FUND" | "MF" => SecurityType::MutualFund,
      _ => SecurityType::Unknown,
    }
  }
}

/// Exchange enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exchange {
  NYSE,
  NASDAQ,
  AMEX,
  ARCA,
  BATS,
  IEX,
  OTC,
  PINK,
  Other(String),
}

/// Type of top movers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopType {
  TopGainer,
  TopLoser,
  TopActive,
}

impl TopType {
  pub fn as_str(&self) -> &'static str {
    match self {
      TopType::TopGainer => "GAIN",
      TopType::TopLoser => "LOSE",
      TopType::TopActive => "ACTV",
    }
  }
}
