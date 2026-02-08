/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! CoinGecko top gainers/losers data loader.

use crate::error::CryptoLoaderError;
use crate::traits::CryptoCache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const PROVIDER: &str = "CoinGecko";

/// A single coin from the top gainers/losers response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopMoverCoin {
  pub id: String,
  pub symbol: String,
  pub name: String,
  pub image: Option<String>,
  pub market_cap_rank: Option<i32>,
  pub usd: Option<f64>,
  pub usd_24h_vol: Option<f64>,
  pub usd_1h_change: Option<f64>,
  pub usd_24h_change: Option<f64>,
  pub usd_7d_change: Option<f64>,
  pub usd_14d_change: Option<f64>,
  pub usd_30d_change: Option<f64>,
  pub usd_200d_change: Option<f64>,
  pub usd_1y_change: Option<f64>,
}

/// Raw API response from CoinGecko `/coins/top_gainers_losers`
#[derive(Debug, Deserialize)]
struct TopGainersLosersResponse {
  top_gainers: Vec<TopMoverCoin>,
  top_losers: Vec<TopMoverCoin>,
}

/// Output from loading top movers
#[derive(Debug)]
pub struct TopMoversOutput {
  pub gainers: Vec<TopMoverCoin>,
  pub losers: Vec<TopMoverCoin>,
  pub from_cache: bool,
}

/// Configuration for the top movers loader
#[derive(Debug, Clone)]
pub struct TopMoversConfig {
  pub duration: String,
  pub cache_ttl_hours: u32,
}

impl Default for TopMoversConfig {
  fn default() -> Self {
    Self { duration: "24h".to_string(), cache_ttl_hours: 1 }
  }
}

/// Loader for CoinGecko top gainers/losers data
pub struct TopMoversLoader {
  api_key: String,
  cache: Option<Arc<dyn CryptoCache>>,
  config: TopMoversConfig,
}

impl TopMoversLoader {
  pub fn new(api_key: String, config: TopMoversConfig) -> Self {
    Self { api_key, cache: None, config }
  }

  pub fn with_cache(mut self, cache: Arc<dyn CryptoCache>) -> Self {
    self.cache = Some(cache);
    self
  }

  /// Load top gainers and losers from CoinGecko
  #[instrument(name = "TopMoversLoader", skip(self), fields(duration = %self.config.duration))]
  pub async fn load(&self) -> Result<TopMoversOutput, CryptoLoaderError> {
    info!(duration = %self.config.duration, "Loading top gainers/losers from {}", PROVIDER);

    let cache_key = format!("coingecko_top_movers_{}", self.config.duration);

    // Check cache first
    if let Some(cache) = &self.cache {
      if let Ok(Some(cached_data)) = cache.get("coingecko_http", &cache_key).await {
        match serde_json::from_str::<TopGainersLosersResponse>(&cached_data) {
          Ok(response) => {
            info!(
              gainers = response.top_gainers.len(),
              losers = response.top_losers.len(),
              "Cache hit for top movers (duration={})",
              self.config.duration
            );
            return Ok(TopMoversOutput {
              gainers: response.top_gainers,
              losers: response.top_losers,
              from_cache: true,
            });
          }
          Err(e) => {
            warn!("Failed to parse cached top movers data: {}", e);
          }
        }
      }
    }

    // Fetch from API
    let client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .user_agent("CryptoLoaders-Rust/1.0")
      .build()
      .map_err(|e| {
        CryptoLoaderError::NetworkError(format!("Failed to create HTTP client: {}", e))
      })?;

    let response = self.fetch_from_api(&client).await?;

    // Cache the response
    if let Some(cache) = &self.cache {
      if let Ok(json_data) = serde_json::to_string(&response) {
        if let Err(e) =
          cache.set("coingecko_http", &cache_key, &json_data, self.config.cache_ttl_hours).await
        {
          warn!("Failed to cache top movers response: {}", e);
        } else {
          debug!("Cached top movers response (TTL: {}h)", self.config.cache_ttl_hours);
        }
      }
    }

    info!(
      gainers = response.top_gainers.len(),
      losers = response.top_losers.len(),
      "Fetched top movers from {} API",
      PROVIDER
    );

    Ok(TopMoversOutput {
      gainers: response.top_gainers,
      losers: response.top_losers,
      from_cache: false,
    })
  }

  /// Fetch top gainers/losers from the CoinGecko API
  #[instrument(skip(self, client), fields(source = PROVIDER))]
  async fn fetch_from_api(
    &self,
    client: &Client,
  ) -> Result<TopGainersLosersResponse, CryptoLoaderError> {
    let (base_url, auth_param) = if self.api_key.starts_with("CG-") {
      ("https://pro-api.coingecko.com/api/v3", "x_cg_pro_api_key")
    } else {
      ("https://api.coingecko.com/api/v3", "x_cg_demo_api_key")
    };

    let url = format!("{}/coins/top_gainers_losers", base_url);

    debug!("Calling {} API: {}", PROVIDER, url);

    let response = client
      .get(&url)
      .query(&[
        (auth_param, self.api_key.as_str()),
        ("vs_currency", "usd"),
        ("duration", self.config.duration.as_str()),
      ])
      .header("accept", "application/json")
      .send()
      .await?;

    let status = response.status();

    if status == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded {
        provider: PROVIDER.to_string(),
        retry_after_secs: Some(60),
      });
    }

    if !status.is_success() {
      let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
      return Err(CryptoLoaderError::ApiError {
        provider: PROVIDER.to_string(),
        message: format!("HTTP {}: {}", status, error_text),
      });
    }

    let response_text = response.text().await?;
    let parsed: TopGainersLosersResponse = serde_json::from_str(&response_text).map_err(|e| {
      CryptoLoaderError::ParseError(format!(
        "Failed to parse {} top_gainers_losers response: {}",
        PROVIDER, e
      ))
    })?;

    Ok(parsed)
  }
}

// Allow serialization of the full response for caching
impl Serialize for TopGainersLosersResponse {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeStruct;
    let mut state = serializer.serialize_struct("TopGainersLosersResponse", 2)?;
    state.serialize_field("top_gainers", &self.top_gainers)?;
    state.serialize_field("top_losers", &self.top_losers)?;
    state.end()
  }
}
