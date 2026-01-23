/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! CoinGecko detailed coin data loader.

use crate::error::CryptoLoaderError;
use crate::traits::CryptoCache;
use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

/// CoinGecko detailed coin response
#[derive(Debug, Deserialize, Serialize)]
pub struct CoinGeckoDetailedCoin {
  #[serde(default)]
  pub id: Option<String>,

  #[serde(default)]
  pub symbol: Option<String>,

  #[serde(default)]
  pub name: Option<String>,

  #[serde(default)]
  pub links: CoinGeckoLinks,

  #[serde(default)]
  pub community_data: CommunityData,

  #[serde(default)]
  pub developer_data: DeveloperData,

  #[serde(default)]
  pub public_interest_stats: PublicInterestStats,

  #[serde(default)]
  pub sentiment_votes_up_percentage: Option<f64>,

  #[serde(default)]
  pub sentiment_votes_down_percentage: Option<f64>,

  #[serde(default)]
  pub coingecko_score: Option<f64>,

  #[serde(default)]
  pub developer_score: Option<f64>,

  #[serde(default)]
  pub community_score: Option<f64>,

  #[serde(default)]
  pub liquidity_score: Option<f64>,

  #[serde(default)]
  pub public_interest_score: Option<f64>,

  #[serde(default)]
  pub platforms: HashMap<String, Option<String>>,

  #[serde(default)]
  pub genesis_date: Option<String>,

