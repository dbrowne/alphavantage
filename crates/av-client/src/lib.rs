//! # av-client
//!
//! A pure AlphaVantage API client for Rust with no database dependencies.
//!
//! ## Features
//!
//! - **Clean API**: Simple, idiomatic Rust interface
//! - **Async/Await**: Built on tokio for high performance
//! - **Rate Limiting**: Built-in rate limiting to respect API limits
//! - **Type Safe**: Strongly typed responses using av-models
//! - **Configurable**: Environment-based configuration via av-core
//! - **Comprehensive**: Supports all major AlphaVantage endpoints
//!
//! ## Usage
//!
//! ```rust,no_run
//! use av_client::AlphaVantageClient;
//! use av_core::Config;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::from_env()?;
//!     let client = AlphaVantageClient::new(config).await?;
//!     
//!     // Get daily time series for a symbol
//!     let data = client.time_series().daily("AAPL").await?;
//!     println!("Latest close price: {:?}", data.time_series.values().next());
//!     
//!     // Get company overview
//!     let overview = client.fundamentals().company_overview("AAPL").await?;
//!     println!("Market cap: {}", overview.market_capitalization);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Rate Limiting
//!
//! The client automatically handles rate limiting based on your API tier:
//! - Free tier: 25 requests per day, 5 per minute
//! - Premium tier: 75 requests per minute (configurable)
//!
//! ## Error Handling
//!
//! All methods return `Result<T, av_core::Error>` for consistent error handling
//! across the entire av-* ecosystem.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod client;
pub mod endpoints;
pub mod transport;

// Re-export the main client and common types
pub use client::AlphaVantageClient;
pub use av_core::{Config, Error, Result};
pub use av_models::*;

// Re-export endpoint modules for direct access if needed
pub use endpoints::{
    crypto::CryptoEndpoints,
    forex::ForexEndpoints, 
    fundamentals::FundamentalsEndpoints,
    news::NewsEndpoints,
    time_series::TimeSeriesEndpoints,
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_creation() {
        let config = Config::default_with_key("test_key".to_string());
        // Test that we can create the client configuration
        assert_eq!(config.api_key, "test_key");
    }
}
