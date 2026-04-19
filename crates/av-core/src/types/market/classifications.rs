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
//!
//! This module defines three independent enum types used throughout `av_core`
//! for classifying market data:
//!
//! 1. [`TopType`] — Categorizes "top mover" queries (gainers, losers,
//!    most active by volume). Used as a query parameter when fetching
//!    market movers from the AlphaVantage API.
//!
//! 2. [`Sector`] — Represents the GICS-style market sector for an equity
//!    (Technology, Healthcare, etc.). Used for portfolio analysis,
//!    sector-based filtering, and computing analytical metrics like
//!    typical P/E ranges.
//!
//! 3. [`MarketCap`] — Classifies a company by market capitalization tier
//!    (NanoCap through MegaCap). Used for portfolio segmentation and
//!    risk analysis.
//!
//! All three types implement the standard derive set
//! (`Debug`/`Clone`/`Copy`/`PartialEq`/`Eq`/`Hash`/`Serialize`/`Deserialize`)
//! for use as map keys and JSON-serializable values, plus `Display` and
//! `FromStr` for human-readable round-tripping.
//!
//! ## String Parsing Strategy
//!
//! `TopType::from_str` and `Sector::from_str` both **normalize input** by
//! lowercasing/uppercasing and stripping spaces, dashes, and underscores
//! before matching. This makes parsing tolerant of variations like
//! `"Top Gainers"`, `"top-gainers"`, `"TOP_GAINERS"`, all of which parse
//! to `TopType::Gainers`.
//!
//! - `TopType::from_str` returns `Err(String)` for unrecognized input
//! - `Sector::from_str` returns `Ok(Sector::Other)` for unrecognized input
//!   (graceful degradation rather than failure)
//!
//! `MarketCap` does not implement `FromStr` — instead it provides
//! [`MarketCap::from_value`] for classification from a numeric USD value.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Top movers query category, used to fetch lists of best/worst-performing
/// or most-traded securities from the AlphaVantage API.
///
/// # Variants
///
/// - [`Gainers`](TopType::Gainers) — Stocks with the largest positive price
///   change in the trading session.
/// - [`Losers`](TopType::Losers) — Stocks with the largest negative price
///   change in the trading session.
/// - [`MostActive`](TopType::MostActive) — Stocks with the highest trading
///   volume in the session.
///
/// # `Display` Format
///
/// Outputs the AlphaVantage API parameter form (lowercase with underscores):
/// `"gainers"`, `"losers"`, `"most_active"`.
///
/// # `FromStr` Aliases
///
/// Input is normalized (lowercased, whitespace/dashes/underscores stripped)
/// before matching. Accepted aliases:
///
/// | Variant      | Aliases                                  |
/// |--------------|------------------------------------------|
/// | `Gainers`    | `gainers`, `topgainers`, `winners`       |
/// | `Losers`     | `losers`, `toplosers`, `decliners`       |
/// | `MostActive` | `mostactive`, `active`, `volume`         |
///
/// Returns `Err(String)` for unrecognized input.
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

impl FromStr for TopType {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().replace([' ', '-', '_'], "").as_str() {
      "gainers" | "topgainers" | "winners" => Ok(TopType::Gainers),
      "losers" | "toplosers" | "decliners" => Ok(TopType::Losers),
      "mostactive" | "active" | "volume" => Ok(TopType::MostActive),
      _ => Err(format!("Invalid top type: {}", s)),
    }
  }
}

