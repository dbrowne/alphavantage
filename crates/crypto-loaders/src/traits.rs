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

//! Traits for cryptocurrency data providers.

use async_trait::async_trait;
use std::sync::Arc;

use crate::error::CryptoLoaderError;
use crate::types::CryptoSymbol;

/// Cache interface for crypto data providers.
///
/// This trait allows crypto-loaders to cache API responses without depending
/// on a specific database implementation.
#[async_trait]
pub trait CryptoCache: Send + Sync {
  /// Get a cached value by key.
  async fn get(&self, cache_type: &str, key: &str) -> Result<Option<String>, CryptoLoaderError>;

  /// Store a value in the cache with TTL in hours.
  async fn set(
    &self,
    cache_type: &str,
    key: &str,
    value: &str,
    ttl_hours: u32,
  ) -> Result<(), CryptoLoaderError>;

  /// Clean up expired cache entries.
  async fn cleanup_expired(&self, cache_type: &str) -> Result<usize, CryptoLoaderError>;
}

/// Trait for cryptocurrency data providers.
///
/// Implement this trait to add a new crypto data source.
#[async_trait]
pub trait CryptoDataProvider: Send + Sync {
  /// Fetch cryptocurrency symbols from this provider.
  async fn fetch_symbols(
    &self,
    client: &reqwest::Client,
    cache: Option<&Arc<dyn CryptoCache>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError>;

  /// Get the name of this data source.
  fn source_name(&self) -> &'static str;

  /// Get the rate limit delay in milliseconds.
  fn rate_limit_delay(&self) -> u64;

  /// Whether this provider requires an API key.
  fn requires_api_key(&self) -> bool;
}
