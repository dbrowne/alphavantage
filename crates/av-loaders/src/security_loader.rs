/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

//! Security loader that reads symbols from CSV files and searches for them via AlphaVantage API
//!
//! ## Refactored to use Repository Pattern
//! This loader now uses the CacheRepository abstraction instead of direct database access.
//! All database operations are delegated to the repository layer, following clean architecture principles.

use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use crate::error::LoaderError;
use crate::{
  DataLoader, LoaderContext, LoaderResult, csv_processor::CsvProcessor,
  process_tracker::ProcessState,
};
use av_database_postgres::repository::CacheRepositoryExt;
use av_models::common::SymbolMatch;
use av_models::time_series::SymbolSearch;

/// Configuration for symbol matching behavior
#[derive(Debug, Clone)]
pub enum SymbolMatchMode {
  /// Only accept exact symbol matches (case-insensitive)
  ExactMatch,
  /// Accept all symbols returned from search
  AllMatches,
  /// Accept top N matches based on match score
  TopMatches(usize),
}

impl Default for SymbolMatchMode {
  fn default() -> Self {
    SymbolMatchMode::AllMatches
  }
}

/// Configuration for security loader caching behavior
#[derive(Debug, Clone)]
pub struct SecurityLoaderConfig {
  /// Enable caching (requires cache_repository in LoaderContext)
  pub enable_cache: bool,
  /// Cache TTL in hours
  pub cache_ttl_hours: i64,
  /// Force refresh (bypass cache)
  pub force_refresh: bool,
}

impl Default for SecurityLoaderConfig {
  fn default() -> Self {
    Self {
      enable_cache: true,
      cache_ttl_hours: 168, // 7 days - symbol data is relatively stable
      force_refresh: false,
    }
  }
}

/// Security loader - fetches symbol data from AlphaVantage API
///
/// This loader is now database-agnostic. It uses the CacheRepository
/// trait for caching instead of direct database access.
pub struct SecurityLoader {
  semaphore: Arc<Semaphore>,
  match_mode: SymbolMatchMode,
  config: SecurityLoaderConfig,
}

