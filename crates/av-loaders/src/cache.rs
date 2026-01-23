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

//! Unified cache implementation for all loaders.
//!
//! This module provides a consistent caching interface that all loaders can use,
//! abstracting away the details of cache operations and providing common utilities.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use av_loaders::cache::{CacheConfig, CacheHelper, CacheResult};
//!
//! // Create cache helper with configuration
//! let cache = CacheHelper::new(CacheConfig::default());
//!
//! // Get cached data (typed)
//! let result: CacheResult<MyData> = cache.get(&cache_repo, "my_key", "alphavantage").await;
//!
//! // Store data in cache
//! cache.set(&cache_repo, "my_key", "alphavantage", "endpoint", &data).await;
//! ```

use av_database_postgres::repository::{CacheRepository, CacheRepositoryExt};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::error::LoaderError;

/// Common cache configuration that all loaders can use.
///
/// This provides a standardized set of cache options that can be
/// embedded in loader-specific configurations.
#[derive(Debug, Clone)]
pub struct CacheConfig {
  /// Enable caching (requires cache_repository)
  pub enable_cache: bool,
  /// Cache TTL in hours
  pub cache_ttl_hours: i64,
  /// Force refresh (bypass cache reads, but still write)
  pub force_refresh: bool,
  /// API source identifier for cache partitioning
  pub api_source: String,
}

impl Default for CacheConfig {
  fn default() -> Self {
    Self {
      enable_cache: true,
      cache_ttl_hours: 24,
      force_refresh: false,
      api_source: "alphavantage".to_string(),
    }
  }
}

impl CacheConfig {
  /// Create config for AlphaVantage data with custom TTL
  pub fn alphavantage(ttl_hours: i64) -> Self {
    Self { cache_ttl_hours: ttl_hours, api_source: "alphavantage".to_string(), ..Default::default() }
  }

  /// Create config for crypto data with custom TTL
  pub fn crypto(ttl_hours: i64) -> Self {
    Self { cache_ttl_hours: ttl_hours, api_source: "crypto".to_string(), ..Default::default() }
  }

  /// Create config for CoinGecko data
  pub fn coingecko(ttl_hours: i64) -> Self {
    Self { cache_ttl_hours: ttl_hours, api_source: "coingecko".to_string(), ..Default::default() }
  }

  /// Builder: set enable_cache
  pub fn with_enabled(mut self, enabled: bool) -> Self {
    self.enable_cache = enabled;
    self
  }

  /// Builder: set force_refresh
  pub fn with_force_refresh(mut self, force: bool) -> Self {
    self.force_refresh = force;
    self
  }

  /// Builder: set cache_ttl_hours
  pub fn with_ttl_hours(mut self, hours: i64) -> Self {
    self.cache_ttl_hours = hours;
    self
  }

  /// Builder: set api_source
  pub fn with_api_source(mut self, source: impl Into<String>) -> Self {
    self.api_source = source.into();
    self
  }
}

/// Result of a cache get operation
#[derive(Debug)]
pub enum CacheResult<T> {
  /// Cache hit with data
  Hit(T),
  /// Cache miss (no data or expired)
  Miss,
  /// Cache disabled or force refresh
  Skipped,
  /// Cache error (logged, treated as miss)
  Error(String),
}

impl<T> CacheResult<T> {
  /// Returns true if this is a cache hit
  pub fn is_hit(&self) -> bool {
    matches!(self, CacheResult::Hit(_))
  }

  /// Returns true if cache was actually checked (not skipped)
  pub fn was_checked(&self) -> bool {
    !matches!(self, CacheResult::Skipped)
  }

  /// Convert to Option, returning None for non-hits
  pub fn into_option(self) -> Option<T> {
    match self {
      CacheResult::Hit(data) => Some(data),
      _ => None,
    }
  }

  /// Get reference to data if hit
  pub fn as_ref(&self) -> Option<&T> {
    match self {
      CacheResult::Hit(data) => Some(data),
      _ => None,
    }
  }
}

/// Unified cache helper for all loaders.
///
/// This struct provides standardized cache operations that work with
/// the CacheRepository trait, handling common patterns like:
/// - Checking if cache is enabled before operations
/// - Logging cache hits/misses
/// - Handling errors gracefully
/// - Computing expiration times
#[derive(Debug, Clone)]
pub struct CacheHelper {
  config: CacheConfig,
}

impl CacheHelper {
  /// Create a new cache helper with the given configuration
  pub fn new(config: CacheConfig) -> Self {
    Self { config }
  }

  /// Create with default configuration
  pub fn default_alphavantage() -> Self {
    Self::new(CacheConfig::alphavantage(24))
  }

  /// Get the cache configuration
  pub fn config(&self) -> &CacheConfig {
    &self.config
  }

  /// Check if caching is effectively enabled (enabled and not force refresh for reads)
  pub fn is_read_enabled(&self) -> bool {
    self.config.enable_cache && !self.config.force_refresh
  }

