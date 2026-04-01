/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
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

//! CoinPaprika cryptocurrency data provider.

use crate::error::CryptoLoaderError;
use crate::traits::{CryptoCache, CryptoDataProvider};
use crate::types::{CryptoDataSource, CryptoSymbol};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// CoinPaprika data provider.
pub struct CoinPaprikaProvider;

#[derive(Debug, Deserialize)]
struct CoinPaprikaCoin {
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
  async fn fetch_symbols(
    &self,
    client: &Client,
    _cache: Option<&Arc<dyn CryptoCache>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching symbols from CoinPaprika");

    let url = "https://api.coinpaprika.com/v1/coins";
    let response = client.get(url).send().await?;

    if response.status().as_u16() == 429 {
      return Err(CryptoLoaderError::RateLimitExceeded("CoinPaprika".to_string()));
    }

    if !response.status().is_success() {
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "CoinPaprika".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let coins: Vec<CoinPaprikaCoin> = response.json().await?;

    debug!("CoinPaprika returned {} coins", coins.len());

    let symbols: Vec<CryptoSymbol> = coins
      .into_iter()
      .filter(|coin| coin.is_active)
      .map(|coin| CryptoSymbol {
        symbol: coin.symbol.to_uppercase(),
        priority: 9999999,
        name: coin.name,
        base_currency: None,
        quote_currency: Some("USD".to_string()),
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
    500
  }

  fn requires_api_key(&self) -> bool {
    false
  }
}