impl SecurityLoader {
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      match_mode: SymbolMatchMode::default(),
      config: SecurityLoaderConfig::default(),
    }
  }

  /// Set the symbol match mode
  pub fn with_match_mode(mut self, mode: SymbolMatchMode) -> Self {
    self.match_mode = mode;
    self
  }

  /// Set configuration
  pub fn with_config(mut self, config: SecurityLoaderConfig) -> Self {
    self.config = config;
    self
  }

  /// Generate cache key for symbol search requests
  fn generate_cache_key(&self, symbol: &str) -> String {
    // Use simple string key for deterministic caching across runs
    format!("symbol_search_{}", symbol.to_uppercase())
  }

  /// Get cached response if available and not expired
  /// Now uses the CacheRepository instead of direct database access
  async fn get_cached_response(
    &self,
    cache_key: &str,
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) -> Option<SymbolSearch> {
    if !self.config.enable_cache || self.config.force_refresh {
      return None;
    }

    match cache_repo.get::<SymbolSearch>(cache_key, "alphavantage").await {
      Ok(Some(search)) => {
        info!("📦 Cache hit for key: {}", cache_key);
        debug!("Successfully retrieved cached symbol search");
        Some(search)
      }
      Ok(None) => {
        debug!("Cache miss for key: {}", cache_key);
        None
      }
      Err(e) => {
        debug!("Cache read error for key {}: {}", cache_key, e);
        None
      }
    }
  }

  /// Cache the API response
  /// Now uses the CacheRepository instead of direct database access
  async fn cache_response(
    &self,
    cache_key: &str,
    search: &SymbolSearch,
    symbol: &str,
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) {
    if !self.config.enable_cache {
      return;
    }

    let endpoint_url = format!("SYMBOL_SEARCH:{}", symbol);

    match cache_repo
      .set(cache_key, "alphavantage", &endpoint_url, search, self.config.cache_ttl_hours)
      .await
    {
      Ok(()) => {
        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours);
        info!("💾 Cached symbol search for {} (expires: {})", cache_key, expires_at);
      }
      Err(e) => {
        warn!("Failed to cache symbol search for {}: {}", cache_key, e);
      }
    }
  }

  /// Clean expired cache entries
  /// This is now a convenience method that delegates to the repository
  pub async fn cleanup_expired_cache(
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) -> Result<usize, LoaderError> {
    match cache_repo.cleanup_expired("alphavantage").await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired security cache entries", deleted_count);
        }
        Ok(deleted_count)
      }
      Err(e) => Err(LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e))),
    }
  }

  /// Get matching symbols based on the configured match mode
  fn get_matching_symbols(
    &self,
    search_query: &str,
    search_results: SymbolSearch,
  ) -> Vec<SymbolMatch> {
    match &self.match_mode {
      SymbolMatchMode::ExactMatch => search_results
        .best_matches
        .into_iter()
        .filter(|m| m.symbol.eq_ignore_ascii_case(search_query))
        .collect(),
      SymbolMatchMode::AllMatches => search_results.best_matches,
      SymbolMatchMode::TopMatches(n) => {
        let mut matches = search_results.best_matches;
        // Sort by match score (descending)
        matches.sort_by(|a, b| {
          let score_a: f64 = a.match_score.parse().unwrap_or(0.0);
          let score_b: f64 = b.match_score.parse().unwrap_or(0.0);
          score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.into_iter().take(*n).collect()
      }
    }
  }
}

#[async_trait]
impl DataLoader for SecurityLoader {
  type Input = SecurityLoaderInput;
  type Output = SecurityLoaderOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Loading securities from {:?} with match mode {:?}", input.file_path, self.match_mode);

    // Parse CSV file to get symbols
    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(&input.file_path)?;

    info!("Found {} symbols in CSV", symbols.len());

    // Track process if enabled
    if let Some(tracker) = &context.process_tracker {
      tracker.start("security_loader").await?;
    }

    // Use Arc for progress bar to share it across async tasks
    let progress = if context.config.show_progress {
      Some(Arc::new(ProgressBar::new(symbols.len() as u64)))
    } else {
      None
    };

    // Clone for use after the stream processing
    let progress_for_finish = progress.clone();

    // Create owned copies for the async closures
    let exchange = input.exchange.clone();
    let client_ref = context.client.clone();
    let retry_delay = context.config.retry_delay_ms;
    let max_concurrent = context.config.max_concurrent_requests;

    // Get cache repository if available
    let cache_repo_opt = context.cache_repository.clone();

    // Counters for output
    let mut cache_hits = 0usize;
    let mut api_calls = 0usize;

    // Query AlphaVantage API for each symbol
    let results = stream::iter(symbols.into_iter())
      .map(move |symbol| {
        let client = client_ref.clone();
        let semaphore = self.semaphore.clone();
        let progress = progress.clone();
        let exchange = exchange.clone();
        let original_symbol = symbol.clone();
        let loader = self.clone();
        let cache_repo_opt = cache_repo_opt.clone();

        async move {
          let _permit =
            semaphore.acquire().await.expect("Semaphore should not be closed during operation");

          // Generate cache key
          let cache_key = loader.generate_cache_key(&symbol);

          // Check cache first (if cache repository is available)
          let (search_results, from_cache) = if let Some(cache_repo) = &cache_repo_opt {
            if let Some(cached_search) = loader.get_cached_response(&cache_key, cache_repo).await {
              info!("📦 Using cached data for {} (no API call needed)", symbol);
              (cached_search, true)
            } else {
              // Cache miss - need to call API
              info!("🌐 Cache miss - calling API for {}", symbol);

              // Search for the symbol
              let search_res = match client.time_series().symbol_search(&symbol).await {
                Ok(results) => results,
                Err(e) => {
                  warn!("Symbol search failed for {}: {}", symbol, e);
                  if let Some(pb) = &progress {
                    pb.inc(1);
                  }
                  tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
                  return Err((e, false));
                }
              };

              // Cache the successful response
              loader.cache_response(&cache_key, &search_res, &symbol, cache_repo).await;

              (search_res, false)
            }
          } else {
            // No cache repository - just call API
            debug!("No cache repository available - calling API directly");
            let search_res = match client.time_series().symbol_search(&symbol).await {
              Ok(results) => results,
              Err(e) => {
                warn!("Symbol search failed for {}: {}", symbol, e);
                if let Some(pb) = &progress {
                  pb.inc(1);
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
                return Err((e, false));
              }
            };
            (search_res, false)
          };

          // Get matching symbols based on mode
          let matches = loader.get_matching_symbols(&symbol, search_results);

          if matches.is_empty() {
            warn!("No matches found for symbol {}", symbol);
            if let Some(pb) = &progress {
              pb.inc(1);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
            return Ok((vec![], from_cache));
          }

          // Convert matches to SecurityData
          let mut security_data = Vec::new();

          for symbol_match in matches {
            // Validate the symbol match data from API
            if symbol_match.symbol.len() > 20 {
              error!(
                "API PARSING ERROR: Received symbol '{}' with length {} from symbol search",
                symbol_match.symbol,
                symbol_match.symbol.len()
              );
              error!("  API Response: {:?}", symbol_match);
              error!("  Original query was: '{}'", original_symbol);
              continue; // Skip this malformed result
            }
            debug!(
              "Found match for {}: {} (score: {}, type: {}, region: {})",
              original_symbol,
              symbol_match.symbol,
              symbol_match.match_score,
              symbol_match.stock_type,
              symbol_match.region
            );

            // Additional validation
            if symbol_match.symbol.is_empty() {
              warn!("Received empty symbol from API for query '{}'", original_symbol);
              continue;
            }
            security_data.push(SecurityData {
              symbol: symbol_match.symbol,
              name: symbol_match.name,
              stock_type: symbol_match.stock_type,
              region: symbol_match.region,
              market_open: symbol_match.market_open,
              market_close: symbol_match.market_close,
              timezone: symbol_match.timezone,
              currency: symbol_match.currency,
              exchange: exchange.clone(),
              match_score: symbol_match.match_score.parse::<f64>().ok(),
              original_query: Some(original_symbol.clone()),
              from_cache, // Pass through whether this came from cache
            });
          }

          if let Some(pb) = &progress {
            pb.inc(1);
          }

          // Add delay to respect rate limits (only if not from cache)
          if !from_cache {
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
          }

          Ok((security_data, from_cache))
        }
      })
      .buffer_unordered(max_concurrent)
      .collect::<Vec<_>>()
      .await;

    if let Some(pb) = progress_for_finish {
      pb.finish_with_message("Security loading complete");
    }

    // Process results - flatten nested vectors and count cache hits
    let mut loaded = Vec::new();
    let mut errors = 0;
    let mut skipped = 0;

    for result in results {
      match result {
        Ok((data_vec, from_cache)) => {
          if from_cache {
            cache_hits += 1;
          } else {
            api_calls += 1;
          }

          if data_vec.is_empty() {
            skipped += 1;
          } else {
            for data in data_vec {
              loaded.push(data);
            }
          }
        }
        Err((e, from_cache)) => {
          if !from_cache {
            api_calls += 1;
          }
          warn!("Error in security loading: {}", e);
          errors += 1;
        }
      }
    }

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      tracker
        .complete(if errors > 0 {
          ProcessState::CompletedWithErrors
        } else {
          ProcessState::Success
        })
        .await?;
    }

    let total_symbols = loaded.len() + errors + skipped;

    info!(
      "Security loading complete: {} loaded, {} errors, {} skipped, {} cache hits, {} API calls",
      loaded.len(),
      errors,
      skipped,
      cache_hits,
      api_calls
    );

    Ok(SecurityLoaderOutput {
      total_symbols,
      loaded_count: loaded.len(),
      errors,
      skipped_count: skipped,
      duplicates_prevented: 0, // TODO: Implement duplicate tracking
      cache_hits,
      api_calls,
      data: loaded,
    })
  }

  fn name(&self) -> &'static str {
    "SecurityLoader"
  }
}

