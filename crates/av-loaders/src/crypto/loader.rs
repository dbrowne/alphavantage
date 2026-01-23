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

//! Re-exports from crypto-loaders crate for backward compatibility.
//!
//! This module provides a wrapper around the crypto-loaders CryptoSymbolLoader
//! that integrates with the av-database-postgres CacheRepository.

use super::sources::CacheRepositoryAdapter;
use av_database_postgres::repository::CacheRepository;
use std::sync::Arc;

// Re-export the core loader and result type
pub use crypto_loaders::{CryptoSymbolLoader as BaseCryptoSymbolLoader, LoadAllSymbolsResult};

use super::{CryptoDataSource, CryptoLoaderConfig};
use std::collections::HashMap;

/// Extended CryptoSymbolLoader that supports av-database-postgres CacheRepository.
///
/// This struct wraps the base CryptoSymbolLoader from crypto-loaders and adds
/// support for the CacheRepository interface.
pub struct CryptoSymbolLoader {
  inner: BaseCryptoSymbolLoader,
}

impl CryptoSymbolLoader {
  /// Create a new symbol loader with the given configuration.
  pub fn new(config: CryptoLoaderConfig) -> Self {
    Self { inner: BaseCryptoSymbolLoader::new(config) }
  }

  /// Set API keys for providers.
  pub fn with_api_keys(mut self, api_keys: HashMap<CryptoDataSource, String>) -> Self {
    self.inner = self.inner.with_api_keys(api_keys);
    self
  }

  /// Set the cache repository for caching API responses.
  ///
  /// This converts the CacheRepository to a CryptoCache adapter.
  pub fn with_cache_repository(mut self, cache_repo: Arc<dyn CacheRepository>) -> Self {
    let cache_adapter = CacheRepositoryAdapter::as_arc(cache_repo);
    self.inner = self.inner.with_cache(cache_adapter);
    self
  }

  /// Load symbols from all configured sources.
  pub async fn load_all_symbols(
    &self,
  ) -> crypto_loaders::CryptoLoaderResult<LoadAllSymbolsResult> {
    self.inner.load_all_symbols().await
  }

  /// Load symbols from a specific source.
  pub async fn load_from_source(
    &self,
    source: CryptoDataSource,
  ) -> Result<Vec<super::CryptoSymbol>, crypto_loaders::CryptoLoaderError> {
    self.inner.load_from_source(source).await
  }
}

impl Clone for CryptoSymbolLoader {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}