/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Unified error type for the `av-api` facade.
//!
//! [`ApiError`] wraps the error types from every sub-crate behind feature
//! gates so that consumers only see the variants relevant to the features
//! they've enabled.

use thiserror::Error;

/// Unified error type spanning all `av-api` sub-crates.
///
/// Wraps the error types from `av-core`, `av-loaders`, and
/// `av-database-postgres` via `#[from]` for seamless `?` propagation.
/// Variants for disabled features are compiled out.
#[derive(Error, Debug)]
pub enum ApiError {
  /// An error from the core/client layer (config, HTTP, serde, rate limit).
  #[error(transparent)]
  Core(#[from] av_core::Error),

  /// A data-loader error (fetch, parse, batch processing).
  #[cfg(feature = "loaders")]
  #[error(transparent)]
  Loader(#[from] av_loaders::LoaderError),

  /// A database/repository error (pool, query, constraint, transaction).
  #[cfg(feature = "database")]
  #[error(transparent)]
  Repository(#[from] av_database_postgres::RepositoryError),

  /// A catch-all for errors that don't originate from a known sub-crate.
  #[error("{0}")]
  Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience alias for `std::result::Result<T, ApiError>`.
pub type Result<T> = std::result::Result<T, ApiError>;
