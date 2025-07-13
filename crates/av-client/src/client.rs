//! Main AlphaVantage client implementation

use crate::endpoints::{
    crypto::CryptoEndpoints,
    forex::ForexEndpoints,
    fundamentals::FundamentalsEndpoints, 
    news::NewsEndpoints,
    time_series::TimeSeriesEndpoints,
};
use crate::transport::Transport;
use av_core::{Config, Error, Result};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// The main AlphaVantage API client
///
/// Provides access to all AlphaVantage API endpoints with built-in rate limiting
/// and error handling. The client is designed to be long-lived and shared across
/// multiple requests.
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
///     let client = AlphaVantageClient::new(config).await?;
///     
///     // Use the client to make API calls
///     let data = client.time_series().daily("AAPL").await?;
///     
///     Ok(())
/// }
/// ```
pub struct AlphaVantageClient {
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
    config: Config,
}

impl AlphaVantageClient {
    /// Create a new AlphaVantage client with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing API key and other settings
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the client or an error if initialization fails
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
    ///     let client = AlphaVantageClient::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: Config) -> Result<Self> {
        // Create rate limiter based on API tier
        let quota = Quota::per_minute(
            NonZeroU32::new(config.rate_limit).ok_or_else(|| {
                Error::Config("Rate limit must be greater than 0".to_string())
            })?
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        // Create HTTP transport
        let transport = Arc::new(Transport::new(&config).await?);
        
        tracing::info!(
            "AlphaVantage client initialized with rate limit: {} req/min", 
            config.rate_limit
        );
        
        Ok(Self {
            transport,
            rate_limiter,
            config,
        })
    }

    /// Create a client with a custom rate limiter (useful for testing)
    pub fn with_rate_limiter(
        config: Config,
        transport: Arc<Transport>,
        rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
    ) -> Self {
        Self {
            transport,
            rate_limiter,
            config,
        }
    }

    /// Get access to time series endpoints
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::default_with_key("test".to_string());
    /// # let client = AlphaVantageClient::new(config).await?;
    /// let daily_data = client.time_series().daily("AAPL").await?;
    /// let intraday_data = client.time_series().intraday("AAPL", "5min").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn time_series(&self) -> TimeSeriesEndpoints {
        TimeSeriesEndpoints::new(
            self.transport.clone(), 
            self.rate_limiter.clone()
        )
    }

    /// Get access to fundamental data endpoints
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::default_with_key("test".to_string());
    /// # let client = AlphaVantageClient::new(config).await?;
    /// let overview = client.fundamentals().company_overview("AAPL").await?;
    /// let income_statement = client.fundamentals().income_statement("AAPL").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn fundamentals(&self) -> FundamentalsEndpoints {
        FundamentalsEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to news endpoints
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::default_with_key("test".to_string());
    /// # let client = AlphaVantageClient::new(config).await?;
    /// let news = client.news().news_sentiment(None, Some(vec!["AAPL".to_string()])).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn news(&self) -> NewsEndpoints {
        NewsEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to forex endpoints
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::default_with_key("test".to_string());
    /// # let client = AlphaVantageClient::new(config).await?;
    /// let fx_rate = client.forex().exchange_rate("USD", "EUR").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn forex(&self) -> ForexEndpoints {
        ForexEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get access to cryptocurrency endpoints
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::AlphaVantageClient;
    /// # use av_core::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::default_with_key("test".to_string());
    /// # let client = AlphaVantageClient::new(config).await?;
    /// let crypto_data = client.crypto().daily("BTC", "USD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn crypto(&self) -> CryptoEndpoints {
        CryptoEndpoints::new(
            self.transport.clone(),
            self.rate_limiter.clone()
        )
    }

    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Check the current rate limit status
    ///
    /// Returns the number of requests available and when the quota resets
    pub fn rate_limit_status(&self) -> (u32, Option<Duration>) {
        let snapshot = self.rate_limiter.snapshot();
        let available = snapshot.remaining_burst_capacity();
        let reset_after = snapshot.earliest_possible();
        
        (available, reset_after.map(|instant| instant.duration_since(std::time::Instant::now())))
    }

    /// Wait for rate limit to allow the next request
    ///
    /// This is automatically called by endpoint methods, but can be used
    /// manually for fine-grained control.
    pub async fn wait_for_rate_limit(&self) -> Result<()> {
        self.rate_limiter
            .until_ready()
            .await
            .map_err(|_| Error::RateLimit("Rate limiter error".to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use av_core::Config;
    
    #[tokio::test]
    async fn test_client_creation() {
        let config = Config::default_with_key("test_key".to_string());
        let client = AlphaVantageClient::new(config).await;
        assert!(client.is_ok());
    }
    
    #[tokio::test] 
    async fn test_rate_limit_status() {
        let config = Config::default_with_key("test_key".to_string());
        let client = AlphaVantageClient::new(config).await.unwrap();
        
        let (available, _reset) = client.        assert_eq!(available, 75); // Default rate limit
    }

    #[test]
    fn test_config_access() {
        let config = Config::default_with_key("test_key".to_string());
        let transport = Arc::new(Transport::new_mock());
        let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        let client = AlphaVantageClient::with_rate_limiter(
            config.clone(),
            transport,
            rate_limiter
        );
        
        assert_eq!(client.config().api_key, "test_key");
    }
}
