/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Statistics tracking for loader operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Tracks common statistics for loader operations.
///
/// This provides a thread-safe way to accumulate statistics across
/// concurrent loader operations. All counters use atomic operations
/// for safe concurrent access.
///
/// # Example
///
/// ```
/// use loader_base::LoaderStatistics;
///
/// let stats = LoaderStatistics::new();
///
/// // Record some operations
/// stats.record_cache_hit();
/// stats.record_api_call();
/// stats.record_api_call();
/// stats.record_error();
///
/// assert_eq!(stats.cache_hits(), 1);
/// assert_eq!(stats.api_calls(), 2);
/// assert_eq!(stats.errors(), 1);
/// assert_eq!(stats.total_processed(), 4);
/// ```
#[derive(Debug, Default)]
pub struct LoaderStatistics {
  cache_hits: AtomicUsize,
  api_calls: AtomicUsize,
  errors: AtomicUsize,
  skipped: AtomicUsize,
}

impl LoaderStatistics {
  /// Create a new statistics tracker with all counters at zero.
  pub fn new() -> Self {
    Self::default()
  }

  /// Record a cache hit.
  pub fn record_cache_hit(&self) {
    self.cache_hits.fetch_add(1, Ordering::Relaxed);
  }

  /// Record an API call.
  pub fn record_api_call(&self) {
    self.api_calls.fetch_add(1, Ordering::Relaxed);
  }

  /// Record an error.
  pub fn record_error(&self) {
    self.errors.fetch_add(1, Ordering::Relaxed);
  }

  /// Record a skipped item.
  pub fn record_skipped(&self) {
    self.skipped.fetch_add(1, Ordering::Relaxed);
  }

  /// Get the number of cache hits.
  pub fn cache_hits(&self) -> usize {
    self.cache_hits.load(Ordering::Relaxed)
  }

  /// Get the number of API calls.
  pub fn api_calls(&self) -> usize {
    self.api_calls.load(Ordering::Relaxed)
  }

  /// Get the number of errors.
  pub fn errors(&self) -> usize {
    self.errors.load(Ordering::Relaxed)
  }

  /// Get the number of skipped items.
  pub fn skipped(&self) -> usize {
    self.skipped.load(Ordering::Relaxed)
  }

  /// Get the total number of items processed (cache hits + api calls + errors).
  pub fn total_processed(&self) -> usize {
    self.cache_hits() + self.api_calls() + self.errors()
  }

  /// Get the total number of successful items (cache hits + api calls).
  pub fn total_successful(&self) -> usize {
    self.cache_hits() + self.api_calls()
  }

  /// Reset all counters to zero.
  pub fn reset(&self) {
    self.cache_hits.store(0, Ordering::Relaxed);
    self.api_calls.store(0, Ordering::Relaxed);
    self.errors.store(0, Ordering::Relaxed);
    self.skipped.store(0, Ordering::Relaxed);
  }

  /// Create an Arc-wrapped instance for sharing across async tasks.
  pub fn shared() -> Arc<Self> {
    Arc::new(Self::new())
  }
}

impl Clone for LoaderStatistics {
  fn clone(&self) -> Self {
    Self {
      cache_hits: AtomicUsize::new(self.cache_hits()),
      api_calls: AtomicUsize::new(self.api_calls()),
      errors: AtomicUsize::new(self.errors()),
      skipped: AtomicUsize::new(self.skipped()),
    }
  }
}

/// Summary of loader statistics for output structures.
///
/// This is a simple struct that can be included in loader output types
/// to provide consistent statistics reporting.
#[derive(Debug, Clone, Default)]
pub struct StatisticsSummary {
  pub cache_hits: usize,
  pub api_calls: usize,
  pub errors: usize,
  pub skipped: usize,
}

impl From<&LoaderStatistics> for StatisticsSummary {
  fn from(stats: &LoaderStatistics) -> Self {
    Self {
      cache_hits: stats.cache_hits(),
      api_calls: stats.api_calls(),
      errors: stats.errors(),
      skipped: stats.skipped(),
    }
  }
}

impl From<LoaderStatistics> for StatisticsSummary {
  fn from(stats: LoaderStatistics) -> Self {
    Self::from(&stats)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_statistics() {
    let stats = LoaderStatistics::new();
    assert_eq!(stats.cache_hits(), 0);
    assert_eq!(stats.api_calls(), 0);
    assert_eq!(stats.errors(), 0);
    assert_eq!(stats.skipped(), 0);
  }

  #[test]
  fn test_record_operations() {
    let stats = LoaderStatistics::new();

    stats.record_cache_hit();
    stats.record_cache_hit();
    stats.record_api_call();
    stats.record_error();
    stats.record_skipped();

    assert_eq!(stats.cache_hits(), 2);
    assert_eq!(stats.api_calls(), 1);
    assert_eq!(stats.errors(), 1);
    assert_eq!(stats.skipped(), 1);
  }

  #[test]
  fn test_total_processed() {
    let stats = LoaderStatistics::new();

    stats.record_cache_hit();
    stats.record_api_call();
    stats.record_api_call();
    stats.record_error();

    assert_eq!(stats.total_processed(), 4);
    assert_eq!(stats.total_successful(), 3);
  }

  #[test]
  fn test_reset() {
    let stats = LoaderStatistics::new();
    stats.record_cache_hit();
    stats.record_api_call();

    stats.reset();

    assert_eq!(stats.cache_hits(), 0);
    assert_eq!(stats.api_calls(), 0);
  }

  #[test]
  fn test_clone() {
    let stats = LoaderStatistics::new();
    stats.record_cache_hit();
    stats.record_api_call();

    let cloned = stats.clone();
    assert_eq!(cloned.cache_hits(), 1);
    assert_eq!(cloned.api_calls(), 1);

    // Original and clone are independent
    stats.record_cache_hit();
    assert_eq!(stats.cache_hits(), 2);
    assert_eq!(cloned.cache_hits(), 1);
  }

  #[test]
  fn test_statistics_summary() {
    let stats = LoaderStatistics::new();
    stats.record_cache_hit();
    stats.record_api_call();
    stats.record_error();

    let summary: StatisticsSummary = (&stats).into();
    assert_eq!(summary.cache_hits, 1);
    assert_eq!(summary.api_calls, 1);
    assert_eq!(summary.errors, 1);
  }

  #[tokio::test]
  async fn test_concurrent_access() {
    let stats = LoaderStatistics::shared();

    let handles: Vec<_> = (0..10)
      .map(|_| {
        let stats = Arc::clone(&stats);
        tokio::spawn(async move {
          for _ in 0..100 {
            stats.record_api_call();
          }
        })
      })
      .collect();

    for handle in handles {
      handle.await.unwrap();
    }

    assert_eq!(stats.api_calls(), 1000);
  }
}
