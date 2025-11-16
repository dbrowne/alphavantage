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

use crate::crypto::CryptoDataSource;
use crate::crypto::mapping_service::CryptoMappingService;
use crate::{LoaderError, LoaderResult};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[allow(dead_code)]
const MAX_TTL: u32 = 6; //todo:: Refactor

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForMarkets {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub coingecko_id: Option<String>,
  pub alphavantage_symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CryptoMarketData {
  pub sid: i64,
  pub exchange: String,
  pub base: String,
  pub target: String,
  pub market_type: Option<String>,
  pub volume_24h: Option<BigDecimal>,
  pub volume_percentage: Option<BigDecimal>,
  pub bid_ask_spread_pct: Option<BigDecimal>,
  pub liquidity_score: Option<String>,
  pub trust_score: Option<String>,
  pub is_active: bool,
  pub is_anomaly: bool,
  pub is_stale: bool,
  pub last_price: Option<f64>,
  pub last_traded_at: Option<String>,
  pub last_fetch_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CryptoMarketsConfig {
  pub coingecko_api_key: Option<String>,
  pub delay_ms: u64,
  pub batch_size: usize,
  pub max_retries: u32,
  pub timeout_seconds: u64,
  pub max_concurrent_requests: usize,
  pub rate_limit_delay_ms: u64,
  pub enable_progress_bar: bool,
  pub alphavantage_api_key: Option<String>,
  pub fetch_all_exchanges: bool,
  pub min_volume_threshold: Option<f64>,
  pub max_markets_per_symbol: Option<usize>,
  pub enable_response_cache: bool,
  pub cache_ttl_hours: u32, // Time-to-live in hours
  pub force_refresh: bool,  // Skip cache and force fresh API calls
}

#[derive(Debug, Clone)]
pub struct CryptoMarketsInput {
  pub symbols: Option<Vec<CryptoSymbolForMarkets>>,
  pub exchange_filter: Option<Vec<String>>,
  pub min_volume_threshold: Option<f64>,
  pub max_markets_per_symbol: Option<usize>,
  pub update_existing: bool,
  pub sources: Vec<CryptoDataSource>,
  pub batch_size: Option<usize>,
}

pub struct CryptoMarketsLoader {
  config: CryptoMarketsConfig,
  client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoTickersResponse {
  name: String,
  tickers: Vec<CoinGeckoTicker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoTicker {
  base: String,
  target: String,
  market: CoinGeckoMarket,
  last: Option<f64>,
  volume: Option<f64>,
  trust_score: Option<String>,
  bid_ask_spread_percentage: Option<f64>,
  timestamp: Option<String>,
  last_traded_at: Option<String>,
  last_fetch_at: Option<String>,
  is_anomaly: Option<bool>,
  is_stale: Option<bool>,
  trade_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoMarket {
  name: String,
  identifier: String,
  has_trading_incentive: Option<bool>,
}

// FIXED: Custom struct for SQL query result with proper derives
#[derive(Debug, QueryableByName)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: DateTime<Utc>,
}

impl CryptoMarketsConfig {
  #[allow(dead_code)]
  fn default() -> Self {
    //todo:  Refactor
    Self {
      coingecko_api_key: None,
      delay_ms: 500,
      batch_size: 1,
      max_retries: 5,
      timeout_seconds: 30,
      max_concurrent_requests: 1,
      rate_limit_delay_ms: 30,
      enable_progress_bar: true,
      alphavantage_api_key: None,
      fetch_all_exchanges: true,
      min_volume_threshold: None,
      max_markets_per_symbol: Some(2),
      enable_response_cache: true,
      cache_ttl_hours: MAX_TTL,
      force_refresh: false,
    }
  }
}

impl CryptoMarketsLoader {
  pub fn new(config: CryptoMarketsConfig) -> Self {
    let timeout = Duration::from_secs(config.timeout_seconds);
    let client = Client::builder()
      .timeout(timeout)
      .user_agent("AlphaVantage-Rust-Client/1.0")
      .build()
      .expect("Failed to create HTTP client");

    Self { config, client }
  }
  /// adding new mapping functionality
  async fn fetch_market_data_for_symbol_with_dynamic_mapping(
    &self,
    symbol: CryptoSymbolForMarkets,
    mapping_service: &CryptoMappingService,
    conn: &mut PgConnection,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    info!("🔍 Processing symbol: {} ({})", symbol.symbol, symbol.name);

    let mut market_data = Vec::new();

    // Use dynamic discovery instead of hardcoded mapping!
    match mapping_service.get_coingecko_id(conn, symbol.sid, &symbol.symbol).await {
      Ok(Some(coingecko_id)) => {
        info!("✅ {} has CoinGecko ID: {}", symbol.symbol, coingecko_id);

        match self.fetch_coingecko_markets(&coingecko_id, &symbol).await {
          Ok(mut data) => {
            info!("✅ CoinGecko returned {} markets for {}", data.len(), symbol.symbol);
            market_data.append(&mut data);
          }
          Err(e) => {
            error!("❌ CoinGecko API error for {}: {}", symbol.symbol, e);
          }
        }
      }
      Ok(None) => {
        warn!("⚠️ No CoinGecko ID found for {} after discovery attempt", symbol.symbol);
      }
      Err(e) => {
        error!("❌ Failed to get/discover CoinGecko ID for {}: {}", symbol.symbol, e);
      }
    }

    Ok(market_data)
  }
  /// Generate cache key for request
  fn generate_cache_key(&self, coingecko_id: &str) -> String {
    format!("coingecko_tickers:{}", coingecko_id)
  }

  /// Main load method that orchestrates the entire process
  pub async fn load(
    &self,
    _context: &crate::LoaderContext,
    input: CryptoMarketsInput,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    let symbols = input.symbols.unwrap_or_default();

    if symbols.is_empty() {
      return Ok(Vec::new());
    }

    info!("Starting market data fetch for {} symbols", symbols.len());

    let mut all_market_data = Vec::new();

    // Setup progress bar if enabled
    let progress = if self.config.enable_progress_bar {
      let pb = ProgressBar::new(symbols.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
          .expect("Invalid progress template")
          .progress_chars("##-"),
      );
      Some(pb)
    } else {
      None
    };

    // Process symbols in batches
    let batch_size = input.batch_size.unwrap_or(self.config.batch_size);
    let symbol_chunks: Vec<_> = symbols.chunks(batch_size).collect();

    for (chunk_index, symbol_chunk) in symbol_chunks.iter().enumerate() {
      info!("Processing batch {}/{}", chunk_index + 1, symbol_chunks.len());

      // Execute batch tasks with concurrency control
      let semaphore =
        std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_requests));
      let mut handles = Vec::new();

      for symbol in symbol_chunk.iter() {
        let sem = semaphore.clone();
        let symbol_clone = symbol.clone();
        let config = self.config.clone();
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
          let _permit = sem.acquire().await.expect("Semaphore acquire failed");

          // Create a temporary loader for this task
          let temp_loader = CryptoMarketsLoader { config, client };
          temp_loader.fetch_market_data_for_symbol(symbol_clone).await
        });
        handles.push(handle);
      }

      // Collect results from the batch
      for handle in handles {
        match handle.await {
          Ok(Ok(mut market_data)) => {
            all_market_data.append(&mut market_data);
          }
          Ok(Err(e)) => {
            warn!("Failed to fetch market data: {}", e);
          }
          Err(e) => {
            error!("Task join error: {}", e);
          }
        }

        if let Some(ref pb) = progress {
          pb.inc(1);
        }
      }

      // Rate limiting between batches
      if chunk_index < symbol_chunks.len() - 1 {
        sleep(Duration::from_millis(self.config.rate_limit_delay_ms)).await;
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Market data fetch complete");
    }

    info!("Completed market data fetch. Retrieved {} market entries", all_market_data.len());
    Ok(all_market_data)
  }

  pub async fn load_with_cache(
    &self,
    _context: &crate::LoaderContext,
    input: CryptoMarketsInput,
    database_url: &str,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    let symbols = input.symbols.unwrap_or_default();

    if symbols.is_empty() {
      return Ok(Vec::new());
    }

    info!("Starting market data fetch for {} symbols", symbols.len());

    let mut all_market_data = Vec::new();

    // Setup progress bar if enabled
    let progress = if self.config.enable_progress_bar {
      let pb = ProgressBar::new(symbols.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
          .expect("Invalid progress template")
          .progress_chars("##-"),
      );
      Some(pb)
    } else {
      None
    };

    // Process symbols in batches
    let batch_size = input.batch_size.unwrap_or(self.config.batch_size);
    let symbol_chunks: Vec<_> = symbols.chunks(batch_size).collect();

    for (chunk_index, symbol_chunk) in symbol_chunks.iter().enumerate() {
      info!("Processing batch {}/{}", chunk_index + 1, symbol_chunks.len());

      // Execute batch tasks with concurrency control
      let semaphore =
        std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_requests));
      let mut handles = Vec::new();

      let database_url = Arc::<str>::from(database_url);

      for symbol in symbol_chunk.iter() {
        let sem = semaphore.clone();
        let symbol_clone = symbol.clone();
        let config = self.config.clone();
        let client = self.client.clone();
        let database_url = Arc::clone(&database_url);

        let handle = tokio::spawn(async move {
          let _permit = sem.acquire().await.expect("Semaphore acquire failed");

          // Create a temporary loader for this task
          let temp_loader = CryptoMarketsLoader { config, client };
          temp_loader.fetch_market_data_for_symbol_cached(symbol_clone, &database_url).await
        });
        handles.push(handle);
      }

      // Collect results from the batch
      for handle in handles {
        match handle.await {
          Ok(Ok(mut market_data)) => {
            all_market_data.append(&mut market_data);
          }
          Ok(Err(e)) => {
            warn!("Failed to fetch market data: {}", e);
          }
          Err(e) => {
            error!("Task join error: {}", e);
          }
        }

        if let Some(ref pb) = progress {
          pb.inc(1);
        }
      }

      // Rate limiting between batches
      if chunk_index < symbol_chunks.len() - 1 {
        sleep(Duration::from_millis(self.config.rate_limit_delay_ms)).await;
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Market data fetch complete");
    }

    info!("Completed market data fetch. Retrieved {} market entries", all_market_data.len());
    Ok(all_market_data)
  }

  /// Fetch market data for a single symbol
  async fn fetch_market_data_for_symbol(
    &self,
    symbol: CryptoSymbolForMarkets,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    info!("🔍 Processing symbol: {} ({})", symbol.symbol, symbol.name);
    info!("🔍 CoinGecko ID: {:?}", symbol.coingecko_id);

    let mut market_data = Vec::new();

    // Check if we have a CoinGecko ID
    if let Some(ref coingecko_id) = symbol.coingecko_id {
      info!("✅ {} has CoinGecko ID: {}", symbol.symbol, coingecko_id);

      match self.fetch_coingecko_markets(coingecko_id, &symbol).await {
        Ok(mut data) => {
          info!("✅ CoinGecko returned {} markets for {}", data.len(), symbol.symbol);
          market_data.append(&mut data);
        }
        Err(e) => {
          error!("❌ CoinGecko API error for {}: {}", symbol.symbol, e);
        }
      }
    } else {
      warn!("⚠️  No CoinGecko ID for {}", symbol.symbol);
    }

    // Add delay between symbol requests
    if self.config.delay_ms > 0 {
      sleep(Duration::from_millis(self.config.delay_ms)).await;
    }

    Ok(market_data)
  }

  async fn fetch_market_data_for_symbol_cached(
    // todo!  Refacotr this
    &self,
    symbol: CryptoSymbolForMarkets,
    database_url: &str,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    info!("🔍 Processing symbol: {} ({})", symbol.symbol, symbol.name);
    info!("🔍 CoinGecko ID: {:?}", symbol.coingecko_id);

    let mut market_data = Vec::new();

    // Check if we have a CoinGecko ID
    if let Some(ref coingecko_id) = symbol.coingecko_id {
      info!("✅ {} has CoinGecko ID: {}", symbol.symbol, coingecko_id);

      match self.fetch_coingecko_markets_cached(database_url, coingecko_id, &symbol).await {
        Ok(mut data) => {
          info!("✅ CoinGecko returned {} markets for {}", data.len(), symbol.symbol);
          market_data.append(&mut data);
        }
        Err(e) => {
          error!("❌ CoinGecko API error for {}: {}", symbol.symbol, e);
        }
      }
    } else {
      warn!("⚠️  No CoinGecko ID for {}", symbol.symbol);
    }

    // Add delay between symbol requests
    if self.config.delay_ms > 0 {
      sleep(Duration::from_millis(self.config.delay_ms)).await;
    }

    Ok(market_data)
  }

  /// Main CoinGecko API method with comprehensive retry logic
  async fn fetch_coingecko_markets(
    &self,
    coingecko_id: &str,
    symbol: &CryptoSymbolForMarkets,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    let base_url = "https://api.coingecko.com/api/v3";
    let mut url = format!("{}/coins/{}/tickers", base_url, coingecko_id);

    // Add API key if available
    if let Some(ref api_key) = self.config.coingecko_api_key {
      let auth_param = if api_key.starts_with("CG-") {
        url = format!("https://pro-api.coingecko.com/api/v3/coins/{}/tickers", coingecko_id);
        "x_cg_pro_api_key"
      } else {
        "x_cg_demo_api_key"
      };
      url = format!("{}?{}={}", url, auth_param, api_key);
      info!(
        "🔑 Using {} API key for {}",
        if api_key.starts_with("CG-") { "Pro" } else { "Demo" },
        symbol.symbol
      );
    } else {
      warn!(
        "⚠️  No CoinGecko API key provided for {} - using free tier (very limited)",
        symbol.symbol
      );
    }

    info!("🌐 API URL for {}: {}", symbol.symbol, url);

    let mut retries = 0;
    while retries < self.config.max_retries {
      info!(
        "📡 Making API request for {} (attempt {}/{})",
        symbol.symbol,
        retries + 1,
        self.config.max_retries
      );

      match self.client.get(&url).send().await {
        Ok(response) => {
          let status = response.status();
          info!("📡 HTTP Status for {}: {}", symbol.symbol, status);

          if status.is_success() {
            let response_text = response.text().await.map_err(|e| {
              error!("Failed to read response body for {}: {}", symbol.symbol, e);
              LoaderError::IoError(format!("Failed to read response: {}", e))
            })?;

            info!("📄 Response length for {}: {} chars", symbol.symbol, response_text.len());

            // Log first 200 chars for debugging
            if response_text.len() > 200 {
              info!("📄 Response preview for {}: {}...", symbol.symbol, &response_text[..200]);
            } else if response_text.len() > 0 {
              info!("📄 Full response for {}: {}", symbol.symbol, response_text);
            }

            match serde_json::from_str::<CoinGeckoTickersResponse>(&response_text) {
              Ok(tickers_response) => {
                info!(
                  "✅ Successfully parsed JSON for {}: {} tickers",
                  symbol.symbol,
                  tickers_response.tickers.len()
                );
                return self.parse_coingecko_markets(tickers_response, symbol);
              }
              Err(e) => {
                error!("❌ JSON parse error for {}: {}", symbol.symbol, e);
                error!("❌ Problematic response: {}", response_text);
                return Err(LoaderError::SerializationError(format!(
                  "Failed to parse JSON for {}: {}",
                  symbol.symbol, e
                )));
              }
            }
          } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("❌ HTTP {} for {}: {}", status, symbol.symbol, error_text);

            // Handle rate limiting
            if status.as_u16() == 429 {
              retries += 1;
              if retries < self.config.max_retries {
                let delay = Duration::from_millis(self.config.rate_limit_delay_ms * retries as u64);
                warn!(
                  "Rate limited for {}. Waiting {:?} before retry {}/{}",
                  symbol.symbol,
                  delay,
                  retries + 1,
                  self.config.max_retries
                );
                sleep(delay).await;
                continue;
              }
            }

            return Err(LoaderError::ApiError(format!(
              "CoinGecko API error for {}: HTTP {} - {}",
              symbol.symbol, status, error_text
            )));
          }
        }
        Err(e) => {
          error!("❌ Network error for {}: {}", symbol.symbol, e);
          retries += 1;
          if retries < self.config.max_retries {
            let delay = Duration::from_millis(self.config.delay_ms * retries as u64);
            warn!(
              "Network error for {}. Waiting {:?} before retry {}/{}",
              symbol.symbol,
              delay,
              retries + 1,
              self.config.max_retries
            );
            sleep(delay).await;
            continue;
          }
          return Err(LoaderError::IoError(format!("Request failed for {}: {}", symbol.symbol, e)));
        }
      }
    }

    error!("❌ Max retries exceeded for {}", symbol.symbol);
    Err(LoaderError::ApiError(format!("Max retries exceeded for {}", symbol.symbol)))
  }

  fn parse_coingecko_markets(
    &self,
    response: CoinGeckoTickersResponse,
    symbol: &CryptoSymbolForMarkets,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    let mut markets = Vec::new();

    for ticker in response.tickers {
      let market_data = CryptoMarketData {
        sid: symbol.sid,
        exchange: ticker.market.name,
        base: ticker.base,
        target: ticker.target,
        market_type: Some("spot".to_string()),
        volume_24h: ticker.volume.map(|v| BigDecimal::try_from(v).unwrap_or_default()),
        volume_percentage: None, // Not provided by CoinGecko tickers
        bid_ask_spread_pct: ticker
          .bid_ask_spread_percentage
          .map(|s| BigDecimal::try_from(s).unwrap_or_default()),
        liquidity_score: None,
        trust_score: ticker.trust_score.map(|s| s.to_string()),
        is_active: ticker.market.has_trading_incentive.unwrap_or(true),
        is_anomaly: ticker.is_anomaly.unwrap_or(false),
        is_stale: ticker.is_stale.unwrap_or(false),
        last_price: ticker.last,
        last_traded_at: ticker.last_traded_at,
        last_fetch_at: Some(chrono::Utc::now().to_rfc3339()),
      };
      markets.push(market_data);
    }

    Ok(markets)
  }

  /// Clean expired cache entries
  pub async fn cleanup_expired_cache(database_url: &str) -> Result<usize, LoaderError> {
    let mut conn = PgConnection::establish(database_url)
      .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

    let deleted_count =
      diesel::sql_query("DELETE FROM api_response_cache WHERE expires_at < NOW()")
        .execute(&mut conn)
        .map_err(|e| LoaderError::DatabaseError(format!("Cleanup failed: {}", e)))?;

    if deleted_count > 0 {
      info!("🧹 Cleaned up {} expired cache entries", deleted_count);
    }

    Ok(deleted_count)
  }

  /// Check if cached response is valid - FIXED: Using proper struct for SQL result
  async fn get_cached_response(
    &self,
    database_url: &str,
    cache_key: &str,
  ) -> Option<CoinGeckoTickersResponse> {
    if !self.config.enable_response_cache {
      return None;
    }

    let mut conn = match PgConnection::establish(database_url) {
      Ok(conn) => conn,
      Err(_) => return None,
    };

    // FIXED: Use the proper struct instead of tuple
    let cached_entry: Option<CacheQueryResult> = diesel::sql_query(
      "SELECT response_data, expires_at FROM api_response_cache
             WHERE cache_key = $1 AND expires_at > NOW() AND api_source = 'coingecko'",
    )
    .bind::<diesel::sql_types::Text, _>(cache_key)
    .get_result(&mut conn)
    .optional()
    .unwrap_or(None);

    if let Some(cache_result) = cached_entry {
      info!("📦 Using cached response for {} (expires: {})", cache_key, cache_result.expires_at);

      // Parse cached JSON response
      if let Ok(cached_response) =
        serde_json::from_value::<CoinGeckoTickersResponse>(cache_result.response_data)
      {
        return Some(cached_response);
      } else {
        warn!("Failed to parse cached response for {}", cache_key);
      }
    }

    None
  }

  /// Save API response to cache - FIXED: Using proper error handling
  async fn cache_response(
    &self,
    database_url: &str,
    cache_key: &str,
    endpoint_url: &str,
    response: &CoinGeckoTickersResponse,
    status_code: u16,
  ) {
    if !self.config.enable_response_cache {
      return;
    }

    let mut conn = match PgConnection::establish(database_url) {
      Ok(conn) => conn,
      Err(e) => {
        warn!("Failed to connect to database for caching: {}", e);
        return;
      }
    };

    // FIXED: This now works because CoinGeckoTickersResponse implements Serialize
    let response_json = match serde_json::to_value(response) {
      Ok(json) => json,
      Err(e) => {
        warn!("Failed to serialize response for caching: {}", e);
        return;
      }
    };

    let expires_at =
      chrono::Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);

    // Insert or update cache entry
    let result = diesel::sql_query(
      "INSERT INTO api_response_cache
             (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
             VALUES ($1, 'coingecko', $2, $3, $4, $5)
             ON CONFLICT (cache_key) DO UPDATE SET
                response_data = EXCLUDED.response_data,
                status_code = EXCLUDED.status_code,
                expires_at = EXCLUDED.expires_at,
                cached_at = NOW()",
    )
    .bind::<diesel::sql_types::Text, _>(cache_key)
    .bind::<diesel::sql_types::Text, _>(endpoint_url)
    .bind::<diesel::sql_types::Jsonb, _>(&response_json)
    .bind::<diesel::sql_types::Integer, _>(status_code as i32)
    .bind::<diesel::sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn);

    match result {
      Ok(_) => info!("💾 Cached response for {} (expires: {})", cache_key, expires_at),
      Err(e) => warn!("Failed to cache response for {}: {}", cache_key, e),
    }
  }

  /// Enhanced fetch with caching
  async fn fetch_coingecko_markets_cached(
    &self,
    database_url: &str,
    coingecko_id: &str,
    symbol: &CryptoSymbolForMarkets,
  ) -> LoaderResult<Vec<CryptoMarketData>> {
    let cache_key = self.generate_cache_key(coingecko_id);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_response) = self.get_cached_response(database_url, &cache_key).await {
        info!("📦 Using cached market data for {}", symbol.symbol);
        return self.parse_coingecko_markets(cached_response, symbol);
      }
    }

    // Cache miss or force refresh - fetch from API
    info!("🌐 Fetching fresh market data for {} (cache miss)", symbol.symbol);

    match self.fetch_coingecko_markets_fresh(coingecko_id, symbol).await {
      Ok((markets, response, url, status)) => {
        // Cache the successful response
        self.cache_response(database_url, &cache_key, &url, &response, status).await;
        Ok(markets)
      }
      Err(e) => Err(e),
    }
  }

  /// Fetch fresh data from API (extracted from original fetch method)
  async fn fetch_coingecko_markets_fresh(
    &self,
    coingecko_id: &str,
    symbol: &CryptoSymbolForMarkets,
  ) -> LoaderResult<(Vec<CryptoMarketData>, CoinGeckoTickersResponse, String, u16)> {
    let base_url = "https://api.coingecko.com/api/v3";
    let mut url = format!("{}/coins/{}/tickers", base_url, coingecko_id);

    // Add API key if available
    if let Some(ref api_key) = self.config.coingecko_api_key {
      let auth_param = if api_key.starts_with("CG-") {
        url = format!("https://pro-api.coingecko.com/api/v3/coins/{}/tickers", coingecko_id);
        "x_cg_pro_api_key"
      } else {
        "x_cg_demo_api_key"
      };
      url = format!("{}?{}={}", url, auth_param, api_key);
    }

    let response = self
      .client
      .get(&url)
      .send()
      .await
      .map_err(|e| LoaderError::IoError(format!("Request failed: {}", e)))?;

    let status = response.status().as_u16();

    if response.status().is_success() {
      let tickers_response: CoinGeckoTickersResponse = response
        .json()
        .await
        .map_err(|e| LoaderError::SerializationError(format!("Parse failed: {}", e)))?;

      // FIXED: This now works because CoinGeckoTickersResponse implements Clone
      let markets = self.parse_coingecko_markets(tickers_response.clone(), symbol)?;
      Ok((markets, tickers_response, url, status))
    } else {
      Err(LoaderError::ApiError(format!("HTTP {}", status)))
    }
  }
}
