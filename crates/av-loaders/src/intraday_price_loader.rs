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

//! Intraday price loader for TIME_SERIES_INTRADAY data using CSV format

use crate::cache::{CacheConfigProvider, ttl};
use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, process_tracker::ProcessState};
use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use csv::Reader;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Supported intervals for intraday data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntradayInterval {
  #[default]
  Min1,
  Min5,
  Min15,
  Min30,
  Min60,
}

impl IntradayInterval {
  /// Parse from string
  pub fn from_str(s: &str) -> Option<Self> {
    match s {
      "1min" => Some(IntradayInterval::Min1),
      "5min" => Some(IntradayInterval::Min5),
      "15min" => Some(IntradayInterval::Min15),
      "30min" => Some(IntradayInterval::Min30),
      "60min" => Some(IntradayInterval::Min60),
      _ => None,
    }
  }

  /// Convert to string
  pub fn as_str(&self) -> &str {
    match self {
      IntradayInterval::Min1 => "1min",
      IntradayInterval::Min5 => "5min",
      IntradayInterval::Min15 => "15min",
      IntradayInterval::Min30 => "30min",
      IntradayInterval::Min60 => "60min",
    }
  }

  /// Get interval in minutes
  pub fn minutes(&self) -> u32 {
    match self {
      IntradayInterval::Min1 => 1,
      IntradayInterval::Min5 => 5,
      IntradayInterval::Min15 => 15,
      IntradayInterval::Min30 => 30,
      IntradayInterval::Min60 => 60,
    }
  }
}

/// Configuration for intraday price loading
#[derive(Clone)]
pub struct IntradayPriceConfig {
  /// Time interval between data points
  pub interval: IntradayInterval,
  /// Include extended trading hours (default: true)
  pub extended_hours: bool,
  /// Include split/dividend adjustments (default: true)
  pub adjusted: bool,
  /// Optional month for historical data (format: YYYY-MM)
  pub month: Option<String>,
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
  /// Cache repository for unified database access
  pub cache_repository: Option<Arc<dyn CacheRepository>>,
}

impl std::fmt::Debug for IntradayPriceConfig {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("IntradayPriceConfig")
      .field("interval", &self.interval)
      .field("extended_hours", &self.extended_hours)
      .field("adjusted", &self.adjusted)
      .field("month", &self.month)
      .field("max_concurrent", &self.max_concurrent)
      .field("update_existing", &self.update_existing)
      .field("api_delay_ms", &self.api_delay_ms)
      .field("enable_cache", &self.enable_cache)
      .field("cache_ttl_hours", &self.cache_ttl_hours)
      .field("force_refresh", &self.force_refresh)
      .field("cache_repository", &self.cache_repository.is_some())
      .finish()
  }
}

impl Default for IntradayPriceConfig {
  fn default() -> Self {
    Self {
      interval: IntradayInterval::Min1, // Default to 1-minute
      extended_hours: true,
      adjusted: true,
      month: None,
      max_concurrent: 5,
      update_existing: true,
      api_delay_ms: 800,
      enable_cache: true,
      cache_ttl_hours: ttl::INTRADAY_PRICES as u64,
      force_refresh: false,
      cache_repository: None,
    }
  }
}

impl CacheConfigProvider for IntradayPriceConfig {
  fn cache_enabled(&self) -> bool {
    self.enable_cache
  }

  fn cache_ttl_hours(&self) -> i64 {
    self.cache_ttl_hours as i64
  }

  fn force_refresh(&self) -> bool {
    self.force_refresh
  }
}

/// Symbol information for loading
#[derive(Debug, Clone)]
pub struct SymbolInfo {
  pub sid: i64,
  pub symbol: String,
}

/// Input for the intraday price loader
#[derive(Debug, Clone)]
pub struct IntradayPriceLoaderInput {
  /// Symbols to load
  pub symbols: Vec<SymbolInfo>,
  /// Interval for intraday data (defaults to 1min)
  pub interval: String,
  /// Include extended trading hours (defaults to true)
  pub extended_hours: bool,
  /// Include adjustments (defaults to true)
  pub adjusted: bool,
  /// Optional month for historical data
  pub month: Option<String>,
  pub output_size: String,
}

/// Single intraday price record
#[derive(Debug, Clone)]
pub struct IntradayPriceData {
  pub eventid: i64,
  pub tstamp: DateTime<Utc>,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
}

