/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Concurrency management utilities for loaders.

use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::LoaderBaseError;

/// Manages concurrent request limits using a semaphore.
///
/// This provides a consistent way to limit concurrent API requests across loaders,
/// preventing rate limit violations and resource exhaustion.
///
/// # Example
///
/// ```ignore
/// use loader_base::ConcurrentLoader;
///
/// struct MyLoader {
///     concurrent: ConcurrentLoader,
///     // ... other fields
/// }
///
/// impl MyLoader {
///     pub fn new(max_concurrent: usize) -> Self {
///         Self {
///             concurrent: ConcurrentLoader::new(max_concurrent),
///         }
///     }
///
///     async fn fetch_item(&self, id: &str) -> Result<Data, Error> {
///         let _permit = self.concurrent.acquire().await?;
///         // Permit is held until _permit is dropped
///         self.do_fetch(id).await
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConcurrentLoader {
  semaphore: Arc<Semaphore>,
  max_concurrent: usize,
}

impl ConcurrentLoader {
  /// Create a new concurrent loader with the specified maximum concurrent requests.
  ///
  /// # Arguments
  ///
  /// * `max_concurrent` - Maximum number of concurrent requests allowed
  pub fn new(max_concurrent: usize) -> Self {
    Self { semaphore: Arc::new(Semaphore::new(max_concurrent)), max_concurrent }
  }

  /// Get the maximum concurrent requests setting.
  pub fn max_concurrent(&self) -> usize {
    self.max_concurrent
  }

  /// Update the concurrency limit.
  ///
  /// This creates a new semaphore with the new limit. Existing permits
  /// from the old semaphore will still be valid until dropped.
  pub fn set_max_concurrent(&mut self, max_concurrent: usize) {
    self.max_concurrent = max_concurrent;
    self.semaphore = Arc::new(Semaphore::new(max_concurrent));
  }

  /// Acquire a permit to make a concurrent request.
  ///
  /// This will wait if the maximum number of concurrent requests is reached.
  /// The permit is automatically released when dropped.
  ///
  /// # Errors
  ///
  /// Returns an error if the semaphore is closed (which should not happen
  /// during normal operation).
  pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, LoaderBaseError> {
    self.semaphore.acquire().await.map_err(|e| LoaderBaseError::PermitError(e.to_string()))
  }

  /// Get a clone of the inner semaphore for use in async closures.
  ///
  /// This is useful when you need to move the semaphore into an async block
  /// that will outlive the borrow.
  pub fn semaphore(&self) -> Arc<Semaphore> {
    Arc::clone(&self.semaphore)
  }
}

impl Default for ConcurrentLoader {
  fn default() -> Self {
    Self::new(5)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_concurrent_loader_creation() {
    let loader = ConcurrentLoader::new(10);
    assert_eq!(loader.max_concurrent(), 10);
  }

  #[tokio::test]
  async fn test_acquire_permit() {
    let loader = ConcurrentLoader::new(2);

    let permit1 = loader.acquire().await;
    assert!(permit1.is_ok());

    let permit2 = loader.acquire().await;
    assert!(permit2.is_ok());

    // Both permits acquired successfully
    drop(permit1);
    drop(permit2);
  }

  #[tokio::test]
  async fn test_set_max_concurrent() {
    let mut loader = ConcurrentLoader::new(5);
    assert_eq!(loader.max_concurrent(), 5);

    loader.set_max_concurrent(10);
    assert_eq!(loader.max_concurrent(), 10);
  }

  #[test]
  fn test_default() {
    let loader = ConcurrentLoader::default();
    assert_eq!(loader.max_concurrent(), 5);
  }
}