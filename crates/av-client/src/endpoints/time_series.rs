//! Time series data endpoints for stock prices
//!
//! This module provides access to AlphaVantage's time series data including:
//! - Intraday prices (1min, 5min, 15min, 30min, 60min)
//! - Daily prices with full/compact output
//! - Weekly prices
//! - Monthly prices
//! - Adjusted prices with dividends and splits

use super::{impl_endpoint_base, EndpointBase};
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::time_series::*;
use governor::RateLimiter;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Time series endpoints for historical and intraday price data
pub struct TimeSeriesEndpoints {
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
}

impl TimeSeriesEndpoints {
    /// Create a new time series endpoints instance
    pub fn new(
        transport: Arc<Transport>,
        rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
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
        
        if adjusted {
            params.insert("adjusted".to_string(), "true".to_string());
        }
        
        if extended_hours {
            params.insert("extended_hours".to_string(), "true".to_string());
        }

        self.transport.get(FuncType::TimeSeriesIntraday, params).await
    }

    /// Get daily time series data
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    /// * `output_size` - "compact" (latest 100 data points) or "full" (up to 20 years)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get compact daily data (latest 100 days)
    /// let data = endpoints.daily("AAPL").await?;
    /// 
    /// // Get full historical data
    /// let full_data = endpoints.daily_full("AAPL").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn daily(&self, symbol: &str) -> Result<DailyTimeSeries> {
        self.daily_with_size(symbol, "compact").await
    }

    /// Get full daily time series data (up to 20 years)
    #[instrument(skip(self), fields(symbol))]
    pub async fn daily_full(&self, symbol: &str) -> Result<DailyTimeSeries> {
        self.daily_with_size(symbol, "full").await
    }

    /// Get daily time series data with specific output size
    #[instrument(skip(self), fields(symbol, output_size))]
    pub async fn daily_with_size(&self, symbol: &str, output_size: &str) -> Result<DailyTimeSeries> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("outputsize".to_string(), output_size.to_string());

        self.transport.get(FuncType::TimeSeriesDaily, params).await
    }

    /// Get daily adjusted time series data (includes dividends and splits)
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    /// * `output_size` - "compact" or "full"
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let adjusted_data = endpoints.daily_adjusted("AAPL", "compact").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol, output_size))]
    pub async fn daily_adjusted(&self, symbol: &str, output_size: &str) -> Result<DailyAdjustedTimeSeries> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("outputsize".to_string(), output_size.to_string());

        self.transport.get(FuncType::TimeSeriesDailyAdjusted, params).await
    }

    /// Get weekly time series data
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let weekly_data = endpoints.weekly("AAPL").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
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
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let monthly_data = endpoints.monthly("AAPL").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
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

    /// Get global market open/close status
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let market_status = endpoints.market_status().await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self))]
    pub async fn market_status(&self) -> Result<MarketStatus> {
        self.wait_for_rate_limit().await?;

        let params = HashMap::new();
        self.transport.get(FuncType::MarketStatus, params).await
    }

    /// Search for symbols matching a query
    ///
    /// # Arguments
    ///
    /// * `keywords` - Search keywords (company name, symbol, etc.)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::TimeSeriesEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = TimeSeriesEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let results = endpoints.symbol_search("Apple").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(keywords))]
    pub async fn symbol_search(&self, keywords: &str) -> Result<SymbolSearch> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("keywords".to_string(), keywords.to_string());

        self.transport.get(FuncType::SymbolSearch, params).await
    }
}

impl_endpoint_base!(TimeSeriesEndpoints);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;

    fn create_test_endpoints() -> TimeSeriesEndpoints {
        let transport = Arc::new(Transport::new_mock());
        let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        TimeSeriesEndpoints::new(transport, rate_limiter)
    }

    #[test]
    fn test_endpoints_creation() {
        let endpoints = create_test_endpoints();
        assert_eq!(endpoints.transport.base_url(), "https://mock.alphavantage.co");
    }

    #[tokio::test]
    async fn test_rate_limit_wait() {
        let endpoints = create_test_endpoints();
        let result = endpoints.wait_for_rate_limit().await;
        assert!(result.is_ok());
    }
}