  /// Check if caching is enabled for writes
  pub fn is_write_enabled(&self) -> bool {
    self.config.enable_cache
  }

  /// Generate a simple cache key from a prefix and identifier
  pub fn make_key(prefix: &str, identifier: &str) -> String {
    format!("{}_{}", prefix, identifier.to_uppercase())
  }

  /// Generate a cache key with multiple parts
  pub fn make_key_parts(parts: &[&str]) -> String {
    parts.iter().map(|p| p.to_uppercase()).collect::<Vec<_>>().join("_")
  }

  /// Get cached data with typed deserialization.
  ///
  /// Returns `CacheResult::Hit(data)` on cache hit,
  /// `CacheResult::Miss` if not found,
  /// `CacheResult::Skipped` if cache disabled or force refresh,
  /// `CacheResult::Error` if an error occurred (logged).
  pub async fn get<T>(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
  ) -> CacheResult<T>
  where
    T: for<'de> Deserialize<'de> + Send + 'static,
  {
    if !self.is_read_enabled() {
      return CacheResult::Skipped;
    }

    match cache_repo.get::<T>(cache_key, &self.config.api_source).await {
      Ok(Some(data)) => {
        info!("📦 Cache hit for key: {}", cache_key);
        CacheResult::Hit(data)
      }
      Ok(None) => {
        debug!("Cache miss for key: {}", cache_key);
        CacheResult::Miss
      }
      Err(e) => {
        debug!("Cache read error for key {}: {}", cache_key, e);
        CacheResult::Error(e.to_string())
      }
    }
  }

  /// Get cached JSON data (for untyped or CSV-wrapped data).
  pub async fn get_json(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
  ) -> CacheResult<serde_json::Value> {
    if !self.is_read_enabled() {
      return CacheResult::Skipped;
    }

    match cache_repo.get_json(cache_key, &self.config.api_source).await {
      Ok(Some(data)) => {
        info!("📦 Cache hit for key: {}", cache_key);
        CacheResult::Hit(data)
      }
      Ok(None) => {
        debug!("Cache miss for key: {}", cache_key);
        CacheResult::Miss
      }
      Err(e) => {
        debug!("Cache read error for key {}: {}", cache_key, e);
        CacheResult::Error(e.to_string())
      }
    }
  }

  /// Store data in cache with typed serialization.
  ///
  /// Logs success/failure and returns Ok(true) if cached, Ok(false) if cache disabled.
  pub async fn set<T>(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
    endpoint_url: &str,
    data: &T,
  ) -> Result<bool, LoaderError>
  where
    T: Serialize + Send + Sync,
  {
    if !self.is_write_enabled() {
      return Ok(false);
    }

    match cache_repo
      .set(cache_key, &self.config.api_source, endpoint_url, data, self.config.cache_ttl_hours)
      .await
    {
      Ok(()) => {
        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours);
        info!("💾 Cached {} (expires: {})", cache_key, expires_at);
        Ok(true)
      }
      Err(e) => {
        warn!("Failed to cache {}: {}", cache_key, e);
        // Don't fail the operation, just warn
        Ok(false)
      }
    }
  }

  /// Store JSON data in cache.
  pub async fn set_json(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
    endpoint_url: &str,
    data: serde_json::Value,
  ) -> Result<bool, LoaderError> {
    if !self.is_write_enabled() {
      return Ok(false);
    }

    match cache_repo
      .set_json(cache_key, &self.config.api_source, endpoint_url, data, self.config.cache_ttl_hours)
      .await
    {
      Ok(()) => {
        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours);
        info!("💾 Cached {} (expires: {})", cache_key, expires_at);
        Ok(true)
      }
      Err(e) => {
        warn!("Failed to cache {}: {}", cache_key, e);
        Ok(false)
      }
    }
  }

  /// Clean up expired cache entries.
  ///
  /// Returns the number of entries deleted.
  pub async fn cleanup_expired(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<usize, LoaderError> {
    match cache_repo.cleanup_expired(&self.config.api_source).await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired {} cache entries", deleted_count, self.config.api_source);
        }
        Ok(deleted_count)
      }
      Err(e) => Err(LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e))),
    }
  }

  /// Static cleanup method for use without a CacheHelper instance.
  pub async fn cleanup_for_source(
    cache_repo: &Arc<dyn CacheRepository>,
    api_source: &str,
  ) -> Result<usize, LoaderError> {
    match cache_repo.cleanup_expired(api_source).await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired {} cache entries", deleted_count, api_source);
        }
        Ok(deleted_count)
      }
      Err(e) => Err(LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e))),
    }
  }
}

/// Trait for loader configurations that support caching.
///
/// Implement this trait on your loader config struct to enable
/// integration with CacheHelper.
pub trait CacheConfigProvider {
  /// Whether caching is enabled
  fn cache_enabled(&self) -> bool;

  /// Cache TTL in hours
  fn cache_ttl_hours(&self) -> i64;

  /// Whether to force refresh (bypass cache reads)
  fn force_refresh(&self) -> bool;

