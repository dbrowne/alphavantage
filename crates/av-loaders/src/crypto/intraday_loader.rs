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

//! Crypto intraday price loader for CRYPTO_INTRADAY data using CSV format
//!
//! This loader fetches intraday cryptocurrency OHLCV data from AlphaVantage
//! in CSV format and prepares it for insertion into the intradayprices table.

use crate::{
  DataLoader, IntradayInterval, LoaderContext, LoaderError, LoaderResult,
  process_tracker::ProcessState,
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use csv::Reader;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Configuration for crypto intraday price loading
#[derive(Debug, Clone)]
pub struct CryptoIntradayConfig {
  /// Time interval between data points
  pub interval: IntradayInterval,
  /// Market/currency for pricing (e.g., "USD", "EUR")
  pub market: String,
  /// Output size ("compact" for 100 data points, "full" for full history)
  pub outputsize: String,
  /// Maximum number of concurrent API requests
  pub max_concurrent: usize,
  /// Whether to update existing records
  pub update_existing: bool,
  /// Delay between API calls in milliseconds (for rate limiting)
  pub api_delay_ms: u64,
  /// Enable response caching
  pub enable_cache: bool,
  /// Cache TTL in hours (shorter for intraday data)
  pub cache_ttl_hours: u64,
  /// Force refresh (bypass cache)
  pub force_refresh: bool,
  /// Database URL for caching
  pub database_url: String,
  /// Only load primary crypto symbols (priority != 9999999)
  pub primary_only: bool,
}

impl Default for CryptoIntradayConfig {
  fn default() -> Self {
    Self {
      interval: IntradayInterval::Min1, // Default to 1-minute intervals
      market: "USD".to_string(),
      outputsize: "compact".to_string(),
      max_concurrent: 5,
      update_existing: true,
      api_delay_ms: 800,
      enable_cache: true,
      cache_ttl_hours: 2,
      force_refresh: false,
      database_url: String::new(),
      primary_only: true,
    }
  }
}

/// Symbol information for crypto loading
#[derive(Debug, Clone)]
pub struct CryptoSymbolInfo {
  pub sid: i64,
  pub symbol: String,
  pub priority: i32,
}

/// Input for the crypto intraday price loader
#[derive(Debug, Clone)]
pub struct CryptoIntradayLoaderInput {
  /// Crypto symbols to load
  pub symbols: Vec<CryptoSymbolInfo>,
  /// Market/currency (e.g., "USD", "EUR")
  pub market: String,
  /// Interval for intraday data
  pub interval: String,
  /// Output size ("compact" or "full")
  pub outputsize: String,
}

/// Single crypto intraday price record
#[derive(Debug, Clone)]
pub struct CryptoIntradayPriceData {
  pub eventid: i64,
  pub tstamp: DateTime<Utc>,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
  pub price_source_id: i32,
}

/// Output from the crypto intraday price loader
#[derive(Debug, Clone)]
pub struct CryptoIntradayLoaderOutput {
  /// Loaded price data
  pub data: Vec<CryptoIntradayPriceData>,
  /// Number of symbols successfully loaded
  pub symbols_loaded: usize,
  /// Number of symbols that failed
  pub symbols_failed: usize,
  /// Number of symbols skipped
  pub symbols_skipped: usize,
  /// List of failed symbols
  pub failed_symbols: Vec<String>,
}

/// Crypto intraday price loader implementation
#[derive(Clone)]
pub struct CryptoIntradayLoader {
  semaphore: Arc<Semaphore>,
  config: CryptoIntradayConfig,
  next_eventid: Arc<AtomicI64>,
}

// Cache query result structure
#[derive(Debug, Clone, QueryableByName)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: chrono::DateTime<chrono::Utc>,
}

