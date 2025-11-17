/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::{DataLoader, LoaderContext, LoaderResult, ProcessState};

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

/// Loader for CoinGecko detailed coin data
pub struct CoinGeckoDetailsLoader {
  api_key: String,
  semaphore: Arc<Semaphore>,
  cache_repository: Option<Arc<dyn CacheRepository>>,
}

impl CoinGeckoDetailsLoader {
  pub fn new(api_key: String, max_concurrent: usize) -> Self {
    Self { api_key, semaphore: Arc::new(Semaphore::new(max_concurrent)), cache_repository: None }
  }

  pub fn with_cache_repository(mut self, cache_repo: Arc<dyn CacheRepository>) -> Self {
    self.cache_repository = Some(cache_repo);
    self
  }

  /// Fetch detailed coin data with HTTP-level caching
  async fn fetch_coin_details(
    &self,
    client: &Client,
    coingecko_id: &str,
  ) -> Result<FetchResult, Box<dyn std::error::Error + Send + Sync>> {
    let cache_key = format!("coingecko_http_details_{}", coingecko_id);

    // Check HTTP cache first
    if let Some(cache_repo) = &self.cache_repository {
      if let Ok(Some(cached_data)) = cache_repo.get_json(&cache_key, "coingecko_http").await {
        if let Ok(coin) = serde_json::from_value::<CoinGeckoDetailedCoin>(cached_data) {
          debug!("📦 HTTP cache hit for coin details: {}", coingecko_id);
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

    debug!("🌐 HTTP call: Fetching details for {}", coingecko_id);

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
      return Err(format!("HTTP {}: {} - {}", status, coingecko_id, error_text).into());
    }

    // Get response text for better error messages
    let response_text = response.text().await?;
    let coin: CoinGeckoDetailedCoin = match serde_json::from_str(&response_text) {
      Ok(coin) => coin,
      Err(e) => {
        error!("Failed to parse CoinGecko response for {}: {}", coingecko_id, e);
        error!(
          "Response text (first 1000 chars): {}",
          &response_text.chars().take(1000).collect::<String>()
        );
        return Err(format!("JSON parse error for {}: {}", coingecko_id, e).into());
      }
    };

    // Cache immediately
    if let Some(cache_repo) = &self.cache_repository {
      if let Ok(json_data) = serde_json::to_value(&coin) {
        let _ = cache_repo.set_json(&cache_key, "coingecko_http", &url, json_data, 168).await; // 7 days TTL
        debug!("💾 Cached HTTP response for {}", coingecko_id);
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
    // Extract platform info
    let (blockchain_platform, token_standard) = Self::extract_platform_info(&coin.platforms);

    // Extract blockchain sites as JSON
    let blockchain_sites = if !coin.links.blockchain_site.is_empty() {
      serde_json::to_value(&coin.links.blockchain_site).ok()
    } else {
      None
    };

    // Extract GitHub URL
    let github_url =
      coin.links.repos_url.get("github").and_then(|urls| urls.first()).map(|s| s.to_string());

    // Extract Discord URL from chat_url
    let discord_url =
      coin.links.chat_url.iter().find(|url| url.contains("discord")).map(|s| s.to_string());

    // Parse genesis date
    let genesis_date =
      coin.genesis_date.and_then(|date_str| NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok());

    // Categorize coin
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
      discord_members: None, // CoinGecko doesn't provide this
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

    // Get the first platform with a valid (non-empty, non-null) value
    let platform_name = platforms
      .iter()
      .find(|(k, v)| {
        !k.is_empty() && v.is_some() && v.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
      })
      .map(|(k, _)| k.clone());

    // Try to determine token standard
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

/// Input for the loader
#[derive(Debug)]
pub struct CoinGeckoDetailsInput {
  pub coins: Vec<CoinInfo>,
}

#[derive(Debug, Clone)]
pub struct CoinInfo {
  pub sid: i64,
  pub symbol: String,
  pub coingecko_id: String,
}

/// Output from the loader
#[derive(Debug)]
pub struct CoinGeckoDetailsOutput {
  pub total_coins: usize,
  pub loaded_count: usize,
  pub errors: usize,
  pub cache_hits: usize,
  pub api_calls: usize,
  pub data: Vec<CryptoDetailedData>,
}

#[async_trait]
impl DataLoader for CoinGeckoDetailsLoader {
  type Input = CoinGeckoDetailsInput;
  type Output = CoinGeckoDetailsOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    info!("Loading detailed CoinGecko data for {} coins", input.coins.len());

    // Track process if enabled
    if let Some(tracker) = &context.process_tracker {
      tracker.start("coingecko_details_loader").await?;
    }

    let progress = if context.config.show_progress {
      Some(Arc::new(ProgressBar::new(input.coins.len() as u64)))
    } else {
      None
    };

    let progress_for_finish = progress.clone();
    let retry_delay = context.config.retry_delay_ms;
    let max_concurrent = context.config.max_concurrent_requests;

    // Create HTTP client for API calls
    let http_client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .user_agent("AlphaVantage-Rust-Client/1.0")
      .build()
      .map_err(|e| crate::LoaderError::IoError(format!("Failed to create HTTP client: {}", e)))?;

    let http_client = Arc::new(http_client);

    // Process coins concurrently
    let results = stream::iter(input.coins.into_iter())
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

          // Add delay to respect rate limits
          tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;

          match result {
            Ok(fetch_result) => {
              let detailed_data = Self::convert_to_crypto_data(
                coin_info.sid,
                coin_info.coingecko_id,
                fetch_result.coin,
              );
              Ok((detailed_data, fetch_result.from_cache))
            }
            Err(e) => {
              error!("Failed to load details for {}: {}", coin_info.symbol, e);
              Err(e)
            }
          }
        }
      })
      .buffer_unordered(max_concurrent)
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

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      tracker
        .complete(if errors > 0 {
          ProcessState::CompletedWithErrors
        } else {
          ProcessState::Success
        })
        .await?;
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

  fn name(&self) -> &'static str {
    "CoinGeckoDetailsLoader"
  }
}

impl Clone for CoinGeckoDetailsLoader {
  fn clone(&self) -> Self {
    Self {
      api_key: self.api_key.clone(),
      semaphore: Arc::clone(&self.semaphore),
      cache_repository: self.cache_repository.clone(),
    }
  }
}
