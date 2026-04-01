/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Type definitions for the crypto metadata loader.
//!
//! This module re-exports types from crypto-loaders and adds
//! additional types specific to av-loaders.

use super::CryptoDataSource;

// Re-export types from crypto-loaders for backward compatibility
pub use crypto_loaders::{
  CryptoMetadataConfig, CryptoMetadataOutput, CryptoSymbolForMetadata, MetadataSourceResult,
  ProcessedCryptoMetadata,
};

/// Input for crypto metadata loader
#[derive(Debug, Clone)]
pub struct CryptoMetadataInput {
  /// Specific symbols to process (if None, processes all crypto symbols)
  pub symbols: Option<Vec<CryptoSymbolForMetadata>>,

  /// Sources to use for metadata
  pub sources: Vec<CryptoDataSource>,

  /// Whether to update existing entries
  pub update_existing: bool,

  /// Maximum number of symbols to process (for testing)
  pub limit: Option<usize>,
}