  #[serde(default)]
  pub categories: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CoinGeckoLinks {
  #[serde(default)]
  pub homepage: Vec<String>,

  #[serde(default)]
  pub whitepaper: Option<String>,

  #[serde(default)]
  pub blockchain_site: Vec<String>,

  #[serde(default)]
  pub official_forum_url: Vec<String>,

  #[serde(default)]
  pub chat_url: Vec<String>,

  #[serde(default)]
  pub announcement_url: Vec<String>,

  #[serde(default)]
  pub twitter_screen_name: Option<String>,

  #[serde(default)]
  pub facebook_username: Option<String>,

  #[serde(default)]
  pub subreddit_url: Option<String>,

  #[serde(default)]
  pub repos_url: HashMap<String, Vec<String>>,

  #[serde(default)]
  pub telegram_channel_identifier: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CommunityData {
  #[serde(default)]
  pub twitter_followers: Option<i32>,

  #[serde(default)]
  pub reddit_subscribers: Option<i32>,

  #[serde(default)]
  pub reddit_accounts_active_48h: Option<i32>,

  #[serde(default)]
  pub telegram_channel_user_count: Option<i32>,

  #[serde(default)]
  pub facebook_likes: Option<i32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DeveloperData {
  #[serde(default)]
  pub forks: Option<i32>,

  #[serde(default)]
  pub stars: Option<i32>,

  #[serde(default)]
  pub subscribers: Option<i32>,

  #[serde(default)]
  pub total_issues: Option<i32>,

  #[serde(default)]
  pub closed_issues: Option<i32>,

  #[serde(default)]
  pub pull_requests_merged: Option<i32>,

  #[serde(default)]
  pub pull_request_contributors: Option<i32>,

  #[serde(default)]
  pub commit_count_4_weeks: Option<i32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PublicInterestStats {
  #[serde(default)]
  pub alexa_rank: Option<i32>,

  #[serde(default)]
  pub bing_matches: Option<i32>,
}

/// Output data combining social and technical info
#[derive(Debug)]
pub struct CryptoDetailedData {
  pub sid: i64,
  pub coingecko_id: String,
  pub social: CryptoSocialData,
  pub technical: CryptoTechnicalData,
}

#[derive(Debug)]
pub struct CryptoSocialData {
  pub website_url: Option<String>,
  pub whitepaper_url: Option<String>,
  pub github_url: Option<String>,
  pub twitter_handle: Option<String>,
  pub twitter_followers: Option<i32>,
  pub telegram_url: Option<String>,
  pub telegram_members: Option<i32>,
  pub discord_url: Option<String>,
  pub discord_members: Option<i32>,
  pub reddit_url: Option<String>,
  pub reddit_subscribers: Option<i32>,
  pub facebook_url: Option<String>,
  pub facebook_likes: Option<i32>,
  pub coingecko_score: Option<f64>,
  pub developer_score: Option<f64>,
  pub community_score: Option<f64>,
  pub liquidity_score: Option<f64>,
  pub public_interest_score: Option<f64>,
  pub sentiment_votes_up_pct: Option<f64>,
  pub sentiment_votes_down_pct: Option<f64>,
  pub blockchain_sites: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct CryptoTechnicalData {
  pub blockchain_platform: Option<String>,
  pub token_standard: Option<String>,
  pub github_forks: Option<i32>,
  pub github_stars: Option<i32>,
  pub github_subscribers: Option<i32>,
  pub github_total_issues: Option<i32>,
  pub github_closed_issues: Option<i32>,
  pub github_pull_requests: Option<i32>,
  pub github_contributors: Option<i32>,
  pub github_commits_4_weeks: Option<i32>,
  pub is_defi: Option<bool>,
  pub is_stablecoin: Option<bool>,
  pub is_nft_platform: Option<bool>,
  pub is_exchange_token: Option<bool>,
  pub is_gaming: Option<bool>,
  pub is_metaverse: Option<bool>,
  pub is_privacy_coin: Option<bool>,
  pub is_layer2: Option<bool>,
  pub is_wrapped: Option<bool>,
  pub genesis_date: Option<NaiveDate>,
}

/// Result from fetching coin details, tracking cache vs API
#[derive(Debug)]
struct FetchResult {
  coin: CoinGeckoDetailedCoin,
  from_cache: bool,
}

/// Input for coin details loading
#[derive(Debug, Clone)]
pub struct CoinInfo {
  pub sid: i64,
  pub symbol: String,
  pub coingecko_id: String,
}

/// Output from loading coin details
#[derive(Debug)]
pub struct CoinGeckoDetailsOutput {
  pub total_coins: usize,
  pub loaded_count: usize,
  pub errors: usize,
  pub cache_hits: usize,
  pub api_calls: usize,
  pub data: Vec<CryptoDetailedData>,
}

/// Configuration for the details loader
#[derive(Debug, Clone)]
pub struct DetailsLoaderConfig {
  pub max_concurrent: usize,
  pub retry_delay_ms: u64,
  pub show_progress: bool,
}

impl Default for DetailsLoaderConfig {
  fn default() -> Self {
    Self { max_concurrent: 5, retry_delay_ms: 200, show_progress: true }
  }
}

/// Loader for CoinGecko detailed coin data
pub struct CoinGeckoDetailsLoader {
  api_key: String,
  semaphore: Arc<Semaphore>,
  cache: Option<Arc<dyn CryptoCache>>,
  config: DetailsLoaderConfig,
}

impl CoinGeckoDetailsLoader {
  pub fn new(api_key: String, config: DetailsLoaderConfig) -> Self {
    Self {
      api_key,
      semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
      cache: None,
      config,
    }
  }

  pub fn with_cache(mut self, cache: Arc<dyn CryptoCache>) -> Self {
    self.cache = Some(cache);
    self
  }

  /// Load details for multiple coins
  pub async fn load(
    &self,
    coins: Vec<CoinInfo>,
  ) -> Result<CoinGeckoDetailsOutput, CryptoLoaderError> {
    info!("Loading detailed CoinGecko data for {} coins", coins.len());

    let progress = if self.config.show_progress {
      Some(Arc::new(ProgressBar::new(coins.len() as u64)))
    } else {
      None
    };

    let progress_for_finish = progress.clone();

    // Create HTTP client for API calls
    let http_client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .user_agent("CryptoLoaders-Rust/1.0")
      .build()
      .map_err(|e| CryptoLoaderError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

    let http_client = Arc::new(http_client);

    // Process coins concurrently
    let results = stream::iter(coins.into_iter())
      .map(|coin_info| {
        let client = http_client.clone();
        let semaphore = self.semaphore.clone();
        let progress = progress.clone();
        let loader = self.clone();

        async move {
          let _permit =
            semaphore.acquire().await.expect("Semaphore should not be closed during operation");

          if let Some(pb) = &progress {
            pb.set_message(format!("Processing {}", coin_info.symbol));
          }

          let result = loader.fetch_coin_details(&client, &coin_info.coingecko_id).await;

          if let Some(pb) = &progress {
            pb.inc(1);
          }

          match result {
            Ok(fetch_result) => {
              // Only add delay if data came from API (not from cache)
              if !fetch_result.from_cache {
                tokio::time::sleep(tokio::time::Duration::from_millis(loader.config.retry_delay_ms))
                  .await;
              }

              let detailed_data = Self::convert_to_crypto_data(
                coin_info.sid,
                coin_info.coingecko_id,
                fetch_result.coin,
              );
              Ok((detailed_data, fetch_result.from_cache))
            }
            Err(e) => {
              // Add delay on error (API was called)
              tokio::time::sleep(tokio::time::Duration::from_millis(loader.config.retry_delay_ms))
                .await;
              error!("Failed to load details for {}: {}", coin_info.symbol, e);
              Err(e)
            }
          }
        }
      })
      .buffer_unordered(self.config.max_concurrent)
      .collect::<Vec<_>>()
      .await;

    if let Some(pb) = progress_for_finish {
      pb.finish_with_message("CoinGecko details loading complete");
    }

    // Process results and track cache hits / API calls
    let mut loaded = Vec::new();
    let mut errors = 0;
    let mut cache_hits = 0;
    let mut api_calls = 0;

    for result in results {
      match result {
        Ok((data, from_cache)) => {
          loaded.push(data);
          if from_cache {
            cache_hits += 1;
          } else {
            api_calls += 1;
          }
        }
        Err(_) => errors += 1,
      }
    }

    info!(
      "CoinGecko details loading complete: {} loaded, {} errors, {} cache hits, {} API calls",
      loaded.len(),
      errors,
      cache_hits,
      api_calls
    );

    Ok(CoinGeckoDetailsOutput {
      total_coins: loaded.len() + errors,
      loaded_count: loaded.len(),
      errors,
      cache_hits,
      api_calls,
      data: loaded,
    })
  }

  /// Fetch detailed coin data with HTTP-level caching
  async fn fetch_coin_details(
    &self,
    client: &Client,
    coingecko_id: &str,
  ) -> Result<FetchResult, CryptoLoaderError> {
    let cache_key = format!("coingecko_http_details_{}", coingecko_id);

    // Check HTTP cache first
    if let Some(cache) = &self.cache {
      if let Ok(Some(cached_data)) = cache.get("coingecko_http", &cache_key).await {
        if let Ok(coin) = serde_json::from_str::<CoinGeckoDetailedCoin>(&cached_data) {
          debug!("HTTP cache hit for coin details: {}", coingecko_id);
          return Ok(FetchResult { coin, from_cache: true });
        }
      }
    }

    let (base_url, auth_param) = if self.api_key.starts_with("CG-") {
      ("https://pro-api.coingecko.com/api/v3", "x_cg_pro_api_key")
    } else {
      ("https://api.coingecko.com/api/v3", "x_cg_demo_api_key")
    };

    let url = format!("{}/coins/{}", base_url, coingecko_id);

    debug!("HTTP call: Fetching details for {}", coingecko_id);

    let response = client
      .get(&url)
      .query(&[
        (auth_param, &self.api_key),
        ("localization", &"false".to_string()),
        ("tickers", &"false".to_string()),
        ("market_data", &"false".to_string()),
        ("community_data", &"true".to_string()),
        ("developer_data", &"true".to_string()),
        ("sparkline", &"false".to_string()),
      ])
      .header("accept", "application/json")
      .send()
      .await?;

    let status = response.status();
    if !status.is_success() {
      let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
      return Err(CryptoLoaderError::ApiError(format!(
        "HTTP {}: {} - {}",
        status, coingecko_id, error_text
      )));
    }

    let response_text = response.text().await?;
    let coin: CoinGeckoDetailedCoin = serde_json::from_str(&response_text).map_err(|e| {
      error!("Failed to parse CoinGecko response for {}: {}", coingecko_id, e);
      CryptoLoaderError::ParseError(format!("JSON parse error for {}: {}", coingecko_id, e))
    })?;

    // Cache immediately
    if let Some(cache) = &self.cache {
      if let Ok(json_data) = serde_json::to_string(&coin) {
        let _ = cache.set("coingecko_http", &cache_key, &json_data, 168).await; // 7 days TTL
        debug!("Cached HTTP response for {}", coingecko_id);
      }
    }

    Ok(FetchResult { coin, from_cache: false })
  }

  /// Convert CoinGecko response to our data models
  fn convert_to_crypto_data(
    sid: i64,
    coingecko_id: String,
    coin: CoinGeckoDetailedCoin,
  ) -> CryptoDetailedData {
    let (blockchain_platform, token_standard) = Self::extract_platform_info(&coin.platforms);

    let blockchain_sites = if !coin.links.blockchain_site.is_empty() {
      serde_json::to_value(&coin.links.blockchain_site).ok()
    } else {
      None
    };

    let github_url =
      coin.links.repos_url.get("github").and_then(|urls| urls.first()).map(|s| s.to_string());

    let discord_url =
      coin.links.chat_url.iter().find(|url| url.contains("discord")).map(|s| s.to_string());

    let genesis_date =
      coin.genesis_date.and_then(|date_str| NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok());

    let categories_lower: Vec<String> = coin.categories.iter().map(|c| c.to_lowercase()).collect();

    let social = CryptoSocialData {
      website_url: coin.links.homepage.first().cloned(),
      whitepaper_url: coin.links.whitepaper.filter(|s| !s.is_empty()),
      github_url,
      twitter_handle: coin.links.twitter_screen_name.filter(|s| !s.is_empty()),
      twitter_followers: coin.community_data.twitter_followers,
      telegram_url: coin
        .links
        .telegram_channel_identifier
        .filter(|s| !s.is_empty())
        .map(|id| format!("https://t.me/{}", id)),
      telegram_members: coin.community_data.telegram_channel_user_count,
      discord_url,
      discord_members: None,
      reddit_url: coin.links.subreddit_url.filter(|s| !s.is_empty()),
      reddit_subscribers: coin.community_data.reddit_subscribers,
      facebook_url: coin
        .links
        .facebook_username
        .filter(|s| !s.is_empty())
        .map(|u| format!("https://facebook.com/{}", u)),
      facebook_likes: coin.community_data.facebook_likes,
      coingecko_score: coin.coingecko_score,
      developer_score: coin.developer_score,
      community_score: coin.community_score,
      liquidity_score: coin.liquidity_score,
      public_interest_score: coin.public_interest_score,
      sentiment_votes_up_pct: coin.sentiment_votes_up_percentage,
      sentiment_votes_down_pct: coin.sentiment_votes_down_percentage,
      blockchain_sites,
    };

    let technical = CryptoTechnicalData {
      blockchain_platform,
      token_standard,
      github_forks: coin.developer_data.forks,
      github_stars: coin.developer_data.stars,
      github_subscribers: coin.developer_data.subscribers,
      github_total_issues: coin.developer_data.total_issues,
      github_closed_issues: coin.developer_data.closed_issues,
      github_pull_requests: coin.developer_data.pull_requests_merged,
      github_contributors: coin.developer_data.pull_request_contributors,
      github_commits_4_weeks: coin.developer_data.commit_count_4_weeks,
      is_defi: Some(Self::has_category(&categories_lower, &["defi", "decentralized-finance"])),
      is_stablecoin: Some(Self::has_category(&categories_lower, &["stablecoin", "stablecoins"])),
      is_nft_platform: Some(Self::has_category(&categories_lower, &["nft", "non-fungible-tokens"])),
      is_exchange_token: Some(Self::has_category(
        &categories_lower,
        &["exchange", "centralized-exchange"],
      )),
      is_gaming: Some(Self::has_category(&categories_lower, &["gaming", "play-to-earn"])),
      is_metaverse: Some(Self::has_category(&categories_lower, &["metaverse"])),
      is_privacy_coin: Some(Self::has_category(&categories_lower, &["privacy-coins", "privacy"])),
      is_layer2: Some(Self::has_category(&categories_lower, &["layer-2", "ethereum-layer-2"])),
      is_wrapped: Some(
        coin.name.as_ref().map(|n| n.to_lowercase().contains("wrapped")).unwrap_or(false),
      ),
      genesis_date,
    };

    CryptoDetailedData { sid, coingecko_id, social, technical }
  }

  fn extract_platform_info(
    platforms: &HashMap<String, Option<String>>,
  ) -> (Option<String>, Option<String>) {
    if platforms.is_empty() {
      return (None, None);
    }

    let platform_name = platforms
      .iter()
      .find(|(k, v)| {
        !k.is_empty() && v.is_some() && v.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
      })
      .map(|(k, _)| k.clone());

    let token_standard =
      platform_name.as_ref().and_then(|name| match name.to_lowercase().as_str() {
        "ethereum" => Some("ERC-20".to_string()),
        "binance-smart-chain" | "bsc" => Some("BEP-20".to_string()),
        "polygon-pos" => Some("Polygon".to_string()),
        "solana" => Some("SPL".to_string()),
        "avalanche" => Some("Avalanche C-Chain".to_string()),
        _ => None,
      });

    (platform_name, token_standard)
  }

  fn has_category(categories: &[String], keywords: &[&str]) -> bool {
    categories.iter().any(|cat| keywords.iter().any(|keyword| cat.contains(keyword)))
  }
}

impl Clone for CoinGeckoDetailsLoader {
  fn clone(&self) -> Self {
    Self {
      api_key: self.api_key.clone(),
      semaphore: Arc::clone(&self.semaphore),
      cache: self.cache.clone(),
      config: self.config.clone(),
    }
  }
}