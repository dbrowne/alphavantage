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

//! Crypto metadata loader with caching support
//!
//! This module provides functionality to load cryptocurrency metadata from various sources
//! (primarily CoinGecko and AlphaVantage) and store it in the database with comprehensive caching support.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::{
  DataLoader, LoaderContext, LoaderError, LoaderResult, ProcessState,
  crypto::{CryptoDataSource, CryptoLoaderError},
};
use av_database_postgres::repository::CacheRepository;
use av_models::crypto::CryptoDaily;

/// Configuration for crypto metadata loader
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
      cache_ttl_hours: 24, // 24 hours for metadata (less frequent changes)
      force_refresh: false,
    }
  }
}

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

/// Symbol information needed for metadata loading
#[derive(Debug, Clone)]
pub struct CryptoSymbolForMetadata {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub source: CryptoDataSource,
  pub source_id: String,
  pub is_active: bool,
}

/// Processed crypto metadata ready for database insertion
#[derive(Debug, Clone)]
pub struct ProcessedCryptoMetadata {
  pub sid: i64,
  pub source: String,
  pub source_id: String,
  pub market_cap_rank: Option<i32>,
  pub base_currency: Option<String>,
  pub quote_currency: Option<String>,
  pub is_active: bool,
  pub additional_data: Option<Value>, // JSONB can be NULL
  pub last_updated: DateTime<Utc>,
}

/// Output from crypto metadata loader
#[derive(Debug, Clone)]
pub struct CryptoMetadataOutput {
  pub metadata_processed: Vec<ProcessedCryptoMetadata>,
  pub symbols_processed: usize,
  pub symbols_failed: usize,
  pub processing_time_ms: u64,
  pub source_results: HashMap<CryptoDataSource, MetadataSourceResult>,
}

/// Results from a specific data source
#[derive(Debug, Clone)]
pub struct MetadataSourceResult {
  pub symbols_processed: usize,
  pub symbols_failed: usize,
  pub errors: Vec<String>,
  pub rate_limited: bool,
}

/// Crypto metadata loader
pub struct CryptoMetadataLoader {
  config: CryptoMetadataConfig,
}

impl CryptoMetadataLoader {
  pub fn new(config: CryptoMetadataConfig) -> Self {
    Self { config }
  }

  /// Generate cache key for metadata requests
  fn generate_cache_key(&self, source: &str, identifier: &str) -> String {
    format!("crypto_metadata_{}_{}", source, identifier)
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

  /// Get cached response for a specific cache key
  async fn get_cached_response(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
    api_source: &str,
  ) -> Option<serde_json::Value> {
    if !self.config.enable_response_cache || self.config.force_refresh {
      return None;
    }

    match cache_repo.get_json(cache_key, api_source).await {
      Ok(Some(data)) => {
        info!("📦 Cache hit for {}", cache_key);
        debug!("Successfully retrieved cached metadata for {}", cache_key);
        Some(data)
      }
      Ok(None) => {
        debug!("Cache miss for {}", cache_key);
        None
      }
      Err(e) => {
        debug!("Cache read error for {}: {}", cache_key, e);
        None
      }
    }
  }

  /// Cache API response
  async fn cache_response(
    &self,
    cache_repo: &Arc<dyn CacheRepository>,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    response_data: &serde_json::Value,
  ) {
    if !self.config.enable_response_cache {
      return;
    }

    match cache_repo
      .set_json(
        cache_key,
        api_source,
        endpoint_url,
        response_data.clone(),
        self.config.cache_ttl_hours as i64,
      )
      .await
    {
      Ok(()) => {
        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);
        info!(
          "💾 Cached {} (TTL: {}h, expires: {})",
          cache_key, self.config.cache_ttl_hours, expires_at
        );
      }
      Err(e) => {
        warn!("Failed to cache {}: {}", cache_key, e);
      }
    }
  }

