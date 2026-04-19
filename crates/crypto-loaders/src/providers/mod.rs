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

//! Cryptocurrency data providers.
//!
//! Each sub-module implements the [`CryptoDataProvider`](crate::traits::CryptoDataProvider)
//! trait for a specific external API, providing a `fetch_symbols` method that
//! returns a `Vec<CryptoSymbol>`.
//!
//! # Provider inventory
//!
//! | Provider              | Struct                    | API key required? | API endpoint                                |
//! |-----------------------|---------------------------|-------------------|---------------------------------------------|
//! | [`coingecko`]         | [`CoinGeckoProvider`]     | Yes (Pro or Demo) | `coingecko.com/api/v3/coins/list`           |
//! | [`coinmarketcap`]     | [`CoinMarketCapProvider`] | Yes               | `pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest` |
//! | [`coinpaprika`]       | [`CoinPaprikaProvider`]   | No (free API)     | `api.coinpaprika.com/v1/coins`              |
//! | [`coincap`]           | [`CoinCapProvider`]       | No (free API)     | `api.coincap.io/v2/assets`                  |
//! | [`sosovalue`]         | [`SosoValueProvider`]     | Yes               | SosoValue crypto API                        |
//!
//! All providers implement `CryptoDataProvider + Send + Sync` and are used
//! by [`CryptoSymbolLoader`](crate::loaders::CryptoSymbolLoader) to aggregate
//! coin lists from multiple sources.
//!
//! # Provider construction
//!
//! - **Key-based providers** (CoinGecko, CoinMarketCap, SosoValue) are
//!   constructed with `::new(api_key)`.
//! - **Free providers** (CoinPaprika, CoinCap) are unit structs — no
//!   construction needed.

/// CoinCap (`api.coincap.io`) — free, no API key required.
pub mod coincap;

/// CoinGecko (`coingecko.com`) — supports Pro and Demo API tiers.
pub mod coingecko;

/// CoinMarketCap (`coinmarketcap.com`) — requires API key.
pub mod coinmarketcap;

/// CoinPaprika (`coinpaprika.com`) — free, no API key required.
pub mod coinpaprika;

/// SosoValue — requires API key.
pub mod sosovalue;

// ─── Convenience re-exports ─────────────────────────────────────────────────

pub use coincap::CoinCapProvider;
pub use coingecko::CoinGeckoProvider;
pub use coinmarketcap::CoinMarketCapProvider;
pub use coinpaprika::CoinPaprikaProvider;
pub use sosovalue::SosoValueProvider;
