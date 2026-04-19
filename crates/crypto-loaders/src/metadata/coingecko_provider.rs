/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! CoinGecko metadata provider.
//!
//! Fetches per-coin metadata from the CoinGecko `/coins/{id}` API endpoint
//! and normalizes it into a [`ProcessedCryptoMetadata`] struct suitable for
//! database storage.
//!
//! # Features
//!
//! - **Cache-first loading:** [`load_cached`](CoinGeckoMetadataProvider::load_cached)
//!   checks a [`CryptoCache`] before making an API call. Configurable TTL and
//!   force-refresh override.
//! - **Direct loading:** [`load`](CoinGeckoMetadataProvider::load) bypasses the
//!   cache entirely.
//! - **Pro/free API auto-detection:** when `config.coingecko_api_key` is set,
//!   uses the Pro endpoint (`pro-api.coingecko.com`); otherwise the free tier.
//! - **Rich additional data:** extracts description, links, market data, and
//!   categories from the response and stores them as a `serde_json::Value` map.
//!
//! # Data flow
//!
//! ```text
//! CryptoSymbolForMetadata
//!   └──► load_cached() / load()
//!          └──► load_fresh()
//!                ├── GET /coins/{source_id}
//!                ├── parse JSON → serde_json::Value
//!                └── process_response()
//!                      ├── extract market_cap_rank
//!                      ├── collect additional_data (description, links, market_data, categories)
//!                      └── build ProcessedCryptoMetadata
//! ```

use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, info, warn};

use super::types::{CryptoMetadataConfig, CryptoSymbolForMetadata, ProcessedCryptoMetadata};
use crate::CryptoLoaderError;
use crate::traits::CryptoCache;

/// Async provider that fetches cryptocurrency metadata from the CoinGecko API.
///
/// Borrows a [`CryptoMetadataConfig`] for its lifetime. Construct via
/// [`new`](Self::new) or [`with_client`](Self::with_client).
pub struct CoinGeckoMetadataProvider<'a> {
  config: &'a CryptoMetadataConfig,
  client: reqwest::Client,
}

impl<'a> CoinGeckoMetadataProvider<'a> {
  /// Creates a provider with a default `reqwest::Client`.
  pub fn new(config: &'a CryptoMetadataConfig) -> Self {
    Self { config, client: reqwest::Client::new() }
  }

  /// Creates a provider with a caller-supplied HTTP client (for connection pooling).
  pub fn with_client(config: &'a CryptoMetadataConfig, client: reqwest::Client) -> Self {
    Self { config, client }
  }

  /// Loads metadata with cache-first strategy.
  ///
  /// 1. Unless `config.force_refresh` is set, checks the cache for key
  ///    `crypto_metadata_coingecko_{source_id}`.
  /// 2. On cache miss (or force refresh), calls [`load_fresh`](Self::load_fresh).
  /// 3. Caches the raw JSON response with `config.cache_ttl_hours` TTL.
  ///
  /// Returns a [`ProcessedCryptoMetadata`] on success.
  pub async fn load_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    cache: &Arc<dyn CryptoCache>,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = format!("crypto_metadata_coingecko_{}", symbol.source_id);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh && self.config.enable_response_cache {
      if let Ok(Some(cached_str)) = cache.get("coingecko", &cache_key).await {
        if let Ok(cached_data) = serde_json::from_str::<Value>(&cached_str) {
          debug!("Using cached CoinGecko metadata for {}", symbol.symbol);
          return self.process_response(cached_data, symbol);
        } else {
          warn!("Failed to parse cached CoinGecko response for {}", symbol.symbol);
        }
      }
    }

    // Cache miss or force refresh - fetch from API
    debug!("Fetching fresh CoinGecko metadata for {} (cache miss)", symbol.symbol);

    let (metadata, response, _url) = self.load_fresh(symbol).await?;

    // Cache the successful response
    if self.config.enable_response_cache {
      let response_str = serde_json::to_string(&response).unwrap_or_default();
      if let Err(e) =
        cache.set("coingecko", &cache_key, &response_str, self.config.cache_ttl_hours).await
      {
        warn!("Failed to cache CoinGecko response: {}", e);
      } else {
        info!(
          "Cached CoinGecko metadata for {} (TTL: {}h)",
          symbol.symbol, self.config.cache_ttl_hours
        );
      }
    }

