/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

//! Core types for cryptocurrency data loading.
//!
//! This module defines the foundational types shared across all providers,
//! loaders, and mapping services in the `crypto-loaders` crate.
//!
//! # Type inventory
//!
//! | Type                 | Purpose                                              |
//! |----------------------|------------------------------------------------------|
//! | [`CryptoSymbol`]     | A cryptocurrency with metadata from one provider     |
//! | [`CryptoDataSource`] | Enum of the 5 supported data providers               |
//! | [`CryptoLoaderConfig`] | Configuration for the multi-provider symbol loader |
//! | [`CryptoLoaderResult`] | Aggregated result counters + per-source breakdown  |
//! | [`SourceResult`]     | Per-source fetch statistics                          |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── CryptoSymbol ───────────────────────────────────────────────────────────

/// A cryptocurrency symbol with metadata from a single data provider.
///
/// Produced by [`CryptoDataProvider::fetch_symbols`](crate::traits::CryptoDataProvider::fetch_symbols)
/// and consumed by [`CryptoSymbolLoader`](crate::loaders::CryptoSymbolLoader)
/// for deduplication and merging.
///
/// # Fields
///
/// | Field              | Type                          | Description                                    |
/// |--------------------|-------------------------------|------------------------------------------------|
/// | `symbol`           | `String`                      | Uppercase ticker (e.g., `"BTC"`)               |
/// | `priority`         | `i32`                         | Sort priority (lower = higher; 9999999 = default) |
/// | `name`             | `String`                      | Full coin name (e.g., `"Bitcoin"`)             |
/// | `base_currency`    | `Option<String>`              | Base currency (if applicable)                  |
/// | `quote_currency`   | `Option<String>`              | Quote currency (typically `"USD"`)             |
/// | `market_cap_rank`  | `Option<u32>`                 | Global rank by market cap (if known)           |
/// | `source`           | [`CryptoDataSource`]          | Which provider produced this record            |
/// | `source_id`        | `String`                      | Provider-specific ID (e.g., CoinGecko slug)    |
/// | `is_active`        | `bool`                        | Whether the coin is actively traded            |
/// | `created_at`       | `DateTime<Utc>`               | When this record was fetched                   |
/// | `additional_data`  | `HashMap<String, Value>`      | Provider-specific extra fields (tags, platform, etc.) |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbol {
  /// Uppercase ticker symbol (e.g., `"BTC"`, `"ETH"`).
  pub symbol: String,
  /// Ingestion priority — lower values are processed first.
  /// Defaults to `9999999` for unranked coins.
  pub priority: i32,
  /// Full coin/project name.
  pub name: String,
  /// Base currency of the trading pair (if applicable).
  pub base_currency: Option<String>,
  /// Quote currency (typically `"USD"`).
  pub quote_currency: Option<String>,
  /// Market-capitalization rank. `None` if the provider doesn't supply it.
  pub market_cap_rank: Option<u32>,
  /// Which data provider produced this record.
  pub source: CryptoDataSource,
  /// Provider-specific coin identifier (e.g., CoinGecko slug `"bitcoin"`).
  pub source_id: String,
  /// Whether the coin is currently actively traded.
  pub is_active: bool,
  /// Timestamp when this record was fetched.
  pub created_at: DateTime<Utc>,
  /// Provider-specific extra data (tags, platform info, etc.).
  pub additional_data: HashMap<String, serde_json::Value>,
}

// ─── CryptoDataSource ───────────────────────────────────────────────────────

/// Identifies one of the 5 supported cryptocurrency data providers.
///
/// Used as a key in `HashMap`s (derives `Hash`, `Eq`), as a discriminant
/// in [`CryptoSymbol::source`], and in [`CryptoLoaderConfig::sources`] to
/// select which providers to query.
///
/// # Display output
///
/// `Display` produces the lowercase provider slug (e.g., `"coingecko"`,
/// `"coinmarketcap"`), suitable for cache keys and log messages.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CryptoDataSource {
  /// CoinMarketCap — requires API key.
  CoinMarketCap,
  /// CoinGecko — requires API key (Pro or Demo).
  CoinGecko,
  /// CoinPaprika — free public API.
  CoinPaprika,
  /// CoinCap — free public API.
  CoinCap,
  /// SosoValue — requires API key.
  SosoValue,
}

