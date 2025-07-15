use super::EndpointBase;
use crate::impl_endpoint_base;
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::time_series::*;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Time series endpoints for historical and intraday price data
pub struct TimeSeriesEndpoints {
  transport: Arc<Transport>,
  rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl TimeSeriesEndpoints {
  /// Create a new time series endpoints instance
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
  ) -> Self {
    Self { transport, rate_limiter }
  }

  /// Get intraday time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The stock symbol (e.g., "AAPL", "MSFT")
  /// * `interval` - Time interval: "1min", "5min", "15min", "30min", "60min"
  /// * `month` - Optional month filter in YYYY-MM format for historical data
  /// * `adjusted` - Whether to include split/dividend adjustments
  /// * `extended_hours` - Whether to include extended trading hours
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::TimeSeriesEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get 5-minute intraday data for Apple
  /// let data = endpoints.intraday("AAPL", "5min").await?;
  ///
  /// // Get extended hours data
  /// let extended_data = endpoints.intraday_extended("AAPL", "1min", true, true).await?;
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, interval))]
  pub async fn intraday(&self, symbol: &str, interval: &str) -> Result<IntradayTimeSeries> {
    self.intraday_extended(symbol, interval, false, false).await
  }

  /// Get intraday time series data with extended options
  #[instrument(skip(self), fields(symbol, interval, adjusted, extended_hours))]
  pub async fn intraday_extended(
    &self,
    symbol: &str,
    interval: &str,
    adjusted: bool,
    extended_hours: bool,
  ) -> Result<IntradayTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("interval".to_string(), interval.to_string());
    params.insert("adjusted".to_string(), adjusted.to_string());
    params.insert("extended_hours".to_string(), extended_hours.to_string());

    self.transport.get(FuncType::TimeSeriesIntraday, params).await
  }

  /// Get daily time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The stock symbol
  /// * `outputsize` - "compact" for latest 100 data points, "full" for 20+ years
  #[instrument(skip(self), fields(symbol, outputsize))]
  pub async fn daily(&self, symbol: &str, outputsize: &str) -> Result<DailyTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("outputsize".to_string(), outputsize.to_string());

    self.transport.get(FuncType::TimeSeriesDaily, params).await
  }

  /// Get daily adjusted time series data (includes splits and dividends)
  #[instrument(skip(self), fields(symbol, outputsize))]
  pub async fn daily_adjusted(
    &self,
    symbol: &str,
    outputsize: &str,
  ) -> Result<DailyAdjustedTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("outputsize".to_string(), outputsize.to_string());

    self.transport.get(FuncType::TimeSeriesDailyAdjusted, params).await
  }

  /// Get weekly time series data
  #[instrument(skip(self), fields(symbol))]
  pub async fn weekly(&self, symbol: &str) -> Result<WeeklyTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());

    self.transport.get(FuncType::TimeSeriesWeekly, params).await
  }

  /// Get weekly adjusted time series data
  #[instrument(skip(self), fields(symbol))]
  pub async fn weekly_adjusted(&self, symbol: &str) -> Result<WeeklyAdjustedTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());

    self.transport.get(FuncType::TimeSeriesWeeklyAdjusted, params).await
  }

  /// Get monthly time series data
  #[instrument(skip(self), fields(symbol))]
  pub async fn monthly(&self, symbol: &str) -> Result<MonthlyTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());

    self.transport.get(FuncType::TimeSeriesMonthly, params).await
  }

  /// Get monthly adjusted time series data
  #[instrument(skip(self), fields(symbol))]
  pub async fn monthly_adjusted(&self, symbol: &str) -> Result<MonthlyAdjustedTimeSeries> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());

    self.transport.get(FuncType::TimeSeriesMonthlyAdjusted, params).await
  }

  /// Get market status (open/closed) for major trading venues
  #[instrument(skip(self))]
  pub async fn market_status(&self) -> Result<MarketStatus> {
    self.wait_for_rate_limit().await?;

    let params = HashMap::new();

    self.transport.get(FuncType::MarketStatus, params).await
  }

  /// Search for securities by keywords
  ///
  /// # Arguments
  ///
  /// * `keywords` - Search keywords for company name or ticker symbol
  #[instrument(skip(self), fields(keywords))]
  pub async fn symbol_search(&self, keywords: &str) -> Result<SymbolSearch> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("keywords".to_string(), keywords.to_string());

    self.transport.get(FuncType::SymbolSearch, params).await
  }
}

impl_endpoint_base!(TimeSeriesEndpoints);
