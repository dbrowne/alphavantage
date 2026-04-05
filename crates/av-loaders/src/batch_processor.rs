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

//! Batch processing utilities for efficient data loading

use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::{LoaderError, LoaderResult};

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
  /// Maximum number of items to process in a single batch
  pub batch_size: usize,

  /// Maximum number of concurrent batches
  pub max_concurrent_batches: usize,

  /// Whether to continue processing on errors
  pub continue_on_error: bool,

  /// Delay between batches in milliseconds
  pub batch_delay_ms: Option<u64>,
}

impl Default for BatchConfig {
  fn default() -> Self {
    Self {
      batch_size: 100,
      max_concurrent_batches: 5,
      continue_on_error: true,
      batch_delay_ms: Some(100),
    }
  }
}

/// Result of batch processing
#[derive(Debug, Clone)]
pub struct BatchResult<T> {
  /// Successfully processed items
  pub success: Vec<T>,

  /// Failed items with their errors
  pub failures: Vec<(usize, LoaderError)>,

  /// Total items processed
  pub total_processed: usize,
}

impl<T> Default for BatchResult<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> BatchResult<T> {
  pub fn new() -> Self {
    Self { success: Vec::new(), failures: Vec::new(), total_processed: 0 }
  }

  pub fn success_count(&self) -> usize {
    self.success.len()
  }

  pub fn failure_count(&self) -> usize {
    self.failures.len()
  }

  pub fn success_rate(&self) -> f64 {
    if self.total_processed == 0 {
      0.0
    } else {
      self.success_count() as f64 / self.total_processed as f64
    }
  }
}

/// Batch processor for efficient data processing
#[derive(Debug, Clone)]
pub struct BatchProcessor {
  config: BatchConfig,
  semaphore: Arc<Semaphore>,
}

impl BatchProcessor {
  pub fn new(config: BatchConfig) -> Self {
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_batches));
    Self { config, semaphore }
  }

  /// Process items in batches using indexed approach
  pub async fn process_batches<T, F, O>(
    &self,
    mut items: Vec<T>,
    processor: F,
  ) -> LoaderResult<BatchResult<O>>
  where
    T: Send + 'static,
    F:
      Fn(T) -> futures::future::BoxFuture<'static, LoaderResult<O>> + Send + Sync + Clone + 'static,
    O: Send + 'static,
  {
    let mut result = BatchResult::new();
    let total_items = items.len();
    result.total_processed = total_items;

    debug!("Processing {} items in batches of {}", total_items, self.config.batch_size);

    let mut batch_idx = 0;
    let total_batches = total_items.div_ceil(self.config.batch_size);

    // Process items in chunks by draining from the vector
    while !items.is_empty() {
      let batch_size = std::cmp::min(self.config.batch_size, items.len());
      let batch: Vec<T> = items.drain(..batch_size).collect();

      debug!("Processing batch {} of {}", batch_idx + 1, total_batches);

      let batch_results = self.process_single_batch(batch, processor.clone()).await?;

      // Aggregate results
      for (idx, batch_result) in batch_results.into_iter().enumerate() {
        let global_idx = batch_idx * self.config.batch_size + idx;
        match batch_result {
          Ok(output) => result.success.push(output),
          Err(e) => {
            warn!("Failed to process item {}: {}", global_idx, e);
            result.failures.push((global_idx, e));

            if !self.config.continue_on_error {
              return Err(LoaderError::BatchProcessingError(format!(
                "Batch processing failed at item {}",
                global_idx
              )));
            }
          }
        }
      }

      // Optional delay between batches
      if let Some(delay_ms) = self.config.batch_delay_ms {
        if !items.is_empty() {
          tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
      }

      batch_idx += 1;
    }

    debug!(
      "Batch processing complete: {} successes, {} failures",
      result.success_count(),
      result.failure_count()
    );

    Ok(result)
  }

  /// Process a single batch concurrently
  async fn process_single_batch<T, F, O>(
    &self,
    batch: Vec<T>,
    processor: F,
  ) -> LoaderResult<Vec<Result<O, LoaderError>>>
  where
    T: Send + 'static,
    F: Fn(T) -> futures::future::BoxFuture<'static, LoaderResult<O>> + Send + Sync + Clone,
    O: Send + 'static,
  {
    let semaphore = self.semaphore.clone();

    let results = stream::iter(batch)
      .map(move |item| {
        let processor = processor.clone();
        let semaphore = semaphore.clone();

        async move {
          let _permit = match semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
              return Err(LoaderError::BatchProcessingError(
                "Semaphore closed unexpectedly".to_string(),
              ));
            }
          };
          processor(item).await
        }
      })
      .buffer_unordered(self.config.max_concurrent_batches)
      .collect::<Vec<_>>()
      .await;

    Ok(results)
  }
}

