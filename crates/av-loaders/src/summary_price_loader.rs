//! Summary price loader for daily TIME_SERIES_DAILY data with CSV support
//!
//! This loader fetches daily OHLCV data from AlphaVantage in CSV format
//! and prepares it for insertion into the summaryprices table.

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use csv::Reader;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, process_tracker::ProcessState};

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
  #[diesel(sql_type = diesel::sql_types::Text)]
  response_data: String, // Store CSV data as text
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
    format!("daily_prices_csv_{}_{}", symbol.to_uppercase(), outputsize.to_lowercase())
  }

  /// Get cached CSV response if available and not expired
  async fn get_cached_csv(&self, cache_key: &str) -> Option<String> {
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
      "SELECT response_data::text as response_data, expires_at FROM api_response_cache
       WHERE cache_key = $1 AND expires_at > NOW() AND api_source = 'alphavantage'",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .get_result(&mut conn)
    .optional()
    .unwrap_or(None);

    if let Some(cache_result) = cached_entry {
      info!("📦 Cache hit for {} (expires: {})", cache_key, cache_result.expires_at);
      return Some(cache_result.response_data);
    }

    debug!("Cache miss for key: {}", cache_key);
    None
  }

  /// Cache the CSV response
  async fn cache_csv_response(&self, cache_key: &str, csv_data: &str, symbol: &str) {
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

    let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);
    let endpoint_url = format!(
      "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={}&datatype=csv",
      symbol
    );

    // Convert CSV data to JSON for storage
    let cache_value = serde_json::json!({
      "csv_data": csv_data,
      "format": "csv"
    });

    let insert_result = sql_query(
      "INSERT INTO api_response_cache
       (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
       VALUES ($1, $2, $3, $4, $5, $6)
       ON CONFLICT (cache_key) DO NOTHING",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .bind::<sql_types::Text, _>("alphavantage")
    .bind::<sql_types::Text, _>(&endpoint_url)
    .bind::<sql_types::Jsonb, _>(cache_value.clone())
    .bind::<sql_types::Integer, _>(200)
    .bind::<sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn);

    match insert_result {
      Ok(_) => info!("💾 Cached daily CSV prices for {} (expires: {})", symbol, expires_at),
      Err(_) => {
        // If insert failed due to conflict, try update
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
            info!("✅ Updated cached daily CSV prices for {} (expires: {})", symbol, expires_at)
          }
          Err(e) => warn!("Failed to cache daily CSV prices: {}", e),
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
  ) -> Result<Vec<SummaryPriceData>, LoaderError> {
    let mut reader = Reader::from_reader(csv_data.as_bytes());
    let mut prices = Vec::new();

    // Skip header row and process records
    for result in reader.records() {
      let record =
        result.map_err(|e| LoaderError::InvalidData(format!("Failed to parse CSV: {}", e)))?;

      // CSV columns for TIME_SERIES_DAILY: timestamp, open, high, low, close, volume
      let date_str =
        record.get(0).ok_or_else(|| LoaderError::InvalidData("Missing date".to_string()))?;

      // Parse date
      let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| {
        LoaderError::InvalidData(format!("Failed to parse date '{}': {}", date_str, e))
      })?;

      // Skip weekends if configured
      if self.config.skip_non_trading_days {
        let weekday = date.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
          continue;
        }
      }

      // Create timestamp (using market close time 16:00 EST = 21:00 UTC)
      let time = NaiveTime::from_hms_opt(21, 0, 0)
        .ok_or_else(|| LoaderError::InvalidData("Failed to create time".to_string()))?;
      let naive_dt = date.and_time(time);
      let tstamp = Utc.from_utc_datetime(&naive_dt);

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

      prices.push(SummaryPriceData {
        eventid,
        tstamp,
        date,
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

  /// Fetch daily prices in CSV format from API or cache
  async fn fetch_daily_csv(
    &self,
    context: &LoaderContext,
    symbol: &str,
    outputsize: &str,
    sid: i64,
  ) -> Result<Vec<SummaryPriceData>, LoaderError> {
    // Generate cache key
    let cache_key = self.generate_cache_key(symbol, outputsize);

    // Check cache first
    if let Some(cached_csv) = self.get_cached_csv(&cache_key).await {
      debug!("Using cached CSV data for {}", symbol);
      // Extract CSV data from JSON wrapper if needed
      if let Ok(cache_json) = serde_json::from_str::<serde_json::Value>(&cached_csv) {
        if let Some(csv_str) = cache_json.get("csv_data").and_then(|v| v.as_str()) {
          return self.parse_csv_data(csv_str, sid, symbol);
        }
      }
      // Fallback: assume it's raw CSV
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
    info!("📡 Fetching daily CSV data for {} (outputsize: {})", symbol, outputsize);

    // Get the API key from environment
    let api_key = std::env::var("ALPHA_VANTAGE_API_KEY")
      .map_err(|_| LoaderError::ApiError("ALPHA_VANTAGE_API_KEY not set".to_string()))?;

    let url = format!(
      "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={}&outputsize={}&datatype=csv&apikey={}",
      symbol, outputsize, api_key
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
    self.parse_csv_data(&csv_data, sid, symbol)
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
         AND cache_key LIKE 'daily_prices_csv_%'",
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
}

#[async_trait]
impl DataLoader for SummaryPriceLoader {
  type Input = SummaryPriceLoaderInput;
  type Output = SummaryPriceLoaderOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Starting summary price loader for {} symbols (CSV format)", input.symbols.len());
    info!("Configuration: outputsize={}", input.outputsize);

    // Start process tracking if enabled
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        tracker.start("summary_price_load").await?;
      }
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
      pb.set_message("Loading daily prices (CSV)");
      Some(pb)
    } else {
      None
    };

    let mut all_prices = Vec::new();
    let mut loaded_count = 0;
    let mut error_count = 0;
    let mut skipped_count = 0;

    // Process symbols sequentially with rate limiting
    for (sid, symbol) in input.symbols.iter() {
      if let Some(ref pb) = progress {
        pb.set_message(format!("Loading {}", symbol));
      }

      match self.fetch_daily_csv(context, symbol, &input.outputsize, *sid).await {
        Ok(prices) => {
          info!("✅ Loaded {} price records for {}", prices.len(), symbol);
          loaded_count += 1;
          all_prices.extend(prices);
        }
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

      if let Some(ref pb) = progress {
        pb.inc(1);
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Daily prices loading complete");
    }

    // Complete process tracking
    if context.config.track_process {
      if let Some(tracker) = &context.process_tracker {
        let state =
          if error_count > 0 { ProcessState::CompletedWithErrors } else { ProcessState::Success };
        tracker.complete(state).await?;
      }
    }

    info!(
      "Summary price loading complete: {} symbols loaded, {} errors, {} skipped, {} total records",
      loaded_count,
      error_count,
      skipped_count,
      all_prices.len()
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
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
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

  #[test]
  fn test_csv_parsing() {
    let csv_data = "timestamp,open,high,low,close,volume\n\
                    2024-01-15,100.5,102.3,99.8,101.2,1000000\n\
                    2024-01-16,101.2,103.0,100.5,102.5,1200000";

    let loader = SummaryPriceLoader::new(1);
    let result = loader.parse_csv_data(csv_data, 1, "TEST");

    assert!(result.is_ok());
    let prices = result.unwrap();
    assert_eq!(prices.len(), 2);

    assert_eq!(prices[0].date.to_string(), "2024-01-15");
    assert_eq!(prices[0].open, 100.5);
    assert_eq!(prices[0].volume, 1000000);

    assert_eq!(prices[1].date.to_string(), "2024-01-16");
    assert_eq!(prices[1].close, 102.5);
  }
}
