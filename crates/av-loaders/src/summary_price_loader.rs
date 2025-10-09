//! Summary price loader for daily TIME_SERIES_DAILY data
//!
//! This loader fetches daily OHLCV data from AlphaVantage and prepares it
//! for insertion into the summaryprices table.

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
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

use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, process_tracker::ProcessState};
use av_models::time_series::DailyTimeSeries;

/// Configuration for summary price loading
#[derive(Debug, Clone)]
pub struct SummaryPriceConfig {
  /// Maximum number of concurrent API requests
  pub max_concurrent: usize,
  /// Whether to update existing records
  pub update_existing: bool,
  /// Whether to skip weekends and holidays
  pub skip_non_trading_days: bool,
  /// Delay between API calls in milliseconds (for rate limiting)
  pub api_delay_ms: u64,
  /// Enable response caching
  pub enable_cache: bool,
  /// Cache TTL in hours
  pub cache_ttl_hours: u64,
  /// Force refresh (bypass cache)
  pub force_refresh: bool,
  /// Database URL for caching
  pub database_url: String,
}

impl Default for SummaryPriceConfig {
  fn default() -> Self {
    Self {
      max_concurrent: 5,
      update_existing: true,
      skip_non_trading_days: true,
      api_delay_ms: 800, // 800ms for premium tier (75 calls/minute)
      enable_cache: true,
      cache_ttl_hours: 24, // Cache for 24 hours
      force_refresh: false,
      database_url: String::new(),
    }
  }
}

/// Summary price loader implementation
#[derive(Clone)]
pub struct SummaryPriceLoader {
  semaphore: Arc<Semaphore>,
  config: SummaryPriceConfig,
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

impl SummaryPriceLoader {
  /// Create a new summary price loader
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      config: SummaryPriceConfig { max_concurrent, ..Default::default() },
      next_eventid: Arc::new(AtomicI64::new(0)),
    }
  }

  /// Set configuration
  pub fn with_config(mut self, config: SummaryPriceConfig) -> Self {
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

  /// Generate cache key for daily price requests
  fn generate_cache_key(&self, symbol: &str, outputsize: &str) -> String {
    let mut hasher = DefaultHasher::new();
    symbol.hash(&mut hasher);
    outputsize.hash(&mut hasher);
    "daily".hash(&mut hasher); // Add function type to make it unique
    format!("daily_prices_{:x}", hasher.finish())
  }

  /// Get cached response if available and not expired
  async fn get_cached_response(&self, cache_key: &str) -> Option<DailyTimeSeries> {
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

      match serde_json::from_value::<DailyTimeSeries>(cache_result.response_data) {
        Ok(daily_data) => {
          debug!("Successfully parsed cached daily time series");
          return Some(daily_data);
        }
        Err(e) => {
          warn!("Failed to parse cached daily time series: {}", e);
          return None;
        }
      }
    }

    debug!("Cache miss for key: {}", cache_key);
    None
  }

  /// Cache the API response
  async fn cache_response(&self, cache_key: &str, daily_data: &DailyTimeSeries, symbol: &str) {
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

    let response_json = match serde_json::to_value(daily_data) {
      Ok(json) => json,
      Err(e) => {
        warn!("Failed to serialize daily data for caching: {}", e);
        return;
      }
    };

    let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);
    let endpoint_url =
      format!("https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={}", symbol);

    let result = sql_query(
      "INSERT INTO api_response_cache
             (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
             VALUES ($1, 'alphavantage', $2, $3, 200, $4)
             ON CONFLICT (cache_key) DO UPDATE SET
                response_data = EXCLUDED.response_data,
                status_code = EXCLUDED.status_code,
                expires_at = EXCLUDED.expires_at,
                cached_at = NOW()",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .bind::<sql_types::Text, _>(&endpoint_url)
    .bind::<sql_types::Jsonb, _>(&response_json)
    .bind::<sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn);

    match result {
      Ok(_) => info!("💾 Cached daily prices for {} (expires: {})", symbol, expires_at),
      Err(e) => warn!("Failed to cache daily prices: {}", e),
    }
  }

  /// Clean expired cache entries
  pub async fn cleanup_expired_cache(database_url: &str) -> Result<usize, LoaderError> {
    use tokio::task;

    let db_url = database_url.to_string();

    task::spawn_blocking(move || -> Result<usize, LoaderError> {
      let mut conn = PgConnection::establish(&db_url)
        .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

      let deleted_count = sql_query(
        "DELETE FROM api_response_cache
                 WHERE expires_at < NOW() AND api_source = 'alphavantage'
                 AND cache_key LIKE 'daily_prices_%'",
      )
      .execute(&mut conn)
      .map_err(|e| LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e)))?;

      if deleted_count > 0 {
        info!("🧹 Cleaned up {} expired daily price cache entries", deleted_count);
      }

      Ok(deleted_count)
    })
    .await
    .map_err(|e| LoaderError::DatabaseError(format!("Task join error: {}", e)))?
  }

  /// Parse date string from API response
  fn parse_date(date_str: &str) -> Result<NaiveDate, LoaderError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
      .map_err(|e| LoaderError::InvalidData(format!("Failed to parse date '{}': {}", date_str, e)))
  }

  /// Create timestamp from date (using market close time 16:00 EST)
  fn create_timestamp(date: NaiveDate) -> chrono::DateTime<Utc> {
    // Use 16:00 (4 PM) EST as the standard market close time
    // EST is UTC-5, so 16:00 EST = 21:00 UTC
    let time = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
    let naive_dt = date.and_time(time);
    Utc.from_utc_datetime(&naive_dt)
  }

  /// Convert API response to internal data structure
  async fn process_symbol_data(
    &self,
    sid: i64,
    symbol: String,
    daily_data: DailyTimeSeries,
  ) -> Result<Vec<SummaryPriceData>, LoaderError> {
    let mut prices = Vec::new();

    for (date_str, ohlcv) in daily_data.time_series.iter() {
      // Parse the date
      let date = Self::parse_date(date_str)?;

      // Skip weekends if configured
      if self.config.skip_non_trading_days {
        let weekday = date.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
          continue;
        }
      }

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

      prices.push(SummaryPriceData {
        eventid,
        tstamp: Self::create_timestamp(date),
        date,
        sid,
        symbol: symbol.clone(),
        open,
        high,
        low,
        close,
        volume,
      });
    }

    Ok(prices)
  }
}