  /// Load metadata from AlphaVantage for a single symbol with caching
  async fn load_alphavantage_metadata_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = self.generate_cache_key("alphavantage", &symbol.symbol);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_data) =
        self.get_cached_response(cache_repo, &cache_key, "alphavantage").await
      {
        debug!("📦 Using cached AlphaVantage metadata for {}", symbol.symbol);

        // Parse cached response
        if let Ok(crypto_daily) = serde_json::from_value::<CryptoDaily>(cached_data) {
          return self.process_alphavantage_response(crypto_daily, symbol);
        } else {
          warn!("Failed to parse cached AlphaVantage response for {}", symbol.symbol);
        }
      }
    }

    // Cache miss or force refresh - fetch from API
    debug!("🌐 Fetching fresh AlphaVantage metadata for {} (cache miss)", symbol.symbol);

    match self.load_alphavantage_metadata_fresh(symbol).await {
      Ok((metadata, response, url, _status)) => {
        // Cache the successful response
        let response_json =
          serde_json::to_value(&response).unwrap_or_else(|_| serde_json::Value::Null);

        self.cache_response(cache_repo, &cache_key, "alphavantage", &url, &response_json).await;

        Ok(metadata)
      }
      Err(e) => Err(e),
    }
  }

  /// Load fresh AlphaVantage metadata from API
  async fn load_alphavantage_metadata_fresh(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<(ProcessedCryptoMetadata, CryptoDaily, String, u16), CryptoLoaderError> {
    debug!("Loading AlphaVantage metadata for {}", symbol.symbol);

    // Check if API key is available
    let api_key = self.config.alphavantage_api_key.as_ref().ok_or_else(|| {
      CryptoLoaderError::ApiError("AlphaVantage API key not provided".to_string())
    })?;

    // Build AlphaVantage API URL - using direct HTTP request since client doesn't have the method we need
    let url = format!(
      "https://www.alphavantage.co/query?function=DIGITAL_CURRENCY_DAILY&symbol={}&market=USD&apikey={}",
      symbol.symbol, api_key
    );

    let client = reqwest::Client::new();
    let response = client
      .get(&url)
      .timeout(Duration::from_secs(self.config.timeout_seconds))
      .send()
      .await
      .map_err(|e| CryptoLoaderError::ApiError(format!("AlphaVantage request failed: {}", e)))?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
      return Err(CryptoLoaderError::ApiError(format!(
        "AlphaVantage API returned status: {}",
        response.status()
      )));
    }

    let response_text = response.text().await.map_err(|e| {
      CryptoLoaderError::ApiError(format!("Failed to read AlphaVantage response: {}", e))
    })?;

    // Parse response to extract metadata
    let crypto_daily: CryptoDaily = serde_json::from_str(&response_text).map_err(|e| {
      CryptoLoaderError::ParseError(format!("Failed to parse AlphaVantage response: {}", e))
    })?;

    let metadata = self.process_alphavantage_response(crypto_daily.clone(), symbol)?;

    Ok((metadata, crypto_daily, url, status))
  }

  /// Process AlphaVantage API response into metadata
  fn process_alphavantage_response(
    &self,
    crypto_daily: CryptoDaily,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    // Extract metadata and build additional data
    let mut additional_data = HashMap::new();
    additional_data.insert(
      "alphavantage_info".to_string(),
      serde_json::to_value(&crypto_daily.meta_data).unwrap_or(serde_json::Value::Null),
    );

    // Determine base and quote currencies from metadata
    let base_currency = Some(crypto_daily.meta_data.digital_currency_code.clone());
    let quote_currency = Some(crypto_daily.meta_data.market_code.clone());

    Ok(ProcessedCryptoMetadata {
      sid: symbol.sid,
      source: "alphavantage".to_string(),
      source_id: format!("{}_{}", symbol.symbol, crypto_daily.meta_data.market_code),
      market_cap_rank: None, // AlphaVantage doesn't provide market cap rank
      base_currency,
      quote_currency,
      is_active: symbol.is_active,
      additional_data: Some(
        serde_json::to_value(additional_data).unwrap_or(serde_json::Value::Null),
      ),
      last_updated: Utc::now(),
    })
  }

  /// Load enhanced metadata from CoinGecko with caching
  async fn load_coingecko_metadata_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = self.generate_cache_key("coingecko", &symbol.source_id);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_data) = self.get_cached_response(cache_repo, &cache_key, "coingecko").await
      {
        debug!("📦 Using cached CoinGecko metadata for {}", symbol.symbol);

        // Parse cached response and process
        return self.process_coingecko_response(cached_data, symbol);
      }
    }

    // Cache miss or force refresh - fetch from API
    debug!("🌐 Fetching fresh CoinGecko metadata for {} (cache miss)", symbol.symbol);

    match self.load_coingecko_metadata_fresh(symbol).await {
      Ok((metadata, response, url, _status)) => {
        // Cache the successful response
        self.cache_response(cache_repo, &cache_key, "coingecko", &url, &response).await;

        Ok(metadata)
      }
      Err(e) => Err(e),
    }
  }

  /// Load fresh CoinGecko metadata from API
  async fn load_coingecko_metadata_fresh(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<(ProcessedCryptoMetadata, serde_json::Value, String, u16), CryptoLoaderError> {
    debug!("Loading CoinGecko metadata for {}", symbol.source_id);

    // Build CoinGecko API URL
    let url = if let Some(api_key) = &self.config.coingecko_api_key {
      format!(
        "https://pro-api.coingecko.com/api/v3/coins/{}?x_cg_pro_api_key={}",
        symbol.source_id, api_key
      )
    } else {
      format!("https://api.coingecko.com/api/v3/coins/{}?localization=false", symbol.source_id)
    };

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add API key if available
    if let Some(api_key) = &self.config.coingecko_api_key {
      request = request.header("X-CG-Pro-API-Key", api_key);
    }

    let response =
      request
        .timeout(Duration::from_secs(self.config.timeout_seconds))
        .send()
        .await
        .map_err(|e| CryptoLoaderError::ApiError(format!("CoinGecko request failed: {}", e)))?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
      return Err(CryptoLoaderError::ApiError(format!(
        "CoinGecko API returned status: {}",
        response.status()
      )));
    }

    let coin_data: Value = response.json().await.map_err(|e| {
      CryptoLoaderError::ParseError(format!("Failed to parse CoinGecko response: {}", e))
    })?;

    let metadata = self.process_coingecko_response(coin_data.clone(), symbol)?;

    Ok((metadata, coin_data, url, status))
  }

  /// Process CoinGecko API response into metadata
  fn process_coingecko_response(
    &self,
    coin_data: serde_json::Value,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    // Extract relevant metadata
    let market_cap_rank =
      coin_data.get("market_cap_rank").and_then(|v| v.as_i64()).map(|v| v as i32);

    let mut additional_data = HashMap::new();

    // Add comprehensive metadata from CoinGecko
    if let Some(description) = coin_data.get("description").and_then(|d| d.get("en")) {
      additional_data.insert("description".to_string(), description.clone());
    }

    if let Some(links) = coin_data.get("links") {
      additional_data.insert("links".to_string(), links.clone());
    }

    if let Some(market_data) = coin_data.get("market_data") {
      additional_data.insert("market_data".to_string(), market_data.clone());
    }

    // Add categories, if available
    if let Some(categories) = coin_data.get("categories") {
      additional_data.insert("categories".to_string(), categories.clone());
    }

    Ok(ProcessedCryptoMetadata {
      sid: symbol.sid,
      source: "coingecko".to_string(),
      source_id: symbol.source_id.clone(),
      market_cap_rank,
      base_currency: Some(symbol.symbol.clone()),
      quote_currency: Some("USD".to_string()),
      is_active: symbol.is_active,
      additional_data: if additional_data.is_empty() {
        None
      } else {
        Some(serde_json::to_value(additional_data)?)
      },
      last_updated: Utc::now(),
    })
  }

  /// Process a batch of symbols for metadata with caching
  async fn process_batch(
    &self,
    _context: &LoaderContext,
    symbols: Vec<CryptoSymbolForMetadata>,
    source: CryptoDataSource,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> (Vec<ProcessedCryptoMetadata>, MetadataSourceResult) {
    let mut processed_metadata = Vec::new();
    let mut errors = Vec::new();
    let mut symbols_failed = 0;
    let mut rate_limited = false;

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
          CryptoDataSource::CoinGecko => {
            self.load_coingecko_metadata_cached(&symbol, cache_repo).await
          }
          _ => {
            // Check if this is an AlphaVantage request (API key provided but not CoinGecko)
            if self.config.alphavantage_api_key.is_some() && source != CryptoDataSource::CoinGecko {
              // Treat as AlphaVantage request
              self.load_alphavantage_metadata_cached(&symbol, cache_repo).await
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
        let (metadata, result) =
          self.process_batch(context, batch.to_vec(), source, cache_repo).await;

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
