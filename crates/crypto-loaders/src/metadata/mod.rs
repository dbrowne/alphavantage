/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency metadata loading.
//!
//! This module provides types and providers for loading cryptocurrency
//! metadata from various sources.

pub mod coingecko_provider;
pub mod types;

pub use coingecko_provider::CoinGeckoMetadataProvider;
pub use types::{
  CryptoMetadataConfig, CryptoMetadataOutput, CryptoSymbolForMetadata, MetadataSourceResult,
  ProcessedCryptoMetadata,
};