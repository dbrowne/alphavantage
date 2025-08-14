use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

pub struct CoinMarketCapProvider {
  pub api_key: String,
}

impl CoinMarketCapProvider {
  pub fn new(api_key: String) -> Self {
    Self { api_key }
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct CmcResponse {
  status: CmcStatus,
  data: Vec<CmcCryptocurrency>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CmcStatus {
  timestamp: String,
  error_code: i32,
  error_message: Option<String>,
  elapsed: i32,
  credit_count: i32,
  notice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CmcCryptocurrency {
  id: u64,
  name: String,
  symbol: String,
  slug: String,
  num_market_pairs: Option<u32>,
  date_added: String,
  tags: Vec<String>,
  max_supply: Option<f64>,
  circulating_supply: Option<f64>,
  total_supply: Option<f64>,
  is_active: Option<u8>,
  platform: Option<CmcPlatform>,
  cmc_rank: Option<u32>,
  is_fiat: Option<u8>,
  self_reported_circulating_supply: Option<f64>,
  self_reported_market_cap: Option<f64>,
  tvl_ratio: Option<f64>,
  last_updated: String,
  quote: HashMap<String, CmcQuote>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CmcPlatform {
  id: u64,
  name: String,
  symbol: String,
  slug: String,
  token_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CmcQuote {
  price: Option<f64>,
  volume_24h: Option<f64>,
  volume_change_24h: Option<f64>,
  percent_change_1h: Option<f64>,
  percent_change_24h: Option<f64>,
  percent_change_7d: Option<f64>,
  percent_change_30d: Option<f64>,
  percent_change_60d: Option<f64>,
  percent_change_90d: Option<f64>,
  market_cap: Option<f64>,
  market_cap_dominance: Option<f64>,
  fully_diluted_market_cap: Option<f64>,
  tvl: Option<f64>,
  last_updated: String,
}

#[async_trait]
impl CryptoDataProvider for CoinMarketCapProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from CoinMarketCap");

    // Use listings endpoint for comprehensive data
    let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";

    let response = client
      .get(url)
      .header("X-CMC_PRO_API_KEY", &self.api_key)
      .header("Accept", "application/json")
      .query(&[
        ("start", "1"),
        ("limit", "5000"), // Adjust based on subscription tier
        ("convert", "USD"),
      ])
      .send()
      .await?;

    if response.status().as_u16() == 401 {
      return Err(CryptoLoaderError::ApiKeyMissing("CoinMarketCap".to_string()));
    }

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinMarketCap".to_string()));
    }

    if !response.status().is_success() {
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "CoinMarketCap".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let cmc_response: CmcResponse = response.json().await?;

    if cmc_response.status.error_code != 0 {
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "CoinMarketCap".to_string(),
        message: cmc_response
          .status
          .error_message
          .unwrap_or_else(|| "Unknown CMC error".to_string()),
      });
    }

    debug!("CoinMarketCap returned {} cryptocurrencies", cmc_response.data.len());

    let symbols: Vec<CryptoSymbol> = cmc_response
      .data
      .into_iter()
      .map(|crypto| {
        let mut additional_data = HashMap::new();
        additional_data
          .insert("tags".to_string(), serde_json::to_value(&crypto.tags).unwrap_or_default());
        additional_data.insert(
          "num_market_pairs".to_string(),
          serde_json::to_value(crypto.num_market_pairs).unwrap_or_default(),
        );
        additional_data.insert(
          "date_added".to_string(),
          serde_json::to_value(&crypto.date_added).unwrap_or_default(),
        );

        if let Some(platform) = crypto.platform {
          additional_data
            .insert("platform".to_string(), serde_json::to_value(platform).unwrap_or_default());
        }

        CryptoSymbol {
          symbol: crypto.symbol.to_uppercase(),
          name: crypto.name,
          base_currency: None,
          quote_currency: Some("USD".to_string()),
          market_cap_rank: crypto.cmc_rank,
          source: CryptoDataSource::CoinMarketCap,
          source_id: crypto.id.to_string(),
          is_active: crypto.is_active.unwrap_or(1) == 1,
          created_at: Utc::now(),
          additional_data,
        }
      })
      .collect();

    info!("Successfully processed {} symbols from CoinMarketCap", symbols.len());
    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "CoinMarketCap"
  }

  fn rate_limit_delay(&self) -> u64 {
    300 // Conservative rate limiting for paid API
  }

  fn requires_api_key(&self) -> bool {
    true
  }
}
