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

//! Crypto metadata loader with caching support.
//!
//! This module provides the DataLoader implementation for loading cryptocurrency
//! metadata from various sources (primarily CoinGecko and AlphaVantage).
//!
//! # Architecture
//!
//! The loader is split into three modules for separation of concerns:
//! - `metadata_types` - Configuration, input, and output types
//! - `metadata_providers` - API-specific fetching logic (AlphaVantage, CoinGecko)
//! - `metadata_loader` - DataLoader trait implementation and orchestration

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, ProcessState};
use av_database_postgres::repository::CacheRepository;

use super::metadata_providers::{AlphaVantageMetadataProvider, CoinGeckoMetadataProvider};
use super::metadata_types::{
  CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataOutput, CryptoSymbolForMetadata,
  MetadataSourceResult, ProcessedCryptoMetadata,
};
use super::{CryptoDataSource, CryptoLoaderError};

/// Crypto metadata loader
pub struct CryptoMetadataLoader {
  config: CryptoMetadataConfig,
}

impl CryptoMetadataLoader {
  pub fn new(config: CryptoMetadataConfig) -> Self {
    Self { config }
  }

  /// Clean expired cache entries
  pub async fn cleanup_expired_cache(
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<usize, LoaderError> {
    match cache_repo.cleanup_expired("crypto_metadata").await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired crypto metadata cache entries", deleted_count);
        }
        Ok(deleted_count)
      }
      Err(e) => Err(LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e))),
    }
  }

  /// Process a batch of symbols for metadata with caching
  async fn process_batch(
    &self,
    symbols: Vec<CryptoSymbolForMetadata>,
    source: CryptoDataSource,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> (Vec<ProcessedCryptoMetadata>, MetadataSourceResult) {
    let mut processed_metadata = Vec::new();
    let mut errors = Vec::new();
    let mut symbols_failed = 0;
    let mut rate_limited = false;

    let alphavantage_provider = AlphaVantageMetadataProvider::new(&self.config);
    let coingecko_provider = CoinGeckoMetadataProvider::new(&self.config);

    for symbol in symbols {
      // Add delay between requests to respect rate limits
      if !processed_metadata.is_empty() {
        sleep(Duration::from_millis(self.config.delay_ms)).await;
      }

      // Retry logic
      let mut attempts = 0;
      let mut success = false;

      while attempts < self.config.max_retries && !success {
        attempts += 1;

        let result = match source {
          CryptoDataSource::CoinGecko => coingecko_provider.load_cached(&symbol, cache_repo).await,
          _ => {
            // Check if this is an AlphaVantage request
            if self.config.alphavantage_api_key.is_some() && source != CryptoDataSource::CoinGecko {
              alphavantage_provider.load_cached(&symbol, cache_repo).await
            } else {
              Err(CryptoLoaderError::ApiError(format!(
                "Source {:?} not supported for metadata",
                source
              )))
            }
          }
        };

        match result {
          Ok(metadata) => {
            processed_metadata.push(metadata);
            success = true;
            debug!("Successfully loaded metadata for {}", symbol.symbol);
          }
          Err(e) => {
            let error_msg = e.to_string();

            // Check for permanent errors (404 Not Found) - don't retry
            if error_msg.contains("404") || error_msg.contains("Not Found") {
              debug!("Coin {} not found on {:?} (404) - skipping retries", symbol.symbol, source);
              errors.push(format!("Coin not found on {:?}: {}", source, symbol.symbol));
              symbols_failed += 1;
              break;
            }

            // Check for rate limiting
            if error_msg.contains("rate limit") || error_msg.contains("429") {
              rate_limited = true;
              warn!(
                "Rate limited for {}, attempt {}/{}",
                symbol.symbol, attempts, self.config.max_retries
              );

              // Exponential backoff for rate limiting
              let backoff_delay = self.config.delay_ms * 2_u64.pow(attempts as u32);
              sleep(Duration::from_millis(backoff_delay)).await;
            } else {
              error!(
                "Failed to load metadata for {} (attempt {}/{}): {}",
                symbol.symbol, attempts, self.config.max_retries, error_msg
              );
            }

            if attempts >= self.config.max_retries {
              errors.push(format!("Failed to load metadata for {}: {}", symbol.symbol, error_msg));
              symbols_failed += 1;
            }
          }
        }
      }
    }

    let result = MetadataSourceResult {
      symbols_processed: processed_metadata.len(),
      symbols_failed,
      errors,
      rate_limited,
    };

    (processed_metadata, result)
  }
}

