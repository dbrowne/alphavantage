use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info};

pub struct CoinPaprikaProvider;

#[derive(Debug, Deserialize)]
struct CoinPaprikaResponse {
  id: String,
  name: String,
  symbol: String,
  rank: Option<u32>,
  is_active: bool,
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl CryptoDataProvider for CoinPaprikaProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from CoinPaprika");

    let url = "https://api.coinpaprika.com/v1/coins";
    let response = client.get(url).send().await?;

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinPaprika".to_string()));
    }

    if !response.status().is_success() {
      return Err(CryptoLoaderError::InvalidResponse {
        source: "CoinPaprika".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let coins: Vec<CoinPaprikaResponse> = response.json().await?;
    debug!("CoinPaprika returned {} coins", coins.len());

    let symbols = coins
      .into_iter()
      .filter(|coin| coin.is_active) // Only active coins
      .map(|coin| CryptoSymbol {
        symbol: coin.symbol.to_uppercase(),
        name: coin.name,
        base_currency: None,
        quote_currency: None,
        market_cap_rank: coin.rank,
        source: CryptoDataSource::CoinPaprika,
        source_id: coin.id,
        is_active: coin.is_active,
        created_at: Utc::now(),
        additional_data: coin.extra,
      })
      .collect();

    info!("Successfully processed {} active symbols from CoinPaprika", symbols.len());
    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "CoinPaprika"
  }

  fn rate_limit_delay(&self) -> u64 {
    100 // 10 requests per second
  }

  fn requires_api_key(&self) -> bool {
    false
  }
}
