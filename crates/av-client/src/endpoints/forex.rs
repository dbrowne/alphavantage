use super::EndpointBase;
use crate::impl_endpoint_base;

use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::forex::*;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Foreign exchange (forex) endpoints
pub struct ForexEndpoints {
  transport: Arc<Transport>,
  rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl ForexEndpoints {
  /// Create a new forex endpoints instance
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
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
  pub async fn exchange_rate(
    &self,
    from_currency: &str,
    to_currency: &str,
  ) -> Result<ExchangeRate> {
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
  /// * `from_symbol` - The base currency (e.g., "EUR", "GBP")
  /// * `to_symbol` - The quote currency (e.g., "USD", "JPY")
  /// * `interval` - Time interval: "1min", "5min", "15min", "30min", "60min"
  /// * `outputsize` - "compact" for latest 100 data points, "full" for full history
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::ForexEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let data = endpoints.intraday("EUR", "USD", "5min", "compact").await?;
  /// for (timestamp, fx_data) in &data.time_series {
  ///     println!("{}: {}", timestamp, fx_data.close);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(from_symbol, to_symbol, interval, outputsize))]
  pub async fn intraday(
    &self,
    from_symbol: &str,
    to_symbol: &str,
    interval: &str,
    outputsize: &str,
  ) -> Result<FxIntraday> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("from_symbol".to_string(), from_symbol.to_string());
    params.insert("to_symbol".to_string(), to_symbol.to_string());
    params.insert("interval".to_string(), interval.to_string());
    params.insert("outputsize".to_string(), outputsize.to_string());

    self.transport.get(FuncType::FxIntraday, params).await
  }

  /// Get daily forex time series data
  ///
  /// # Arguments
  ///
  /// * `from_symbol` - The base currency (e.g., "EUR", "GBP")
  /// * `to_symbol` - The quote currency (e.g., "USD", "JPY")
  /// * `outputsize` - "compact" for latest 100 data points, "full" for full history
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::ForexEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = ForexEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let data = endpoints.daily("EUR", "USD", "compact").await?;
  /// for (date, fx_data) in &data.time_series {
  ///     println!("{}: {}", date, fx_data.close);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(from_symbol, to_symbol, outputsize))]
  pub async fn daily(
    &self,
    from_symbol: &str,
    to_symbol: &str,
    outputsize: &str,
  ) -> Result<FxDaily> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("from_symbol".to_string(), from_symbol.to_string());
    params.insert("to_symbol".to_string(), to_symbol.to_string());
    params.insert("outputsize".to_string(), outputsize.to_string());

    self.transport.get(FuncType::FxDaily, params).await
  }

  /// Get weekly forex time series data
  ///
  /// # Arguments
  ///
  /// * `from_symbol` - The base currency (e.g., "EUR", "GBP")
  /// * `to_symbol` - The quote currency (e.g., "USD", "JPY")
  #[instrument(skip(self), fields(from_symbol, to_symbol))]
  pub async fn weekly(&self, from_symbol: &str, to_symbol: &str) -> Result<FxWeekly> {
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
  /// * `from_symbol` - The base currency (e.g., "EUR", "GBP")
  /// * `to_symbol` - The quote currency (e.g., "USD", "JPY")
  #[instrument(skip(self), fields(from_symbol, to_symbol))]
  pub async fn monthly(&self, from_symbol: &str, to_symbol: &str) -> Result<FxMonthly> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("from_symbol".to_string(), from_symbol.to_string());
    params.insert("to_symbol".to_string(), to_symbol.to_string());

    self.transport.get(FuncType::FxMonthly, params).await
  }
}

impl_endpoint_base!(ForexEndpoints);
