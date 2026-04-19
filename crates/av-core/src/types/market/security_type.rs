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

//! Security type definitions and bitmap encoding for security identifiers.
//!
//! This module provides two core types:
//!
//! - [`SecurityType`] — an enum representing the category of a financial instrument
//!   (equity, bond, derivative, etc.) with rich metadata methods and bidirectional
//!   Alpha Vantage API mapping.
//! - [`SecurityIdentifier`] — a compact, bitmap-encoded identifier that packs both
//!   the security type and a unique numeric ID into a single `i64` value.
//!
//! # Bitmap encoding scheme
//!
//! The encoding uses a **variable-length prefix** strategy to maximize the ID space
//! for high-volume security types while keeping the total width at 64 bits:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  i64 (64 bits total)                                           │
//! ├──────────────┬──────────────────────────────────────────────────┤
//! │ Type prefix  │  Unique ID (remaining bits)                     │
//! │ (4-6 bits)   │  (58-60 bits)                                   │
//! └──────────────┴──────────────────────────────────────────────────┘
//! ```
//!
//! | Prefix length | ID bits | Max ID space  | Used for                                           |
//! |---------------|---------|---------------|----------------------------------------------------|
//! | 4 bits        | 60 bits | ~1.15 × 10¹⁸ | High-volume: equities, options, futures, ETFs, etc.|
//! | 5 bits        | 59 bits | ~5.76 × 10¹⁷ | Medium-volume: bonds, crypto, REITs                |
//! | 6 bits        | 58 bits | ~2.88 × 10¹⁷ | Low-volume: currencies, indexes, commodities       |
//!
//! The encoding is designed so that each security type's prefix is **non-overlapping**,
//! enabling unambiguous decoding by checking 4-bit prefixes first, then 5-bit, then 6-bit.
//!
//! # Examples
//!
//! ```rust
//! use av_core::types::market::security_type::{SecurityType, SecurityIdentifier};
//!
//! // Encode a security
//! let sid = SecurityType::encode(SecurityType::Equity, 12345);
//!
//! // Decode back
//! let identifier = SecurityIdentifier::decode(sid).unwrap();
//! assert_eq!(identifier.security_type, SecurityType::Equity);
//! assert_eq!(identifier.raw_id, 12345);
//!
//! // Category checks
//! assert!(SecurityType::Equity.is_equity());
//! assert!(SecurityType::Bond.is_fixed_income());
//! assert!(SecurityType::Option.is_derivative());
//! ```

use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ─── Bitmap type prefix constants ───────────────────────────────────────────
//
// Variable-length prefix codes for each security type. Shorter prefixes are
// assigned to security types with larger universes, maximizing the number of
// bits available for the unique ID portion.
//
// Bit allocation budget:
//   Total: 64 bits (i64), 1 sign bit unused, 63 bits usable.
//   Format: [SecurityType prefix (4-6 bits)] [Unique ID (remaining bits)]

/// 4-bit prefixes — high-volume types (up to 2⁶⁰ unique IDs each).
const TYPE_COMMON_STOCK: u8 = 0b0000; // Common stock / equities — millions of instruments globally
const TYPE_PREFERRED: u8 = 0b0001; // Preferred stock — tens of thousands
const TYPE_ETF: u8 = 0b0010; // Exchange-traded funds — tens of thousands
const TYPE_MUTUAL_FUND: u8 = 0b0011; // Mutual funds — tens of thousands
const TYPE_OPTION: u8 = 0b0100; // Options contracts — hundreds of millions (strike × expiry combinations)
const TYPE_FUTURE: u8 = 0b0101; // Futures contracts — tens of millions
const TYPE_WARRANT: u8 = 0b0110; // Warrants — hundreds of thousands
const TYPE_ADR: u8 = 0b0111; // American Depositary Receipts — thousands