#[async_trait]
impl DataLoader for SummaryPriceLoader {
  type Input = SummaryPriceLoaderInput;
  type Output = SummaryPriceLoaderOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Starting summary price loader for {} symbols", input.symbols.len());

    // Start process tracking if enabled
    if let Some(tracker) = &context.process_tracker {
      tracker.start("summary_price_loader").await?;
    }

    // Set up progress bar
    let progress = if context.config.show_progress {
      let pb = ProgressBar::new(input.symbols.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
          .unwrap()
          .progress_chars("##-"),
      );
      pb.set_message("Loading daily prices");
      Some(pb)
    } else {
      None
    };

    let mut all_prices = Vec::new();
    let mut loaded_count = 0;
    let mut error_count = 0;
    let skipped_count = 0;
    let mut cache_hits = 0;
    let mut api_calls = 0;

    // Process symbols concurrently with semaphore limiting
    let client = context.client.clone();
    let semaphore = self.semaphore.clone();
    let outputsize = input.outputsize.clone();
    let api_delay_ms = self.config.api_delay_ms;
    let progress_clone = progress.clone();
    let config = self.config.clone();
    let self_clone = self.clone();

    let mut futures =
      stream::iter(input.symbols.into_iter().enumerate().map(move |(idx, (sid, symbol))| {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let outputsize = outputsize.clone();
        let progress = progress_clone.clone();
        let delay_ms = api_delay_ms;
        let loader = self_clone.clone();

        async move {
          let _permit = semaphore.acquire().await.unwrap();

          if let Some(ref pb) = progress {
            pb.set_message(format!("Loading {}", symbol));
          }

          // Generate cache key
          let cache_key = loader.generate_cache_key(&symbol, &outputsize);

          // Try to get from cache first
          if let Some(cached_data) = loader.get_cached_response(&cache_key).await {
            debug!("Using cached data for {}", symbol);
            return (sid, symbol, Ok(cached_data), true);
          }

          // Add delay for rate limiting if configured (skip for first symbol)
          if idx > 0 && delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms)).await;
          }

          // Fetch from API
          match client.time_series().daily(&symbol, &outputsize).await {
            Ok(daily_data) => {
              debug!("Successfully fetched daily data for {}", symbol);

              // Cache the successful response
              loader.cache_response(&cache_key, &daily_data, &symbol).await;

              (sid, symbol, Ok(daily_data), false)
            }
            Err(e) => {
              error!("Failed to fetch daily data for {}: {}", symbol, e);
              (sid, symbol, Err(e), false)
            }
          }
        }
      }))
      .buffer_unordered(config.max_concurrent);

    while let Some((sid, symbol, result, from_cache)) = futures.next().await {
      if from_cache {
        cache_hits += 1;
      } else {
        api_calls += 1;
      }

      match result {
        Ok(daily_data) => match self.process_symbol_data(sid, symbol.clone(), daily_data).await {
          Ok(prices) => {
            info!(
              "Loaded {} price records for {} ({})",
              prices.len(),
              symbol,
              if from_cache { "cached" } else { "fresh" }
            );
            loaded_count += 1;
            all_prices.extend(prices);
          }
          Err(e) => {
            error!("Failed to process data for {}: {}", symbol, e);
            error_count += 1;
          }
        },
        Err(e) => {
          // Log specific error for debugging
          if e.to_string().contains("rate limit") || e.to_string().contains("API call frequency") {
            error!("Rate limit hit for {}: {}. Consider reducing concurrent requests.", symbol, e);
          } else if e.to_string().contains("Invalid API call")
            || e.to_string().contains("Error Message")
          {
            error!("Invalid symbol or no data for {}: {}", symbol, e);
          } else {
            error!("API error for {}: {}", symbol, e);
          }
          error_count += 1;
        }
      }

      if let Some(ref pb) = &progress {
        pb.inc(1);
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Daily prices loading complete");
    }

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      let state =
        if error_count > 0 { ProcessState::CompletedWithErrors } else { ProcessState::Success };
      tracker.complete(state).await?;
    }

    info!(
      "Summary price loading complete: {} symbols loaded, {} errors, {} skipped, {} total records",
      loaded_count,
      error_count,
      skipped_count,
      all_prices.len()
    );
    info!(
      "Cache statistics: {} hits, {} API calls ({}% cache hit rate)",
      cache_hits,
      api_calls,
      if cache_hits + api_calls > 0 { (cache_hits * 100) / (cache_hits + api_calls) } else { 0 }
    );

    Ok(SummaryPriceLoaderOutput {
      data: all_prices,
      symbols_loaded: loaded_count,
      symbols_failed: error_count,
      symbols_skipped: skipped_count,
    })
  }

  fn name(&self) -> &'static str {
    "SummaryPriceLoader"
  }
}

