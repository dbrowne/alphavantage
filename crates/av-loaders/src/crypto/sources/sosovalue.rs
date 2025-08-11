use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

pub struct SosoValueProvider {
  pub api_key: Option<String>,
}

impl SosoValueProvider {
  pub fn new(api_key: Option<String>) -> Self {
    Self { api_key }
  }
}

// FIXED: Response structure based on actual API response
#[derive(Debug, Deserialize)]
struct SosoValueResponse {
  code: i32,
  msg: Option<String>,
  #[serde(rename = "traceId")]
  trace_id: Option<String>,
  tid: Option<String>, // Additional field in actual response
  data: Option<Vec<SosoValueCrypto>>,
}

// FIXED: Crypto structure based on actual API response
#[derive(Debug, Deserialize)]
struct SosoValueCrypto {
  #[serde(rename = "currencyId")]
  currency_id: i64, // This is the ID field (was "id" in docs, actually "currencyId")
  #[serde(rename = "currencyName")]
  currency_name: String, // This is the symbol (was "name" in docs, actually "currencyName")
  #[serde(rename = "fullName")]
  full_name: String, // This matches the docs
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl CryptoDataProvider for SosoValueProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from SosoValue");

    // FIXED: Use the correct endpoint from documentation
    let url = "https://openapi.sosovalue.com/openapi/v1/data/default/coin/list";

    // FIXED: Use POST method as specified in documentation
    let mut request = client.post(url).header("Content-Type", "application/json");

    // FIXED: Use the correct header name from documentation
    if let Some(ref key) = self.api_key {
      request = request.header("x-soso-api-key", key);
    }

    // FIXED: Add the required empty JSON body from documentation
    request = request.json(&serde_json::json!({}));

    debug!("SosoValue request: POST {} with headers", url);

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
        api_source: "SosoValue".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    // DEBUG: Get the raw response text first to see what we're actually receiving
    let response_text = response.text().await?;
    debug!("SosoValue raw response: {}", response_text);

    // Try to parse the response text as JSON to see the structure
    let response_value: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
      error!("Failed to parse SosoValue response as JSON: {}", e);
      error!("Raw response was: {}", response_text);
      CryptoLoaderError::InvalidResponse {
        api_source: "SosoValue".to_string(),
        message: format!("Invalid JSON response: {}", e),
      }
    })?;

    debug!("SosoValue parsed JSON: {:#}", response_value);

    // Now try to deserialize into our expected structure
    let api_response: SosoValueResponse =
      serde_json::from_value(response_value.clone()).map_err(|e| {
        error!("Failed to deserialize SosoValue response: {}", e);
        error!("Expected structure: SosoValueResponse with code, msg, traceId, data fields");
        error!("Actual JSON: {:#}", response_value);
        CryptoLoaderError::InvalidResponse {
          api_source: "SosoValue".to_string(),
          message: format!("Response structure mismatch: {}", e),
        }
      })?;

    if api_response.code != 0 {
      let error_msg = api_response.msg.unwrap_or_else(|| "Unknown error".to_string());
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "SosoValue".to_string(),
        message: format!("API Error: {}", error_msg),
      });
    }

    let data = api_response.data.ok_or_else(|| CryptoLoaderError::InvalidResponse {
      api_source: "SosoValue".to_string(),
      message: "No data field in response".to_string(),
    })?;

    debug!("SosoValue returned {} cryptocurrencies", data.len());

    let symbols: Vec<CryptoSymbol> = data
      .into_iter()
      .map(|crypto| CryptoSymbol {
        // Based on actual response: currencyName is the symbol, fullName is the name
        symbol: crypto.currency_name.to_uppercase(),
        name: crypto.full_name,
        base_currency: None,
        quote_currency: Some("USD".to_string()),
        market_cap_rank: None, // Not provided in the response
        source: CryptoDataSource::SosoValue,
        source_id: crypto.currency_id.to_string(), // Use currencyId as source_id
        is_active: true,                           // Assume all returned symbols are active
        created_at: Utc::now(),
        additional_data: crypto.extra,
      })
      .collect();

    info!("Successfully processed {} symbols from SosoValue", symbols.len());
    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "SosoValue"
  }

  fn rate_limit_delay(&self) -> u64 {
    500 // Conservative rate limiting
  }

  fn requires_api_key(&self) -> bool {
    true // SosoValue requires API key
  }
}