/// Output from the intraday price loader
#[derive(Debug, Clone)]
pub struct IntradayPriceLoaderOutput {
  /// Loaded price data
  pub data: Vec<IntradayPriceData>,
  /// Number of symbols successfully loaded
  pub symbols_loaded: usize,
  /// Number of symbols that failed
  pub symbols_failed: usize,
  /// Number of symbols skipped
  pub symbols_skipped: usize,
  /// List of failed symbols
  pub failed_symbols: Vec<String>,
}

/// Intraday price loader implementation
#[derive(Clone)]
pub struct IntradayPriceLoader {
  semaphore: Arc<Semaphore>,
  config: IntradayPriceConfig,
  next_eventid: Arc<AtomicI64>,
}


impl IntradayPriceLoader {
  /// Create a new intraday price loader
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      config: IntradayPriceConfig { max_concurrent, ..Default::default() },
      next_eventid: Arc::new(AtomicI64::new(0)),
    }
  }

  /// Set configuration
  pub fn with_config(mut self, config: IntradayPriceConfig) -> Self {
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

  /// Clean up expired cache entries
  pub async fn cleanup_expired_cache(
    cache_repository: &Arc<dyn CacheRepository>,
  ) -> Result<usize, LoaderError> {
    let deleted = cache_repository
      .cleanup_expired("alphavantage")
      .await
      .map_err(|e| LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e)))?;

    if deleted > 0 {
      info!("🧹 Cleaned up {} expired intraday price cache entries", deleted);
    }

    Ok(deleted)
  }

  /// Generate cache key for intraday price requests
  fn generate_cache_key(&self, symbol: &str, interval: &str, month: Option<&str>) -> String {
    let mut hasher = DefaultHasher::new();
    symbol.hash(&mut hasher);
    interval.hash(&mut hasher);
    if let Some(m) = month {
      m.hash(&mut hasher);
    } else {
      "current".hash(&mut hasher);
    }
    "equity_intraday_csv".hash(&mut hasher);

    let month_str = month.unwrap_or("current");
    format!("equity_intraday_csv_{}_{}_{}_{:x}", symbol, interval, month_str, hasher.finish())
  }

  /// Get cached CSV response if available and not expired
  async fn get_cached_csv(&self, cache_key: &str) -> Option<String> {
    if !self.config.enable_cache || self.config.force_refresh {
      return None;
    }

    let cache_repo = match &self.config.cache_repository {
      Some(repo) => repo,
      None => {
        debug!("No cache repository configured");
        return None;
      }
    };

    match cache_repo.get_json(cache_key, "alphavantage").await {
      Ok(Some(json_value)) => {
        info!("📦 Cache hit for {}", cache_key);
        // Extract CSV data from JSON wrapper
        if let Some(csv_data) = json_value.get("csv_data") {
          if let Some(csv_str) = csv_data.as_str() {
            return Some(csv_str.to_string());
          }
        }
        // Fallback: return the whole JSON as string
        Some(json_value.to_string())
      }
      Ok(None) => {
        debug!("Cache miss for {}", cache_key);
        None
      }
      Err(e) => {
        debug!("Cache lookup error for {}: {}", cache_key, e);
        None
      }
    }
  }

  /// Cache the CSV response
  async fn cache_csv_response(&self, cache_key: &str, csv_data: &str, symbol: &str) {
    if !self.config.enable_cache {
      return;
    }

    let cache_repo = match &self.config.cache_repository {
      Some(repo) => repo,
      None => {
        debug!("No cache repository configured for caching");
        return;
      }
    };

    let endpoint_url = format!("TIME_SERIES_INTRADAY_CSV:{}", symbol);

    // Wrap CSV data in JSON for storage
    let cache_value = serde_json::json!({
        "format": "csv",
        "csv_data": csv_data,
        "symbol": symbol,
        "cached_at": Utc::now()
    });

    match cache_repo
      .set_json(
        cache_key,
        "alphavantage",
        &endpoint_url,
        cache_value,
        self.config.cache_ttl_hours as i64,
      )
      .await
    {
      Ok(_) => debug!("💾 Cached equity intraday CSV for {}", symbol),
      Err(e) => warn!("Failed to cache equity intraday CSV for {}: {}", symbol, e),
    }
  }

  /// Parse CSV data into price records
  fn parse_csv_data(
    &self,
    csv_data: &str,
    sid: i64,
    symbol: &str,
  ) -> Result<Vec<IntradayPriceData>, LoaderError> {
    let mut reader = Reader::from_reader(csv_data.as_bytes());
    let mut prices = Vec::new();

    // Skip header row and process records
    for result in reader.records() {
      let record =
        result.map_err(|e| LoaderError::InvalidData(format!("Failed to parse CSV: {}", e)))?;

      // CSV columns for TIME_SERIES_INTRADAY: time, open, high, low, close, volume
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

      prices.push(IntradayPriceData {
        eventid,
        tstamp,
        sid,
        symbol: symbol.to_string(),
        open,
        high,
        low,
        close,
        volume,
      });
    }

    debug!("Parsed {} price records from CSV for {}", prices.len(), symbol);
    Ok(prices)
  }

  /// Fetch intraday data in CSV format from API or cache
  async fn fetch_intraday_csv(
    &self,
    _context: &LoaderContext,
    symbol: &str,
    interval: &str,
    month: Option<&str>,
    sid: i64,
  ) -> Result<Vec<IntradayPriceData>, LoaderError> {
    // Generate cache key
    let cache_key = self.generate_cache_key(symbol, interval, month);

    // Check cache first
    if let Some(cached_csv) = self.get_cached_csv(&cache_key).await {
      debug!("Using cached CSV data for {}", symbol);
      return self.parse_csv_data(&cached_csv, sid, symbol);
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
      "📡 Fetching equity intraday CSV data for {} (interval: {}, month: {:?})",
      symbol, interval, month
    );

    // Get the API key from environment
    let api_key = std::env::var("ALPHA_VANTAGE_API_KEY")
      .map_err(|_| LoaderError::ApiError("ALPHA_VANTAGE_API_KEY not set".to_string()))?;

    let mut url = format!(
      "https://www.alphavantage.co/query?function=TIME_SERIES_INTRADAY&symbol={}&interval={}&datatype=csv&apikey={}",
      symbol, interval, api_key
    );

    // Add optional parameters
    if let Some(m) = month {
      url.push_str(&format!("&month={}", m));
    }

    if self.config.adjusted {
      url.push_str("&adjusted=true");
    }

    if self.config.extended_hours {
      url.push_str("&extended_hours=true");
    }

    // Add output size
    url.push_str(&format!("&outputsize={}", if month.is_some() { "full" } else { "compact" }));

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
    self.parse_csv_data(&csv_data, sid, symbol)
  }
}