// Need to implement Clone for SecurityLoader to use it in async closures
impl Clone for SecurityLoader {
  fn clone(&self) -> Self {
    Self {
      semaphore: Arc::clone(&self.semaphore),
      match_mode: self.match_mode.clone(),
      config: self.config.clone(),
    }
  }
}

#[derive(Debug)]
pub struct SecurityLoaderInput {
  pub file_path: String,
  pub exchange: String,
}

#[derive(Debug, Clone)]
pub struct SecurityData {
  /// Stock symbol
  pub symbol: String,
  /// Company name
  pub name: String,
  /// Stock type (e.g., "Equity", "ETF")
  pub stock_type: String,
  /// Region (e.g., "United States")
  pub region: String,
  /// Market open time
  pub market_open: String,
  /// Market close time
  pub market_close: String,
  /// Timezone
  pub timezone: String,
  /// Currency
  pub currency: String,
  /// Exchange (from input, not from API)
  pub exchange: String,
  /// Match score from symbol search (if available)
  pub match_score: Option<f64>,
  /// Original symbol queried (useful when match mode returns different symbols)
  pub original_query: Option<String>,
  /// Whether this data came from cache (for database save optimization)
  pub from_cache: bool,
}

#[derive(Debug)]
pub struct SecurityLoaderOutput {
  pub total_symbols: usize,
  pub loaded_count: usize,
  pub errors: usize,
  pub skipped_count: usize,
  pub duplicates_prevented: usize,
  pub cache_hits: usize,
  pub api_calls: usize,
  pub data: Vec<SecurityData>,
}