impl CryptoIntradayLoader {
  /// Create a new crypto intraday price loader
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      config: CryptoIntradayConfig { max_concurrent, ..Default::default() },
      next_eventid: Arc::new(AtomicI64::new(0)),
    }
  }

  /// Set configuration
  pub fn with_config(mut self, config: CryptoIntradayConfig) -> Self {
    let max_concurrent = config.max_concurrent;
    self.config = config;
    self.semaphore = Arc::new(Semaphore::new(max_concurrent));
    self
  }

  /// Initialize the next event ID from database max value
  pub fn with_starting_eventid(mut self, eventid: i64) -> Self {
    self.next_eventid = Arc::new(AtomicI64::new(eventid));
    self
  }

  /// Generate cache key for crypto intraday price requests
  fn generate_cache_key(&self, symbol: &str, market: &str, interval: &str) -> String {
    format!(
      "crypto_intraday_csv_{}_{}_{}_compact",
      symbol.to_uppercase(),
      market.to_uppercase(),
      interval.to_lowercase()
    )
  }

  /// Get cached CSV response if available and not expired
  async fn get_cached_csv(&self, cache_key: &str) -> Option<String> {
    if !self.config.enable_cache || self.config.force_refresh || self.config.database_url.is_empty()
    {
      return None;
    }

    let mut conn = match diesel::PgConnection::establish(&self.config.database_url) {
      Ok(conn) => conn,
      Err(e) => {
        debug!("Failed to connect for cache check: {}", e);
        return None;
      }
    };

    let cached_entry: Option<CacheQueryResult> = sql_query(
      "SELECT response_data, expires_at FROM api_response_cache
             WHERE cache_key = $1 AND expires_at > NOW() AND api_source = 'alphavantage'",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .get_result(&mut conn)
    .optional()
    .unwrap_or(None);

    if let Some(cache_result) = cached_entry {
      info!("📦 Cache hit for {} (expires: {})", cache_key, cache_result.expires_at);

      // Extract CSV data from JSON wrapper
      if let Some(csv_data) = cache_result.response_data.get("csv_data") {
        if let Some(csv_str) = csv_data.as_str() {
          return Some(csv_str.to_string());
        }
      }
    }

    debug!("Cache miss for {}", cache_key);
    None
  }

  /// Cache the CSV response
  async fn cache_csv_response(&self, cache_key: &str, csv_data: &str, symbol: &str) {
    if !self.config.enable_cache || self.config.database_url.is_empty() {
      return;
    }

    let mut conn = match diesel::PgConnection::establish(&self.config.database_url) {
      Ok(conn) => conn,
      Err(e) => {
        debug!("Failed to connect for caching: {}", e);
        return;
      }
    };

    // Wrap CSV data in JSON for storage
    let cache_value = serde_json::json!({
        "format": "csv",
        "csv_data": csv_data,
        "symbol": symbol,
        "cached_at": Utc::now()
    });

    let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);

    // Try to insert, if it fails due to duplicate, update instead
    let insert_result = sql_query(
      "INSERT INTO api_response_cache
             (cache_key, api_source, endpoint_url, response_data, status_code, expires_at, cached_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
    )
        .bind::<sql_types::Text, _>(cache_key)
        .bind::<sql_types::Text, _>("alphavantage")
        .bind::<sql_types::Text, _>(format!("CRYPTO_INTRADAY_CSV:{}", symbol))
        .bind::<sql_types::Jsonb, _>(cache_value.clone())
        .bind::<sql_types::Integer, _>(200)
        .bind::<sql_types::Timestamptz, _>(expires_at)
        .execute(&mut conn);

    match insert_result {
      Ok(_) => debug!("✅ Cached crypto CSV response for {} (expires: {})", symbol, expires_at),
      Err(_) => {
        // If insert failed, try update
        let update_result = sql_query(
          "UPDATE api_response_cache
                     SET response_data = $3, status_code = $4, expires_at = $5, cached_at = NOW()
                     WHERE cache_key = $1 AND api_source = $2",
        )
        .bind::<sql_types::Text, _>(cache_key)
        .bind::<sql_types::Text, _>("alphavantage")
        .bind::<sql_types::Jsonb, _>(cache_value)
        .bind::<sql_types::Integer, _>(200)
        .bind::<sql_types::Timestamptz, _>(expires_at)
        .execute(&mut conn);

        match update_result {
          Ok(_) => {
            debug!("✅ Updated cached crypto CSV response for {} (expires: {})", symbol, expires_at)
          }
          Err(e) => warn!("Failed to cache crypto CSV response: {}", e),
        }
      }
    }
  }

  /// Parse CSV data into price records
  fn parse_csv_data(
    &self,
    csv_data: &str,
    sid: i64,
    symbol: &str,
    price_source: i32,
  ) -> Result<Vec<CryptoIntradayPriceData>, LoaderError> {
    let mut reader = Reader::from_reader(csv_data.as_bytes());
    let mut prices = Vec::new();

    // Skip header row and process records
    for result in reader.records() {
      let record =
        result.map_err(|e| LoaderError::InvalidData(format!("Failed to parse CSV: {}", e)))?;

      // CSV columns: timestamp, open, high, low, close, volume
      let timestamp_str =
        record.get(0).ok_or_else(|| LoaderError::InvalidData("Missing timestamp".to_string()))?;

      // Parse timestamp
      let tstamp = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse timestamp: {}", e)))?;
      let tstamp = Utc.from_utc_datetime(&tstamp);

      // Parse OHLCV values
      let open = record
        .get(1)
        .ok_or_else(|| LoaderError::InvalidData("Missing open price".to_string()))?
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse open: {}", e)))?;

      let high = record
        .get(2)
        .ok_or_else(|| LoaderError::InvalidData("Missing high price".to_string()))?
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse high: {}", e)))?;

      let low = record
        .get(3)
        .ok_or_else(|| LoaderError::InvalidData("Missing low price".to_string()))?
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse low: {}", e)))?;

      let close = record
        .get(4)
        .ok_or_else(|| LoaderError::InvalidData("Missing close price".to_string()))?
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse close: {}", e)))?;

      let volume = record
        .get(5)
        .ok_or_else(|| LoaderError::InvalidData("Missing volume".to_string()))?
        .parse::<i64>()
        .unwrap_or(0);

      let eventid = self.next_eventid.fetch_add(1, Ordering::SeqCst);

      prices.push(CryptoIntradayPriceData {
        eventid,
        tstamp,
        sid,
        symbol: symbol.to_string(),
        open,
        high,
        low,
        close,
        volume,
        price_source_id: price_source,
      });
    }

    debug!("Parsed {} price records from CSV for {}", prices.len(), symbol);
    Ok(prices)
  }

  /// Fetch crypto intraday data in CSV format from API or cache
  async fn fetch_crypto_intraday_csv(
    &self,
    _context: &LoaderContext,
    symbol: &str,
    market: &str,
    interval: &str,
    sid: i64,
  ) -> Result<Vec<CryptoIntradayPriceData>, LoaderError> {
    // Generate cache key
    let cache_key = self.generate_cache_key(symbol, market, interval);

    // Check cache first
    if let Some(cached_csv) = self.get_cached_csv(&cache_key).await {
      debug!("Using cached CSV data for {} in {}", symbol, market);
      return self.parse_csv_data(&cached_csv, sid, symbol, 1);
    }

    // Acquire permit for rate limiting
    let _permit = self
      .semaphore
      .acquire()
      .await
      .map_err(|e| LoaderError::ApiError(format!("Failed to acquire permit: {}", e)))?;

    // Add delay for rate limiting
    if self.config.api_delay_ms > 0 {
      sleep(Duration::from_millis(self.config.api_delay_ms)).await;
    }

    // Build request URL with CSV format
    info!(
      "📡 Fetching crypto intraday CSV data for {} (market: {}, interval: {})",
      symbol, market, interval
    );

    // Get the API key from environment or configuration
    let api_key = std::env::var("ALPHA_VANTAGE_API_KEY")
      .map_err(|_| LoaderError::ApiError("ALPHA_VANTAGE_API_KEY not set".to_string()))?;

    let url = format!(
      "https://www.alphavantage.co/query?function=CRYPTO_INTRADAY&symbol={}&market={}&interval={}&outputsize={}&datatype=csv&apikey={}",
      symbol, market, interval, self.config.outputsize, api_key
    );

    // Make the request
    let response = reqwest::get(&url)
      .await
      .map_err(|e| LoaderError::ApiError(format!("Failed to fetch CSV data: {}", e)))?;

    if !response.status().is_success() {
      return Err(LoaderError::ApiError(format!(
        "API returned status {} for {}",
        response.status(),
        symbol
      )));
    }

    let csv_data = response
      .text()
      .await
      .map_err(|e| LoaderError::ApiError(format!("Failed to read response: {}", e)))?;

    // Check for API error messages
    if csv_data.contains("Error Message") || csv_data.contains("Invalid API call") {
      return Err(LoaderError::ApiError(format!("API error for {}: {}", symbol, csv_data)));
    }

    // Cache the CSV response
    self.cache_csv_response(&cache_key, &csv_data, symbol).await;

    // Parse and return the data
    self.parse_csv_data(&csv_data, sid, symbol, 1) //Alpha vantage is source 1
  }
}

