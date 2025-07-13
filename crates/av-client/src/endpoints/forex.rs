//! Foreign exchange (forex) endpoints
//!
//! This module provides access to AlphaVantage's foreign exchange data:
//! - Real-time exchange rates between currency pairs
//! - Historical intraday, daily, weekly, and monthly FX data
//! - Support for major and minor currency pairs

use super::{impl_endpoint_base, EndpointBase};
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::forex::*;
use governor::RateLimiter;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Foreign exchange (forex) endpoints
pub struct ForexEndpoints {
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
}

impl ForexEndpoints {
    /// Create a new forex endpoints instance
    pub fn new(
        transport: Arc<Transport>,
        rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
    ) -> Self {
        Self { transport, rate_limiter }
    }

    /// Get real-time exchange rate between two currencies
    ///
    /// # Arguments
    ///
    /// * `from_currency` - The base currency code (e.g., "USD", "EUR", "GBP")
    /// * `to_currency` - The target currency code (e.g., "EUR", "JPY", "CAD")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get USD to EUR exchange rate
    /// let rate = endpoints.exchange_rate("USD", "EUR").await?;
    /// println!("1 {} = {} {}", 
    ///          rate.from_currency_name, 
    ///          rate.exchange_rate, 
    ///          rate.to_currency_name);
    /// 
    /// // Get GBP to JPY exchange rate  
    /// let gbp_jpy = endpoints.exchange_rate("GBP", "JPY").await?;
    /// println!("Exchange rate: {} {}/{}", 
    ///          gbp_jpy.exchange_rate,
    ///          gbp_jpy.from_currency_code,
    ///          gbp_jpy.to_currency_code);
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(from_currency, to_currency))]
    pub async fn exchange_rate(&self, from_currency: &str, to_currency: &str) -> Result<ExchangeRate> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("from_currency".to_string(), from_currency.to_string());
        params.insert("to_currency".to_string(), to_currency.to_string());

        self.transport.get(FuncType::CurrencyExchangeRate, params).await
    }

    /// Get intraday forex time series data
    ///
    /// # Arguments
    ///
    /// * `from_symbol` - The base currency code
    /// * `to_symbol` - The target currency code  
    /// * `interval` - Time interval: "1min", "5min", "15min", "30min", "60min"
    /// * `output_size` - "compact" (latest 100 data points) or "full" (up to 30 days)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get 5-minute EUR/USD intraday data
    /// let data = endpoints.fx_intraday("EUR", "USD", "5min", "compact").await?;
    /// 
    /// // Print latest rates
    /// for (timestamp, rate) in data.time_series.iter().take(5) {
    ///     println!("{}: Open={}, High={}, Low={}, Close={}", 
    ///              timestamp, rate.open, rate.high, rate.low, rate.close);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(from_symbol, to_symbol, interval, output_size))]
    pub async fn fx_intraday(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        interval: &str,
        output_size: &str,
    ) -> Result<FxIntraday> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("from_symbol".to_string(), from_symbol.to_string());
        params.insert("to_symbol".to_string(), to_symbol.to_string());
        params.insert("interval".to_string(), interval.to_string());
        params.insert("outputsize".to_string(), output_size.to_string());

        self.transport.get(FuncType::FxIntraday, params).await
    }

    /// Get daily forex time series data
    ///
    /// # Arguments
    ///
    /// * `from_symbol` - The base currency code
    /// * `to_symbol` - The target currency code
    /// * `output_size` - "compact" (latest 100 data points) or "full" (up to 20 years)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get daily USD/JPY data
    /// let data = endpoints.fx_daily("USD", "JPY", "compact").await?;
    /// 
    /// // Calculate recent volatility
    /// let mut prices: Vec<f64> = data.time_series.values()
    ///     .take(30)
    ///     .map(|rate| rate.close.parse().unwrap_or(0.0))
    ///     .collect();
    /// 
    /// if prices.len() >= 2 {
    ///     let mut daily_returns = Vec::new();
    ///     for i in 1..prices.len() {
    ///         let return_rate = (prices[i-1] / prices[i] - 1.0) * 100.0;
    ///         daily_returns.push(return_rate);
    ///     }
    ///     
    ///     let avg_return: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    ///     let variance: f64 = daily_returns.iter()
    ///         .map(|x| (x - avg_return).powi(2))
    ///         .sum::<f64>() / daily_returns.len() as f64;
    ///     let volatility = variance.sqrt();
    ///     
    ///     println!("30-day volatility: {:.2}%", volatility);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(from_symbol, to_symbol, output_size))]
    pub async fn fx_daily(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        output_size: &str,
    ) -> Result<FxDaily> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("from_symbol".to_string(), from_symbol.to_string());
        params.insert("to_symbol".to_string(), to_symbol.to_string());
        params.insert("outputsize".to_string(), output_size.to_string());

        self.transport.get(FuncType::FxDaily, params).await
    }

    /// Get weekly forex time series data
    ///
    /// # Arguments
    ///
    /// * `from_symbol` - The base currency code
    /// * `to_symbol` - The target currency code
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get weekly GBP/USD data
    /// let data = endpoints.fx_weekly("GBP", "USD").await?;
    /// 
    /// // Find the week with highest volatility
    /// let mut max_volatility = 0.0;
    /// let mut max_week = String::new();
    /// 
    /// for (week, rate) in &data.time_series {
    ///     let high: f64 = rate.high.parse().unwrap_or(0.0);
    ///     let low: f64 = rate.low.parse().unwrap_or(0.0);
    ///     let volatility = ((high - low) / low) * 100.0;
    ///     
    ///     if volatility > max_volatility {
    ///         max_volatility = volatility;
    ///         max_week = week.clone();
    ///     }
    /// }
    /// 
    /// println!("Highest volatility week: {} ({:.2}%)", max_week, max_volatility);
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(from_symbol, to_symbol))]
    pub async fn fx_weekly(&self, from_symbol: &str, to_symbol: &str) -> Result<FxWeekly> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("from_symbol".to_string(), from_symbol.to_string());
        params.insert("to_symbol".to_string(), to_symbol.to_string());

        self.transport.get(FuncType::FxWeekly, params).await
    }

    /// Get monthly forex time series data
    ///
    /// # Arguments
    ///
    /// * `from_symbol` - The base currency code
    /// * `to_symbol` - The target currency code
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get monthly EUR/GBP data
    /// let data = endpoints.fx_monthly("EUR", "GBP").await?;
    /// 
    /// // Calculate year-over-year change
    /// let rates: Vec<_> = data.time_series.iter().collect();
    /// if rates.len() >= 12 {
    ///     let current_rate: f64 = rates[0].1.close.parse().unwrap_or(0.0);
    ///     let year_ago_rate: f64 = rates[11].1.close.parse().unwrap_or(0.0);
    ///     
    ///     if year_ago_rate > 0.0 {
    ///         let yoy_change = ((current_rate - year_ago_rate) / year_ago_rate) * 100.0;
    ///         println!("Year-over-year change: {:.2}%", yoy_change);
    ///     }
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(from_symbol, to_symbol))]
    pub async fn fx_monthly(&self, from_symbol: &str, to_symbol: &str) -> Result<FxMonthly> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("from_symbol".to_string(), from_symbol.to_string());
        params.insert("to_symbol".to_string(), to_symbol.to_string());

        self.transport.get(FuncType::FxMonthly, params).await
    }

    /// Get exchange rates for multiple currency pairs at once
    ///
    /// This is a convenience method that makes multiple exchange rate calls.
    /// Note: Each currency pair counts as a separate API call for rate limiting.
    ///
    /// # Arguments
    ///
    /// * `currency_pairs` - List of (from_currency, to_currency) tuples
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::ForexEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get rates for major USD pairs
    /// let pairs = vec![
    ///     ("USD", "EUR"),
    ///     ("USD", "GBP"), 
    ///     ("USD", "JPY"),
    ///     ("USD", "CHF"),
    /// ];
    /// 
    /// let rates = endpoints.exchange_rates_bulk(pairs).await?;
    /// 
    /// for rate in rates {
    ///     println!("{}/{}: {}", 
    ///              rate.from_currency_code,
    ///              rate.to_currency_code, 
    ///              rate.exchange_rate);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(pairs_count = currency_pairs.len()))]
    pub async fn exchange_rates_bulk(
        &self,
        currency_pairs: Vec<(&str, &str)>,
    ) -> Result<Vec<ExchangeRate>> {
        let mut results = Vec::new();
        
        for (from_currency, to_currency) in currency_pairs {
            let rate = self.exchange_rate(from_currency, to_currency).await?;
            results.push(rate);
        }
        
        Ok(results)
    }
}

impl_endpoint_base!(ForexEndpoints);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;

    fn create_test_endpoints() -> ForexEndpoints {
        let transport = Arc::new(Transport::new_mock());
        let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        ForexEndpoints::new(transport, rate_limiter)
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
