use super::EndpointBase;
use crate::impl_endpoint_base;
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::crypto::*;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
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
  /// * `market` - The market currency (e.g., "USD", "EUR")
  /// * `interval` - Time interval: "1min", "5min", "15min", "30min", "60min"
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let data = endpoints.intraday("BTC", "USD", "5min").await?;
  /// for (timestamp, price) in &data.time_series {
  ///     println!("{}: ${}", timestamp, price.close_usd);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market, interval))]
  pub async fn intraday(
    &self,
    symbol: &str,
    market: &str,
    interval: &str,
  ) -> Result<CryptoIntraday> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());
    params.insert("interval".to_string(), interval.to_string());

    self.transport.get(FuncType::CryptoIntraday, params).await
  }

  /// Get daily cryptocurrency time series data
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol (e.g., "BTC", "ETH")
  /// * `market` - The market currency (e.g., "USD", "EUR")
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let data = endpoints.daily("BTC", "USD").await?;
  /// for (date, price) in &data.time_series {
  ///     println!("{}: ${}", date, price.close_usd);
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
  /// * `symbol` - The cryptocurrency symbol (e.g., "BTC", "ETH")
  /// * `market` - The market currency (e.g., "USD", "EUR")
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
  /// * `symbol` - The cryptocurrency symbol (e.g., "BTC", "ETH")
  /// * `market` - The market currency (e.g., "USD", "EUR")
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn monthly(&self, symbol: &str, market: &str) -> Result<CryptoMonthly> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());
    params.insert("market".to_string(), market.to_string());

    self.transport.get(FuncType::CryptoMonthly, params).await
  }

  /// Calculate crypto health score based on price volatility and volume
  ///
  /// # Arguments
  ///
  /// * `symbol` - The cryptocurrency symbol
  /// * `market` - The market currency
  ///
  /// # Returns
  ///
  /// A health score between 0.0 and 100.0, where higher scores indicate better stability
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::CryptoEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = CryptoEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let health_score = endpoints.health_score("BTC", "USD").await?;
  /// println!("Bitcoin health score: {:.2}", health_score);
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(symbol, market))]
  pub async fn health_score(&self, symbol: &str, market: &str) -> Result<f64> {
    let data = self.daily(symbol, market).await?;

    // Extract closing prices for the last 30 days
    let prices: Vec<f64> =
      data.time_series.values().take(30).filter_map(|price| price.close_usd.parse().ok()).collect();

    if prices.len() < 10 {
      return Err(av_core::Error::Parse("Insufficient data for health analysis".to_string()));
    }

    // Calculate volatility (standard deviation)
    let mean = prices.iter().sum::<f64>() / prices.len() as f64;
    let variance = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
    let volatility = variance.sqrt();

    // Health score: lower volatility = higher health (inverted and normalized)
    let health_score: f64 = 100.0 - (volatility / mean * 100.0).min(100.0);

    Ok(health_score.max(0.0))
  }
}

impl_endpoint_base!(CryptoEndpoints);
