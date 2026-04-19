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

//! Cryptocurrency symbol loader.
//!
//! Loads cryptocurrency symbols from up to 5 external data providers,
//! deduplicates the results, and returns a unified symbol list.
//!
//! # Supported providers
//!
//! | Provider       | Env var for API key    | Key required? | Priority |
//! |----------------|-----------------------|---------------|----------|
//! | CoinGecko      | `COINGECKO_API_KEY`   | Yes           | 1 (highest) |
//! | CoinMarketCap  | `CMC_API_KEY`         | Yes           | 2        |
//! | SosoValue      | `SOSOVALUE_API_KEY`   | Yes           | 3        |
//! | CoinPaprika    | —                     | No (free)     | 4        |
//! | CoinCap        | —                     | No (free)     | 5 (lowest) |
//!
//! Providers that require API keys are **silently skipped** if the
//! corresponding environment variable is not set (with a `warn!` log).
//!
//! # Deduplication strategy
//!
//! When the same ticker symbol appears from multiple sources, the loader
//! keeps the "best" record using this priority:
//!
//! 1. Prefer the record that has a `market_cap_rank` over one that doesn't.
//! 2. Among records with ranks, prefer the lower (better) rank.
//! 3. As a tiebreaker, prefer the source with higher priority (see table above).
//!
//! # Caching
//!
//! When a [`CryptoCache`] is provided via [`with_cache`](CryptoSymbolLoader::with_cache),
//! fetched symbol lists are cached with a configurable TTL (default 24h).
//! Cache keys follow the format `crypto_symbols_{source}`.
//!
//! # Data flow
//!
//! ```text
//! CryptoSymbolLoader::load_all_symbols()
//!   ├── for each source (sequential, respecting rate limits):
//!   │     ├── check cache → hit? return cached symbols
//!   │     └── miss → provider.fetch_symbols() → cache result
//!   ├── collect all symbols
//!   ├── deduplicate_symbols() → keep best per ticker
//!   └── return LoadAllSymbolsResult
//! ```

use crate::error::CryptoLoaderError;
use crate::providers::{
  CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider, SosoValueProvider,
};
use crate::traits::{CryptoCache, CryptoDataProvider};
use crate::types::{CryptoDataSource, CryptoLoaderConfig, CryptoSymbol, SourceResult};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

/// Source priority constants (lower = higher priority).
const COINGECKO: u8 = 1;
const COINMARKETCAP: u8 = 2;
const SOSOVALUE: u8 = 3;
const COINPAPRIKA: u8 = 4;
const COINCAP: u8 = 5;

// ─── Loader ─────────────────────────────────────────────────────────────────

/// Multi-provider cryptocurrency symbol loader.
///
/// Constructed via [`new`](Self::new) from a [`CryptoLoaderConfig`],
/// optionally configured with [`with_api_keys`](Self::with_api_keys) and
/// [`with_cache`](Self::with_cache), then invoked via
/// [`load_all_symbols`](Self::load_all_symbols).
///
/// Provider instances are created during construction based on the
/// `config.sources` list and available environment variables.
pub struct CryptoSymbolLoader {
  config: CryptoLoaderConfig,
  client: Client,
  providers: HashMap<CryptoDataSource, Box<dyn CryptoDataProvider + Send + Sync>>,
  cache: Option<Arc<dyn CryptoCache>>,
}

impl CryptoSymbolLoader {
  /// Creates a new loader, initializing providers based on `config.sources`
  /// and available environment variables.
  ///
  /// Providers requiring API keys are silently skipped if the env var is
  /// not set. The HTTP client uses a 30-second timeout.
  pub fn new(config: CryptoLoaderConfig) -> Self {
    let client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .user_agent("CryptoLoaders-Rust/1.0")
      .build()
      .expect("Failed to create HTTP client");

    let mut providers: HashMap<CryptoDataSource, Box<dyn CryptoDataProvider + Send + Sync>> =
      HashMap::new();

    // Initialize providers based on config
    for source in &config.sources {
      match source {
        CryptoDataSource::CoinMarketCap => {
          if let Ok(api_key) = std::env::var("CMC_API_KEY") {
            providers.insert(
              CryptoDataSource::CoinMarketCap,
              Box::new(CoinMarketCapProvider::new(api_key)),
            );
          } else {
            warn!("CoinMarketCap provider skipped: CMC_API_KEY environment variable not set");
          }
        }
        CryptoDataSource::CoinGecko => {
          if let Ok(api_key) = std::env::var("COINGECKO_API_KEY") {
            providers
              .insert(CryptoDataSource::CoinGecko, Box::new(CoinGeckoProvider::new(Some(api_key))));
          } else {
            warn!("CoinGecko provider skipped: COINGECKO_API_KEY environment variable not set");
          }
        }
        CryptoDataSource::CoinPaprika => {
          providers.insert(CryptoDataSource::CoinPaprika, Box::new(CoinPaprikaProvider));
        }
        CryptoDataSource::CoinCap => {
          providers.insert(CryptoDataSource::CoinCap, Box::new(CoinCapProvider));
        }
        CryptoDataSource::SosoValue => {
          if let Ok(api_key) = std::env::var("SOSOVALUE_API_KEY") {
            providers
              .insert(CryptoDataSource::SosoValue, Box::new(SosoValueProvider::new(Some(api_key))));
          } else {
            warn!("SosoValue provider skipped: SOSOVALUE_API_KEY environment variable not set");
          }
        }
      }
    }

    Self { config, client, providers, cache: None }
  }