    Ok(metadata)
  }

  /// Loads metadata directly from the API, bypassing the cache entirely.
  pub async fn load(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let (metadata, _, _) = self.load_fresh(symbol).await?;
    Ok(metadata)
  }

  /// Fetches metadata from the CoinGecko `/coins/{source_id}` endpoint.
  ///
  /// Auto-detects the Pro vs. free API tier based on whether
  /// `config.coingecko_api_key` is set. Uses `config.timeout_seconds` for
  /// the HTTP request timeout.
  ///
  /// Returns `(processed_metadata, raw_json, request_url)`. The raw JSON
  /// is returned so the caller can cache it.
  ///
  /// # Errors
  ///
  /// - [`RateLimitExceeded`](CryptoLoaderError::RateLimitExceeded) — HTTP 429.
  /// - [`ApiError`](CryptoLoaderError::ApiError) — any other non-success status
  ///   or network failure.
  /// - [`ParseError`](CryptoLoaderError::ParseError) — JSON deserialization failure.
  async fn load_fresh(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<(ProcessedCryptoMetadata, Value, String), CryptoLoaderError> {
    debug!("Loading CoinGecko metadata for {}", symbol.source_id);

    let url = if let Some(api_key) = &self.config.coingecko_api_key {
      format!(
        "https://pro-api.coingecko.com/api/v3/coins/{}?x_cg_pro_api_key={}",
        symbol.source_id, api_key
      )
    } else {
      format!("https://api.coingecko.com/api/v3/coins/{}?localization=false", symbol.source_id)
    };

    let mut request = self.client.get(&url);

    if let Some(api_key) = &self.config.coingecko_api_key {
      request = request.header("X-CG-Pro-API-Key", api_key);
    }

    let response =
      request
        .timeout(Duration::from_secs(self.config.timeout_seconds))
        .send()
        .await
        .map_err(|e| CryptoLoaderError::ApiError(format!("CoinGecko request failed: {}", e)))?;

    if response.status() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string()));
    }

    if !response.status().is_success() {
      return Err(CryptoLoaderError::ApiError(format!(
        "CoinGecko API returned status: {}",
        response.status()
      )));
    }

    let coin_data: Value = response.json().await.map_err(|e| {
      CryptoLoaderError::ParseError(format!("Failed to parse CoinGecko response: {}", e))
    })?;

    let metadata = self.process_response(coin_data.clone(), symbol)?;

    Ok((metadata, coin_data, url))
  }

  /// Extracts structured metadata from a raw CoinGecko JSON response.
  ///
  /// Pulls out `market_cap_rank` as a direct field, then collects
  /// `description`, `links`, `market_data`, and `categories` into an
  /// `additional_data` JSON map. Sets `source = "coingecko"`,
  /// `base_currency` to the symbol, `quote_currency` to `"USD"`,
  /// and `last_updated` to now.
  fn process_response(
    &self,
    coin_data: Value,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let market_cap_rank =
      coin_data.get("market_cap_rank").and_then(|v| v.as_i64()).map(|v| v as i32);

    let mut additional_data = HashMap::new();

    if let Some(description) = coin_data.get("description").and_then(|d| d.get("en")) {
      additional_data.insert("description".to_string(), description.clone());
    }

    if let Some(links) = coin_data.get("links") {
      additional_data.insert("links".to_string(), links.clone());
    }

    if let Some(market_data) = coin_data.get("market_data") {
      additional_data.insert("market_data".to_string(), market_data.clone());
    }

    if let Some(categories) = coin_data.get("categories") {
      additional_data.insert("categories".to_string(), categories.clone());
    }

    Ok(ProcessedCryptoMetadata {
      sid: symbol.sid,
      source: "coingecko".to_string(),
      source_id: symbol.source_id.clone(),
      market_cap_rank,
      base_currency: Some(symbol.symbol.clone()),
      quote_currency: Some("USD".to_string()),
      is_active: symbol.is_active,
      additional_data: if additional_data.is_empty() {
        None
      } else {
        Some(serde_json::to_value(additional_data).map_err(|e| {
          CryptoLoaderError::ParseError(format!("Failed to serialize additional data: {}", e))
        })?)
      },
      last_updated: Utc::now(),
    })
  }
}
