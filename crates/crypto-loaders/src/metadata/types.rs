/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Type definitions for cryptocurrency metadata loading.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::CryptoDataSource;

/// Configuration for crypto metadata loading.
#[derive(Debug, Clone)]
pub struct CryptoMetadataConfig {
  /// AlphaVantage API key
  pub alphavantage_api_key: Option<String>,

  /// CoinGecko API key for enhanced metadata
  pub coingecko_api_key: Option<String>,

  /// Delay between requests (ms)
  pub delay_ms: u64,

  /// Batch size for processing
  pub batch_size: usize,

  /// Maximum retries per symbol
  pub max_retries: usize,

  /// Timeout per request (seconds)
  pub timeout_seconds: u64,

  /// Whether to update existing metadata
  pub update_existing: bool,

  /// Whether to fetch enhanced metadata from CoinGecko
  pub fetch_enhanced_metadata: bool,

  /// Enable response caching to reduce API costs
  pub enable_response_cache: bool,

  /// Cache TTL in hours
  pub cache_ttl_hours: u32,

  /// Force refresh - ignore cache and fetch fresh data
  pub force_refresh: bool,
}

impl Default for CryptoMetadataConfig {
  fn default() -> Self {
    Self {
      alphavantage_api_key: None,
      coingecko_api_key: None,
      delay_ms: 1000,
      batch_size: 50,
      max_retries: 3,
      timeout_seconds: 30,
      update_existing: false,
      fetch_enhanced_metadata: true,
      enable_response_cache: true,
      cache_ttl_hours: 24,
      force_refresh: false,
    }
  }
}

/// Symbol information needed for metadata loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForMetadata {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub source: CryptoDataSource,
  pub source_id: String,
  pub is_active: bool,
}

/// Processed crypto metadata ready for database insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedCryptoMetadata {
  pub sid: i64,
  pub source: String,
  pub source_id: String,
  pub market_cap_rank: Option<i32>,
  pub base_currency: Option<String>,
  pub quote_currency: Option<String>,
  pub is_active: bool,
  pub additional_data: Option<Value>,
  pub last_updated: DateTime<Utc>,
}

/// Output from crypto metadata loader.
#[derive(Debug, Clone)]
pub struct CryptoMetadataOutput {
  pub metadata_processed: Vec<ProcessedCryptoMetadata>,
  pub symbols_processed: usize,
  pub symbols_failed: usize,
  pub processing_time_ms: u64,
  pub source_results: HashMap<CryptoDataSource, MetadataSourceResult>,
}

/// Results from a specific data source.
#[derive(Debug, Clone)]
pub struct MetadataSourceResult {
  pub symbols_processed: usize,
  pub symbols_failed: usize,
  pub errors: Vec<String>,
  pub rate_limited: bool,
}