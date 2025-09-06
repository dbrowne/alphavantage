use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info};

pub struct CoinCapProvider;

#[derive(Debug, Deserialize)]
struct CoinCapResponse {
  data: Vec<CoinCapAsset>,
}

#[derive(Debug, Deserialize)]
struct CoinCapAsset {
  id: String,
  rank: Option<String>,
  symbol: String,
  name: String,
  #[serde(rename = "marketCapUsd")]
  #[allow(dead_code)]
  market_cap_usd: Option<String>,
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl CryptoDataProvider for CoinCapProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from CoinCap");

    // CoinCap limits to 2000 assets per request, but we can paginate
    let mut all_symbols = Vec::new();
    let mut offset = 0;
    let limit = 2000;

    loop {
      let url = format!("https://api.coincap.io/v2/assets?limit={}&offset={}", limit, offset);
      let response = client.get(&url).send().await?;

      if response.status().as_u16() == 429 {
        return Err(CryptoLoaderError::RateLimitExceeded("CoinCap".to_string()));
      }

      if !response.status().is_success() {
        return Err(CryptoLoaderError::InvalidResponse {
          api_source: "CoinCap".to_string(),
          message: format!("HTTP {}", response.status()),
        });
      }

      let api_response: CoinCapResponse = response.json().await?;
      let batch_size = api_response.data.len();

      debug!("CoinCap returned {} assets at offset {}", batch_size, offset);

      let batch_symbols: Vec<CryptoSymbol> = api_response
        .data
        .into_iter()
        .map(|asset| {
          let rank = asset.rank.and_then(|r| r.parse::<u32>().ok());
          CryptoSymbol {
            symbol: asset.symbol.to_uppercase(),
            name: asset.name,
            base_currency: None,
            quote_currency: Some("USD".to_string()),
            market_cap_rank: rank,
            source: CryptoDataSource::CoinCap,
            source_id: asset.id,
            is_active: true,
            created_at: Utc::now(),
            additional_data: asset.extra,
          }
        })
        .collect();

      all_symbols.extend(batch_symbols);

      // Break if we got less than the limit (last page)
      if batch_size < limit {
        break;
      }

      offset += limit;
      tokio::time::sleep(tokio::time::Duration::from_millis(self.rate_limit_delay())).await;
    }

    info!("Successfully processed {} symbols from CoinCap", all_symbols.len());
    Ok(all_symbols)
  }

  fn source_name(&self) -> &'static str {
    "CoinCap"
  }

  fn rate_limit_delay(&self) -> u64 {
    200 // Conservative rate limiting
  }

  fn requires_api_key(&self) -> bool {
    false
  }
}
