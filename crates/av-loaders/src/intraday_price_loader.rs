//! Intraday price loader for TIME_SERIES_INTRADAY data
//!
//! This loader fetches intraday OHLCV data from AlphaVantage and prepares it
//! for insertion into the intradayprices table.

use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, process_tracker::ProcessState};
use async_trait::async_trait;
use av_models::time_series::IntradayTimeSeries;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntradayInterval {
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

impl Default for IntradayInterval {
  fn default() -> Self {
    IntradayInterval::Min1 // Default to 1-minute intervals
  }
}

/// Configuration for intraday price loading
#[derive(Debug, Clone)]
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
  /// Database URL for caching
  pub database_url: String,
}

impl Default for IntradayPriceConfig {
  fn default() -> Self {
    Self {
      interval: IntradayInterval::Min1, // Default to 1-minute
      extended_hours: true,             // Default to include extended hours
      adjusted: true,
      month: None,
      max_concurrent: 5,
      update_existing: true,
      api_delay_ms: 800, // 800ms for premium tier (75 calls/minute)
      enable_cache: true,
      cache_ttl_hours: 2, // 2-hour cache for intraday data
      force_refresh: false,
      database_url: String::new(),
    }
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

// Cache query result structure
#[derive(Debug, Clone, QueryableByName)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: chrono::DateTime<chrono::Utc>,
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
    "intraday".hash(&mut hasher);

    let month_str = month.unwrap_or("current");
    format!("intraday_{}_{}_{}_{:x}", symbol, interval, month_str, hasher.finish())
  }

  /// Get cached response if available and not expired
  async fn get_cached_response(&self, cache_key: &str) -> Option<IntradayTimeSeries> {
    if !self.config.enable_cache || self.config.force_refresh || self.config.database_url.is_empty()
    {
      return None;
    }

    let mut conn = match PgConnection::establish(&self.config.database_url) {
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

      match serde_json::from_value::<IntradayTimeSeries>(cache_result.response_data) {
        Ok(intraday_data) => {
          debug!("Successfully parsed cached intraday time series");
          return Some(intraday_data);
        }
        Err(e) => {
          warn!("Failed to parse cached intraday time series: {}", e);
          return None;
        }
      }
    }

    debug!("Cache miss for key: {}", cache_key);
    None
  }

  /// Cache the API response
  async fn cache_response(
    &self,
    cache_key: &str,
    intraday_data: &IntradayTimeSeries,
    symbol: &str,
  ) {
    if !self.config.enable_cache || self.config.database_url.is_empty() {
      return;
    }

    let mut conn = match PgConnection::establish(&self.config.database_url) {
      Ok(conn) => conn,
      Err(e) => {
        warn!("Failed to connect for caching: {}", e);
        return;
      }
    };

    let response_json = match serde_json::to_value(intraday_data) {
      Ok(json) => json,
      Err(e) => {
        warn!("Failed to serialize intraday data for caching: {}", e);
        return;
      }
    };

    let expires_at =
      chrono::Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);

    let result = sql_query(
            "INSERT INTO api_response_cache (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
             VALUES ($1, 'alphavantage', $2, $3, $4, $5)
             ON CONFLICT (cache_key)
             DO UPDATE SET
                response_data = EXCLUDED.response_data,
                expires_at = EXCLUDED.expires_at,
                cached_at = NOW()",
        )
            .bind::<sql_types::Text, _>(cache_key)
            .bind::<sql_types::Text, _>("")  // endpoint_url (we can leave empty for now)
            .bind::<sql_types::Jsonb, _>(&response_json)
            .bind::<sql_types::Integer, _>(200)  // status_code
            .bind::<sql_types::Timestamptz, _>(&expires_at)
            .execute(&mut conn);

