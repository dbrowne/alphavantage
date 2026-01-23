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

//! Base traits and types for data loaders

use crate::{LoaderResult, ProcessTracker};
use async_trait::async_trait;
use av_client::AlphaVantageClient;
use av_database_postgres::repository::{CacheRepository, NewsRepository};
use std::sync::Arc;

/// Configuration for data loaders
#[derive(Debug, Clone)]
pub struct LoaderConfig {
  /// Maximum concurrent requests
  pub max_concurrent_requests: usize,

  /// Retry attempts for failed requests
  pub retry_attempts: u32,

  /// Delay between retries in milliseconds
  pub retry_delay_ms: u64,

  /// Enable progress tracking
  pub show_progress: bool,

  /// Enable process state tracking
  pub track_process: bool,

  /// Batch size for bulk operations
  pub batch_size: usize,
}

impl Default for LoaderConfig {
  fn default() -> Self {
    Self {
      max_concurrent_requests: 10,
      retry_attempts: 3,
      retry_delay_ms: 1000,
      show_progress: true,
      track_process: true,
      batch_size: 1000,
    }
  }
}

/// Shared context for all loaders
/// Now includes optional repositories to avoid direct database dependencies
pub struct LoaderContext {
  pub client: Arc<AlphaVantageClient>,
  pub config: LoaderConfig,
  pub process_tracker: Option<ProcessTracker>,
  /// Optional cache repository for API response caching
  pub cache_repository: Option<Arc<dyn CacheRepository>>,
  /// Optional news repository for symbol operations
  pub news_repository: Option<Arc<dyn NewsRepository>>,
}

impl LoaderContext {
  pub fn new(client: Arc<AlphaVantageClient>, config: LoaderConfig) -> Self {
    Self { client, config, process_tracker: None, cache_repository: None, news_repository: None }
  }

  pub fn with_process_tracker(mut self, tracker: ProcessTracker) -> Self {
    self.process_tracker = Some(tracker);
    self
  }

  pub fn with_cache_repository(mut self, cache_repo: Arc<dyn CacheRepository>) -> Self {
    self.cache_repository = Some(cache_repo);
    self
  }

  pub fn with_news_repository(mut self, news_repo: Arc<dyn NewsRepository>) -> Self {
    self.news_repository = Some(news_repo);
    self
  }
}

/// Base trait for all data loaders
#[async_trait]
pub trait DataLoader: Send + Sync {
  /// The type of data this loader processes
  type Input;

  /// The result type after loading
  type Output;

  /// Load data from the given input
  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output>;

  /// Validate input before loading
  async fn validate_input(&self, _input: &Self::Input) -> LoaderResult<()> {
    Ok(())
  }

  /// Get loader name for logging/tracking
  fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_loader_config_default() {
    let config = LoaderConfig::default();
    assert_eq!(config.max_concurrent_requests, 10);
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay_ms, 1000);
    assert!(config.show_progress);
    assert!(config.track_process);
    assert_eq!(config.batch_size, 1000);
  }

  #[test]
  fn test_loader_config_custom() {
    let config = LoaderConfig {
      max_concurrent_requests: 5,
      retry_attempts: 5,
      retry_delay_ms: 2000,
      show_progress: false,
      track_process: false,
      batch_size: 500,
    };
    assert_eq!(config.max_concurrent_requests, 5);
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.retry_delay_ms, 2000);
    assert!(!config.show_progress);
    assert!(!config.track_process);
    assert_eq!(config.batch_size, 500);
  }

  #[test]
  fn test_loader_config_clone() {
    let config = LoaderConfig::default();
    let cloned = config.clone();
    assert_eq!(config.max_concurrent_requests, cloned.max_concurrent_requests);
    assert_eq!(config.batch_size, cloned.batch_size);
  }

  #[test]
  fn test_loader_config_debug() {
    let config = LoaderConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("LoaderConfig"));
    assert!(debug_str.contains("max_concurrent_requests"));
  }

  #[test]
  fn test_loader_context_new() {
    let av_config = av_core::Config::default_with_key("test_key".to_string());
    let client = Arc::new(AlphaVantageClient::new(av_config).expect("Failed to create client"));
    let loader_config = LoaderConfig::default();

    let context = LoaderContext::new(client.clone(), loader_config);

    assert!(context.process_tracker.is_none());
    assert!(context.cache_repository.is_none());
    assert!(context.news_repository.is_none());
    assert_eq!(context.config.batch_size, 1000);
  }

  #[test]
  fn test_loader_context_with_process_tracker() {
    let av_config = av_core::Config::default_with_key("test_key".to_string());
    let client = Arc::new(AlphaVantageClient::new(av_config).expect("Failed to create client"));
    let loader_config = LoaderConfig::default();
    let tracker = ProcessTracker::new();

    let context = LoaderContext::new(client.clone(), loader_config).with_process_tracker(tracker);

    assert!(context.process_tracker.is_some());
  }

  #[test]
  fn test_loader_context_builder_chain() {
    let av_config = av_core::Config::default_with_key("test_key".to_string());
    let client = Arc::new(AlphaVantageClient::new(av_config).expect("Failed to create client"));
    let loader_config = LoaderConfig { batch_size: 50, ..LoaderConfig::default() };
    let tracker = ProcessTracker::new();

    let context = LoaderContext::new(client, loader_config).with_process_tracker(tracker);

    assert!(context.process_tracker.is_some());
    assert_eq!(context.config.batch_size, 50);
  }
}
