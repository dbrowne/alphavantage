//! Crypto metadata loader with caching support
//!
//! This module provides functionality to load cryptocurrency metadata from various sources
//! (primarily CoinGecko and AlphaVantage) and store it in the database with comprehensive caching support.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{prelude::*, sql_query, sql_types};
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::{
  DataLoader, LoaderContext, LoaderError, LoaderResult, ProcessState,
  crypto::{CryptoDataSource, CryptoLoaderError},
};

use av_models::crypto::CryptoDaily;

/// Cache query result structure for SQL queries
#[derive(QueryableByName, Debug)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: DateTime<Utc>,
}

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
  pub async fn cleanup_expired_cache(database_url: &str) -> Result<usize, LoaderError> {
    use diesel::Connection;

    let mut conn = diesel::PgConnection::establish(database_url)
      .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

    let deleted_count = sql_query("DELETE FROM api_response_cache WHERE expires_at < NOW()")
      .execute(&mut conn)
      .map_err(|e| LoaderError::DatabaseError(format!("Cleanup failed: {}", e)))?;

    if deleted_count > 0 {
      info!("🧹 Cleaned up {} expired cache entries", deleted_count);
    }

    Ok(deleted_count)
  }

  /// Get cached response for a specific cache key
  async fn get_cached_response(
    &self,
    database_url: &str,
    cache_key: &str,
    api_source: &str,
  ) -> Option<serde_json::Value> {
    if !self.config.enable_response_cache {
      return None;
    }

    use diesel::Connection;
    let mut conn = match diesel::PgConnection::establish(database_url) {
      Ok(conn) => conn,
      Err(_) => return None,
    };

    let cached_entry: Option<CacheQueryResult> = sql_query(
      "SELECT response_data, expires_at FROM api_response_cache
             WHERE cache_key = $1 AND expires_at > NOW() AND api_source = $2",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .bind::<sql_types::Text, _>(api_source)
    .get_result(&mut conn)
    .optional()
    .unwrap_or(None);

    if let Some(cache_result) = cached_entry {
      info!("📦 Using cached response for {} (expires: {})", cache_key, cache_result.expires_at);
      return Some(cache_result.response_data);
    }

    None
  }

  /// Cache API response
  async fn cache_response(
    &self,
    database_url: &str,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    response_data: &serde_json::Value,
    status_code: u16,
  ) {
    if !self.config.enable_response_cache {
      return;
    }

    use diesel::Connection;
    let mut conn = match diesel::PgConnection::establish(database_url) {
      Ok(conn) => conn,
      Err(e) => {
        warn!("Failed to connect to database for caching: {}", e);
        return;
      }
    };

    let expires_at =
      chrono::Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);

    // Insert or update cache entry
    let result = sql_query(
      "INSERT INTO api_response_cache
             (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (cache_key) DO UPDATE SET
                response_data = EXCLUDED.response_data,
                status_code = EXCLUDED.status_code,
                expires_at = EXCLUDED.expires_at,
                cached_at = NOW()",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .bind::<sql_types::Text, _>(api_source)
    .bind::<sql_types::Text, _>(endpoint_url)
    .bind::<sql_types::Jsonb, _>(response_data)
    .bind::<sql_types::Integer, _>(status_code as i32)
    .bind::<sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn);

    match result {
      Ok(_) => info!("💾 Cached response for {} (expires: {})", cache_key, expires_at),
      Err(e) => warn!("Failed to cache response for {}: {}", cache_key, e),
    }
  }

  /// Load metadata from AlphaVantage for a single symbol with caching
  async fn load_alphavantage_metadata_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    database_url: &str,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = self.generate_cache_key("alphavantage", &symbol.symbol);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_data) =
        self.get_cached_response(database_url, &cache_key, "alphavantage").await
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
      Ok((metadata, response, url, status)) => {
        // Cache the successful response
        let response_json =
          serde_json::to_value(&response).unwrap_or_else(|_| serde_json::Value::Null);

        self
          .cache_response(database_url, &cache_key, "alphavantage", &url, &response_json, status)
          .await;

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
    database_url: &str,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = self.generate_cache_key("coingecko", &symbol.source_id);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_data) =
        self.get_cached_response(database_url, &cache_key, "coingecko").await
      {
        debug!("📦 Using cached CoinGecko metadata for {}", symbol.symbol);

        // Parse cached response and process
        return self.process_coingecko_response(cached_data, symbol);
      }
    }

    // Cache miss or force refresh - fetch from API
    debug!("🌐 Fetching fresh CoinGecko metadata for {} (cache miss)", symbol.symbol);

    match self.load_coingecko_metadata_fresh(symbol).await {
      Ok((metadata, response, url, status)) => {
        // Cache the successful response
        self.cache_response(database_url, &cache_key, "coingecko", &url, &response, status).await;

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
    database_url: &str,
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
            self.load_coingecko_metadata_cached(&symbol, database_url).await
          }
          _ => {
            // Check if this is an AlphaVantage request (API key provided but not CoinGecko)
            if self.config.alphavantage_api_key.is_some() && source != CryptoDataSource::CoinGecko {
              // Treat as AlphaVantage request
              self.load_alphavantage_metadata_cached(&symbol, database_url).await
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

    // Extract database URL from context (this would need to be added to LoaderContext)
    // For now, we'll use a placeholder - this should be improved in the actual integration
    let database_url = std::env::var("DATABASE_URL").unwrap();

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
          self.process_batch(context, batch.to_vec(), source, &database_url).await;

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