/// 5-bit prefixes — medium-volume types (up to 2⁵⁹ unique IDs each).
const TYPE_BOND: u8 = 0b10000; // Generic bonds — millions
const TYPE_GOVT_BOND: u8 = 0b10001; // Government bonds — tens of thousands
const TYPE_CORP_BOND: u8 = 0b10010; // Corporate bonds — hundreds of thousands
const TYPE_MUNI_BOND: u8 = 0b10011; // Municipal bonds — hundreds of thousands
const TYPE_CRYPTO: u8 = 0b10100; // Cryptocurrencies — tens of thousands
const TYPE_REIT: u8 = 0b10101; // Real Estate Investment Trusts — thousands

/// 6-bit prefixes — low-volume types (up to 2⁵⁸ unique IDs each).
const TYPE_CURRENCY: u8 = 0b110000; // Currency / forex pairs — ~200 major pairs
const TYPE_INDEX: u8 = 0b110001; // Market indexes — thousands
const TYPE_COMMODITY: u8 = 0b110010; // Commodities — hundreds
const TYPE_CD: u8 = 0b110011; // Certificates of Deposit — tens of thousands
const TYPE_T_BILL: u8 = 0b110100; // Treasury bills — hundreds
const TYPE_OTHER: u8 = 0b111111; // Catch-all for unclassified instruments

/// Bit-shift amounts that position the type prefix at the top of the `i64`.
/// Computed as `64 - prefix_length`, leaving the remaining lower bits for the unique ID.
const SHIFT_4BIT: u8 = 60; // 64 - 4 = 60 bits for ID (capacity: ~1.15 × 10¹⁸)
const SHIFT_5BIT: u8 = 59; // 64 - 5 = 59 bits for ID (capacity: ~5.76 × 10¹⁷)
const SHIFT_6BIT: u8 = 58; // 64 - 6 = 58 bits for ID (capacity: ~2.88 × 10¹⁷)

/// Classifies a financial instrument into one of 20 security categories.
///
/// `SecurityType` is the primary way the `alphavantage` crate distinguishes between
/// different kinds of traded instruments. It supports:
///
/// - **String parsing** ([`FromStr`]) with case-insensitive, flexible input
///   (e.g., `"Common Stock"`, `"EQUITY"`, `"stock"` all parse to [`Equity`](SecurityType::Equity)).
/// - **Display** formatting returning the human-readable name.
/// - **Bitmap encoding/decoding** via [`encode`](SecurityType::encode) and
///   [`decode_type`](SecurityType::decode_type) for compact `i64` storage.
/// - **Category queries**: [`is_equity`](SecurityType::is_equity),
///   [`is_fixed_income`](SecurityType::is_fixed_income),
///   [`is_derivative`](SecurityType::is_derivative).
/// - **Alpha Vantage API interop**: [`from_alpha_vantage`](SecurityType::from_alpha_vantage)
///   and [`to_alpha_vantage`](SecurityType::to_alpha_vantage).
///
/// # Variant groupings
///
/// | Group          | Variants                                                          |
/// |----------------|-------------------------------------------------------------------|
/// | **Equity**     | `Equity`, `PreferredStock`, `ETF`, `MutualFund`, `REIT`, `ADR`    |
/// | **Fixed income** | `Bond`, `GovernmentBond`, `CorporateBond`, `MunicipalBond`, `TreasuryBill`, `CD` |
/// | **Derivatives** | `Option`, `Future`, `Warrant`                                    |
/// | **Other**      | `Index`, `Currency`, `Commodity`, `Cryptocurrency`, `Other`       |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
  /// Common stock / ordinary shares — the most common equity instrument.
  Equity,
  /// Preferred stock — equity with priority dividend rights and liquidation preference.
  PreferredStock,
  /// Exchange-Traded Fund — a pooled investment fund traded on stock exchanges.
  ETF,
  /// Mutual Fund — an open-ended pooled investment vehicle priced once daily at NAV.
  MutualFund,
  /// Real Estate Investment Trust — a company that owns or finances income-producing real estate.
  REIT,
  /// American Depositary Receipt — a certificate representing shares of a foreign company
  /// traded on U.S. exchanges.
  ADR,
  /// Certificate of Deposit — a time deposit offered by banks with a fixed interest rate
  /// and maturity date.
  CD,
  /// Generic bond — a debt instrument; use the more specific bond variants when the
  /// issuer type is known.
  Bond,
  /// Government bond — sovereign debt instruments (e.g., U.S. Treasuries, UK Gilts).
  GovernmentBond,
  /// Corporate bond — debt issued by a corporation to fund operations or expansion.
  CorporateBond,
  /// Municipal bond — debt issued by a state, city, or county, often tax-exempt.
  MunicipalBond,
  /// Treasury bill — short-term government debt with maturity of one year or less,
  /// sold at a discount.
  TreasuryBill,
  /// Options contract — a derivative giving the holder the right (not obligation) to
  /// buy or sell an underlying asset at a specified price.
  Option,
  /// Futures contract — a standardized derivative obligating the buyer/seller to
  /// transact at a predetermined price and date.
  Future,
  /// Warrant — a long-dated option-like instrument typically issued by the company itself.
  Warrant,
  /// Market index — a statistical measure of a section of the market (e.g., S&P 500).
  /// Not directly tradable.
  Index,
  /// Foreign currency / forex pair (e.g., EUR/USD).
  Currency,
  /// Physical or financial commodity (e.g., gold, crude oil, wheat).
  Commodity,
  /// Cryptocurrency / digital currency (e.g., Bitcoin, Ethereum).
  Cryptocurrency,
  /// Catch-all for instruments that do not fit any other category.
  /// [`FromStr`] maps unrecognized strings here rather than returning an error.
  Other,
}

