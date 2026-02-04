/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Base traits and utilities for loader implementations.
//!
//! This module provides common abstractions to reduce code duplication across loaders:
//!
//! - [`CacheableConfig`]: Trait for configs with cache settings
//! - [`ConcurrentLoader`]: Semaphore-based concurrency management
//! - [`LoaderStatistics`]: Common statistics tracking (cache hits, API calls, errors)
//! - [`ProgressManager`]: Conditional progress bar creation and styling

mod concurrent;
mod config;
mod progress;
mod statistics;

pub use concurrent::ConcurrentLoader;
pub use config::CacheableConfig;
pub use progress::{ProgressManager, ProgressStyle as LoaderProgressStyle};
pub use statistics::{LoaderStatistics, StatisticsSummary};
