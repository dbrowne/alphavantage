/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency ID discovery via external APIs.
//!
//! This module provides functions to discover cryptocurrency IDs from
//! various data providers by searching their APIs.

use crate::CryptoLoaderError;

/// Discover CoinGecko ID for a symbol using their API.
///
/// Searches the CoinGecko coins list for an exact symbol match.
///
/// # Arguments
/// * `client` - HTTP client to use for the request
/// * `symbol` - The cryptocurrency symbol to search for (e.g., "BTC")
/// * `api_key` - Optional CoinGecko Pro API key
///
/// # Returns
/// * `Ok(Some(id))` - If a matching coin is found
/// * `Ok(None)` - If no matching coin is found
/// * `Err(_)` - If the API request fails
pub async fn discover_coingecko_id(
  client: &reqwest::Client,
  symbol: &str,
  api_key: Option<&str>,
) -> Result<Option<String>, CryptoLoaderError> {
  let mut url = "https://pro-api.coingecko.com/api/v3/coins/list".to_string();
  if let Some(key) = api_key {
    url = format!("{}?x_cg_pro_api_key={}", url, key);
  }

  let response = client.get(&url).send().await?;

  if response.status() == 429 {
    return Err(CryptoLoaderError::RateLimitExceeded {
      provider: "CoinGecko".to_string(),
      retry_after_secs: None,
    });
  }

  if !response.status().is_success() {
    return Err(CryptoLoaderError::ApiError {
      provider: "CoinGecko".to_string(),
      message: format!("HTTP {}", response.status()),
    });
  }

  let coins: Vec<serde_json::Value> = response.json().await?;

  // Look for exact symbol match
  for coin in coins {
    if let (Some(id), Some(coin_symbol)) = (coin.get("id"), coin.get("symbol")) {
      if coin_symbol.as_str() == Some(&symbol.to_lowercase()) {
        return Ok(id.as_str().map(|s| s.to_string()));
      }
    }
  }

  Ok(None)
}

/// Discover CoinPaprika ID for a symbol using their API.
///
/// Searches the CoinPaprika coins list for an exact symbol match.
///
/// # Arguments
/// * `client` - HTTP client to use for the request
/// * `symbol` - The cryptocurrency symbol to search for (e.g., "BTC")
///
/// # Returns
/// * `Ok(Some(id))` - If a matching coin is found
/// * `Ok(None)` - If no matching coin is found
/// * `Err(_)` - If the API request fails
pub async fn discover_coinpaprika_id(
  client: &reqwest::Client,
  symbol: &str,
) -> Result<Option<String>, CryptoLoaderError> {
  let url = "https://api.coinpaprika.com/v1/coins";
  let response = client.get(url).send().await?;

  if response.status() == 429 {
    return Err(CryptoLoaderError::RateLimitExceeded {
      provider: "CoinPaprika".to_string(),
      retry_after_secs: None,
    });
  }

  if !response.status().is_success() {
    return Err(CryptoLoaderError::ApiError {
      provider: "CoinPaprika".to_string(),
      message: format!("HTTP {}", response.status()),
    });
  }

  let coins: Vec<serde_json::Value> = response.json().await?;

  // Look for exact symbol match
  for coin in coins {
    if let Some(coin_symbol) = coin.get("symbol") {
      if coin_symbol.as_str() == Some(&symbol.to_uppercase()) {
        return Ok(coin.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()));
      }
    }
  }

  Ok(None)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_discovery_error_handling() {
    // Test that errors are properly typed
    let err = CryptoLoaderError::RateLimitExceeded {
      provider: "CoinGecko".to_string(),
      retry_after_secs: None,
    };
    assert!(err.to_string().contains("CoinGecko"));
  }
}
