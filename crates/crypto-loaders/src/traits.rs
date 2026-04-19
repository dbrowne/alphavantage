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

//! Core trait definitions for the `crypto-loaders` crate.
//!
//! This module defines the two fundamental abstractions that the rest of the
//! crate depends on:
//!
//! - [`CryptoCache`] — a storage-agnostic cache for API responses.
//! - [`CryptoDataProvider`] — the interface every data-source provider must implement.
//!
//! Both traits are `Send + Sync` and use `#[async_trait]` for async methods.
//!
//! # Implementing a new provider
//!
//! To add a new crypto data source:
//! 1. Create a struct in [`providers`](crate::providers).
//! 2. Implement [`CryptoDataProvider`] on it.
//! 3. Register it in [`CryptoSymbolLoader::new`](crate::loaders::CryptoSymbolLoader::new).

use async_trait::async_trait;
use std::sync::Arc;

use crate::error::CryptoLoaderError;
use crate::types::CryptoSymbol;

// ─── CryptoCache ────────────────────────────────────────────────────────────

/// Storage-agnostic cache for API responses.
///
/// Allows the loader layer to cache raw JSON responses without depending on
/// a specific database or in-memory store. Implementations are injected via
/// `Arc<dyn CryptoCache>`.
///
/// # Key structure
///
/// Cache entries are keyed by a `(cache_type, key)` pair:
/// - `cache_type` groups entries by purpose (e.g., `"coingecko_http"`,
///   `"crypto_loader"`).
/// - `key` is a unique identifier within that group (e.g.,
///   `"coingecko_http_coins_list"`, `"crypto_symbols_CoinGecko"`).
///
/// Values are opaque `String`s (typically serialized JSON).
#[async_trait]
pub trait CryptoCache: Send + Sync {
  /// Returns the cached value for `(cache_type, key)`, or `None` if missing
  /// or expired.
  async fn get(&self, cache_type: &str, key: &str) -> Result<Option<String>, CryptoLoaderError>;

  /// Stores a value in the cache with the given TTL (in hours).
  ///
  /// If an entry with the same key already exists, it is overwritten.
  async fn set(
    &self,
    cache_type: &str,
    key: &str,
    value: &str,
    ttl_hours: u32,
  ) -> Result<(), CryptoLoaderError>;

  /// Removes all expired entries for the given `cache_type`.
  ///
  /// Returns the number of entries removed.
  async fn cleanup_expired(&self, cache_type: &str) -> Result<usize, CryptoLoaderError>;
}

// ─── CryptoDataProvider ─────────────────────────────────────────────────────

/// Interface for a cryptocurrency data source.
///
/// Each provider (CoinGecko, CoinMarketCap, etc.) implements this trait.
/// The [`CryptoSymbolLoader`](crate::loaders::CryptoSymbolLoader) calls
/// `fetch_symbols` on each configured provider and merges the results.
///
/// # Required methods
///
/// | Method              | Purpose                                           |
/// |---------------------|---------------------------------------------------|
/// | `fetch_symbols`     | Fetch the coin list from the external API          |
/// | `source_name`       | Human-readable provider name (e.g., `"CoinGecko"`) |
/// | `rate_limit_delay`  | Milliseconds to wait between API calls             |
/// | `requires_api_key`  | Whether the provider needs an API key              |
#[async_trait]
pub trait CryptoDataProvider: Send + Sync {
  /// Fetches the cryptocurrency symbol list from this provider's API.
  ///
  /// An optional [`CryptoCache`] is passed for providers that support
  /// HTTP-level response caching (e.g., CoinGecko caches `/coins/list`).
  /// The `client` is a shared `reqwest::Client` for connection pooling.
  async fn fetch_symbols(
    &self,
    client: &reqwest::Client,
    cache: Option<&Arc<dyn CryptoCache>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError>;

  /// Returns the human-readable name of this data source (e.g., `"CoinGecko"`).
  fn source_name(&self) -> &'static str;

  /// Returns the recommended delay in milliseconds between consecutive
  /// API calls to this provider.
  fn rate_limit_delay(&self) -> u64;

  /// Returns `true` if this provider requires an API key to function.
  fn requires_api_key(&self) -> bool;
}
