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

//! Market-related types for financial data.
//!
//! This is the public facade for the `market` module group within `av_core::types`.
//! It hides internal organization (three private submodules) behind a flat
//! re-export surface so consumers can write `use av_core::types::market::Exchange`
//! rather than `use av_core::types::market::exchange::Exchange`. This indirection
//! exists primarily for **backward compatibility** — earlier versions of the
//! crate defined these types directly in this file before they were split.
//!
//! ## Type Categories
//!
//! The types exported here cover three orthogonal aspects of financial market data:
//!
//! ### Exchange Identification
//!
//! - [`Exchange`] — Enum of 25 global stock exchange identifiers (NYSE, NASDAQ,
//!   AMEX, CBOT, CME, LSE, TSX, TSE, HKSE, SSE, SZSE, EURONEXT, FRA, SIX, ASX,
//!   BSE, NSE, BOVESPA, MOEX, KRX, TWSE, SGX, JSE, TASE, OTHER). Provides
//!   metadata accessors: `full_name()`, `timezone()`, `primary_currency()`,
//!   `is_major()`. Implements `Display` and `FromStr` (case-insensitive,
//!   accepts both abbreviations and full names) plus the standard derive set
//!   (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`,
//!   `Deserialize`).
//!
//! ### Security Type & Identifier Encoding
//!
//! - [`SecurityType`] — Enum of 20 security asset types covering equities
//!   (`Equity`, `PreferredStock`, `ETF`, `MutualFund`, `REIT`, `ADR`), fixed
//!   income (`CD`, `Bond`, `GovernmentBond`, `CorporateBond`, `MunicipalBond`,
//!   `TreasuryBill`), derivatives (`Option`, `Future`, `Warrant`), and other
//!   instruments (`Index`, `Currency`, `Commodity`, `Cryptocurrency`, `Other`).
//!   Provides classification helpers (`is_equity`, `is_fixed_income`,
//!   `is_derivative`), settlement metadata (`settlement_days() -> u8`), and
//!   AlphaVantage interop (`from_alpha_vantage`, `to_alpha_vantage`).
//!
//! - [`SecurityIdentifier`] — Struct that packs a `SecurityType` and a 32-bit
//!   `raw_id` into a single `i64` using **variable-length bit prefixes**:
//!   - 4-bit prefix for high-volume types (equity, ETF, options)
//!   - 5-bit prefix for medium-volume types (bonds, crypto)
//!   - 6-bit prefix for low-volume types (currency, indices, commodities)
//!
//!   This compact encoding allows a single `i64` database column to store
//!   both the type tag and the unique ID, with the remaining bits hosting
//!   the 32-bit ID space. Round-trip via [`SecurityType::encode`] and
//!   [`SecurityIdentifier::decode`].
//!
//! ### Market Classifications
//!
//! - [`TopType`] — Enum for top-mover queries: `Gainers`, `Losers`, `MostActive`.
//!   `FromStr` accepts aliases like `"winners"`, `"topgainers"`, `"decliners"`
//!   for user convenience.
//!
//! - [`Sector`] — Enum of 12 GICS-style market sectors: `Technology`,
//!   `Healthcare`, `FinancialServices`, `ConsumerDiscretionary`,
//!   `ConsumerStaples`, `Industrials`, `Energy`, `Materials`, `RealEstate`,
//!   `Utilities`, `CommunicationServices`, `Other`. Provides analytical helpers:
//!   `is_cyclical()`, `is_defensive()`, `typical_pe_range() -> (f64, f64)`.
//!
//! - [`MarketCap`] — Enum of 6 market-capitalization tiers (`NanoCap`, `MicroCap`,
//!   `SmallCap`, `MidCap`, `LargeCap`, `MegaCap`). Provides
//!   `from_value(f64) -> Self` for classifying a USD market cap value,
//!   `range() -> (f64, Option<f64>)` for the tier's USD bounds, and
//!   convenience predicates `is_large()` / `is_small()`.
//!
//! ## Internal Module Layout
//!
//! ```text
//! market/
//! ├── mod.rs           ← this file (public facade, re-exports only)
//! ├── classifications.rs   → TopType, Sector, MarketCap
//! ├── exchange.rs          → Exchange
//! └── security_type.rs     → SecurityType, SecurityIdentifier
//! ```
//!
//! Submodules are declared as private (`mod`, not `pub mod`), so the only way
//! to access these types from outside the `market` module is through the
//! `pub use` re-exports below. Code in other parts of the crate (and external
//! consumers) should always import via `av_core::types::market::TypeName`.
//!
//! ## Common Trait Implementations
//!
//! All exported types implement a consistent set of derives suitable for
//! database storage, serialization, and use as map keys:
//!
//! | Trait                  | Purpose                              |
//! |------------------------|--------------------------------------|
//! | `Debug`                | Diagnostic printing                  |
//! | `Clone`, `Copy`        | Value semantics (all are POD-sized)  |
//! | `PartialEq`, `Eq`      | Equality comparison                  |
//! | `Hash`                 | Use as `HashMap` / `HashSet` key     |
//! | `Serialize`, `Deserialize` | JSON/serde interoperability      |
//! | `Display`, `FromStr`   | String round-tripping (most types)   |

mod classifications;
mod exchange;
mod security_type;

pub use classifications::{MarketCap, Sector, TopType};
pub use exchange::Exchange;
pub use security_type::{SecurityIdentifier, SecurityType};