/// A decoded security identifier comprising a [`SecurityType`] and a numeric ID.
///
/// `SecurityIdentifier` is the "unpacked" form of a bitmap-encoded `i64` SID.
/// Use [`SecurityType::encode`] to pack a type + ID into an `i64`, and
/// [`SecurityIdentifier::decode`] to unpack it back.
///
/// # Fields
///
/// - `security_type` — the category of the instrument.
/// - `raw_id` — the unique numeric identifier within that category (up to `u32::MAX`).
///
/// # Examples
///
/// ```rust
/// use av_core::types::market::security_type::{SecurityType, SecurityIdentifier};
///
/// let encoded = SecurityType::encode(SecurityType::ETF, 42);
/// let decoded = SecurityIdentifier::decode(encoded).unwrap();
/// assert_eq!(decoded.security_type, SecurityType::ETF);
/// assert_eq!(decoded.raw_id, 42);
/// ```
#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash, Deserialize)]
pub struct SecurityIdentifier {
  /// The category of financial instrument.
  pub security_type: SecurityType,
  /// The unique numeric ID within the security type's namespace.
  pub raw_id: u32,
}

impl SecurityIdentifier {
  /// Decodes a bitmap-encoded `i64` SID into a [`SecurityIdentifier`].
  ///
  /// Extracts the type prefix to determine the [`SecurityType`], then masks out
  /// the remaining bits to recover the `raw_id`. Always returns `Some` — the
  /// `Option` return type is reserved for future validation (e.g., range checks).
  pub fn decode(encoded_id: i64) -> Option<SecurityIdentifier> {
    let security_type = SecurityType::decode_type(encoded_id);
    let shift = SecurityType::get_shift(security_type);
    let mask = (1i64 << shift) - 1;
    let raw_id = (encoded_id & mask) as u32;

    Some(SecurityIdentifier { security_type, raw_id })
  }
}

/// Formats the security type as a human-readable name (e.g., `"Common Stock"`, `"ETF"`).
///
/// This matches the strings returned by [`SecurityType::to_alpha_vantage`].
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