/// Input for summary price loader
#[derive(Debug, Clone)]
pub struct SummaryPriceLoaderInput {
  /// List of (sid, symbol) pairs to load
  pub symbols: Vec<(i64, String)>,
  /// Output size: "compact" (100 days) or "full" (20+ years)
  pub outputsize: String,
}

/// Individual summary price data record
#[derive(Debug, Clone)]
pub struct SummaryPriceData {
  pub eventid: i64,
  pub tstamp: chrono::DateTime<chrono::Utc>,
  pub date: NaiveDate,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
}

/// Output from summary price loader
#[derive(Debug)]
pub struct SummaryPriceLoaderOutput {
  /// All price data records
  pub data: Vec<SummaryPriceData>,
  /// Number of symbols successfully loaded
  pub symbols_loaded: usize,
  /// Number of symbols that failed
  pub symbols_failed: usize,
  /// Number of symbols skipped
  pub symbols_skipped: usize,
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Datelike;

  #[test]
  fn test_parse_date() {
    let date_str = "2024-01-15";
    let date = SummaryPriceLoader::parse_date(date_str).unwrap();
    assert_eq!(date.year(), 2024);
    assert_eq!(date.month(), 1);
    assert_eq!(date.day(), 15);
  }

  #[test]
  fn test_skip_weekend() {
    let saturday = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
    let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

    assert_eq!(saturday.weekday(), chrono::Weekday::Sat);
    assert_eq!(sunday.weekday(), chrono::Weekday::Sun);
    assert_eq!(monday.weekday(), chrono::Weekday::Mon);
  }
}
