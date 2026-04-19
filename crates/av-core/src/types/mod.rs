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

//! Shared type definitions for the `av-core` crate.
//!
//! This module serves as the **top-level type façade** for the crate. It aggregates
//! types from two public sub-modules — [`common`] and [`market`] — and re-exports
//! the most frequently used types at this level so consumers can write concise
//! imports like:
//!
//! ```rust
//! use av_core::types::{Interval, OutputSize, Exchange, SecurityType};
//! ```
//!
//! rather than the fully-qualified paths:
//!
//! ```rust
//! use av_core::types::common::Interval;
//! use av_core::types::market::Exchange;
//! ```
//!
//! Both import styles are valid; the re-exports exist purely for convenience.
//!
//! # Sub-module overview
//!
//! ## [`common`] — API request/response primitives
//!
//! Types that parameterize Alpha Vantage API calls and appear across multiple
//! endpoint categories:
//!
//! | Type              | Purpose                                                           |
//! |-------------------|-------------------------------------------------------------------|
//! | [`DataType`]      | Response format selector: `Json` or `Csv`.                        |
//! | [`Interval`]      | Intraday bar width: `Min1`, `Min5`, `Min15`, `Min30`, `Min60`.    |
//! | [`OutputSize`]    | Result set size: `Compact` (latest 100 points) or `Full` (up to 20 years). |
//! | `SortOrder`       | Ordering for news/search results: `Latest`, `Earliest`, `Relevance`. |
//! | `TimeHorizon`     | Calendar data range: `ThreeMonth`, `SixMonth`, `TwelveMonth`.    |
//! | `ListingState`    | Security listing status: `Active` or `Delisted`.                  |
//! | `SentimentLabel`  | News sentiment classification: `Bullish`, `Neutral`, `Bearish`.   |
//! | `CurrencyCode`    | ISO 4217 fiat currency codes (29 currencies).                     |
//! | `CryptoSymbol`    | Major cryptocurrency ticker symbols (20 coins).                  |
//!
//! ## [`market`] — Financial instrument & exchange metadata
//!
//! Types that describe securities, exchanges, and market classifications:
//!
//! | Type                   | Purpose                                                     |
//! |------------------------|-------------------------------------------------------------|
//! | [`Exchange`]           | 25-variant enum of global stock exchanges with timezone, currency, and classification metadata. |
//! | [`SecurityType`]       | 20-variant enum of instrument categories (equity, bond, derivative, etc.) with bitmap encoding and Alpha Vantage API mapping. |
//! | [`SecurityIdentifier`] | Compact `i64` bitmap packing a [`SecurityType`] + `u32` ID using variable-length prefixes. |
//! | [`TopType`]            | Top-mover query type: `Gainers`, `Losers`, `MostActive`.    |
//! | [`Sector`]             | 12 GICS-style market sectors with cyclical/defensive classification and typical P/E ranges. |
//! | [`MarketCap`]          | 6 market-capitalization tiers from `NanoCap` to `MegaCap` with USD range boundaries. |
//!
//! # Re-exports
//!
//! The `pub use` statements below hoist the most commonly needed types to the
//! `av_core::types` namespace. The full sub-modules remain accessible for types
//! not re-exported here (e.g., `SortOrder`, `CurrencyCode`, `CryptoSymbol`).

/// API request/response primitives shared across endpoint categories.
///
/// See [`common::DataType`], [`common::Interval`], [`common::OutputSize`],
/// and other types defined in `common.rs`.
pub mod common;

/// Financial instrument metadata, exchange identifiers, and market classifications.
///
/// See [`market::Exchange`], [`market::SecurityType`], [`market::SecurityIdentifier`],
/// [`market::TopType`], [`market::Sector`], and [`market::MarketCap`].
pub mod market;

// ─── Convenience re-exports ─────────────────────────────────────────────────
//
// Hoist the most frequently used types to `av_core::types::*` so downstream
// code doesn't need to spell out the sub-module path for everyday imports.

/// Re-exported from [`common`]: API response format (`Json` / `Csv`), intraday
/// bar interval, and result set size.
pub use common::{DataType, Interval, OutputSize};

/// Re-exported from [`market`]: exchange identifiers, security type enum and
/// bitmap identifier, top-mover query type, GICS sector classification, and
/// market-capitalization tiers.
pub use market::{Exchange, MarketCap, Sector, SecurityIdentifier, SecurityType, TopType};
