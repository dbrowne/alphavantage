/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Common configuration traits for loaders.

/// Trait for loader configurations that support caching.
///
/// Implement this trait to enable consistent cache behavior across loaders.
/// This works with [`CacheConfigProvider`](crate::CacheConfigProvider) but provides
/// a simpler interface for basic cache control.
///
/// # Example
///
/// ```ignore
/// #[derive(Debug, Clone, Default)]
/// pub struct MyLoaderConfig {
///     pub enable_cache: bool,
///     pub cache_ttl_hours: i64,
///     pub force_refresh: bool,
///     // ... loader-specific fields
/// }
///
/// impl CacheableConfig for MyLoaderConfig {
///     fn cache_enabled(&self) -> bool { self.enable_cache }
///     fn cache_ttl_hours(&self) -> i64 { self.cache_ttl_hours }
///     fn force_refresh(&self) -> bool { self.force_refresh }
/// }
/// ```
pub trait CacheableConfig {
  /// Whether caching is enabled for this loader.
  fn cache_enabled(&self) -> bool;

  /// Time-to-live for cached entries in hours.
  fn cache_ttl_hours(&self) -> i64;

  /// Whether to bypass cache and force fresh data fetch.
  fn force_refresh(&self) -> bool;

  /// Check if we should attempt to read from cache.
  ///
  /// Returns `true` if caching is enabled and we're not forcing a refresh.
  fn should_check_cache(&self) -> bool {
    self.cache_enabled() && !self.force_refresh()
  }

  /// Check if we should write to cache after fetching.
  ///
  /// Returns `true` if caching is enabled.
  fn should_write_cache(&self) -> bool {
    self.cache_enabled()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Default)]
  struct TestConfig {
    enable_cache: bool,
    cache_ttl_hours: i64,
    force_refresh: bool,
  }

  impl CacheableConfig for TestConfig {
    fn cache_enabled(&self) -> bool {
      self.enable_cache
    }
    fn cache_ttl_hours(&self) -> i64 {
      self.cache_ttl_hours
    }
    fn force_refresh(&self) -> bool {
      self.force_refresh
    }
  }

  #[test]
  fn test_should_check_cache_enabled() {
    let config = TestConfig { enable_cache: true, cache_ttl_hours: 24, force_refresh: false };
    assert!(config.should_check_cache());
  }

  #[test]
  fn test_should_check_cache_disabled() {
    let config = TestConfig { enable_cache: false, cache_ttl_hours: 24, force_refresh: false };
    assert!(!config.should_check_cache());
  }

  #[test]
  fn test_should_check_cache_force_refresh() {
    let config = TestConfig { enable_cache: true, cache_ttl_hours: 24, force_refresh: true };
    assert!(!config.should_check_cache());
  }

  #[test]
  fn test_should_write_cache() {
    let config = TestConfig { enable_cache: true, cache_ttl_hours: 24, force_refresh: true };
    // Even with force_refresh, we still write to cache
    assert!(config.should_write_cache());
  }
}
