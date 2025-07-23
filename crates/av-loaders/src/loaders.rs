//! Base traits and types for data loaders
//!
//! This module defines:
//! - `DataLoader` trait: Common interface for all loaders
//! - `LoaderConfig`: Configuration options for loaders
//! - `LoaderContext`: Shared resources for loader operations
//!
//! All loaders in this crate implement the `DataLoader` trait,
//! providing a consistent interface for loading different data types.

use async_trait::async_trait;
use std::sync::Arc;
use av_client::Client;
use av_database::connection::DbPool;
use crate::{LoaderResult, ProcessTracker};

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

    /// Enable process state tracking in database
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
pub struct LoaderContext {
    pub client: Arc<Client>,
    pub db_pool: Arc<DbPool>,
    pub config: LoaderConfig,
    pub process_tracker: Option<ProcessTracker>,
}

impl LoaderContext {
    pub fn new(
        client: Arc<Client>,
        db_pool: Arc<DbPool>,
        config: LoaderConfig,
    ) -> Self {
        Self {
            client,
            db_pool,
            config,
            process_tracker: None,
        }
    }

    pub fn with_process_tracker(mut self, tracker: ProcessTracker) -> Self {
        self.process_tracker = Some(tracker);
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
    async fn load(
        &self,
        context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output>;

    /// Validate input before loading
    async fn validate_input(
        &self,
        input: &Self::Input,
    ) -> LoaderResult<()> {
        Ok(())
    }

    /// Get loader name for logging/tracking
    fn name(&self) -> &'static str;
}