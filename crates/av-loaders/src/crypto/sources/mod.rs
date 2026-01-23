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
//! This module re-exports providers from the `crypto-loaders` crate and provides
//! an adapter for the av-database-postgres CacheRepository.

use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use std::sync::Arc;

// Re-export types and providers from crypto-loaders
pub use crypto_loaders::{
  CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider,
  CryptoCache, CryptoDataProvider, CryptoLoaderError, CryptoSymbol, SosoValueProvider,
};

/// Adapter that implements the crypto-loaders CryptoCache trait
/// using the av-database-postgres CacheRepository.
pub struct CacheRepositoryAdapter {
  repo: Arc<dyn CacheRepository>,
}

impl CacheRepositoryAdapter {
  pub fn new(repo: Arc<dyn CacheRepository>) -> Self {
    Self { repo }
  }

  pub fn as_arc(repo: Arc<dyn CacheRepository>) -> Arc<dyn CryptoCache> {
    Arc::new(Self::new(repo))
  }
}

#[async_trait]
impl CryptoCache for CacheRepositoryAdapter {
  async fn get(&self, cache_type: &str, key: &str) -> Result<Option<String>, CryptoLoaderError> {
    match self.repo.get_json(key, cache_type).await {
      Ok(Some(value)) => {
        // Convert serde_json::Value to String
        Ok(Some(value.to_string()))
      }
      Ok(None) => Ok(None),
      Err(e) => Err(CryptoLoaderError::CacheError(e.to_string())),
    }
  }

  async fn set(
    &self,
    cache_type: &str,
    key: &str,
    value: &str,
    ttl_hours: u32,
  ) -> Result<(), CryptoLoaderError> {
    // Parse the string value as JSON
    let json_value: serde_json::Value =
      serde_json::from_str(value).map_err(|e| CryptoLoaderError::CacheError(e.to_string()))?;

    // Use empty string for endpoint since we don't have that info here
    self
      .repo
      .set_json(key, cache_type, "", json_value, ttl_hours.into())
      .await
      .map_err(|e| CryptoLoaderError::CacheError(e.to_string()))
  }

  async fn cleanup_expired(&self, cache_type: &str) -> Result<usize, CryptoLoaderError> {
    self
      .repo
      .cleanup_expired(cache_type)
      .await
      .map_err(|e| CryptoLoaderError::CacheError(e.to_string()))
  }
}