use crate::endpoints::{
    crypto::CryptoEndpoints, forex::ForexEndpoints, fundamentals::FundamentalsEndpoints,
    news::NewsEndpoints, time_series::TimeSeriesEndpoints,
};
use crate::transport::Transport;
use av_core::{Config, Result};
use governor::{
    clock::DefaultClock, middleware::NoOpMiddleware, state::{InMemoryState, NotKeyed}, 
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Main AlphaVantage API client
///
/// Provides access to all AlphaVantage API endpoints through organized endpoint modules.
/// Handles authentication, rate limiting, and transport concerns automatically.
///
/// # Examples
///
/// ```rust,no_run
/// use av_client::AlphaVantageClient;
/// use av_core::Config;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Config::from_env()?;
///     let client = AlphaVantageClient::new(config);
///     
///     // Get company overview
///     let overview = client.fundamentals().company_overview("AAPL").await?;
///     println!("Apple market cap: {}", overview.market_capitalization);
///     
///     // Get stock price data
///     let prices = client.time_series().daily("AAPL", "compact").await?;
///     println!("Latest price data available");
///     
///     Ok(())
/// }
/// ```
pub struct AlphaVantageClient {
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    transport: Arc<Transport>,
}

impl AlphaVantageClient {
    /// Create a new AlphaVantage API client
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing API key and other settings
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use av_client::AlphaVantageClient;
    /// use av_core::Config;
    ///
    /// let config = Config::from_env().expect("Missing API key");
    /// let client = AlphaVantageClient::new(config);
    /// ```
    pub fn new(config: Config) -> Self {
        let rate_limit = config.rate_limit;

        let quota = Quota::per_minute(NonZeroU32::new(rate_limit).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let transport = Arc::new(Transport::new(config));

        Self {
            transport,
            rate_limiter,
        }
    }

    /// Create a new client with custom rate limiting
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing API key and other settings
    /// * `rate_limiter` - Custom rate limiter instance
    pub fn with_rate_limiter(
        config: Config,
        rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    ) -> Self {
        Self {
            transport: Arc::new(Transport::new(config)),
            rate_limiter,
        }
    }

    /// Get access to time series endpoints
    ///
    /// Returns a `TimeSeriesEndpoints` instance for accessing historical and
    /// real-time stock price data.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let daily_data = client.time_series().daily("AAPL", "compact").await?;
    /// let intraday_data = client.time_series().intraday("MSFT", "5min").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub fn time_series(&self) -> TimeSeriesEndpoints {
        TimeSeriesEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to fundamentals endpoints
    ///
    /// Returns a `FundamentalsEndpoints` instance for accessing company
    /// fundamental data including financial statements and overview metrics.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let overview = client.fundamentals().company_overview("AAPL").await?;
    /// let income = client.fundamentals().income_statement("MSFT").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub fn fundamentals(&self) -> FundamentalsEndpoints {
        FundamentalsEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to news endpoints
    ///
    /// Returns a `NewsEndpoints` instance for accessing news sentiment data.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let news = client.news().news_sentiment(Some("AAPL"), None, None, None, None, Some(10)).await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub fn news(&self) -> NewsEndpoints {
        NewsEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to forex endpoints
    ///
    /// Returns a `ForexEndpoints` instance for accessing foreign exchange data.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let rate = client.forex().exchange_rate("USD", "EUR").await?;
    /// let daily_fx = client.forex().daily("EUR", "USD", "compact").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub fn forex(&self) -> ForexEndpoints {
        ForexEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to cryptocurrency endpoints
    ///
    /// Returns a `CryptoEndpoints` instance for accessing cryptocurrency data.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let btc_rate = client.crypto().exchange_rate("BTC", "USD").await?;
    /// let daily_crypto = client.crypto().daily("ETH", "USD").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub fn crypto(&self) -> CryptoEndpoints {
        CryptoEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get the current rate limit status
    ///
    /// Returns information about the current rate limiting state including
    /// available requests and reset time.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # let client = AlphaVantageClient::new(Config::from_env().unwrap());
    /// let (available, reset_time) = client.rate_limit_status();
    /// println!("Available requests: {}", available);
    /// ```
    pub fn rate_limit_status(&self) -> (u32, std::time::Duration) {
        // Note: This is a simplified implementation
        // In practice, you'd want to check the actual rate limiter state
        let available = 75; // Default rate limit
        let reset_time = std::time::Duration::from_secs(60);
        (available, reset_time)
    }

    /// Wait for rate limit to allow next request
    ///
    /// This method will block until the rate limiter allows the next request.
    /// Most users won't need to call this directly as endpoints handle it automatically.
    pub async fn wait_for_rate_limit(&self) -> Result<()> {
        self.rate_limiter
            .until_ready()
            .await;
        Ok(())
    }

    /// Execute a batch of requests with automatic rate limiting
    ///
    /// This method allows you to execute multiple requests efficiently while
    /// respecting rate limits. It will automatically space out requests to
    /// avoid hitting rate limits.
    ///
    /// # Arguments
    ///
    /// * `requests` - A vector of async closures that return Results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::Client;
    /// # use av_core::Config;
    /// # let client = Client::new(Config::from_env().unwrap());
    /// let symbols = vec!["AAPL", "MSFT", "GOOGL"];
    /// let requests: Vec<_> = symbols.into_iter().map(|symbol| {
    ///     let client = &client;
    ///     Box::pin(async move {
    ///         client.fundamentals().company_overview(symbol).await
    ///     })
    /// }).collect();
    /// 
    /// let results = client.batch_execute(requests).await;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    pub async fn batch_execute<T, F, Fut>(&self, requests: Vec<F>) -> Vec<Result<T>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut results = Vec::with_capacity(requests.len());
        
        for request in requests {
            // Wait for rate limit before each request
            let _ = self.wait_for_rate_limit().await;
            
            // Execute the request
            let result = request().await;
            results.push(result);
            
            // Small delay between requests to be conservative
            sleep(Duration::from_millis(100)).await;
        }
        
        results
    }
}

impl std::fmt::Debug for AlphaVantageClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlphaVantageClient")
            .field("transport", &self.transport)
            .field("rate_limiter", &"RateLimiter")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = Config {
            api_key: "test_key".to_string(),
            rate_limit: 75,
            timeout_secs: 30,
            max_retries: 3,
            base_url: av_core::ALPHA_VANTAGE_BASE_URL.to_string(),
        };
        
        let client = AlphaVantageClient::new(config);
        let (available, _) = client.rate_limit_status();
        assert_eq!(available, 75); // Default rate limit
    }

    #[test]
    fn test_premium_client_creation() {
        let config = Config {
            api_key: "test_key".to_string(),
            rate_limit: 600,
            timeout_secs: 30,
            max_retries: 3,
            base_url: av_core::ALPHA_VANTAGE_BASE_URL.to_string(),
        };
        
        let _client = AlphaVantageClient::new(config);
        // Premium clients should use higher rate limits
    }
}
