/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

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
  msg: Option<String>,
  #[serde(rename = "traceId")]
  #[allow(dead_code)]
  trace_id: Option<String>,
  #[allow(dead_code)]
  tid: Option<String>, // Additional field in actual response
  data: Option<Vec<SosoValueCrypto>>,
}

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
  async fn fetch_symbols(
    &self,
    client: &Client,
    _cache_repo: Option<&Arc<dyn CacheRepository>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from SosoValue");

    let url = "https://openapi.sosovalue.com/openapi/v1/data/default/coin/list";

    let mut request = client.post(url).header("Content-Type", "application/json");

    if let Some(ref key) = self.api_key {
      request = request.header("x-soso-api-key", key);
    }

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
        priority: 9999999,
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