  /// API source identifier (default: "alphavantage")
  fn api_source(&self) -> &str {
    "alphavantage"
  }

  /// Create a CacheConfig from this provider
  fn to_cache_config(&self) -> CacheConfig {
    CacheConfig {
      enable_cache: self.cache_enabled(),
      cache_ttl_hours: self.cache_ttl_hours(),
      force_refresh: self.force_refresh(),
      api_source: self.api_source().to_string(),
    }
  }

  /// Create a CacheHelper from this provider
  fn to_cache_helper(&self) -> CacheHelper {
    CacheHelper::new(self.to_cache_config())
  }
}

/// Common cache key prefixes used by loaders
pub mod keys {
  /// Symbol search results
  pub const SYMBOL_SEARCH: &str = "symbol_search";
  /// Company overview data
  pub const OVERVIEW: &str = "overview";
  /// News sentiment data
  pub const NEWS_SENTIMENT: &str = "news_sentiment";
  /// Top market movers
  pub const TOP_MOVERS: &str = "top_movers";
  /// Daily price data (CSV)
  pub const DAILY_PRICES_CSV: &str = "daily_prices_csv";
  /// Intraday price data (CSV)
  pub const INTRADAY_PRICES_CSV: &str = "intraday_prices_csv";
  /// Crypto metadata
  pub const CRYPTO_METADATA: &str = "crypto_metadata";
  /// Crypto markets data
  pub const CRYPTO_MARKETS: &str = "crypto_markets";
  /// CoinGecko details
  pub const COINGECKO_DETAILS: &str = "coingecko_details";
}

/// Default TTL values for different data types (in hours)
pub mod ttl {
  /// Symbol data - relatively stable (7 days)
  pub const SYMBOL_SEARCH: i64 = 168;
  /// Company fundamentals - changes infrequently (30 days)
  pub const OVERVIEW: i64 = 720;
  /// News - changes frequently (1 day)
  pub const NEWS: i64 = 24;
  /// Top movers - changes daily (1 day)
  pub const TOP_MOVERS: i64 = 24;
  /// Daily prices - end of day data (1 day)
  pub const DAILY_PRICES: i64 = 24;
  /// Intraday prices - equity data (1 day)
  pub const INTRADAY_PRICES: i64 = 24;
  /// Crypto intraday - crypto moves fast (2 hours)
  pub const CRYPTO_INTRADAY: i64 = 2;
  /// Crypto metadata - changes occasionally (1 day)
  pub const CRYPTO_METADATA: i64 = 24;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cache_config_defaults() {
    let config = CacheConfig::default();
    assert!(config.enable_cache);
    assert_eq!(config.cache_ttl_hours, 24);
    assert!(!config.force_refresh);
    assert_eq!(config.api_source, "alphavantage");
  }

  #[test]
  fn test_cache_config_builders() {
    let config = CacheConfig::alphavantage(168).with_force_refresh(true).with_enabled(false);

    assert!(!config.enable_cache);
    assert_eq!(config.cache_ttl_hours, 168);
    assert!(config.force_refresh);
    assert_eq!(config.api_source, "alphavantage");
  }

  #[test]
  fn test_cache_config_crypto() {
    let config = CacheConfig::crypto(2);
    assert_eq!(config.api_source, "crypto");
    assert_eq!(config.cache_ttl_hours, 2);
  }

  #[test]
  fn test_make_key() {
    assert_eq!(CacheHelper::make_key("symbol_search", "aapl"), "symbol_search_AAPL");
    assert_eq!(CacheHelper::make_key("overview", "MSFT"), "overview_MSFT");
  }

  #[test]
  fn test_make_key_parts() {
    let key = CacheHelper::make_key_parts(&["news_sentiment", "aapl", "2024-01", "latest"]);
    assert_eq!(key, "NEWS_SENTIMENT_AAPL_2024-01_LATEST");
  }

  #[test]
  fn test_cache_helper_read_enabled() {
    let helper = CacheHelper::new(CacheConfig::default());
    assert!(helper.is_read_enabled());

    let helper_disabled = CacheHelper::new(CacheConfig::default().with_enabled(false));
    assert!(!helper_disabled.is_read_enabled());

    let helper_force = CacheHelper::new(CacheConfig::default().with_force_refresh(true));
    assert!(!helper_force.is_read_enabled());
    assert!(helper_force.is_write_enabled()); // Force refresh still writes
  }

  #[test]
  fn test_cache_result() {
    let hit: CacheResult<String> = CacheResult::Hit("data".to_string());
    assert!(hit.is_hit());
    assert!(hit.was_checked());
    assert_eq!(hit.into_option(), Some("data".to_string()));

    let miss: CacheResult<String> = CacheResult::Miss;
    assert!(!miss.is_hit());
    assert!(miss.was_checked());
    assert_eq!(miss.into_option(), None);

    let skipped: CacheResult<String> = CacheResult::Skipped;
    assert!(!skipped.is_hit());
    assert!(!skipped.was_checked());
  }
}