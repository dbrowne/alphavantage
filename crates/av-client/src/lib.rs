#![warn(clippy::all)]

pub mod client;
pub mod endpoints;
pub mod transport;

// Re-export the main client and common types
pub use av_core::{Config, Error, Result};
pub use av_models::*;
pub use client::AlphaVantageClient;

// Re-export endpoint modules for direct access if needed
pub use endpoints::{
  crypto::CryptoEndpoints,
  crypto_social::CryptoSocialEndpoints,
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
