/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Base traits and utilities for loader implementations.
//!
//! This module re-exports types from the `loader-base` crate to provide common
//! abstractions that reduce code duplication across loaders:
//!
//! - [`CacheableConfig`]: Trait for configs with cache settings
//! - [`ConcurrentLoader`]: Semaphore-based concurrency management
//! - [`LoaderStatistics`]: Common statistics tracking (cache hits, API calls, errors)
//! - [`ProgressManager`]: Conditional progress bar creation and styling

// Re-export everything from loader-base
pub use loader_base::CacheableConfig;
pub use loader_base::ConcurrentLoader;
pub use loader_base::LoaderStatistics;
pub use loader_base::ProgressManager;
pub use loader_base::ProgressStyle as LoaderProgressStyle;
pub use loader_base::StatisticsSummary;