  /// Overrides API keys for specific providers. Builder pattern.
  ///
  /// Replaces the provider instance for each source in the map. Sources
  /// that don't support API keys (CoinPaprika, CoinCap) log a warning.
  pub fn with_api_keys(mut self, api_keys: HashMap<CryptoDataSource, String>) -> Self {
    for (source, api_key) in api_keys {
      match source {
        CryptoDataSource::CoinGecko => {
          self.providers.insert(source, Box::new(CoinGeckoProvider::new(Some(api_key))));
        }
        CryptoDataSource::CoinMarketCap => {
          self.providers.insert(source, Box::new(CoinMarketCapProvider::new(api_key)));
        }
        CryptoDataSource::SosoValue => {
          self.providers.insert(source, Box::new(SosoValueProvider::new(Some(api_key))));
        }
        _ => {
          warn!("API key provided for source that doesn't support it: {}", source);
        }
      }
    }
    self
  }

  /// Enables response caching with the given cache implementation. Builder pattern.
  ///
  /// Cache TTL defaults to 24 hours (overridden by `config.cache_ttl_hours`).
  pub fn with_cache(mut self, cache: Arc<dyn CryptoCache>) -> Self {
    self.cache = Some(cache);
    self
  }

  /// Loads symbols from all configured sources, deduplicates, and returns
  /// a [`LoadAllSymbolsResult`].
  ///
  /// Sources are queried **sequentially** to respect rate limits. Each source
  /// checks the cache first; on a miss, calls the provider API and caches
  /// the result. A progress bar is shown when `config.enable_progress_bar`
  /// is `true`.
  pub async fn load_all_symbols(&self) -> Result<LoadAllSymbolsResult, CryptoLoaderError> {
    let start_time = Instant::now();
    info!("Loading symbols from {} sources", self.config.sources.len());

    let mut all_symbols = Vec::new();
    let mut source_results = HashMap::new();
    let mut total_failed = 0;
    let mut total_loaded = 0;

    // Show progress bar if enabled
    let progress = if self.config.enable_progress_bar {
      let pb = ProgressBar::new(self.config.sources.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
          .unwrap()
          .progress_chars("#>-"),
      );
      pb.set_message("Loading crypto symbols");
      Some(pb)
    } else {
      None
    };

    // Load from each source sequentially to respect rate limits
    for source in &self.config.sources {
      let start = Instant::now();

      if let Some(pb) = &progress {
        pb.set_message(format!("Loading from {}", source));
      }

      let response_time = start.elapsed().as_millis() as u64;

      match self.load_from_source(*source).await {
        Ok(symbols) => {
          info!("Successfully loaded {} symbols from {}", symbols.len(), source);
          total_loaded += symbols.len();
          all_symbols.extend(symbols.clone());

          source_results.insert(
            *source,
            SourceResult {
              symbols_fetched: symbols.len(),
              errors: vec![],
              rate_limited: false,
              response_time_ms: response_time,
            },
          );
        }
        Err(e) => {
          error!("Failed to load symbols from {}: {}", source, e);
          total_failed += 1;

          let rate_limited = matches!(e, CryptoLoaderError::RateLimitExceeded(_));
          source_results.insert(
            *source,
            SourceResult {
              symbols_fetched: 0,
              errors: vec![e.to_string()],
              rate_limited,
              response_time_ms: response_time,
            },
          );
        }
      }

      if let Some(pb) = &progress {
        pb.inc(1);
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Symbol loading complete");
    }

    // Deduplicate symbols by symbol+source combination
    let unique_symbols = self.deduplicate_symbols(all_symbols);
    let final_count = unique_symbols.len();
    let processing_time = start_time.elapsed().as_millis() as u64;

    info!(
      "Crypto symbol loading complete: {} unique symbols loaded, {} sources failed in {}ms",
      final_count, total_failed, processing_time
    );

    Ok(LoadAllSymbolsResult {
      symbols_loaded: final_count,
      symbols_failed: total_failed,
      symbols_skipped: total_loaded - final_count,
      symbols: unique_symbols,
      source_results,
      processing_time_ms: processing_time,
    })
  }

  /// Deduplicates symbols by ticker, keeping the "best" record.
  ///
  /// Priority rules:
  /// 1. Prefer records with a `market_cap_rank` over those without.
  /// 2. Among records with ranks, prefer the lower (better) rank.
  /// 3. Tiebreaker: prefer the source with higher priority (CoinGecko > … > CoinCap).
  fn deduplicate_symbols(&self, symbols: Vec<CryptoSymbol>) -> Vec<CryptoSymbol> {
    let mut unique_symbols: HashMap<String, CryptoSymbol> = HashMap::new();

    let source_priority = |source: &CryptoDataSource| -> u8 {
      match source {
        CryptoDataSource::CoinGecko => COINGECKO,
        CryptoDataSource::CoinMarketCap => COINMARKETCAP,
        CryptoDataSource::SosoValue => SOSOVALUE,
        CryptoDataSource::CoinPaprika => COINPAPRIKA,
        CryptoDataSource::CoinCap => COINCAP,
      }
    };

    for symbol in symbols {
      let key = symbol.symbol.clone();

      match unique_symbols.get(&key) {
        Some(existing) => {
          let should_replace = match (&symbol.market_cap_rank, &existing.market_cap_rank) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(new_rank), Some(existing_rank)) => new_rank < existing_rank,
            (None, None) => source_priority(&symbol.source) > source_priority(&existing.source),
          };

          if should_replace {
            unique_symbols.insert(key, symbol);
          }
        }
        None => {
          unique_symbols.insert(key, symbol);
        }
      }
    }