/// Parses a string into a [`SecurityType`].
///
/// Parsing is **case-insensitive** and strips spaces, hyphens, and underscores before
/// matching, so inputs like `"Common Stock"`, `"common-stock"`, and `"COMMONSTOCK"` all
/// resolve to [`SecurityType::Equity`].
///
/// # Infallible by design
///
/// Unrecognized strings are mapped to [`SecurityType::Other`] rather than returning an
/// error. The `Err` type is `String` only to satisfy the [`FromStr`] trait contract.
///
/// # Accepted aliases
///
/// | Variant            | Aliases (after normalization)                                |
/// |--------------------|--------------------------------------------------------------|
/// | `Equity`           | `COMMONSTOCK`, `EQUITY`, `STOCK`                             |
/// | `PreferredStock`   | `PREFERREDSTOCK`, `PREFERRED`                                |
/// | `ETF`              | `ETF`, `EXCHANGETRADEDFUND`                                  |
/// | `MutualFund`       | `MUTUALFUND`, `FUND`                                         |
/// | `REIT`             | `REIT`, `REALESTATEINVESTMENTTRUST`                          |
/// | `ADR`              | `ADR`, `AMERICANDEPOSITARYRECEIPT`                           |
/// | `CD`               | `CD`, `CERTIFICATEOFDEPOSIT`                                 |
/// | `Bond`             | `BOND`                                                       |
/// | `GovernmentBond`   | `GOVERNMENTBOND`, `GOVBOND`                                  |
/// | `CorporateBond`    | `CORPORATEBOND`, `CORPBOND`                                  |
/// | `MunicipalBond`    | `MUNICIPALBOND`, `MUNIBOND`                                  |
/// | `TreasuryBill`     | `TREASURYBILL`, `TBILL`                                      |
/// | `Option`           | `OPTION`                                                     |
/// | `Future`           | `FUTURE`, `FUTURES`                                          |
/// | `Warrant`          | `WARRANT`                                                    |
/// | `Index`            | `INDEX`                                                      |
/// | `Currency`         | `CURRENCY`, `FX`, `FOREX`                                    |
/// | `Commodity`        | `COMMODITY`                                                  |
/// | `Cryptocurrency`   | `CRYPTOCURRENCY`, `CRYPTO`                                   |
impl FromStr for SecurityType {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
      "COMMONSTOCK" | "EQUITY" | "STOCK" => Ok(SecurityType::Equity),
      "PREFERREDSTOCK" | "PREFERRED" => Ok(SecurityType::PreferredStock),
      "ETF" | "EXCHANGETRADEDFUND" => Ok(SecurityType::ETF),
      "MUTUALFUND" | "FUND" => Ok(SecurityType::MutualFund),
      "REIT" | "REALESTATEINVESTMENTTRUST" => Ok(SecurityType::REIT),
      "ADR" | "AMERICANDEPOSITARYRECEIPT" => Ok(SecurityType::ADR),
      "CD" | "CERTIFICATEOFDEPOSIT" => Ok(SecurityType::CD),
      "BOND" => Ok(SecurityType::Bond),
      "GOVERNMENTBOND" | "GOVBOND" => Ok(SecurityType::GovernmentBond),
      "CORPORATEBOND" | "CORPBOND" => Ok(SecurityType::CorporateBond),
      "MUNICIPALBOND" | "MUNIBOND" => Ok(SecurityType::MunicipalBond),
      "TREASURYBILL" | "TBILL" => Ok(SecurityType::TreasuryBill),
      "OPTION" => Ok(SecurityType::Option),
      "FUTURE" | "FUTURES" => Ok(SecurityType::Future),
      "WARRANT" => Ok(SecurityType::Warrant),
      "INDEX" => Ok(SecurityType::Index),
      "CURRENCY" | "FX" | "FOREX" => Ok(SecurityType::Currency),
      "COMMODITY" => Ok(SecurityType::Commodity),
      "CRYPTOCURRENCY" | "CRYPTO" => Ok(SecurityType::Cryptocurrency),
      _ => Ok(SecurityType::Other),
    }
  }
}

