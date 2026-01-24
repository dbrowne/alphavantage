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

//! Market classification types: sectors, market cap, and top movers.

use serde::{Deserialize, Serialize};

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
    assert!(Sector::Technology.is_cyclical());
    assert!(Sector::ConsumerDiscretionary.is_cyclical());
    assert!(Sector::Industrials.is_cyclical());
    assert!(Sector::Energy.is_cyclical());
    assert!(Sector::Materials.is_cyclical());
    assert!(Sector::FinancialServices.is_cyclical());
    assert!(!Sector::Healthcare.is_cyclical());
    assert!(!Sector::Utilities.is_cyclical());

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
}
