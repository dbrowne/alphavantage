/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

use anyhow::{Result, anyhow};
use av_client::AlphaVantageClient;
use av_database_postgres::models::crypto::{NewCryptoSocial, NewCryptoTechnical};
use av_database_postgres::repository::{CryptoRepository, DatabaseContext};
use av_loaders::crypto::{CoinGeckoDetailsInput, CoinGeckoDetailsLoader, CoinInfo};
use av_loaders::{DataLoader, LoaderConfig, LoaderContext};
use bigdecimal::BigDecimal;
use chrono::Utc;
use clap::Args;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};

use crate::config::Config;

#[derive(Args, Clone, Debug)]
pub struct CryptoDetailsArgs {
  /// Limit number of coins to load (for debugging)
  #[arg(short, long)]
  limit: Option<usize>,

  /// Number of concurrent requests
  #[arg(short, long, default_value = "5")]
  concurrent: usize,

  /// Continue on error instead of stopping
  #[arg(long)]
  continue_on_error: bool,

  /// Dry run - fetch data but don't save to database
  #[arg(long)]
  dry_run: bool,
}

pub async fn execute(args: CryptoDetailsArgs, config: Config) -> Result<()> {
  info!("Starting CoinGecko details loader");

  // Create database context
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow!("Failed to create database context: {}", e))?;
  let crypto_repo = db_context.crypto_repository();
  let cache_repo = Arc::new(db_context.cache_repository());

  // Get coins with CoinGecko mappings
  let coins = crypto_repo
    .get_cryptos_with_coingecko_ids(args.limit)
    .await
    .map_err(|e| anyhow!("Failed to query coins: {}", e))?;

  if coins.is_empty() {
    info!("No coins with CoinGecko mappings found");
    return Ok(());
  }

  info!("Found {} coins with CoinGecko mappings", coins.len());

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
  }

  // Get CoinGecko API key
  let api_key = std::env::var("COINGECKO_API_KEY")
    .map_err(|_| anyhow!("COINGECKO_API_KEY environment variable not set"))?;

  // Create API client
  let client = Arc::new(
    AlphaVantageClient::new(config.api_config)
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: false,
    batch_size: 100,
  };

  // Create loader context
  let context = LoaderContext::new(client, loader_config);

  // Create CoinGecko details loader with cache
  let loader =
    CoinGeckoDetailsLoader::new(api_key, args.concurrent).with_cache_repository(cache_repo);

  // Prepare input
  let coin_infos: Vec<CoinInfo> = coins
    .into_iter()
    .map(|(sid, symbol, coingecko_id)| CoinInfo { sid, symbol, coingecko_id })
    .collect();

  let input = CoinGeckoDetailsInput { coins: coin_infos };

  // Load data from API
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load details: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      return Ok(());
    }
  };

  info!(
    "API loading complete: {} loaded, {} errors, {} cache hits, {} API calls",
    output.loaded_count, output.errors, output.cache_hits, output.api_calls
  );

  // Save to database unless dry run
  if !args.dry_run && !output.data.is_empty() {
    let (social_saved, technical_saved) = save_details_to_db(&crypto_repo, output.data).await?;

    info!("Saved {} social and {} technical records to database", social_saved, technical_saved);
  } else if args.dry_run {
    info!("Dry run complete - would have saved {} records", output.loaded_count);
  }

  Ok(())
}

async fn save_details_to_db<R: CryptoRepository>(
  repo: &R,
  data: Vec<av_loaders::crypto::CryptoDetailedData>,
) -> Result<(usize, usize)> {
  let now = Utc::now();

  // Build social records
  let social_records: Vec<NewCryptoSocial> = data
    .iter()
    .map(|d| NewCryptoSocial {
      sid: d.sid,
      website_url: d.social.website_url.clone(),
      whitepaper_url: d.social.whitepaper_url.clone(),
      github_url: d.social.github_url.clone(),
      twitter_handle: d.social.twitter_handle.clone(),
      twitter_followers: d.social.twitter_followers,
      telegram_url: d.social.telegram_url.clone(),
      telegram_members: d.social.telegram_members,
      discord_url: d.social.discord_url.clone(),
      discord_members: d.social.discord_members,
      reddit_url: d.social.reddit_url.clone(),
      reddit_subscribers: d.social.reddit_subscribers,
      facebook_url: d.social.facebook_url.clone(),
      facebook_likes: d.social.facebook_likes,
      coingecko_score: d
        .social
        .coingecko_score
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      developer_score: d
        .social
        .developer_score
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      community_score: d
        .social
        .community_score
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      liquidity_score: d
        .social
        .liquidity_score
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      public_interest_score: d
        .social
        .public_interest_score
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      sentiment_votes_up_pct: d
        .social
        .sentiment_votes_up_pct
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      sentiment_votes_down_pct: d
        .social
        .sentiment_votes_down_pct
        .as_ref()
        .and_then(|f| BigDecimal::from_str(&f.to_string()).ok()),
      c_time: now,
      m_time: now,
    })
    .collect();

  // Build technical records
  let technical_records: Vec<NewCryptoTechnical> = data
    .iter()
    .map(|d| NewCryptoTechnical {
      sid: d.sid,
      blockchain_platform: d.technical.blockchain_platform.clone(),
      token_standard: d.technical.token_standard.clone(),
      consensus_mechanism: None,
      hashing_algorithm: None,
      block_time_minutes: None,
      block_reward: None,
      block_height: None,
      hash_rate: None,
      difficulty: None,
      github_forks: d.technical.github_forks,
      github_stars: d.technical.github_stars,
      github_subscribers: d.technical.github_subscribers,
      github_total_issues: d.technical.github_total_issues,
      github_closed_issues: d.technical.github_closed_issues,
      github_pull_requests: d.technical.github_pull_requests,
      github_contributors: d.technical.github_contributors,
      github_commits_4_weeks: d.technical.github_commits_4_weeks,
      is_defi: d.technical.is_defi.unwrap_or(false),
      is_stablecoin: d.technical.is_stablecoin.unwrap_or(false),
      is_nft_platform: d.technical.is_nft_platform.unwrap_or(false),
      is_exchange_token: d.technical.is_exchange_token.unwrap_or(false),
      is_gaming: d.technical.is_gaming.unwrap_or(false),
      is_metaverse: d.technical.is_metaverse.unwrap_or(false),
      is_privacy_coin: d.technical.is_privacy_coin.unwrap_or(false),
      is_layer2: d.technical.is_layer2.unwrap_or(false),
      is_wrapped: d.technical.is_wrapped.unwrap_or(false),
      genesis_date: d.technical.genesis_date,
      ico_price: None,
      ico_date: None,
      c_time: now,
      m_time: now,
    })
    .collect();

  // Save to database
  let social_saved = repo
    .batch_upsert_social(&social_records)
    .await
    .map_err(|e| anyhow!("Failed to save social data: {}", e))?;

  let technical_saved = repo
    .batch_upsert_technical(&technical_records)
    .await
    .map_err(|e| anyhow!("Failed to save technical data: {}", e))?;

  Ok((social_saved, technical_saved))
}