/// GICS-style market sector classification.
///
/// Represents the 11 standard Global Industry Classification Standard (GICS)
/// sectors plus an `Other` catch-all for unrecognized or non-standard
/// classifications.
///
/// # Variants
///
/// - `Technology` — Software, hardware, semiconductors, IT services
/// - `Healthcare` — Pharmaceuticals, biotech, medical devices, providers
/// - `FinancialServices` — Banks, insurance, capital markets, fintech
/// - `ConsumerDiscretionary` — Retail, automotive, leisure, apparel
/// - `ConsumerStaples` — Food, beverages, household products
/// - `Industrials` — Aerospace, machinery, transportation, construction
/// - `Energy` — Oil & gas, equipment, services
/// - `Materials` — Chemicals, mining, metals, paper
/// - `RealEstate` — REITs, real estate management
/// - `Utilities` — Electric, gas, water utilities
/// - `CommunicationServices` — Telecom, media, entertainment
/// - `Other` — Catch-all for unrecognized or non-standard sectors
///
/// # Cyclical vs. Defensive
///
/// Sectors are categorized for analytical purposes:
/// - **Cyclical** (sensitive to economic cycles): Technology,
///   ConsumerDiscretionary, Industrials, Energy, Materials, FinancialServices
/// - **Defensive** (relatively stable across cycles): Healthcare,
///   ConsumerStaples, Utilities
///
/// See [`Sector::is_cyclical`] and [`Sector::is_defensive`].
///
/// # `Display` Format
///
/// Outputs the human-readable form with spaces:
/// `"Technology"`, `"Financial Services"`, `"Consumer Discretionary"`, etc.
///
/// # `FromStr` Aliases
///
/// Input is normalized (uppercased, whitespace/dashes/underscores stripped).
/// Accepts both abbreviations and common alternative names. Examples:
///
/// | Variant                | Accepted Aliases                                            |
/// |------------------------|-------------------------------------------------------------|
/// | `Technology`           | `TECHNOLOGY`, `TECH`, `IT`, `INFORMATIONTECHNOLOGY`         |
/// | `Healthcare`           | `HEALTHCARE`, `HEALTH`, `MEDICAL`, `PHARMA`, `PHARMACEUTICAL` |
/// | `FinancialServices`    | `FINANCIALSERVICES`, `FINANCIAL`, `FINANCE`, `BANKING`, `FINTECH` |
/// | `ConsumerDiscretionary`| `CONSUMERDISCRETIONARY`, `CONSUMER`, `RETAIL`, `DISCRETIONARY` |
/// | `ConsumerStaples`      | `CONSUMERSTAPLES`, `STAPLES`, `DEFENSIVE`                   |
/// | `Industrials`          | `INDUSTRIALS`, `INDUSTRIAL`, `MANUFACTURING`                |
/// | `Energy`               | `ENERGY`, `OIL`, `GAS`, `PETROLEUM`                         |
/// | `Materials`            | `MATERIALS`, `BASIC`, `BASICMATERIALS`, `MINING`            |
/// | `RealEstate`           | `REALESTATE`, `PROPERTY`, `REIT`                            |
/// | `Utilities`            | `UTILITIES`, `UTILITY`, `POWER`, `ELECTRIC`                 |
/// | `CommunicationServices`| `COMMUNICATIONSERVICES`, `COMMUNICATION`, `TELECOM`, `MEDIA`|
///
/// Unrecognized inputs return `Ok(Sector::Other)` (never an error).
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

impl FromStr for Sector {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
      "TECHNOLOGY" | "TECH" | "IT" | "INFORMATIONTECHNOLOGY" => Ok(Sector::Technology),
      "HEALTHCARE" | "HEALTH" | "MEDICAL" | "PHARMA" | "PHARMACEUTICAL" => Ok(Sector::Healthcare),
      "FINANCIALSERVICES" | "FINANCIAL" | "FINANCE" | "BANKING" | "FINTECH" => {
        Ok(Sector::FinancialServices)
      }
      "CONSUMERDISCRETIONARY" | "CONSUMER" | "RETAIL" | "DISCRETIONARY" => {
        Ok(Sector::ConsumerDiscretionary)
      }
      "CONSUMERSTAPLES" | "STAPLES" | "DEFENSIVE" => Ok(Sector::ConsumerStaples),
      "INDUSTRIALS" | "INDUSTRIAL" | "MANUFACTURING" => Ok(Sector::Industrials),
      "ENERGY" | "OIL" | "GAS" | "PETROLEUM" => Ok(Sector::Energy),
      "MATERIALS" | "BASIC" | "BASICMATERIALS" | "MINING" => Ok(Sector::Materials),
      "REALESTATE" | "PROPERTY" | "REIT" => Ok(Sector::RealEstate),
      "UTILITIES" | "UTILITY" | "POWER" | "ELECTRIC" => Ok(Sector::Utilities),
      "COMMUNICATIONSERVICES" | "COMMUNICATION" | "TELECOM" | "MEDIA" => {
        Ok(Sector::CommunicationServices)
      }
      _ => Ok(Sector::Other),
    }
  }
}