#[async_trait]
impl DataLoader for IntradayPriceLoader {
  type Input = IntradayPriceLoaderInput;
  type Output = IntradayPriceLoaderOutput;

  fn name(&self) -> &'static str {
    "IntradayPriceLoader"
  }

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Starting intraday price loader for {} symbols (CSV format)", input.symbols.len());
    info!(
      "Configuration: interval={}, extended_hours={}, adjusted={}, month={:?}",
      input.interval, input.extended_hours, input.adjusted, input.month
    );

    // Validate interval
    let interval = IntradayInterval::from_str(&input.interval)
      .ok_or_else(|| LoaderError::InvalidData(format!("Invalid interval: {}", input.interval)))?;

    // Start process tracking if enabled
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        tracker.start("intraday_price_load").await?;
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
    let symbols_skipped = 0;
    let mut failed_symbols = Vec::new();

    // Process symbols concurrently
    let symbols_owned: Vec<_> = input.symbols.into_iter().collect();
    let mut tasks = stream::iter(symbols_owned.into_iter())
      .map(|symbol_info| {
        let loader = self.clone();
        let context = context;
        let symbol = symbol_info.symbol.clone();
        let sid = symbol_info.sid;
        let interval_str = interval.as_str().to_string();
        let month = input.month.clone();
        let progress = progress.clone();

        async move {
          progress.set_message(format!("Loading {}", symbol));

          match loader
            .fetch_intraday_csv(context, &symbol, &interval_str, month.as_deref(), sid)
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

    progress.finish_with_message("Intraday loading complete");

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
      "Intraday loading complete: {} symbols loaded, {} failed, {} skipped",
      symbols_loaded, symbols_failed, symbols_skipped
    );

    Ok(IntradayPriceLoaderOutput {
      data: all_prices,
      symbols_loaded,
      symbols_failed,
      symbols_skipped,
      failed_symbols,
    })
  }
}

// Re-export type alias for compatibility
pub use SymbolInfo as IntradaySymbolInfo;
