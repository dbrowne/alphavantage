//! Cryptocurrency endpoints
//!
//! This module provides access to AlphaVantage's cryptocurrency data:
//! - Real-time and historical cryptocurrency prices
//! - Daily, weekly, and monthly time series data
//! - Support for major cryptocurrencies like BTC, ETH, etc.
//! - Pricing in various fiat currencies

use super::{EndpointBase, impl_endpoint_base};
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::crypto::*;
use governor::{
  NotKeyed, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware, state::InMemoryState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Cryptocurrency endpoints
pub struct CryptoEndpoints {
  transport: Arc<Transport>,
  rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl CryptoEndpoints {
  /// Create a new crypto endpoints instance
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
  ) -> Self {
    Self { transport, rate_limiter }
  }

  /// Get current exchange rate for a cryptocurrency
  ///
  /// # Arguments
  ///
  /// * `from_currency` - The cryptocurrency symbol (e.g., "BTC", "ETH", "LTC")
  /// * `to_currency` - The target currency (e.g., "USD", "EUR", "CNY")
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get Bitcoin price in USD
  /// let btc_usd = endpoints.exchange_rate("BTC", "USD").await?;
  /// println!("Bitcoin price: ${}", btc_usd.exchange_rate);
  ///
  /// // Get Ethereum price in EUR
  /// let eth_eur = endpoints.exchange_rate("ETH", "EUR").await?;
  /// println!("Ethereum price: €{}", eth_eur.exchange_rate);
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(from_currency, to_currency))]
  pub async fn exchange_rate(
    &self,
    from_currency: &str,
    to_currency: &str,
  ) -> Result<CryptoExchangeRate> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("from_currency".to_string(), from_currency.to_string());
    params.insert("to_currency".to_string(), to_currency.to_string());

    self.transport.get(FuncType::CryptoExchangeRate, params).await
  }

  /// Get intraday cryptocurrency time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol (e.g., "BTC", "ETH")
  /// * `market` - The market/currency to price against (e.g., "USD", "EUR")
  /// * `interval` - Time interval: "1min", "5min", "15min", "30min", "60min"
  /// * `output_size` - "compact" (latest 100 data points) or "full"
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get 5-minute Bitcoin price data
  /// let data = endpoints.intraday("BTC", "USD", "5min", "compact").await?;
  ///
  /// // Print recent price action
  /// for (timestamp, price) in data.time_series.iter().take(10) {
  ///     println!("{}: ${} (Vol: {})",
  ///              timestamp, price.close, price.volume);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market, interval, output_size))]
  pub async fn intraday(
    &self,
    symbol: &str,
    market: &str,
    interval: &str,
    output_size: &str,
  ) -> Result<CryptoIntraday> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());
    params.insert("interval".to_string(), interval.to_string());
    params.insert("outputsize".to_string(), output_size.to_string());

    self.transport.get(FuncType::CryptoIntraday, params).await
  }

  /// Get daily cryptocurrency time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol
  /// * `market` - The market/currency to price against
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get daily Ethereum price data
  /// let data = endpoints.daily("ETH", "USD").await?;
  ///
  /// // Calculate recent volatility
  /// let prices: Vec<f64> = data.time_series.values()
  ///     .take(30)
  ///     .filter_map(|price| price.close.parse().ok())
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
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn daily(&self, symbol: &str, market: &str) -> Result<CryptoDaily> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());

    self.transport.get(FuncType::CryptoDaily, params).await
  }

  /// Get weekly cryptocurrency time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol
  /// * `market` - The market/currency to price against
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get weekly Bitcoin price data
  /// let data = endpoints.weekly("BTC", "USD").await?;
  ///
  /// // Find weeks with highest trading volume
  /// let mut volume_weeks: Vec<_> = data.time_series.iter()
  ///     .filter_map(|(week, price)| {
  ///         price.volume.parse::<f64>().ok().map(|vol| (week.clone(), vol))
  ///     })
  ///     .collect();
  ///
  /// volume_weeks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
  ///
  /// println!("Top 5 weeks by volume:");
  /// for (week, volume) in volume_weeks.iter().take(5) {
  ///     println!("  {}: {:.0} BTC", week, volume);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn weekly(&self, symbol: &str, market: &str) -> Result<CryptoWeekly> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());

    self.transport.get(FuncType::CryptoWeekly, params).await
  }

  /// Get monthly cryptocurrency time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol
  /// * `market` - The market/currency to price against
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get monthly Bitcoin price data
  /// let data = endpoints.monthly("BTC", "USD").await?;
  ///
  /// // Calculate year-over-year performance
  /// let prices: Vec<_> = data.time_series.iter().collect();
  /// if prices.len() >= 12 {
  ///     let current_price: f64 = prices[0].1.close.parse().unwrap_or(0.0);
  ///     let year_ago_price: f64 = prices[11].1.close.parse().unwrap_or(0.0);
  ///     
  ///     if year_ago_price > 0.0 {
  ///         let yoy_return = ((current_price - year_ago_price) / year_ago_price) * 100.0;
  ///         println!("Year-over-year return: {:.1}%", yoy_return);
  ///     }
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn monthly(&self, symbol: &str, market: &str) -> Result<CryptoMonthly> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());

    self.transport.get(FuncType::CryptoMonthly, params).await
  }

  /// Get cryptocurrency prices for multiple symbols
  ///
  /// This is a convenience method that makes multiple exchange rate calls.
  /// Note: Each cryptocurrency counts as a separate API call for rate limiting.
  ///
  /// # Arguments
  ///
  /// * `symbols` - List of cryptocurrency symbols
  /// * `market` - The market/currency to price against
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get prices for top cryptocurrencies
  /// let symbols = vec!["BTC", "ETH", "ADA", "DOT", "LINK"];
  /// let prices = endpoints.prices_bulk(symbols, "USD").await?;
  ///
  /// // Sort by market cap (price * circulating supply approximation)
  /// let mut crypto_prices = Vec::new();
  /// for price in prices {
  ///     crypto_prices.push((
  ///         price.from_currency_name.clone(),
  ///         price.exchange_rate.parse::<f64>().unwrap_or(0.0)
  ///     ));
  /// }
  ///
  /// crypto_prices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
  ///
  /// println!("Cryptocurrency prices (USD):");
  /// for (name, price) in crypto_prices {
  ///     println!("  {}: ${:.2}", name, price);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbols_count = symbols.len(), market))]
  pub async fn prices_bulk(
    &self,
    symbols: Vec<&str>,
    market: &str,
  ) -> Result<Vec<CryptoExchangeRate>> {
    let mut results = Vec::new();

    for symbol in symbols {
      let rate = self.exchange_rate(symbol, market).await?;
      results.push(rate);
    }

    Ok(results)
  }

  /// Get cryptocurrency health index for a symbol
  ///
  /// This is a utility method that analyzes recent price and volume data
  /// to provide a simple health indicator.
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol
  /// * `market` - The market/currency to price against
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get health metrics for Bitcoin
  /// let health = endpoints.health_index("BTC", "USD").await?;
  /// println!("Bitcoin Health Index:");
  /// println!("  Price Trend: {}", health.price_trend);
  /// println!("  Volume Trend: {}", health.volume_trend);
  /// println!("  Volatility: {:.2}%", health.volatility);
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn health_index(&self, symbol: &str, market: &str) -> Result<CryptoHealthIndex> {
    // Get recent daily data to analyze trends
    let data = self.daily(symbol, market).await?;

    let prices: Vec<f64> =
      data.time_series.values().take(30).filter_map(|price| price.close.parse().ok()).collect();

    let volumes: Vec<f64> =
      data.time_series.values().take(30).filter_map(|price| price.volume.parse().ok()).collect();

    if prices.len() < 5 || volumes.len() < 5 {
      return Err(av_core::Error::Parse("Insufficient data for health analysis".to_string()));
    }

    // Calculate price trend (simple moving average comparison)
    let recent_avg = prices[..5].iter().sum::<f64>() / 5.0;
    let older_avg = prices[10..15].iter().sum::<f64>() / 5.0;
    let price_trend = if recent_avg > older_avg * 1.02 {
      "Bullish".to_string()
    } else if recent_avg < older_avg * 0.98 {
      "Bearish".to_string()
    } else {
      "Neutral".to_string()
    };

    // Calculate volume trend
    let recent_vol_avg = volumes[..5].iter().sum::<f64>() / 5.0;
    let older_vol_avg = volumes[10..15].iter().sum::<f64>() / 5.0;
    let volume_trend = if recent_vol_avg > older_vol_avg * 1.1 {
      "Increasing".to_string()
    } else if recent_vol_avg < older_vol_avg * 0.9 {
      "Decreasing".to_string()
    } else {
      "Stable".to_string()
    };

    // Calculate volatility (standard deviation of returns)
    let mut daily_returns = Vec::new();
    for i in 1..prices.len().min(20) {
      let return_rate = (prices[i - 1] / prices[i] - 1.0) * 100.0;
      daily_returns.push(return_rate);
    }

    let avg_return: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    let variance: f64 = daily_returns.iter().map(|x| (x - avg_return).powi(2)).sum::<f64>()
      / daily_returns.len() as f64;
    let volatility = variance.sqrt();

    Ok(CryptoHealthIndex {
      symbol: symbol.to_string(),
      market: market.to_string(),
      price_trend,
      volume_trend,
      volatility,
      last_updated: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    })
  }
}

impl_endpoint_base!(CryptoEndpoints);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transport::Transport;
  use governor::{Quota, RateLimiter};
  use std::num::NonZeroU32;

  fn create_test_endpoints() -> CryptoEndpoints {
    let transport = Arc::new(Transport::new_mock());
    let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
    let rate_limiter = Arc::new(RateLimiter::direct(quota));

    CryptoEndpoints::new(transport, rate_limiter)
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
