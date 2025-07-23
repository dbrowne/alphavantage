//! Configuration management for AlphaVantage client

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::env;
use dotenvy::dotenv;

/// Main configuration struct for AlphaVantage client
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
  /// AlphaVantage API key
  pub api_key: String,

  /// API rate limit (requests per minute)
  pub rate_limit: u32,

  /// Request timeout in seconds
  pub timeout_secs: u64,

  /// Maximum retries for failed requests
  pub max_retries: u32,

  /// Base URL for AlphaVantage API
  pub base_url: String,
}

impl Config {
  /// Load configuration from environment variables
  pub fn from_env() -> Result<Self> {

    dotenv().ok();

    let api_key = env::var("ALPHA_VANTAGE_API_KEY")
      .map_err(|_| Error::ApiKey("ALPHA_VANTAGE_API_KEY not set".to_string()))?;

    let rate_limit = env::var("AV_RATE_LIMIT")
      .unwrap_or_else(|_| "75".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_RATE_LIMIT".to_string()))?;

    let timeout_secs = env::var("AV_TIMEOUT_SECS")
      .unwrap_or_else(|_| "30".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_TIMEOUT_SECS".to_string()))?;

    let max_retries = env::var("AV_MAX_RETRIES")
      .unwrap_or_else(|_| "3".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_MAX_RETRIES".to_string()))?;

    let base_url =
      env::var("AV_BASE_URL").unwrap_or_else(|_| crate::ALPHA_VANTAGE_BASE_URL.to_string());

    Ok(Config { api_key, rate_limit, timeout_secs, max_retries, base_url })
  }

  /// Create a config with default values (for testing)
  pub fn default_with_key(api_key: String) -> Self {
    Config {
      api_key,
      rate_limit: crate::DEFAULT_RATE_LIMIT,
      timeout_secs: 30,
      max_retries: 3,
      base_url: crate::ALPHA_VANTAGE_BASE_URL.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_from_env() {
    env::set_var("ALPHA_VANTAGE_API_KEY", "test_key");
    let config = Config::from_env().unwrap();
    assert_eq!(config.api_key, "test_key");
    assert_eq!(config.rate_limit, 75);
  }
}