    match result {
      Ok(_) => info!("💾 Cached intraday response for {} (expires: {})", symbol, expires_at),
      Err(e) => warn!("Failed to cache intraday response: {}", e),
    }
  }

  /// Clean up expired cache entries
  pub async fn cleanup_expired_cache(database_url: &str) -> Result<usize, LoaderError> {
    tokio::task::spawn_blocking({
      let db_url = database_url.to_string();
      move || -> Result<usize, LoaderError> {
        let mut conn = PgConnection::establish(&db_url)
          .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect: {}", e)))?;

        let deleted_count = sql_query(
          "DELETE FROM api_response_cache
                     WHERE expires_at < NOW() AND api_source = 'alphavantage'
                     AND cache_key LIKE 'intraday_%'",
        )
        .execute(&mut conn)
        .map_err(|e| LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e)))?;

        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired intraday cache entries", deleted_count);
        }

        Ok(deleted_count)
      }
    })
    .await
    .map_err(|e| LoaderError::DatabaseError(format!("Task join error: {}", e)))?
  }

  /// Parse timestamp string from API response
  fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>, LoaderError> {
    // AlphaVantage returns timestamps in format: "YYYY-MM-DD HH:MM:SS"
    // These are in US/Eastern time zone
    let naive_dt =
      NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
        LoaderError::InvalidData(format!("Failed to parse timestamp '{}': {}", timestamp_str, e))
      })?;

    // Convert from Eastern time to UTC
    // Note: This is a simplified conversion. In production, you'd want to handle DST properly
    // using a timezone library like chrono-tz
    let eastern_offset = chrono::FixedOffset::west_opt(5 * 3600).unwrap(); // EST is UTC-5
    let dt_eastern = eastern_offset.from_local_datetime(&naive_dt).unwrap();

    Ok(dt_eastern.with_timezone(&Utc))
  }

  /// Convert API response to internal data structure
  async fn process_symbol_data(
    &self,
    sid: i64,
    symbol: String,
    intraday_data: IntradayTimeSeries,
  ) -> Result<Vec<IntradayPriceData>, LoaderError> {
    let mut prices = Vec::new();

    for (timestamp_str, ohlcv) in intraday_data.time_series.iter() {
      // Parse the timestamp
      let tstamp = Self::parse_timestamp(timestamp_str)?;

      // Parse price values
      let open = ohlcv
        .open
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse open price: {}", e)))?;
      let high = ohlcv
        .high
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse high price: {}", e)))?;
      let low = ohlcv
        .low
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse low price: {}", e)))?;
      let close = ohlcv
        .close
        .parse::<f32>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse close price: {}", e)))?;
      let volume = ohlcv
        .volume
        .parse::<i64>()
        .map_err(|e| LoaderError::InvalidData(format!("Failed to parse volume: {}", e)))?;

      // Generate event ID
      let eventid = self.next_eventid.fetch_add(1, Ordering::SeqCst);

      prices.push(IntradayPriceData {
        eventid,
        tstamp,
        sid,
        symbol: symbol.clone(),
        open,
        high,
        low,
        close,
        volume,
      });
    }

    debug!("Processed {} intraday price points for {}", prices.len(), symbol);
    Ok(prices)
  }

  /// Fetch intraday data from API or cache
  async fn fetch_intraday_data(
    &self,
    context: &LoaderContext,
    symbol: &str,
    interval: &str,
    month: Option<&str>,
  ) -> Result<IntradayTimeSeries, LoaderError> {
    // Generate cache key
    let cache_key = self.generate_cache_key(symbol, interval, month);

    // Check cache first
    if let Some(cached_data) = self.get_cached_response(&cache_key).await {
      debug!("Using cached data for {}", symbol);
      return Ok(cached_data);
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

    // Call the API
    info!("📡 Fetching intraday data for {} (interval: {}, month: {:?})", symbol, interval, month);

    // For now, use the regular intraday method since intraday_extended might have issues
    let intraday_data =
      context.client.time_series().intraday(symbol, interval).await.map_err(|e| {
        LoaderError::ApiError(format!("Failed to fetch intraday data for {}: {}", symbol, e))
      })?;

    // Cache the response
    self.cache_response(&cache_key, &intraday_data, symbol).await;

    Ok(intraday_data)
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
    info!("Starting intraday price loader for {} symbols", input.symbols.len());
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

    // Process symbols concurrently - convert iterator to owned values to avoid lifetime issues
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

          match loader.fetch_intraday_data(&context, &symbol, &interval_str, month.as_deref()).await
          {
            Ok(intraday_data) => {
              match loader.process_symbol_data(sid, symbol.clone(), intraday_data).await {
                Ok(prices) => {
                  let count = prices.len();
                  progress.inc(1);
                  Ok((symbol, prices, count))
                }
                Err(e) => {
                  error!("Failed to process data for {}: {}", symbol, e);
                  progress.inc(1);
                  Err((symbol, e))
                }
              }
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
        Err((symbol, _error)) => {
          failed_symbols.push(symbol);
          symbols_failed += 1;
        }
      }
    }

    progress.finish_with_message(format!(
      "Completed: {} loaded, {} failed, {} skipped",
      symbols_loaded, symbols_failed, symbols_skipped
    ));

    // Update process tracking
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        let state = if symbols_failed > 0 {
          ProcessState::CompletedWithErrors
        } else {
          ProcessState::Success
        };
        tracker.complete(state).await?;
      }
    }

    Ok(IntradayPriceLoaderOutput {
      data: all_prices,
      symbols_loaded,
      symbols_failed,
      symbols_skipped,
      failed_symbols,
    })
  }
}
