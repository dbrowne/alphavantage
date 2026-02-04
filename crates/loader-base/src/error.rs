/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Error types for loader-base operations.

use thiserror::Error;

/// Errors that can occur in loader base operations.
#[derive(Error, Debug)]
pub enum LoaderBaseError {
  /// Failed to acquire concurrency permit
  #[error("Failed to acquire concurrency permit: {0}")]
  PermitError(String),
}