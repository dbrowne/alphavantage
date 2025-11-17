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

use super::CryptoDataProvider;
use crate::crypto::{CryptoDataSource, CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct CoinGeckoProvider {
  pub api_key: Option<String>,
}

impl CoinGeckoProvider {
  pub fn new(api_key: Option<String>) -> Self {
    Self { api_key }
  }

  /// Fetch all coins from /coins/list (complete universe) with HTTP-level caching
  async fn fetch_all_coins_list(
    &self,
    client: &Client,
    cache_repo: Option<&Arc<dyn CacheRepository>>,
  ) -> Result<Vec<CoinGeckoCoin>, CryptoLoaderError> {
    let cache_key = "coingecko_http_coins_list";

    // Check HTTP response cache first
    if let Some(repo) = cache_repo {
      if let Ok(Some(cached_data)) = repo.get_json(cache_key, "coingecko_http").await {
        if let Ok(coins) = serde_json::from_value::<Vec<CoinGeckoCoin>>(cached_data) {
          info!("📦 HTTP cache hit for /coins/list: {} coins from cache", coins.len());
          return Ok(coins);
        }
      }
    }

    let api_key = self.api_key.as_ref().ok_or_else(|| {
      CryptoLoaderError::ApiKeyMissing("CoinGecko API key is required".to_string())
    })?;

    let (base_url, auth_param) = if api_key.starts_with("CG-") {
      ("https://pro-api.coingecko.com/api/v3", "x_cg_pro_api_key")
    } else {
      ("https://api.coingecko.com/api/v3", "x_cg_demo_api_key")
    };

    let url = format!("{}/coins/list", base_url);

    info!("🌐 HTTP call: Fetching /coins/list from CoinGecko API");

    let response = client
      .get(&url)
      .query(&[(auth_param, api_key)])
      .header("accept", "application/json")
      .send()
      .await?;

    if !response.status().is_success() {
      return Err(CryptoLoaderError::InvalidResponse {
        api_source: "CoinGecko".to_string(),
        message: format!("HTTP {}", response.status()),
      });
    }

    let coins: Vec<CoinGeckoCoin> = response.json().await?;
    info!("Fetched {} total coins from CoinGecko /coins/list", coins.len());

    // Cache HTTP response immediately
    if let Some(repo) = cache_repo {
      if let Ok(json_data) = serde_json::to_value(&coins) {
        match repo.set_json(cache_key, "coingecko_http", &url, json_data, 24).await {
          Ok(()) => {
            info!("💾 Cached HTTP response for /coins/list (api_source: coingecko_http)");
          }
          Err(e) => {
            warn!("❌ Failed to cache HTTP response for /coins/list: {}", e);
          }
        }
      } else {
        warn!("❌ Failed to serialize coins for HTTP caching");
      }
    }

    Ok(coins)
  }

  /// Fetch market cap rankings in batches from /coins/markets
  async fn fetch_rankings_batch(
    &self,
    client: &Client,
    page: u32,
  ) -> Result<Vec<CoinGeckoMarketCoin>, CryptoLoaderError> {
    let api_key = self.api_key.as_ref().unwrap(); // Already validated

    let (base_url, auth_param) = if api_key.starts_with("CG-") {
      ("https://pro-api.coingecko.com/api/v3", "x_cg_pro_api_key")
    } else {
      ("https://api.coingecko.com/api/v3", "x_cg_demo_api_key")
    };

    let url = format!("{}/coins/markets", base_url);

    debug!("Fetching rankings page {} from CoinGecko /coins/markets", page);

    let response = client
      .get(&url)
      .query(&[
        (auth_param, api_key),
        ("vs_currency", &"usd".to_string()),
        ("order", &"market_cap_desc".to_string()),
        ("per_page", &"250".to_string()),
        ("page", &page.to_string()),
        ("sparkline", &"false".to_string()),
      ])
      .header("accept", "application/json")
      .send()
      .await?;

    if !response.status().is_success() {
      warn!("Failed to fetch rankings page {}: HTTP {}", page, response.status());
      return Ok(Vec::new()); // Return empty instead of failing
    }

    let coins: Vec<CoinGeckoMarketCoin> = response.json().await?;
    debug!("Fetched {} ranked coins from page {}", coins.len(), page);

    // Add delay to respect rate limits
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    Ok(coins)
  }

  /// Build ranking map from multiple pages
  async fn build_rankings_map(
    &self,
    client: &Client,
  ) -> Result<HashMap<String, u32>, CryptoLoaderError> {
    let mut rankings_map = HashMap::new();

    info!("Building market cap rankings map from CoinGecko");

    // Fetch multiple pages to get rankings for top cryptocurrencies
    for page in 1..=20 {
      // Top 5000 cryptocurrencies (250 per page * 20 pages)
      match self.fetch_rankings_batch(client, page).await {
        Ok(ranked_batch) => {
          if ranked_batch.is_empty() {
            debug!("Page {} returned no results, stopping pagination", page);
            break;
          }

          for coin in ranked_batch {
            if let Some(rank) = coin.market_cap_rank {
              rankings_map.insert(coin.id.clone(), rank);
            }
          }

          debug!("Processed page {}, total rankings collected: {}", page, rankings_map.len());
        }
        Err(e) => {
          warn!("Failed to fetch rankings page {}: {}", page, e);
        }
      }
    }

    info!("Built rankings map with {} entries", rankings_map.len());
    Ok(rankings_map)
  }
}

// Struct for /coins/list response (no market data)
#[derive(Debug, Deserialize, Serialize)]
struct CoinGeckoCoin {
  id: String,
  symbol: String,
  name: String,
  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

// Struct for /coins/markets response (with rankings)
#[derive(Debug, Deserialize, Serialize)]
struct CoinGeckoMarketCoin {
  id: String,
  symbol: String,
  name: String,
  market_cap_rank: Option<u32>,
}

#[async_trait]
impl CryptoDataProvider for CoinGeckoProvider {
  async fn fetch_symbols(
    &self,
    client: &Client,
    cache_repo: Option<&Arc<dyn CacheRepository>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    info!("Fetching complete symbol universe from CoinGecko with rankings");

    // Step 1: Get complete universe of coins (with HTTP-level caching)
    let all_coins = self.fetch_all_coins_list(client, cache_repo).await?;

    // Step 2: Build rankings map for top cryptocurrencies
    let rankings_map = self.build_rankings_map(client).await?;

    // Step 3: Merge data - all coins loaded, ranked ones get priority
    let symbols: Vec<CryptoSymbol> = all_coins
      .into_iter()
      .map(|coin| {
        // Look up ranking, None if not in top rankings
        let looked_up_rank = rankings_map.get(&coin.id).copied();

        CryptoSymbol {
          symbol: coin.symbol.to_uppercase(),
          name: coin.name,
          base_currency: None,
          quote_currency: Some("USD".to_string()),
          market_cap_rank: looked_up_rank,
          priority: looked_up_rank.unwrap_or(9999999) as i32,
          source: CryptoDataSource::CoinGecko,
          source_id: coin.id,
          is_active: true,
          created_at: Utc::now(),
          additional_data: coin.extra,
        }
      })
      .collect();

    info!(
      "Processed {} total symbols ({} with rankings, {} without)",
      symbols.len(),
      symbols.iter().filter(|s| s.market_cap_rank.is_some()).count(),
      symbols.iter().filter(|s| s.market_cap_rank.is_none()).count()
    );

    Ok(symbols)
  }

  fn source_name(&self) -> &'static str {
    "CoinGecko"
  }

  fn rate_limit_delay(&self) -> u64 {
    2000 // 2 second delay between major operations
  }

  fn requires_api_key(&self) -> bool {
    true
  }
}
