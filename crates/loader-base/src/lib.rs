/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Shared base utilities for data loaders.
//!
//! This crate provides common abstractions to reduce code duplication across loader implementations:
//!
//! - [`CacheableConfig`]: Trait for configs with cache settings
//! - [`ConcurrentLoader`]: Semaphore-based concurrency management
//! - [`LoaderStatistics`]: Common statistics tracking (cache hits, API calls, errors)
//! - [`ProgressManager`]: Conditional progress bar creation and styling

mod concurrent;
mod config;
mod error;
mod progress;
mod statistics;

pub use concurrent::ConcurrentLoader;
pub use config::CacheableConfig;
pub use error::LoaderBaseError;
pub use progress::{ProgressManager, ProgressStyle};
pub use statistics::{LoaderStatistics, StatisticsSummary};