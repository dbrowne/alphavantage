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
    let total_batches = (total_items + self.config.batch_size - 1) / self.config.batch_size;

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
          let _permit = semaphore.acquire().await.unwrap();
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