/// Bitmap encoding/decoding, Alpha Vantage API mapping, and category query methods.
impl SecurityType {
  /// Encodes a [`SecurityType`] and a numeric `id` into a single `i64` bitmap.
  ///
  /// The type is packed into the high-order bits using a variable-length prefix
  /// (4, 5, or 6 bits depending on the expected universe size), and the `id`
  /// occupies the remaining lower bits.
  ///
  /// # Arguments
  ///
  /// - `st` — the security type to encode.
  /// - `id` — the unique numeric identifier (up to `u32::MAX`).
  ///
  /// # Returns
  ///
  /// A non-negative `i64` containing the packed bitmap. Use
  /// [`SecurityType::decode_type`] or [`SecurityIdentifier::decode`] to unpack.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::types::market::security_type::SecurityType;
  ///
  /// let sid = SecurityType::encode(SecurityType::Equity, 42);
  /// assert_eq!(SecurityType::decode_type(sid), SecurityType::Equity);
  /// ```
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

  /// Extracts the [`SecurityType`] from a bitmap-encoded `i64` SID.
  ///
  /// Decoding proceeds in prefix-length order: 4-bit prefixes are checked first
  /// (most common types), then 5-bit, then 6-bit. If no prefix matches,
  /// [`SecurityType::Other`] is returned.
  ///
  /// To decode both the type and the ID, use [`SecurityIdentifier::decode`] instead.
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

  /// Returns the bit-shift amount for the given security type's prefix length.
  ///
  /// This determines how many bits are available for the unique ID:
  /// - 4-bit prefix types → shift 60
  /// - 5-bit prefix types → shift 59
  /// - 6-bit prefix types → shift 58
  ///
  /// This is a crate-internal helper used by [`encode`](SecurityType::encode) and
  /// [`SecurityIdentifier::decode`].
  pub(crate) fn get_shift(st: SecurityType) -> u8 {
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

  /// Converts an Alpha Vantage API `asset_type` string into a [`SecurityType`].
  ///
  /// Similar to [`FromStr`] but includes additional aliases specific to Alpha Vantage
  /// API responses (e.g., `"CS"` for common stock, `"PS"` for preferred stock,
  /// `"MF"` for mutual fund, `"WT"` for warrant, `"DIGITALCURRENCY"` for crypto).
  ///
  /// Input is case-insensitive with spaces, underscores, and hyphens stripped.
  /// Unrecognized strings map to [`SecurityType::Other`].
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

  /// Returns the Alpha Vantage API-canonical string for this security type.
  ///
  /// These strings match the `asset_type` field values used in Alpha Vantage
  /// API responses (e.g., `"Common Stock"`, `"Exchange Traded Fund"`).
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

  /// Returns `true` if this security type belongs to the **equity** family.
  ///
  /// Includes: `Equity`, `PreferredStock`, `ETF`, `REIT`, `ADR`.
  ///
  /// Note: `MutualFund` is excluded because mutual funds can hold mixed assets
  /// (equities, bonds, etc.) and are priced differently (NAV-based, T+1 settlement).
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

  /// Returns `true` if this security type belongs to the **fixed income** family.
  ///
  /// Includes: `Bond`, `GovernmentBond`, `CorporateBond`, `MunicipalBond`,
  /// `TreasuryBill`, `CD`.
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

  /// Returns `true` if this security type is a **derivative** instrument.
  ///
  /// Includes: `Option`, `Future`, `Warrant`.
  pub fn is_derivative(&self) -> bool {
    matches!(self, SecurityType::Option | SecurityType::Future | SecurityType::Warrant)
  }

  /// Returns the typical settlement period in business days for this security type.
  ///
  /// Settlement conventions follow standard market practice:
  ///
  /// | Settlement | Security types                                             |
  /// |------------|-----------------------------------------------------------|
  /// | **T+0**    | `Future` (daily mark-to-market), `Commodity`, `Cryptocurrency` (immediate) |
  /// | **T+1**    | `MutualFund`, `Bond` (all sub-types), `TreasuryBill`, `Option` |
  /// | **T+2**    | `Equity`, `PreferredStock`, `ETF`, `REIT`, `ADR`, `Currency` |
  ///
  /// All other / unclassified types default to **T+2**.
  ///
  /// **Note:** These are typical conventions and may vary by jurisdiction or
  /// specific instrument. Always verify with the relevant exchange or clearinghouse.
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_security_type_parsing() {
    assert_eq!("Common Stock".parse::<SecurityType>(), Ok(SecurityType::Equity));
    assert_eq!("ETF".parse::<SecurityType>(), Ok(SecurityType::ETF));
    assert!(SecurityType::Equity.is_equity());
    assert!(SecurityType::Bond.is_fixed_income());
    assert!(SecurityType::Option.is_derivative());
    assert_eq!(SecurityType::Equity.settlement_days(), 2);
  }

  #[test]
  fn test_security_type_encode_decode_4bit() {
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

      let identifier = SecurityIdentifier::decode(encoded).unwrap();
      assert_eq!(identifier.security_type, sec_type);
      assert_eq!(identifier.raw_id, id);
    }
  }