impl Sector {
  /// Returns `true` if this sector is **cyclical** — meaning its performance
  /// is sensitive to broader economic cycles (expansions and recessions).
  ///
  /// Cyclical sectors typically outperform during economic expansion and
  /// underperform during contractions. They are: `Technology`,
  /// `ConsumerDiscretionary`, `Industrials`, `Energy`, `Materials`,
  /// and `FinancialServices`.
  ///
  /// Note: A sector is either cyclical or defensive, but `Other`,
  /// `RealEstate`, and `CommunicationServices` belong to neither category
  /// in this classification (they return `false` from both
  /// `is_cyclical` and `is_defensive`).
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

  /// Returns `true` if this sector is **defensive** — meaning its performance
  /// is relatively stable across economic cycles.
  ///
  /// Defensive sectors provide essential goods and services that consumers
  /// continue to purchase during recessions. They are: `Healthcare`,
  /// `ConsumerStaples`, and `Utilities`.
  pub fn is_defensive(&self) -> bool {
    matches!(self, Sector::Healthcare | Sector::ConsumerStaples | Sector::Utilities)
  }

  /// Returns the **typical (min, max) P/E ratio range** for this sector,
  /// useful for relative valuation analysis.
  ///
  /// These ranges represent rule-of-thumb historical norms — actual P/E
  /// ratios vary significantly based on growth rate, interest rates, and
  /// market sentiment. They are intended as a quick reference for whether
  /// a stock's P/E is unusually high or low for its sector, not as
  /// strict valuation bounds.
  ///
  /// | Sector                  | Min P/E | Max P/E |
  /// |-------------------------|---------|---------|
  /// | `Technology`            | 15.0    | 35.0    |
  /// | `Healthcare`            | 12.0    | 25.0    |
  /// | `FinancialServices`     | 8.0     | 15.0    |
  /// | `ConsumerDiscretionary` | 12.0    | 25.0    |
  /// | `ConsumerStaples`       | 15.0    | 25.0    |
  /// | `Industrials`           | 12.0    | 20.0    |
  /// | `Energy`                | 8.0     | 15.0    |
  /// | `Materials`             | 10.0    | 18.0    |
  /// | `RealEstate`            | 15.0    | 30.0    |
  /// | `Utilities`             | 12.0    | 20.0    |
  /// | `CommunicationServices` | 10.0    | 25.0    |
  /// | `Other`                 | 10.0    | 25.0    |
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

/// Market capitalization tier classification.
///
/// Categorizes a company by its market cap into one of six tiers ranging
/// from `NanoCap` (under $50M) to `MegaCap` (over $200B). Used for
/// portfolio segmentation, risk analysis, and screening.
///
/// # Tier Boundaries (USD)
///
/// | Variant     | Range                              |
/// |-------------|------------------------------------|
/// | `NanoCap`   | `< $50M`                           |
/// | `MicroCap`  | `$50M – $300M`                     |
/// | `SmallCap`  | `$300M – $2B`                      |
/// | `MidCap`    | `$2B – $10B`                       |
/// | `LargeCap`  | `$10B – $200B`                     |
/// | `MegaCap`   | `> $200B`                          |
///
/// All boundaries are **half-open**: a value of exactly `$50M` is `MicroCap`,
/// not `NanoCap`. See [`MarketCap::from_value`] for the classification logic
/// and [`MarketCap::range`] for tier boundary access.
///
/// # No `FromStr`
///
/// Unlike [`TopType`] and [`Sector`], `MarketCap` does not implement `FromStr`.
/// Classification is done numerically via [`from_value`](MarketCap::from_value)
/// from a USD value, since string representations of market cap tiers are
/// rarely standardized in financial data feeds.
///
/// # `Display` Format
///
/// Outputs the human-readable form with a space:
/// `"Nano Cap"`, `"Micro Cap"`, `"Small Cap"`, `"Mid Cap"`, `"Large Cap"`, `"Mega Cap"`.
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
  /// Classifies a market capitalization value (in USD) into the appropriate tier.
  ///
  /// Uses **half-open intervals** — a value equal to a tier boundary belongs
  /// to the higher tier (e.g., exactly `$50_000_000.0` is `MicroCap`, not `NanoCap`).
  ///
  /// # Arguments
  ///
  /// * `market_cap_usd` — Market capitalization in US dollars (not millions or billions).
  ///
  /// # Returns
  ///
  /// The matching [`MarketCap`] tier:
  ///
  /// | Range                                  | Tier        |
  /// |----------------------------------------|-------------|
  /// | `< 50_000_000`                         | `NanoCap`   |
  /// | `< 300_000_000`                        | `MicroCap`  |
  /// | `< 2_000_000_000`                      | `SmallCap`  |
  /// | `< 10_000_000_000`                     | `MidCap`    |
  /// | `< 200_000_000_000`                    | `LargeCap`  |
  /// | `>= 200_000_000_000`                   | `MegaCap`   |
  ///
  /// # Example
  ///
  /// ```
  /// use av_core::types::market::MarketCap;
  /// assert_eq!(MarketCap::from_value(100_000_000.0), MarketCap::MicroCap);
  /// assert_eq!(MarketCap::from_value(50_000_000.0),  MarketCap::MicroCap); // boundary
  /// assert_eq!(MarketCap::from_value(2_500_000_000_000.0), MarketCap::MegaCap);
  /// ```
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

