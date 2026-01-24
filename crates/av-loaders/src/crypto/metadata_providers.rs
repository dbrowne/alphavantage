/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Metadata providers for fetching crypto metadata from various API sources.

use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, info, warn};

use super::CryptoLoaderError;
use super::metadata_types::{
  CryptoMetadataConfig, CryptoSymbolForMetadata, ProcessedCryptoMetadata,
};
use super::sources::CacheRepositoryAdapter;
use av_database_postgres::repository::CacheRepository;
use av_models::crypto::CryptoDaily;

// Re-export CoinGeckoMetadataProvider from crypto-loaders
pub use crypto_loaders::CoinGeckoMetadataProvider as BaseCoinGeckoMetadataProvider;

/// Wrapper for CoinGeckoMetadataProvider that uses CacheRepository.
pub struct CoinGeckoMetadataProvider<'a> {
  inner: BaseCoinGeckoMetadataProvider<'a>,
}

impl<'a> CoinGeckoMetadataProvider<'a> {
  pub fn new(config: &'a CryptoMetadataConfig) -> Self {
    Self { inner: BaseCoinGeckoMetadataProvider::new(config) }
  }

  /// Load metadata with caching support using CacheRepository.
  pub async fn load_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_adapter = CacheRepositoryAdapter::as_arc(cache_repo.clone());
    self.inner.load_cached(symbol, &cache_adapter).await.map_err(Into::into)
  }

  /// Load metadata without caching.
  pub async fn load(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    self.inner.load(symbol).await.map_err(Into::into)
  }
}

/// AlphaVantage metadata provider.
///
/// This provider stays in av-loaders because it uses av_models::crypto::CryptoDaily
/// which is specific to the AlphaVantage API.
pub struct AlphaVantageMetadataProvider<'a> {
  config: &'a CryptoMetadataConfig,
}

impl<'a> AlphaVantageMetadataProvider<'a> {
  pub fn new(config: &'a CryptoMetadataConfig) -> Self {
    Self { config }
  }

  /// Load metadata with caching support
  pub async fn load_cached(
    &self,
    symbol: &CryptoSymbolForMetadata,
    cache_repo: &Arc<dyn CacheRepository>,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let cache_key = format!("crypto_metadata_alphavantage_{}", symbol.symbol);

    // Try cache first (unless force refresh is enabled)
    if !self.config.force_refresh {
      if let Some(cached_data) =
        get_cached_response(self.config, cache_repo, &cache_key, "alphavantage").await
      {
        debug!("Using cached AlphaVantage metadata for {}", symbol.symbol);

        if let Ok(crypto_daily) = serde_json::from_value::<CryptoDaily>(cached_data) {
          return self.process_response(crypto_daily, symbol);
        } else {
          warn!("Failed to parse cached AlphaVantage response for {}", symbol.symbol);
        }
      }
    }

    // Cache miss or force refresh - fetch from API
    debug!("Fetching fresh AlphaVantage metadata for {} (cache miss)", symbol.symbol);

    let (metadata, response, url, _status) = self.load_fresh(symbol).await?;

    // Cache the successful response
    let response_json = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);
    cache_response(self.config, cache_repo, &cache_key, "alphavantage", &url, &response_json).await;

    Ok(metadata)
  }

  /// Load fresh metadata from AlphaVantage API
  async fn load_fresh(
    &self,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<(ProcessedCryptoMetadata, CryptoDaily, String, u16), CryptoLoaderError> {
    debug!("Loading AlphaVantage metadata for {}", symbol.symbol);

    let api_key = self.config.alphavantage_api_key.as_ref().ok_or_else(|| {
      CryptoLoaderError::ApiError("AlphaVantage API key not provided".to_string())
    })?;

    let url = format!(
      "https://www.alphavantage.co/query?function=DIGITAL_CURRENCY_DAILY&symbol={}&market=USD&apikey={}",
      symbol.symbol, api_key
    );

    let client = reqwest::Client::new();
    let response = client
      .get(&url)
      .timeout(Duration::from_secs(self.config.timeout_seconds))
      .send()
      .await
      .map_err(|e| CryptoLoaderError::ApiError(format!("AlphaVantage request failed: {}", e)))?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
      return Err(CryptoLoaderError::ApiError(format!(
        "AlphaVantage API returned status: {}",
        response.status()
      )));
    }

    let response_text = response.text().await.map_err(|e| {
      CryptoLoaderError::ApiError(format!("Failed to read AlphaVantage response: {}", e))
    })?;

    let crypto_daily: CryptoDaily = serde_json::from_str(&response_text).map_err(|e| {
      CryptoLoaderError::ParseError(format!("Failed to parse AlphaVantage response: {}", e))
    })?;

    let metadata = self.process_response(crypto_daily.clone(), symbol)?;

    Ok((metadata, crypto_daily, url, status))
  }

  /// Process AlphaVantage API response into metadata
  fn process_response(
    &self,
    crypto_daily: CryptoDaily,
    symbol: &CryptoSymbolForMetadata,
  ) -> Result<ProcessedCryptoMetadata, CryptoLoaderError> {
    let mut additional_data = HashMap::new();
    additional_data.insert(
      "alphavantage_info".to_string(),
      serde_json::to_value(&crypto_daily.meta_data).unwrap_or(serde_json::Value::Null),
    );

    let base_currency = Some(crypto_daily.meta_data.digital_currency_code.clone());
    let quote_currency = Some(crypto_daily.meta_data.market_code.clone());

    Ok(ProcessedCryptoMetadata {
      sid: symbol.sid,
      source: "alphavantage".to_string(),
      source_id: format!("{}_{}", symbol.symbol, crypto_daily.meta_data.market_code),
      market_cap_rank: None,
      base_currency,
      quote_currency,
      is_active: symbol.is_active,
      additional_data: Some(
        serde_json::to_value(additional_data).unwrap_or(serde_json::Value::Null),
      ),
      last_updated: Utc::now(),
    })
  }
}

// ============================================================================
// Shared caching utilities
// ============================================================================

/// Get cached response for a specific cache key
pub async fn get_cached_response(
  config: &CryptoMetadataConfig,
  cache_repo: &Arc<dyn CacheRepository>,
  cache_key: &str,
  api_source: &str,
) -> Option<serde_json::Value> {
  if !config.enable_response_cache || config.force_refresh {
    return None;
  }

  match cache_repo.get_json(cache_key, api_source).await {
    Ok(Some(data)) => {
      info!("Cache hit for {}", cache_key);
      debug!("Successfully retrieved cached metadata for {}", cache_key);
      Some(data)
    }
    Ok(None) => {
      debug!("Cache miss for {}", cache_key);
      None
    }
    Err(e) => {
      debug!("Cache read error for {}: {}", cache_key, e);
      None
    }
  }
}

/// Cache API response
pub async fn cache_response(
  config: &CryptoMetadataConfig,
  cache_repo: &Arc<dyn CacheRepository>,
  cache_key: &str,
  api_source: &str,
  endpoint_url: &str,
  response_data: &serde_json::Value,
) {
  if !config.enable_response_cache {
    return;
  }

  match cache_repo
    .set_json(
      cache_key,
      api_source,
      endpoint_url,
      response_data.clone(),
      config.cache_ttl_hours as i64,
    )
    .await
  {
    Ok(()) => {
      let expires_at = Utc::now() + chrono::Duration::hours(config.cache_ttl_hours as i64);
      info!("Cached {} (TTL: {}h, expires: {})", cache_key, config.cache_ttl_hours, expires_at);
    }
    Err(e) => {
      warn!("Failed to cache {}: {}", cache_key, e);
    }
  }
}