#[async_trait]
impl DataLoader for CryptoMetadataLoader {
  type Input = CryptoMetadataInput;
  type Output = CryptoMetadataOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    let start_time = std::time::Instant::now();
    info!(
      "Starting crypto metadata loader with caching enabled: {}",
      self.config.enable_response_cache
    );

    if let Some(tracker) = &context.process_tracker {
      tracker
        .start("crypto_metadata_loader")
        .await
        .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
    }

    let symbols = input.symbols.unwrap_or_default();
    let symbols_count = symbols.len();

    if symbols.is_empty() {
      warn!("No symbols provided for metadata loading");
      return Ok(CryptoMetadataOutput {
        metadata_processed: Vec::new(),
        symbols_processed: 0,
        symbols_failed: 0,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        source_results: HashMap::new(),
      });
    }

    info!(
      "Processing metadata for {} symbols from {} sources (cache TTL: {}h)",
      symbols_count,
      input.sources.len(),
      self.config.cache_ttl_hours
    );

    // Get cache repository from context
    let cache_repo = context.cache_repository.as_ref().ok_or_else(|| {
      LoaderError::DatabaseError("Cache repository not available in context".to_string())
    })?;

    let mut all_metadata = Vec::new();
    let mut source_results = HashMap::new();
    let mut total_failed = 0;

    // Process each data source
    for source in input.sources {
      info!("Processing {} symbols from {:?} with caching", symbols_count, source);

      // Split symbols into batches
      let mut batch_metadata = Vec::new();
      let mut combined_result = MetadataSourceResult {
        symbols_processed: 0,
        symbols_failed: 0,
        errors: Vec::new(),
        rate_limited: false,
      };

      for batch in symbols.chunks(self.config.batch_size) {
        let (metadata, result) = self.process_batch(batch.to_vec(), source, cache_repo).await;

        batch_metadata.extend(metadata);
        combined_result.symbols_processed += result.symbols_processed;
        combined_result.symbols_failed += result.symbols_failed;
        combined_result.errors.extend(result.errors);
        combined_result.rate_limited = combined_result.rate_limited || result.rate_limited;
      }

      info!(
        "Completed {:?}: {} processed, {} failed, caching: {}",
        source,
        combined_result.symbols_processed,
        combined_result.symbols_failed,
        if self.config.enable_response_cache { "enabled" } else { "disabled" }
      );

      total_failed += combined_result.symbols_failed;
      all_metadata.extend(batch_metadata);
      source_results.insert(source, combined_result);
    }

    let processing_time = start_time.elapsed().as_millis() as u64;

    if let Some(tracker) = &context.process_tracker {
      let state =
        if total_failed > 0 { ProcessState::CompletedWithErrors } else { ProcessState::Success };
      tracker
        .complete(state)
        .await
        .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
    }

    info!(
      "Crypto metadata loader completed in {}ms: {} processed, {} failed, caching: {}",
      processing_time,
      all_metadata.len(),
      total_failed,
      if self.config.enable_response_cache { "enabled" } else { "disabled" }
    );

    Ok(CryptoMetadataOutput {
      metadata_processed: all_metadata,
      symbols_processed: symbols_count,
      symbols_failed: total_failed,
      processing_time_ms: processing_time,
      source_results,
    })
  }

  fn name(&self) -> &'static str {
    "CryptoMetadataLoader"
  }
}