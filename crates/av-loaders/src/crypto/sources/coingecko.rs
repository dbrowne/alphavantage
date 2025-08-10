use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub struct CoinGeckoProvider {
  pub api_key: Option<String>,
}

impl CoinGeckoProvider {
  pub fn new(api_key: Option<String>) -> Self {
    Self { api_key }
  }
}

#[derive(Debug, Deserialize)]
struct CoinGeckoResponse {
  id: String,
  symbol: String,
  name: String,
  market_cap_rank: Option<u32>,
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl CryptoDataProvider for CoinGeckoProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from CoinGecko");

    let url = if self.api_key.is_some() {
      "https://pro-api.coingecko.com/api/v3/coins/list"
    } else {
      "https://api.coingecko.com/api/v3/coins/list"
    };

    let mut request = client.get(url);

    if let Some(ref key) = self.api_key {
      request = request.header("x-cg-pro-api-key", key);
    }

    let response = request.send().await?;

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string()));
    }

    if !response.status().is_success() {
      return Err(CryptoLoaderError::InvalidResponse {
        source: "CoinGecko".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let coins: Vec<CoinGeckoResponse> = response.json().await?;
    debug!("CoinGecko returned {} coins", coins.len());

    let symbols = coins
      .into_iter()
      .map(|coin| CryptoSymbol {
        symbol: coin.symbol.to_uppercase(),
        name: coin.name,
        base_currency: None,
        quote_currency: None,
        market_cap_rank: coin.market_cap_rank,
        source: CryptoDataSource::CoinGecko,
        source_id: coin.id,
        is_active: true,
        created_at: Utc::now(),
        additional_data: coin.extra,
      })
      .collect();

    info!("Successfully processed {} symbols from CoinGecko", symbols.len());
    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "CoinGecko"
  }

  fn rate_limit_delay(&self) -> u64 {
    if self.api_key.is_some() { 50 } else { 1000 } // Pro API vs Free API
  }

  fn requires_api_key(&self) -> bool {
    false // Free tier available
  }
}