    unique_symbols.into_values().collect()
  }

  /// Loads symbols from a single source, using the cache when available.
  ///
  /// Returns [`CryptoLoaderError::SourceUnavailable`] if no provider is
  /// configured for the given source.
  pub async fn load_from_source(
    &self,
    source: CryptoDataSource,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    if let Some(provider) = self.providers.get(&source) {
      let cache_key = format!("crypto_symbols_{}", source);

      // Check cache first if available
      if let Some(cache) = &self.cache {
        info!("Checking cache for {} (key: {})", source, cache_key);
        match cache.get("crypto_loader", &cache_key).await {
          Ok(Some(cached_data)) => match serde_json::from_str::<Vec<CryptoSymbol>>(&cached_data) {
            Ok(symbols) => {
              info!("Cache hit for {}: {} symbols loaded from cache", source, symbols.len());
              return Ok(symbols);
            }
            Err(e) => {
              warn!("Failed to deserialize cached data for {}: {}", source, e);
            }
          },
          Ok(None) => {
            info!("Cache miss for {} - no cached data found", source);
          }
          Err(e) => {
            warn!("Cache read error for {}: {}", source, e);
          }
        }
      } else {
        warn!("No cache configured for {} - caching disabled", source);
      }

      // Cache miss or no cache - fetch from API
      info!("Loading symbols from {} API", source);
      let cache_ref = self.cache.as_ref();
      let symbols = provider.fetch_symbols(&self.client, cache_ref).await?;
      info!("Fetched {} symbols from {}", symbols.len(), source);

      // Cache the result if cache is available
      if let Some(cache) = &self.cache {
        match serde_json::to_string(&symbols) {
          Ok(json_data) => {
            let cache_ttl_hours = self.config.cache_ttl_hours.unwrap_or(24) as u32;

            match cache.set("crypto_loader", &cache_key, &json_data, cache_ttl_hours).await {
              Ok(()) => {
                info!(
                  "Cached {} symbols for {} (TTL: {}h)",
                  symbols.len(),
                  source,
                  cache_ttl_hours
                );
              }
              Err(e) => {
                warn!("Failed to cache symbols for {}: {}", source, e);
              }
            }
          }
          Err(e) => {
            warn!("Failed to serialize symbols for caching: {}", e);
          }
        }
      }

      Ok(symbols)
    } else {
      Err(CryptoLoaderError::SourceUnavailable(source.to_string()))
    }
  }
}

impl Clone for CryptoSymbolLoader {
  fn clone(&self) -> Self {
    Self::new(self.config.clone())
  }
}

// ─── Result type ────────────────────────────────────────────────────────────

/// Aggregated result from [`CryptoSymbolLoader::load_all_symbols`].
///
/// # Fields
///
/// | Field               | Description                                          |
/// |---------------------|------------------------------------------------------|
/// | `symbols_loaded`    | Number of unique symbols after deduplication          |
/// | `symbols_failed`    | Number of sources that returned errors                |
/// | `symbols_skipped`   | Duplicate symbols removed during deduplication        |
/// | `symbols`           | The deduplicated [`CryptoSymbol`] list                |
/// | `source_results`    | Per-source [`SourceResult`] with counts and timing    |
/// | `processing_time_ms`| Total wall-clock time for the entire operation        |
#[derive(Debug)]
pub struct LoadAllSymbolsResult {
  /// Unique symbols after deduplication.
  pub symbols_loaded: usize,
  /// Number of sources that failed to load.
  pub symbols_failed: usize,
  /// Symbols removed by deduplication (`total_loaded - symbols_loaded`).
  pub symbols_skipped: usize,
  /// The final deduplicated symbol list.
  pub symbols: Vec<CryptoSymbol>,
  /// Per-source fetch results with counts, errors, and timing.
  pub source_results: HashMap<CryptoDataSource, SourceResult>,
  /// Total wall-clock time in milliseconds.
  pub processing_time_ms: u64,
}
