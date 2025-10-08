//! Summary price loader for daily TIME_SERIES_DAILY data
//!
//! This loader fetches daily OHLCV data from AlphaVantage and prepares it
//! for insertion into the summaryprices table.

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc, TimeZone};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::{DataLoader, LoaderContext, LoaderResult, LoaderError, process_tracker::ProcessState};
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
}

impl Default for SummaryPriceConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            update_existing: true,
            skip_non_trading_days: true,
        }
    }
}

/// Summary price loader implementation
pub struct SummaryPriceLoader {
    semaphore: Arc<Semaphore>,
    config: SummaryPriceConfig,
    next_eventid: Arc<AtomicI64>,
}

impl SummaryPriceLoader {
    /// Create a new summary price loader
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            config: SummaryPriceConfig {
                max_concurrent,
                ..Default::default()
            },
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
            let open = ohlcv.open.parse::<f32>()
                .map_err(|e| LoaderError::InvalidData(format!("Failed to parse open price: {}", e)))?;
            let high = ohlcv.high.parse::<f32>()
                .map_err(|e| LoaderError::InvalidData(format!("Failed to parse high price: {}", e)))?;
            let low = ohlcv.low.parse::<f32>()
                .map_err(|e| LoaderError::InvalidData(format!("Failed to parse low price: {}", e)))?;
            let close = ohlcv.close.parse::<f32>()
                .map_err(|e| LoaderError::InvalidData(format!("Failed to parse close price: {}", e)))?;
            let volume = ohlcv.volume.parse::<i64>()
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
        let mut skipped_count = 0;

        // Process symbols concurrently with semaphore limiting
        let client = context.client.clone();
        let semaphore = self.semaphore.clone();
        let outputsize = input.outputsize.clone();
        let progress_clone = progress.clone();

        let mut futures = stream::iter(input.symbols.into_iter().map(move |(sid, symbol)| {
            let client = client.clone();
            let semaphore = semaphore.clone();
            let outputsize = outputsize.clone();
            let progress = progress_clone.clone();

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(ref pb) = progress {
                    pb.set_message(format!("Loading {}", symbol));
                }

                match client.time_series().daily(&symbol, &outputsize).await {
                    Ok(daily_data) => {
                        debug!("Successfully fetched daily data for {}", symbol);
                        (sid, symbol, Ok(daily_data))
                    }
                    Err(e) => {
                        error!("Failed to fetch daily data for {}: {}", symbol, e);
                        (sid, symbol, Err(e))
                    }
                }
            }
        }))
            .buffer_unordered(self.config.max_concurrent);

        while let Some((sid, symbol, result)) = futures.next().await {
            match result {
                Ok(daily_data) => {
                    match self.process_symbol_data(sid, symbol.clone(), daily_data).await {
                        Ok(prices) => {
                            info!("Loaded {} price records for {}", prices.len(), symbol);
                            loaded_count += 1;
                            all_prices.extend(prices);
                        }
                        Err(e) => {
                            error!("Failed to process data for {}: {}", symbol, e);
                            error_count += 1;
                        }
                    }
                }
                Err(_) => {
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
            let state = if error_count > 0 {
                ProcessState::CompletedWithErrors
            } else {
                ProcessState::Success
            };
            tracker.complete(state).await?;
        }

        info!(
            "Summary price loading complete: {} symbols loaded, {} errors, {} skipped, {} total records",
            loaded_count, error_count, skipped_count, all_prices.len()
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
    fn test_create_timestamp() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let timestamp = SummaryPriceLoader::create_timestamp(date);

        // Should be 21:00 UTC (16:00 EST)
        assert_eq!(timestamp.hour(), 21);
        assert_eq!(timestamp.minute(), 0);
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