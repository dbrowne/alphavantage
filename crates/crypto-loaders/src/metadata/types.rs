/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Type definitions for cryptocurrency metadata loading.
//!
//! This module defines the configuration, input, output, and result types
//! used by the metadata loading pipeline. These types are consumed by
//! [`CoinGeckoMetadataProvider`](super::coingecko_provider::CoinGeckoMetadataProvider)
//! and any future metadata providers.
//!
//! # Type roles
//!
//! ```text
//! CryptoMetadataConfig        ← controls the loader's behavior
//! CryptoSymbolForMetadata     ← input: which coins to load
//! ProcessedCryptoMetadata     ← output: one coin's normalized metadata
//! CryptoMetadataOutput        ← batch output: all results + stats
//! MetadataSourceResult        ← per-source success/failure counters
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::CryptoDataSource;

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for the cryptocurrency metadata loading pipeline.
///
/// Controls API credentials, rate limiting, batching, caching, and
/// retry behavior.
///
/// # Defaults
///
/// | Field                    | Default  | Description                              |
/// |--------------------------|----------|------------------------------------------|
/// | `alphavantage_api_key`   | `None`   | Alpha Vantage API key (optional)         |
/// | `coingecko_api_key`      | `None`   | CoinGecko Pro API key (optional)         |
/// | `delay_ms`               | `1000`   | Delay between requests (ms)              |
/// | `batch_size`             | `50`     | Symbols per batch                        |
/// | `max_retries`            | `3`      | Retries per symbol on transient failure  |
/// | `timeout_seconds`        | `30`     | HTTP request timeout                     |
/// | `update_existing`        | `false`  | Overwrite existing metadata?             |
/// | `fetch_enhanced_metadata`| `true`   | Fetch from CoinGecko in addition to AV?  |
/// | `enable_response_cache`  | `true`   | Cache raw API responses?                 |
/// | `cache_ttl_hours`        | `24`     | Cache time-to-live in hours              |
/// | `force_refresh`          | `false`  | Ignore cache, always fetch fresh data    |
#[derive(Debug, Clone)]
pub struct CryptoMetadataConfig {
  /// Alpha Vantage API key (optional — not all metadata requires it).
  pub alphavantage_api_key: Option<String>,

  /// CoinGecko API key. When set, uses the Pro endpoint for higher rate limits.
  pub coingecko_api_key: Option<String>,

  /// Delay in milliseconds between consecutive API requests.
  pub delay_ms: u64,

  /// Number of symbols to process per batch.
  pub batch_size: usize,

  /// Maximum retry attempts per symbol on transient failures.
  pub max_retries: usize,

  /// HTTP request timeout in seconds.
  pub timeout_seconds: u64,

  /// When `true`, overwrites existing metadata; when `false`, skips
  /// symbols that already have metadata.
  pub update_existing: bool,

  /// When `true`, fetches enhanced metadata from CoinGecko in addition
  /// to any Alpha Vantage data.
  pub fetch_enhanced_metadata: bool,

  /// When `true`, raw API responses are cached to reduce API costs
  /// on repeated runs.
  pub enable_response_cache: bool,

  /// Time-to-live for cached responses in hours.
  pub cache_ttl_hours: u32,

  /// When `true`, ignores cached data and always fetches fresh from the API.
  /// Takes precedence over `enable_response_cache`.
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

// ─── Input ──────────────────────────────────────────────────────────────────

/// Input DTO specifying which coin to load metadata for.
///
/// Carries the internal security ID (`sid`), human-readable identifiers
/// (`symbol`, `name`), the external data source, the provider-specific
/// coin ID (`source_id`), and whether the coin is currently active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForMetadata {
  /// Internal security ID (FK to `symbols` table).
  pub sid: i64,
  /// Ticker symbol (e.g., `"BTC"`).
  pub symbol: String,
  /// Full coin name (e.g., `"Bitcoin"`).
  pub name: String,
  /// Which external provider this metadata request targets.
  pub source: CryptoDataSource,
  /// Provider-specific coin ID (e.g., CoinGecko slug `"bitcoin"`).
  pub source_id: String,
  /// Whether the coin is currently actively traded.
  pub is_active: bool,
}

// ─── Output ─────────────────────────────────────────────────────────────────

/// Normalized metadata for a single cryptocurrency, ready for database insertion.
///
/// Produced by [`CoinGeckoMetadataProvider::process_response`](super::coingecko_provider::CoinGeckoMetadataProvider).
/// Maps directly to the `crypto_metadata` database table.
///
/// `additional_data` is an optional JSON object containing provider-specific
/// details (description, links, market data, categories) that don't have
/// dedicated columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedCryptoMetadata {
  /// Internal security ID.
  pub sid: i64,
  /// Source provider name (e.g., `"coingecko"`).
  pub source: String,
  /// Provider-specific coin ID.
  pub source_id: String,
  /// Global market-cap rank (if available).
  pub market_cap_rank: Option<i32>,
  /// Base currency of the coin (typically the symbol itself).
  pub base_currency: Option<String>,
  /// Quote currency for pricing (typically `"USD"`).
  pub quote_currency: Option<String>,
  /// Whether the coin is currently active.
  pub is_active: bool,
  /// Provider-specific extra data as a JSON object.
  pub additional_data: Option<Value>,
  /// When this metadata was last fetched.
  pub last_updated: DateTime<Utc>,
}

/// Aggregated result from a metadata loading run across all symbols.
///
/// Contains the processed metadata records, counts, timing, and
/// per-source breakdowns.
#[derive(Debug, Clone)]
pub struct CryptoMetadataOutput {
  /// Successfully processed metadata records.
  pub metadata_processed: Vec<ProcessedCryptoMetadata>,
  /// Total symbols attempted.
  pub symbols_processed: usize,
  /// Symbols that failed to load.
  pub symbols_failed: usize,
  /// Total wall-clock time in milliseconds.
  pub processing_time_ms: u64,
  /// Per-source success/failure breakdown.
  pub source_results: HashMap<CryptoDataSource, MetadataSourceResult>,
}

/// Per-source result from a metadata loading run.
#[derive(Debug, Clone)]
pub struct MetadataSourceResult {
  /// Symbols successfully processed from this source.
  pub symbols_processed: usize,
  /// Symbols that failed from this source.
  pub symbols_failed: usize,
  /// Error messages collected during loading.
  pub errors: Vec<String>,
  /// Whether this source hit a rate limit during the run.
  pub rate_limited: bool,
}