/// Helper function to create batches from an iterator
pub fn create_batches<T>(items: impl Iterator<Item = T>, batch_size: usize) -> Vec<Vec<T>> {
  let mut batches = Vec::new();
  let mut current_batch = Vec::with_capacity(batch_size);

  for item in items {
    current_batch.push(item);
    if current_batch.len() >= batch_size {
      batches.push(std::mem::replace(&mut current_batch, Vec::with_capacity(batch_size)));
    }
  }

  if !current_batch.is_empty() {
    batches.push(current_batch);
  }

  batches
}

#[cfg(test)]
mod tests {
  use super::*;

  // BatchConfig tests
  #[test]
  fn test_batch_config_default() {
    let config = BatchConfig::default();
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.max_concurrent_batches, 5);
    assert!(config.continue_on_error);
    assert_eq!(config.batch_delay_ms, Some(100));
  }

  #[test]
  fn test_batch_config_custom() {
    let config = BatchConfig {
      batch_size: 50,
      max_concurrent_batches: 10,
      continue_on_error: false,
      batch_delay_ms: None,
    };
    assert_eq!(config.batch_size, 50);
    assert_eq!(config.max_concurrent_batches, 10);
    assert!(!config.continue_on_error);
    assert!(config.batch_delay_ms.is_none());
  }

  #[test]
  fn test_batch_config_clone() {
    let config = BatchConfig::default();
    let cloned = config.clone();
    assert_eq!(config.batch_size, cloned.batch_size);
  }

  #[test]
  fn test_batch_config_debug() {
    let config = BatchConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("BatchConfig"));
    assert!(debug_str.contains("batch_size"));
  }

  // BatchResult tests
  #[test]
  fn test_batch_result_new() {
    let result: BatchResult<i32> = BatchResult::new();
    assert!(result.success.is_empty());
    assert!(result.failures.is_empty());
    assert_eq!(result.total_processed, 0);
  }

  #[test]
  fn test_batch_result_default() {
    let result: BatchResult<String> = BatchResult::default();
    assert_eq!(result.success_count(), 0);
    assert_eq!(result.failure_count(), 0);
  }

  #[test]
  fn test_batch_result_success_count() {
    let mut result: BatchResult<i32> = BatchResult::new();
    result.success.push(1);
    result.success.push(2);
    result.success.push(3);
    assert_eq!(result.success_count(), 3);
  }

  #[test]
  fn test_batch_result_failure_count() {
    let mut result: BatchResult<i32> = BatchResult::new();
    result.failures.push((0, LoaderError::InvalidData("test".to_string())));
    result.failures.push((1, LoaderError::ApiError("fail".to_string())));
    assert_eq!(result.failure_count(), 2);
  }

  #[test]
  fn test_batch_result_success_rate_empty() {
    let result: BatchResult<i32> = BatchResult::new();
    assert_eq!(result.success_rate(), 0.0);
  }

  #[test]
  fn test_batch_result_success_rate_all_success() {
    let mut result: BatchResult<i32> = BatchResult::new();
    result.success = vec![1, 2, 3, 4, 5];
    result.total_processed = 5;
    assert_eq!(result.success_rate(), 1.0);
  }

  #[test]
  fn test_batch_result_success_rate_partial() {
    let mut result: BatchResult<i32> = BatchResult::new();
    result.success = vec![1, 2, 3];
    result.failures.push((3, LoaderError::InvalidData("test".to_string())));
    result.failures.push((4, LoaderError::InvalidData("test".to_string())));
    result.total_processed = 5;
    assert_eq!(result.success_rate(), 0.6);
  }

  #[test]
  fn test_batch_result_success_rate_all_failures() {
    let mut result: BatchResult<i32> = BatchResult::new();
    result.failures.push((0, LoaderError::InvalidData("test".to_string())));
    result.failures.push((1, LoaderError::InvalidData("test".to_string())));
    result.total_processed = 2;
    assert_eq!(result.success_rate(), 0.0);
  }

  // create_batches tests
  #[test]
  fn test_create_batches_empty() {
    let items: Vec<i32> = vec![];
    let batches = create_batches(items.into_iter(), 10);
    assert!(batches.is_empty());
  }

  #[test]
  fn test_create_batches_single_batch() {
    let items = vec![1, 2, 3, 4, 5];
    let batches = create_batches(items.into_iter(), 10);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], vec![1, 2, 3, 4, 5]);
  }

  #[test]
  fn test_create_batches_exact_fit() {
    let items = vec![1, 2, 3, 4, 5, 6];
    let batches = create_batches(items.into_iter(), 3);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], vec![1, 2, 3]);
    assert_eq!(batches[1], vec![4, 5, 6]);
  }

  #[test]
  fn test_create_batches_with_remainder() {
    let items = vec![1, 2, 3, 4, 5, 6, 7];
    let batches = create_batches(items.into_iter(), 3);
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0], vec![1, 2, 3]);
    assert_eq!(batches[1], vec![4, 5, 6]);
    assert_eq!(batches[2], vec![7]);
  }

  #[test]
  fn test_create_batches_batch_size_one() {
    let items = vec![1, 2, 3];
    let batches = create_batches(items.into_iter(), 1);
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0], vec![1]);
    assert_eq!(batches[1], vec![2]);
    assert_eq!(batches[2], vec![3]);
  }

  #[test]
  fn test_create_batches_large_batch_size() {
    let items = vec![1, 2, 3];
    let batches = create_batches(items.into_iter(), 1000);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], vec![1, 2, 3]);
  }

  // BatchProcessor tests
  #[test]
  fn test_batch_processor_new() {
    let config = BatchConfig::default();
    let processor = BatchProcessor::new(config.clone());
    assert_eq!(processor.config.batch_size, config.batch_size);
  }

  #[test]
  fn test_batch_processor_clone() {
    let config = BatchConfig::default();
    let processor = BatchProcessor::new(config);
    let cloned = processor.clone();
    assert_eq!(processor.config.batch_size, cloned.config.batch_size);
  }

  #[tokio::test]
  async fn test_batch_processor_process_empty() {
    let config = BatchConfig::default();
    let processor = BatchProcessor::new(config);

    let items: Vec<i32> = vec![];
    let result =
      processor.process_batches(items, |x| Box::pin(async move { Ok(x * 2) })).await.unwrap();

    assert_eq!(result.total_processed, 0);
    assert_eq!(result.success_count(), 0);
    assert_eq!(result.failure_count(), 0);
  }

  #[tokio::test]
  async fn test_batch_processor_process_all_success() {
    let config = BatchConfig { batch_size: 2, batch_delay_ms: None, ..BatchConfig::default() };
    let processor = BatchProcessor::new(config);

    let items = vec![1, 2, 3, 4, 5];
    let result =
      processor.process_batches(items, |x| Box::pin(async move { Ok(x * 2) })).await.unwrap();

    assert_eq!(result.total_processed, 5);
    assert_eq!(result.success_count(), 5);
    assert_eq!(result.failure_count(), 0);
    assert!(result.success.contains(&2));
    assert!(result.success.contains(&10));
  }

  #[tokio::test]
  async fn test_batch_processor_process_with_failures_continue() {
    let config = BatchConfig {
      batch_size: 2,
      continue_on_error: true,
      batch_delay_ms: None,
      ..BatchConfig::default()
    };
    let processor = BatchProcessor::new(config);

    let items = vec![1, 2, 3, 4, 5];
    let result = processor
      .process_batches(items, |x| {
        Box::pin(async move {
          if x == 3 { Err(LoaderError::InvalidData("three is bad".to_string())) } else { Ok(x * 2) }
        })
      })
      .await
      .unwrap();

    assert_eq!(result.total_processed, 5);
    assert_eq!(result.success_count(), 4);
    assert_eq!(result.failure_count(), 1);
  }

  #[tokio::test]
  async fn test_batch_processor_process_with_failures_stop() {
    let config = BatchConfig {
      batch_size: 10,
      continue_on_error: false,
      batch_delay_ms: None,
      ..BatchConfig::default()
    };
    let processor = BatchProcessor::new(config);

    let items = vec![1, 2, 3, 4, 5];
    let result = processor
      .process_batches(items, |x| {
        Box::pin(async move {
          if x == 3 { Err(LoaderError::InvalidData("three is bad".to_string())) } else { Ok(x * 2) }
        })
      })
      .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), LoaderError::BatchProcessingError(_)));
  }
}