  /// Returns the `(min, max)` USD range for this market cap tier.
  ///
  /// The first element is the inclusive lower bound; the second is the
  /// exclusive upper bound. The upper bound is `None` for the unbounded
  /// `MegaCap` tier.
  ///
  /// # Returns
  ///
  /// | Tier        | `min`             | `max`                       |
  /// |-------------|-------------------|-----------------------------|
  /// | `NanoCap`   | `0.0`             | `Some(50_000_000.0)`        |
  /// | `MicroCap`  | `50_000_000.0`    | `Some(300_000_000.0)`       |
  /// | `SmallCap`  | `300_000_000.0`   | `Some(2_000_000_000.0)`     |
  /// | `MidCap`    | `2_000_000_000.0` | `Some(10_000_000_000.0)`    |
  /// | `LargeCap`  | `10_000_000_000.0`| `Some(200_000_000_000.0)`   |
  /// | `MegaCap`   | `200_000_000_000.0` | `None` (unbounded)        |
  ///
  /// This is the inverse of [`from_value`](MarketCap::from_value): for any
  /// value in `[min, max)`, `from_value(v)` returns this tier.
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

  /// Returns `true` if this tier represents a **large company** —
  /// either [`LargeCap`](MarketCap::LargeCap) or [`MegaCap`](MarketCap::MegaCap).
  ///
  /// Corresponds to market caps of `$10B` or more. `MidCap` is **not**
  /// considered large by this classification.
  pub fn is_large(&self) -> bool {
    matches!(self, MarketCap::LargeCap | MarketCap::MegaCap)
  }

  /// Returns `true` if this tier represents a **small company** —
  /// [`NanoCap`](MarketCap::NanoCap), [`MicroCap`](MarketCap::MicroCap),
  /// or [`SmallCap`](MarketCap::SmallCap).
  ///
  /// Corresponds to market caps under `$2B`. `MidCap` is **not** considered
  /// small by this classification.
  ///
  /// Note: `is_large()` and `is_small()` are **not exhaustive** — `MidCap`
  /// returns `false` from both, representing a neutral middle tier.
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
    assert_eq!("gainers".parse::<TopType>(), Ok(TopType::Gainers));
    assert_eq!("TOP_GAINERS".parse::<TopType>(), Ok(TopType::Gainers));
    assert_eq!("winners".parse::<TopType>(), Ok(TopType::Gainers));
    assert_eq!("losers".parse::<TopType>(), Ok(TopType::Losers));
    assert_eq!("decliners".parse::<TopType>(), Ok(TopType::Losers));
    assert_eq!("most_active".parse::<TopType>(), Ok(TopType::MostActive));
    assert_eq!("volume".parse::<TopType>(), Ok(TopType::MostActive));
    assert!("invalid".parse::<TopType>().is_err());
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
    assert_eq!("Technology".parse::<Sector>(), Ok(Sector::Technology));
    assert_eq!("TECH".parse::<Sector>(), Ok(Sector::Technology));
    assert_eq!("Information Technology".parse::<Sector>(), Ok(Sector::Technology));
    assert_eq!("Healthcare".parse::<Sector>(), Ok(Sector::Healthcare));
    assert_eq!("PHARMA".parse::<Sector>(), Ok(Sector::Healthcare));
    assert_eq!("Financial Services".parse::<Sector>(), Ok(Sector::FinancialServices));
    assert_eq!("BANKING".parse::<Sector>(), Ok(Sector::FinancialServices));
    assert_eq!("UNKNOWN_SECTOR".parse::<Sector>(), Ok(Sector::Other));
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
