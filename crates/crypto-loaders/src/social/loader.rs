/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Social data loader for cryptocurrency projects.
//!
//! Defines the types and [`SocialLoader`] for fetching social media metrics
//! (Twitter, Telegram, Reddit, Discord, Facebook) and composite scores
//! (CoinGecko, developer, community, liquidity) for cryptocurrency projects.
//!
//! # Implementation status
//!
//! The [`SocialLoader::load_data`] method is currently a **stub** that
//! returns [`ProcessedSocialData::empty`] for each input symbol. The
//! private `fetch_coingecko_social` method is also unimplemented (marked
//! `#[allow(dead_code)]`).
//!
//! For production use, prefer
//! [`CoinGeckoDetailsLoader`](crate::loaders::CoinGeckoDetailsLoader)
//! which provides a complete implementation of social data fetching.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::CryptoLoaderError;

/// Convenience alias for `Result<T, CryptoLoaderError>`.
pub type SocialLoaderResult<T> = Result<T, CryptoLoaderError>;

// ─── Input types ────────────────────────────────────────────────────────────

/// Identifies a cryptocurrency for social data fetching.
///
/// `coingecko_id` is optional because not all coins have been mapped to
/// CoinGecko yet. When `None`, the loader cannot fetch CoinGecko-specific
/// social data for that coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForSocial {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub coingecko_id: Option<String>,
}

// ─── Output types ───────────────────────────────────────────────────────────

/// Flat social data for a single cryptocurrency, ready for database insertion.
///
/// Covers 5 social platforms (Twitter, Telegram, Discord, Reddit, Facebook)
/// with URL + follower/subscriber count, plus 7 composite scores using
/// [`Decimal`] for database-compatible precision.
///
/// Use [`empty`](ProcessedSocialData::empty) to construct an all-`None` record.
#[derive(Debug, Clone)]
pub struct ProcessedSocialData {
  pub sid: i64,
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
  pub coingecko_score: Option<Decimal>,
  pub developer_score: Option<Decimal>,
  pub community_score: Option<Decimal>,
  pub liquidity_score: Option<Decimal>,
  pub public_interest_score: Option<Decimal>,
  pub sentiment_votes_up_pct: Option<Decimal>,
  pub sentiment_votes_down_pct: Option<Decimal>,
}

impl ProcessedSocialData {
  /// Creates an all-`None` social data record for the given `sid`.
  ///
  /// Useful as a placeholder when social data is unavailable or when
  /// the loader hasn't been fully implemented yet.
  pub fn empty(sid: i64) -> Self {
    Self {
      sid,
      website_url: None,
      whitepaper_url: None,
      github_url: None,
      twitter_handle: None,
      twitter_followers: None,
      telegram_url: None,
      telegram_members: None,
      discord_url: None,
      discord_members: None,
      reddit_url: None,
      reddit_subscribers: None,
      facebook_url: None,
      facebook_likes: None,
      coingecko_score: None,
      developer_score: None,
      community_score: None,
      liquidity_score: None,
      public_interest_score: None,
      sentiment_votes_up_pct: None,
      sentiment_votes_down_pct: None,
    }
  }
}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for [`SocialLoader`].
///
/// # Defaults
///
/// | Field              | Default | Description                            |
/// |--------------------|---------|----------------------------------------|
/// | `coingecko_api_key` | `None` | CoinGecko API key (Pro or Demo)        |
/// | `github_token`     | `None`  | GitHub personal access token           |
/// | `skip_github`      | `false` | Skip GitHub enrichment entirely        |
/// | `delay_ms`         | `100`   | Delay between API requests (ms)        |
/// | `batch_size`       | `10`    | Symbols per batch                      |
/// | `max_retries`      | `3`     | Retry attempts on transient failure    |
/// | `timeout_seconds`  | `30`    | HTTP request timeout                   |
#[derive(Debug, Clone)]
pub struct CryptoSocialConfig {
  /// CoinGecko API key (optional — free tier works without it).
  pub coingecko_api_key: Option<String>,
  /// GitHub personal access token for enriched repo data.
  pub github_token: Option<String>,
  /// When `true`, skips all GitHub API calls.
  pub skip_github: bool,
  /// Delay between consecutive API requests in milliseconds.
  pub delay_ms: u64,
  /// Number of symbols to process per batch.
  pub batch_size: usize,
  /// Maximum retry attempts per symbol on transient errors.
  pub max_retries: u32,
  /// HTTP request timeout in seconds.
  pub timeout_seconds: u64,
}

impl Default for CryptoSocialConfig {
  fn default() -> Self {
    Self {
      coingecko_api_key: None,
      github_token: None,
      skip_github: false,
      delay_ms: 100,
      batch_size: 10,
      max_retries: 3,
      timeout_seconds: 30,
    }
  }
}

/// Input specifying which symbols to load social data for.
///
/// - `symbols` — the list of coins to process (`None` = error).
/// - `update_existing` — when `true`, overwrites existing social data;
///   when `false`, skips symbols that already have data.
#[derive(Debug, Clone)]
pub struct CryptoSocialInput {
  pub symbols: Option<Vec<CryptoSymbolForSocial>>,
  pub update_existing: bool,
}

// ─── Loader ─────────────────────────────────────────────────────────────────

/// Social data loader for cryptocurrency projects (**stub implementation**).
///
/// Designed to fetch social metrics from CoinGecko and GitHub, but
/// `load_data()` currently returns empty records. For production social
/// data loading, use [`CoinGeckoDetailsLoader`](crate::loaders::CoinGeckoDetailsLoader).
///
/// Implements `Clone` (clones the HTTP client).
#[derive(Clone)]
pub struct SocialLoader {
  config: CryptoSocialConfig,
  #[allow(dead_code)]
  client: reqwest::Client,
}

impl SocialLoader {
  /// Creates a new loader with a default `reqwest::Client`.
  pub fn new(config: CryptoSocialConfig) -> Self {
    Self { config, client: reqwest::Client::new() }
  }

  /// Creates a loader with a caller-supplied HTTP client.
  pub fn with_client(config: CryptoSocialConfig, client: reqwest::Client) -> Self {
    Self { config, client }
  }

  /// Returns a reference to the loader's configuration.
  pub fn config(&self) -> &CryptoSocialConfig {
    &self.config
  }

  /// Loads social data for the given symbols (**stub — returns empty records**).
  ///
  /// Returns a [`ProcessedSocialData::empty`] for each input symbol.
  /// Returns `Err` if `input.symbols` is `None`.
  pub async fn load_data(
    &self,
    input: &CryptoSocialInput,
  ) -> SocialLoaderResult<Vec<ProcessedSocialData>> {
    let symbols = input
      .symbols
      .as_ref()
      .ok_or_else(|| CryptoLoaderError::ApiError("No symbols provided".to_string()))?;

    let mut results = Vec::with_capacity(symbols.len());

    for symbol in symbols {
      // TODO: Implement actual API calls to fetch social data
      // For now, return empty data for each symbol
      let social_data = ProcessedSocialData::empty(symbol.sid);
      results.push(social_data);
    }

    Ok(results)
  }

  /// Fetches social data for a single coin from CoinGecko (**unimplemented stub**).
  #[allow(dead_code)]
  async fn fetch_coingecko_social(
    &self,
    coingecko_id: &str,
  ) -> SocialLoaderResult<Option<ProcessedSocialData>> {
    // TODO: Implement CoinGecko API call for social data
    // This would call /coins/{id} endpoint and extract social fields
    let _ = coingecko_id;
    Ok(None)
  }
}
