use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
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
struct CoinGeckoCoin {
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

    let mut url = "https://api.coingecko.com/api/v3/coins/list".to_string();

    // Add API key if available
    if let Some(ref key) = self.api_key {
      url = format!("{}?x_cg_demo_api_key={}", url, key);
    }

    let response = client.get(&url).send().await?;

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string()));
    }

    if response.status().as_u16() == 401 {
      return Err(CryptoLoaderError::ApiKeyMissing("CoinGecko".to_string()));
    }

    if !response.status().is_success() {
      warn!("CoinGecko API returned status: {}", response.status());
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "CoinGecko".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let coins: Vec<CoinGeckoCoin> = response.json().await?;

    debug!("CoinGecko returned {} coins", coins.len());

    let symbols: Vec<CryptoSymbol> = coins
      .into_iter()
      .map(|coin| CryptoSymbol {
        symbol: coin.symbol.to_uppercase(),
        name: coin.name,
        base_currency: None,
        quote_currency: Some("USD".to_string()),
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
    1000 // 1 second delay
  }

  fn requires_api_key(&self) -> bool {
    false // CoinGecko has a free tier
  }
}
