/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency ID discovery via external APIs.
//!
//! Provides stateless async functions that resolve a cryptocurrency ticker
//! symbol (e.g., `"BTC"`) to a provider-specific identifier by querying the
//! provider's full coin-list endpoint and scanning for an exact match.
//!
//! # Functions
//!
//! | Function                    | Provider    | Endpoint                               | Match strategy              |
//! |-----------------------------|-------------|----------------------------------------|-----------------------------|
//! | [`discover_coingecko_id`]   | CoinGecko   | `/api/v3/coins/list`                   | `symbol` field, **lowercase** |
//! | [`discover_coinpaprika_id`] | CoinPaprika | `/v1/coins`                            | `symbol` field, **uppercase** |
//!
//! # Important notes
//!
//! - Both functions fetch the **entire coin list** on each call. For batch
//!   operations, consider caching the list externally or using
//!   [`CryptoMappingService`](super::service::CryptoMappingService) which
//!   handles this.
//! - HTTP 429 responses are mapped to [`CryptoLoaderError::RateLimitExceeded`].
//! - Multiple coins may share the same ticker symbol; the **first match**
//!   in the API response is returned.

use crate::CryptoLoaderError;

/// Resolves a cryptocurrency ticker symbol to its CoinGecko coin ID.
///
/// Fetches the full CoinGecko coin list and performs a case-insensitive
/// exact match on the `symbol` field (input is lowercased before comparison).
///
/// # Arguments
///
/// - `client` — a reusable `reqwest::Client` (connection pooling recommended).
/// - `symbol` — the ticker to search for (e.g., `"BTC"`).
/// - `api_key` — optional CoinGecko Pro API key. When provided, uses the
///   Pro endpoint (`pro-api.coingecko.com`); otherwise uses the free tier.
///
/// # Returns
///
/// - `Ok(Some(id))` — the CoinGecko slug (e.g., `"bitcoin"` for `"BTC"`).
/// - `Ok(None)` — no coin with that symbol was found.
/// - `Err(RateLimitExceeded)` — HTTP 429 from CoinGecko.
/// - `Err(ApiError)` — any other non-success HTTP status.
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
    return Err(CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string()));
  }

  if !response.status().is_success() {
    return Err(CryptoLoaderError::ApiError(format!("CoinGecko HTTP {}", response.status())));
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

/// Resolves a cryptocurrency ticker symbol to its CoinPaprika coin ID.
///
/// Fetches the full CoinPaprika coin list and performs a case-insensitive
/// exact match on the `symbol` field (input is uppercased before comparison).
///
/// CoinPaprika's public API does **not** require an API key but is
/// rate-limited.
///
/// # Arguments
///
/// - `client` — a reusable `reqwest::Client`.
/// - `symbol` — the ticker to search for (e.g., `"BTC"`).
///
/// # Returns
///
/// - `Ok(Some(id))` — the CoinPaprika slug (e.g., `"btc-bitcoin"` for `"BTC"`).
/// - `Ok(None)` — no coin with that symbol was found.
/// - `Err(RateLimitExceeded)` — HTTP 429 from CoinPaprika.
/// - `Err(ApiError)` — any other non-success HTTP status.
pub async fn discover_coinpaprika_id(
  client: &reqwest::Client,
  symbol: &str,
) -> Result<Option<String>, CryptoLoaderError> {
  let url = "https://api.coinpaprika.com/v1/coins";
  let response = client.get(url).send().await?;

  if response.status() == 429 {
    return Err(CryptoLoaderError::RateLimitExceeded("CoinPaprika".to_string()));
  }

  if !response.status().is_success() {
    return Err(CryptoLoaderError::ApiError(format!("CoinPaprika HTTP {}", response.status())));
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
    let err = CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string());
    assert!(err.to_string().contains("CoinGecko"));
  }
}