#[async_trait]
impl DataLoader for CryptoIntradayLoader {
  type Input = CryptoIntradayLoaderInput;
  type Output = CryptoIntradayLoaderOutput;

  fn name(&self) -> &'static str {
    "CryptoIntradayPriceLoader"
  }

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Starting crypto intraday price loader for {} symbols", input.symbols.len());
    info!(
      "Configuration: market={}, interval={}, outputsize={}, format=CSV",
      input.market, input.interval, input.outputsize
    );

    // Validate interval
    let interval = IntradayInterval::from_str(&input.interval)
      .ok_or_else(|| LoaderError::InvalidData(format!("Invalid interval: {}", input.interval)))?;

    // Start process tracking if enabled
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        tracker.start("crypto_intraday_price_load").await?;
      }
    }

    // Set up progress bar
    let progress = ProgressBar::new(input.symbols.len() as u64);
    progress.set_style(
      ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("#>-"),
    );

    let mut all_prices = Vec::new();
    let mut symbols_loaded = 0;
    let mut symbols_failed = 0;
    let mut symbols_skipped = 0;
    let mut failed_symbols = Vec::new();

    // Filter symbols if primary_only is set
    let symbols_to_process: Vec<_> = if self.config.primary_only {
      input.symbols.into_iter().filter(|s| s.priority != 9999999).collect()
    } else {
      input.symbols
    };

    info!(
      "Processing {} symbols (primary_only={}, CSV format)",
      symbols_to_process.len(),
      self.config.primary_only
    );

    // Process symbols concurrently
    let mut tasks = stream::iter(symbols_to_process.into_iter())
      .map(|symbol_info| {
        let loader = self.clone();
        let context = context;
        let symbol = symbol_info.symbol.clone();
        let sid = symbol_info.sid;
        let interval_str = interval.as_str().to_string();
        let market = input.market.clone();
        let progress = progress.clone();

        async move {
          progress.set_message(format!("Loading {} ({})", symbol, market));

          match loader
            .fetch_crypto_intraday_csv(&context, &symbol, &market, &interval_str, sid)
            .await
          {
            Ok(prices) => {
              let count = prices.len();
              progress.inc(1);
              Ok((symbol, prices, count))
            }
            Err(e) => {
              error!("Failed to fetch data for {}: {}", symbol, e);
              progress.inc(1);
              Err((symbol, e))
            }
          }
        }
      })
      .buffer_unordered(self.config.max_concurrent);

    // Collect results
    while let Some(result) = tasks.next().await {
      match result {
        Ok((symbol, prices, count)) => {
          info!("✅ Loaded {} price points for {}", count, symbol);
          all_prices.extend(prices);
          symbols_loaded += 1;
        }
        Err((symbol, _e)) => {
          failed_symbols.push(symbol);
          symbols_failed += 1;
        }
      }
    }

    progress.finish_with_message("Crypto intraday loading complete");

    // Update process tracking
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        let state = if symbols_failed > 0 && symbols_loaded == 0 {
          ProcessState::Failed
        } else {
          ProcessState::Success
        };
        tracker.complete(state).await?;
      }
    }

    info!(
      "Crypto intraday loading complete: {} symbols loaded, {} failed, {} skipped",
      symbols_loaded, symbols_failed, symbols_skipped
    );

    Ok(CryptoIntradayLoaderOutput {
      data: all_prices,
      symbols_loaded,
      symbols_failed,
      symbols_skipped,
      failed_symbols,
    })
  }
}

// Re-export for convenience
pub use CryptoIntradayLoaderInput as CryptoIntradayInput;
pub use CryptoIntradayLoaderOutput as CryptoIntradayOutput;