/// Formats as the lowercase provider slug.
impl std::fmt::Display for CryptoDataSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CryptoDataSource::CoinMarketCap => write!(f, "coinmarketcap"),
      CryptoDataSource::CoinGecko => write!(f, "coingecko"),
      CryptoDataSource::CoinPaprika => write!(f, "coinpaprika"),
      CryptoDataSource::CoinCap => write!(f, "coincap"),
      CryptoDataSource::SosoValue => write!(f, "sosovalue"),
    }
  }
}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for [`CryptoSymbolLoader`](crate::loaders::CryptoSymbolLoader).
///
/// Controls concurrency, retries, rate limiting, progress display, provider
/// selection, and caching.
///
/// # Defaults
///
/// | Field                    | Default                                                   |
/// |--------------------------|-----------------------------------------------------------|
/// | `max_concurrent_requests`| `10`                                                      |
/// | `retry_attempts`         | `3`                                                       |
/// | `retry_delay_ms`         | `1000`                                                    |
/// | `rate_limit_delay_ms`    | `200`                                                     |
/// | `enable_progress_bar`    | `true`                                                    |
/// | `sources`                | All 5 (CoinGecko, CoinPaprika, CoinCap, SosoValue, CoinMarketCap) |
/// | `batch_size`             | `250`                                                     |
/// | `cache_ttl_hours`        | `Some(24)`                                                |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderConfig {
  /// Maximum concurrent API requests.
  pub max_concurrent_requests: usize,
  /// Number of retry attempts on transient failure.
  pub retry_attempts: u32,
  /// Delay between retries in milliseconds.
  pub retry_delay_ms: u64,
  /// Delay between rate-limited requests in milliseconds.
  pub rate_limit_delay_ms: u64,
  /// Whether to show a terminal progress bar during loading.
  pub enable_progress_bar: bool,
  /// Which providers to query (order determines processing sequence).
  pub sources: Vec<CryptoDataSource>,
  /// Number of symbols per processing batch.
  pub batch_size: usize,
  /// Cache TTL in hours. `None` disables caching.
  pub cache_ttl_hours: Option<u64>,
}

impl Default for CryptoLoaderConfig {
  fn default() -> Self {
    Self {
      max_concurrent_requests: 10,
      retry_attempts: 3,
      retry_delay_ms: 1000,
      rate_limit_delay_ms: 200,
      enable_progress_bar: true,
      sources: vec![
        CryptoDataSource::CoinGecko,
        CryptoDataSource::CoinPaprika,
        CryptoDataSource::CoinCap,
        CryptoDataSource::SosoValue,
        CryptoDataSource::CoinMarketCap,
      ],
      batch_size: 250,
      cache_ttl_hours: Some(24),
    }
  }
}

// ─── Result types ───────────────────────────────────────────────────────────

/// Aggregated result counters from a crypto loading operation.
///
/// Note: this is a **data struct**, not a `Result` type — it's named
/// `CryptoLoaderResult` for legacy reasons. The actual error type is
/// [`CryptoLoaderError`](crate::CryptoLoaderError).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderResult {
  /// Unique symbols successfully loaded (after deduplication).
  pub symbols_loaded: usize,
  /// Number of source failures.
  pub symbols_failed: usize,
  /// Duplicates removed during deduplication.
  pub symbols_skipped: usize,
  /// Per-source breakdown.
  pub source_results: HashMap<CryptoDataSource, SourceResult>,
  /// Total wall-clock time in milliseconds.
  pub processing_time_ms: u64,
}

/// Per-source fetch statistics from a loading operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
  /// Number of symbols fetched from this source.
  pub symbols_fetched: usize,
  /// Error messages encountered (empty on success).
  pub errors: Vec<String>,
  /// Whether this source hit a rate limit during the operation.
  pub rate_limited: bool,
  /// Round-trip time for this source's fetch in milliseconds.
  pub response_time_ms: u64,
}
