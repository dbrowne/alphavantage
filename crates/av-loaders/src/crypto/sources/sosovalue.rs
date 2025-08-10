use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub struct SosoValueProvider {
  pub api_key: Option<String>,
}

impl SosoValueProvider {
  pub fn new(api_key: Option<String>) -> Self {
    Self { api_key }
  }
}

#[derive(Debug, Deserialize)]
struct SosoValueResponse {
  code: i32,
  message: String,
  data: Option<SosoValueData>,
}

#[derive(Debug, Deserialize)]
struct SosoValueData {
  list: Vec<SosoValueCrypto>,
}

#[derive(Debug, Deserialize)]
struct SosoValueCrypto {
  symbol: String,
  name: String,
  #[serde(rename = "marketCap")]
  market_cap: Option<f64>,
  rank: Option<u32>,
  #[serde(rename = "isActive")]
  is_active: Option<bool>,
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl CryptoDataProvider for SosoValueProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from SosoValue");

    // SosoValue API endpoint - may need adjustment based on actual API
    let url = "https://api.sosovalue.com/api/v1/crypto/coins";

    let mut request = client.get(url);

    // Add API key if available
    if let Some(ref key) = self.api_key {
      request = request.header("Authorization", format!("Bearer {}", key));
    }

    // Add user agent as some APIs require it
    request = request.header("User-Agent", "AlphaVantage-Rust-Client/1.0");

    let response = request.send().await?;

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("SosoValue".to_string()));
    }

    if response.status().as_u16() == 401 {
      return Err(CryptoLoaderError::ApiKeyMissing("SosoValue".to_string()));
    }

    if !response.status().is_success() {
      warn!("SosoValue API returned status: {}", response.status());
      return Err(CryptoLoaderError::InvalidResponse {
        source: "SosoValue".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let api_response: SosoValueResponse = response.json().await?;

    if api_response.code != 0 {
      return Err(CryptoLoaderError::InvalidResponse {
        source: "SosoValue".to_string(),
        message: format!("API Error: {}", api_response.message),
      });
    }

    let data = api_response.data.ok_or_else(|| CryptoLoaderError::InvalidResponse {
      source: "SosoValue".to_string(),
      message: "No data field in response".to_string(),
    })?;

    debug!("SosoValue returned {} cryptocurrencies", data.list.len());

    let symbols = data
      .list
      .into_iter()
      .filter(|crypto| crypto.is_active.unwrap_or(true)) // Filter active only
      .map(|crypto| CryptoSymbol {
        symbol: crypto.symbol.to_uppercase(),
        name: crypto.name,
        base_currency: None,
        quote_currency: Some("USD".to_string()),
        market_cap_rank: crypto.rank,
        source: CryptoDataSource::SosoValue,
        source_id: crypto.symbol.clone(),
        is_active: crypto.is_active.unwrap_or(true),
        created_at: Utc::now(),
        additional_data: crypto.extra,
      })
      .collect();

    info!("Successfully processed {} active symbols from SosoValue", symbols.len());
    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "SosoValue"
  }

  fn rate_limit_delay(&self) -> u64 {
    500 // Conservative rate limiting for unknown API
  }

  fn requires_api_key(&self) -> bool {
    true // Assuming SosoValue requires API key
  }
}
