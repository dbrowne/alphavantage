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

        // CoinGecko now requires API key for all requests
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| CryptoLoaderError::ApiKeyMissing(
                "CoinGecko API key is required. Get your free key at https://www.coingecko.com/en/api/pricing".to_string()
            ))?;

        // Detect Pro vs Demo API key and use appropriate endpoint/auth
        let (base_url, auth_param) = if api_key.starts_with("CG-") {
            ("https://pro-api.coingecko.com/api/v3", "x_cg_pro_api_key")
        } else {
            ("https://api.coingecko.com/api/v3", "x_cg_demo_api_key")
        };

        let url = format!("{}/coins/list", base_url);

        debug!("Requesting CoinGecko coins list with {} authentication",
       if api_key.starts_with("CG-") { "Pro API" } else { "Demo API" });

        let response = client
            .get(&url)
            .query(&[(auth_param, api_key)])
            .header("accept", "application/json")
            .send()
            .await?;

        // Handle specific error codes
        match response.status().as_u16() {
            200 => {
                // Success - continue processing
            }
            400 => {
                return Err(CryptoLoaderError::InvalidResponse {
                    api_source: "CoinGecko".to_string(),
                    message: "Bad Request - check API key format and parameters".to_string(),
                });
            }
            401 => {
                return Err(CryptoLoaderError::ApiKeyMissing("CoinGecko".to_string()));
            }
            403 => {
                return Err(CryptoLoaderError::InvalidResponse {
                    api_source: "CoinGecko".to_string(),
                    message: "Access denied - API key may be invalid or expired".to_string(),
                });
            }
            429 => {
                return Err(CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string()));
            }
            500 => {
                return Err(CryptoLoaderError::InternalServerError("CoinGecko".to_string()));
            }
            503 => {
                return Err(CryptoLoaderError::ServiceUnavailable("CoinGecko".to_string()));
            }
            1020 => {
             return Err(CryptoLoaderError::AccessDenied("CoinGecko".to_string()));
            }
            10005 => {
                return Err(CryptoLoaderError::CoinGeckoEndpoint("CoinGecko".to_string()));
            }
            10002 =>{
                return Err(CryptoLoaderError::MissingAPIKey("CoinGecko".to_string()));
            }
            10010 | 10011 =>{
                return Err(CryptoLoaderError::InvalidAPIKey("CoinGecko".to_string()));
            }

            _ => {
                warn!("CoinGecko API returned status: {}", response.status());
                return Err(CryptoLoaderError::InvalidResponse {
                    api_source: "CoinGecko".to_string(),
                    message: format!("HTTP {}", response.status()),
                });
            }
        }

        let coins: Vec<CoinGeckoCoin> = response.json().await?;

        debug!("CoinGecko returned {} coins", coins.len());

        let symbols: Vec<CryptoSymbol> = coins
            .into_iter()
            .map(|coin| CryptoSymbol {
                symbol: coin.symbol.to_uppercase(),
                priority:9999999,
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
        2000 // 2 second delay for Demo API (30 calls/minute = ~2 second intervals)
    }

    fn requires_api_key(&self) -> bool {
        true // CoinGecko now requires API key for all requests
    }
}