  #[test]
  fn test_security_type_encode_decode_5bit() {
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
    let encoded = SecurityType::encode(SecurityType::Equity, 0);
    let identifier = SecurityIdentifier::decode(encoded).unwrap();
    assert_eq!(identifier.security_type, SecurityType::Equity);
    assert_eq!(identifier.raw_id, 0);

    let encoded = SecurityType::encode(SecurityType::Bond, 1);
    let identifier = SecurityIdentifier::decode(encoded).unwrap();
    assert_eq!(identifier.security_type, SecurityType::Bond);
    assert_eq!(identifier.raw_id, 1);
  }

  #[test]
  fn test_bitmap_non_overlap() {
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

  #[test]
  fn test_security_type_from_str() {
    assert_eq!("Common Stock".parse::<SecurityType>(), Ok(SecurityType::Equity));
    assert_eq!("EQUITY".parse::<SecurityType>(), Ok(SecurityType::Equity));
    assert_eq!("stock".parse::<SecurityType>(), Ok(SecurityType::Equity));
    assert_eq!("ETF".parse::<SecurityType>(), Ok(SecurityType::ETF));
    assert_eq!("Exchange Traded Fund".parse::<SecurityType>(), Ok(SecurityType::ETF));
    assert_eq!("CRYPTO".parse::<SecurityType>(), Ok(SecurityType::Cryptocurrency));
    assert_eq!("fx".parse::<SecurityType>(), Ok(SecurityType::Currency));
    assert_eq!("UNKNOWN".parse::<SecurityType>(), Ok(SecurityType::Other));
  }

  #[test]
  fn test_security_type_display() {
    assert_eq!(format!("{}", SecurityType::Equity), "Common Stock");
    assert_eq!(format!("{}", SecurityType::ETF), "ETF");
    assert_eq!(format!("{}", SecurityType::Cryptocurrency), "Cryptocurrency");
  }

  #[test]
  fn test_security_type_categories() {
    assert!(SecurityType::Equity.is_equity());
    assert!(SecurityType::PreferredStock.is_equity());
    assert!(SecurityType::ETF.is_equity());
    assert!(SecurityType::REIT.is_equity());
    assert!(SecurityType::ADR.is_equity());
    assert!(!SecurityType::Bond.is_equity());
    assert!(!SecurityType::Option.is_equity());

    assert!(SecurityType::Bond.is_fixed_income());
    assert!(SecurityType::GovernmentBond.is_fixed_income());
    assert!(SecurityType::CorporateBond.is_fixed_income());
    assert!(SecurityType::MunicipalBond.is_fixed_income());
    assert!(SecurityType::TreasuryBill.is_fixed_income());
    assert!(SecurityType::CD.is_fixed_income());
    assert!(!SecurityType::Equity.is_fixed_income());
    assert!(!SecurityType::Option.is_fixed_income());

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

  #[test]
  fn test_encoding_roundtrip_all_types() {
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
