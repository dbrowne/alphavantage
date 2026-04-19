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

//! Cryptocurrency data loaders.
//!
//! This module provides two high-level loaders that orchestrate fetching
//! cryptocurrency data from multiple external APIs and preparing it for
//! database insertion.
//!
//! # Loader inventory
//!
//! ## [`symbol_loader`] — Symbol discovery and aggregation
//!
//! | Type                    | Purpose                                              |
//! |-------------------------|------------------------------------------------------|
//! | [`CryptoSymbolLoader`]  | Fetches coin lists from 5 providers, deduplicates, and merges |
//! | [`LoadAllSymbolsResult`]| Aggregated result: merged symbols + per-source stats |
//!
//! The symbol loader queries CoinGecko, CoinMarketCap, CoinPaprika,
//! CoinCap, and SosoValue in parallel, then merges the results into a
//! unified symbol list with cross-source mappings.
//!
//! ## [`details_loader`] — Detailed coin data (social + technical)
//!
//! | Type                        | Purpose                                          |
//! |-----------------------------|--------------------------------------------------|
//! | [`CoinGeckoDetailsLoader`]  | Fetches detailed coin data from CoinGecko API    |
//! | [`DetailsLoaderConfig`]     | Rate limits, batch size, retry configuration      |
//! | [`CoinGeckoDetailsOutput`]  | Batch result: social data + technical data arrays |
//! | [`CoinGeckoDetailedCoin`]   | Raw CoinGecko `/coins/{id}` response              |
//! | [`CoinInfo`]                | Input DTO: `(sid, symbol, coingecko_id)` tuple    |
//! | [`CryptoDetailedData`]      | Combined social + technical data for one coin     |
//! | [`CryptoSocialData`]        | Flat social/community data ready for DB insert    |
//! | [`CryptoTechnicalData`]     | Blockchain, GitHub, and category data for DB      |
//!
//! # Architecture
//!
//! ```text
//! CryptoSymbolLoader
//!   ├── CoinGeckoProvider ──► API
//!   ├── CoinMarketCapProvider ──► API
//!   ├── CoinPaprikaProvider ──► API
//!   ├── CoinCapProvider ──► API
//!   └── SosoValueProvider ──► API
//!         └──► merged CryptoSymbol list ──► database
//!
//! CoinGeckoDetailsLoader
//!   └── CoinGecko /coins/{id} ──► CoinGeckoDetailedCoin
//!         └──► CryptoSocialData + CryptoTechnicalData ──► database
//! ```
//!
//! Both loaders support optional [`CryptoCache`](crate::traits::CryptoCache)
//! for response caching and respect rate limits via configurable delays.

/// Symbol discovery: loads coin lists from 5 providers, deduplicates,
/// merges, and produces a unified [`CryptoSymbol`](crate::types::CryptoSymbol) list.
pub mod symbol_loader;

/// Detailed coin data: fetches social/community, developer/technical,
/// and classification data from the CoinGecko `/coins/{id}` endpoint.
pub mod details_loader;

// ─── Convenience re-exports ─────────────────────────────────────────────────

/// Re-exported from [`details_loader`]: CoinGecko response types, loader,
/// config, and normalized output structs.
pub use details_loader::{
  CoinGeckoDetailedCoin, CoinGeckoDetailsLoader, CoinGeckoDetailsOutput, CoinInfo,
  CryptoDetailedData, CryptoSocialData, CryptoTechnicalData, DetailsLoaderConfig,
};

/// Re-exported from [`symbol_loader`]: the multi-provider symbol loader
/// and its aggregated result type.
pub use symbol_loader::{CryptoSymbolLoader, LoadAllSymbolsResult};
