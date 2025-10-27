// crates/av-cli/src/commands/load/crypto_overview.rs

use anyhow::{Result, anyhow};
use av_database_postgres::models::crypto::{
  NewCryptoOverviewBasic, NewCryptoOverviewMetrics, NewCryptoSocial, NewCryptoTechnical,
};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use clap::Args;
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::commands::load::numeric_helpers::{f64_to_price_bigdecimal, f64_to_supply_bigdecimal};
use crate::config::Config;

#[derive(Args, Debug)]
pub struct CryptoOverviewArgs {
  /// Specific symbols to load (comma-separated)
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Limit number of symbols to load
  #[arg(short, long)]
  limit: Option<usize>,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,

  /// Continue on errors
  #[arg(short = 'k', long)]
  continue_on_error: bool,

  /// Delay between requests in milliseconds
  #[arg(long, default_value = "2000", env = "CRYPTO_API_DELAY_MS")]
  delay_ms: u64,

  /// GitHub personal access token (optional, increases rate limit)
  #[arg(long, env = "GITHUB_TOKEN")]
  github_token: Option<String>,

  /// Check rate limit status before starting
  #[arg(long)]
  check_rate_limit: bool,

  /// Include GitHub data scraping
  #[arg(long, default_value = "true")]
  include_github: bool,

  /// CoinMarketCap API key (overrides environment variable)
  #[arg(long, env = "CMC_API_KEY")]
  pub cmc_api_key: Option<String>,

  /// Enable response caching to reduce API costs
  #[arg(long, default_value = "true")]
  enable_cache: bool,

  /// Cache TTL in hours (default: 24 hours for overview data)
  #[arg(long, default_value = "24")]
  cache_ttl_hours: u32,

  /// Force refresh - ignore cache and fetch fresh data
  #[arg(long)]
  force_refresh: bool,

  /// Include all symbols (including those with priority >= 9999999)
  #[arg(long)]
  no_priority_filter: bool,
}

#[derive(Args, Debug)]
pub struct UpdateGitHubArgs {
  /// Specific symbols to update
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Limit number of symbols to update
  #[arg(short, long)]
  limit: Option<usize>,

  /// Delay between requests (GitHub rate limit)
  #[arg(long, default_value = "3000", env = "GITHUB_API_DELAY_MS")]
  delay_ms: u64,

  /// GitHub personal access token
  #[arg(long, env = "GITHUB_TOKEN")]
  github_token: Option<String>,

  /// Check rate limit status before starting
  #[arg(long)]
  check_rate_limit: bool,
}

/// Cryptocurrency overview data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoOverviewData {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub description: String,
  pub market_cap: i64,
  pub circulating_supply: f64,
  pub total_supply: Option<f64>,
  pub max_supply: Option<f64>,
  pub price_usd: f64,
  pub volume_24h: i64,
  pub price_change_24h: f64,
  pub ath: f64, // All-time high
  pub ath_date: Option<NaiveDate>,
  pub atl: f64, // All-time low
  pub atl_date: Option<NaiveDate>,
  pub rank: u32,
  pub website: Option<String>,
  pub whitepaper: Option<String>,
  pub github: Option<String>,
}

/// GitHub repository data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubData {
  pub forks: Option<i32>,
  pub stars: Option<i32>,
  pub watchers: Option<i32>,
  pub open_issues: Option<i32>,
  pub contributors: Option<i32>,
  pub commits_30d: Option<i32>,
  pub pull_requests: Option<i32>,
  pub last_commit_date: Option<NaiveDate>,
}

// Cache query result structure for SQL queries
#[derive(QueryableByName, Debug)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: DateTime<Utc>,
}

/// Configuration for caching
struct CacheConfig {
  enable_cache: bool,
  cache_ttl_hours: u32,
  force_refresh: bool,
  database_url: String,
}

/// Generate cache key for overview requests
fn generate_cache_key(sid: i64, symbol: &str) -> String {
  format!("crypto_overview_{}_{}", sid, symbol)
}

/// Get cached response if available and not expired
async fn get_cached_overview(
  cache_config: &CacheConfig,
  cache_key: &str,
) -> Option<CryptoOverviewData> {
  if !cache_config.enable_cache || cache_config.force_refresh {
    return None;
  }

  let mut conn = match diesel::PgConnection::establish(&cache_config.database_url) {
    Ok(conn) => conn,
    Err(e) => {
      debug!("Failed to connect for cache check: {}", e);
      return None;
    }
  };

  let cached_entry: Option<CacheQueryResult> = sql_query(
    "SELECT response_data, expires_at FROM api_response_cache
         WHERE cache_key = $1 AND expires_at > NOW()",
  )
  .bind::<sql_types::Text, _>(cache_key)
  .get_result(&mut conn)
  .optional()
  .unwrap_or(None);

  if let Some(cache_result) = cached_entry {
    info!("📦 Cache hit for {} (expires: {})", cache_key, cache_result.expires_at);

    // Deserialize cached overview data
    if let Ok(overview) = serde_json::from_value::<CryptoOverviewData>(cache_result.response_data) {
      return Some(overview);
    }
  }

  debug!("Cache miss for {}", cache_key);
  None
}

/// Store overview data in cache
async fn store_cached_overview(
  cache_config: &CacheConfig,
  cache_key: &str,
  overview: &CryptoOverviewData,
) -> Result<()> {
  if !cache_config.enable_cache {
    return Ok(());
  }

  let mut conn = diesel::PgConnection::establish(&cache_config.database_url)
    .map_err(|e| anyhow!("Cache connection failed: {}", e))?;

  let overview_json = serde_json::to_value(overview)?;
  let now = Utc::now();
  let expires_at = now + chrono::Duration::hours(cache_config.cache_ttl_hours as i64);

  // Dummy values for required fields
  let endpoint_url = format!("crypto_overview/{}", cache_key);
  let status_code = 200;

  sql_query(
    "INSERT INTO api_response_cache
     (cache_key, api_source, endpoint_url, response_data, status_code, cached_at, expires_at)
     VALUES ($1, $2, $3, $4, $5, $6, $7)
     ON CONFLICT (cache_key)
     DO UPDATE SET response_data = $4, cached_at = $6, expires_at = $7",
  )
  .bind::<sql_types::Text, _>(cache_key)
  .bind::<sql_types::Text, _>("crypto_overview")
  .bind::<sql_types::Text, _>(&endpoint_url)
  .bind::<sql_types::Jsonb, _>(&overview_json)
  .bind::<sql_types::Int4, _>(status_code)
  .bind::<sql_types::Timestamptz, _>(now)
  .bind::<sql_types::Timestamptz, _>(expires_at)
  .execute(&mut conn)?;

  debug!("💾 Cached overview for {} (TTL: {}h)", cache_key, cache_config.cache_ttl_hours);

  Ok(())
}

/// Clean expired cache entries
pub async fn cleanup_expired_cache(database_url: &str) -> Result<usize> {
  let mut conn = diesel::PgConnection::establish(database_url)
    .map_err(|e| anyhow!("Connection failed: {}", e))?;

  let deleted_count = sql_query(
    "DELETE FROM api_response_cache WHERE expires_at < NOW() AND api_source = 'crypto_overview'",
  )
  .execute(&mut conn)?;

  if deleted_count > 0 {
    info!("🧹 Cleaned up {} expired crypto overview cache entries", deleted_count);
  }

  Ok(deleted_count)
}

/// Main execute function
pub async fn execute(args: CryptoOverviewArgs, config: Config) -> Result<()> {
  info!("Starting cryptocurrency overview loader");

  // Debug: Check if CMC API key is present
  if let Some(ref key) = args.cmc_api_key {
    info!("✅ CoinMarketCap API key detected (length: {} chars)", key.len());
    if key.len() < 20 {
      warn!("⚠️  CMC API key seems too short - might be invalid");
    }
  } else {
    warn!("❌ No CoinMarketCap API key provided - will use free sources only");
    warn!("   Set CMC_API_KEY environment variable or use --cmc-api-key flag");
  }

  // Create cache configuration
  let cache_config = CacheConfig {
    enable_cache: args.enable_cache,
    cache_ttl_hours: args.cache_ttl_hours,
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
  };

  // Log cache status
  if cache_config.enable_cache {
    if cache_config.force_refresh {
      info!("🔄 Cache enabled but FORCE REFRESH is on - will bypass cache");
    } else {
      info!("💾 Cache ENABLED (TTL: {} hours)", cache_config.cache_ttl_hours);
    }
  } else {
    info!("⚠️  Cache DISABLED - all requests will hit API");
  }

  // Clean up expired cache entries at start
  if cache_config.enable_cache {
    match cleanup_expired_cache(&cache_config.database_url).await {
      Ok(count) if count > 0 => info!("🧹 Cleaned up {} expired cache entries", count),
      Ok(_) => debug!("No expired cache entries to clean"),
      Err(e) => warn!("Failed to clean up cache: {}", e),
    }
  }

  // Create HTTP client
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("Mozilla/5.0 (compatible; CryptoOverviewBot/1.0)")
    .build()?;

  // Check GitHub rate limit if requested
  if args.check_rate_limit && args.include_github {
    check_github_rate_limit(&client, args.github_token.as_ref()).await?;
  }

  // Adjust delay based on GitHub token availability
  let github_delay_ms = if args.github_token.is_some() {
    500 // With auth: up to 5000 requests/hour
  } else {
    3000 // Without auth: 60 requests/hour
  };

  // Get cryptocurrency symbols that need overviews
  let symbols_to_load = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let symbols = args.symbols.clone();
    let limit = args.limit;
    let no_priority_filter = args.no_priority_filter;
    move || get_crypto_symbols_to_load(&database_url, symbols, limit, no_priority_filter)
  })
  .await??;

  if symbols_to_load.is_empty() {
    info!("No cryptocurrency symbols need overview data");
    return Ok(());
  }

  info!("Found {} cryptocurrency symbols to load overviews for", symbols_to_load.len());

  if !args.no_priority_filter {
    info!("📊 Loading only primary symbols (priority < 9999999)");
  } else {
    info!("📊 Loading ALL symbols (including non-primary)");
  }

  if cache_config.enable_cache {
    info!("💾 Caching enabled (TTL: {}h)", cache_config.cache_ttl_hours);
    if cache_config.force_refresh {
      info!("🔄 Force refresh mode - bypassing cache");
    }
  } else {
    info!("⚠️  Caching disabled");
  }

  if args.github_token.is_some() {
    info!("GitHub authentication detected - increased rate limits active");
  } else if args.include_github {
    warn!("No GitHub token found - limited to 60 GitHub requests/hour");
    warn!("Add GITHUB_TOKEN to your .env file for 5000 requests/hour");
  }

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    for (_, symbol, name) in &symbols_to_load {
      info!("Would load overview for: {} - {}", symbol, name);
    }
    return Ok(());
  }

  // Load overviews
  let mut all_overviews_with_github = Vec::new();
  let progress = ProgressBar::new(symbols_to_load.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .unwrap()
      .progress_chars("##-"),
  );

  let mut cache_hits = 0;
  let mut api_calls = 0;

  for (sid, symbol, name) in symbols_to_load {
    progress.set_message(format!("Loading {}", symbol));

    // Try to get from cache first
    let cache_key = generate_cache_key(sid, &symbol);
    let (overview, made_api_call) = if let Some(cached) =
      get_cached_overview(&cache_config, &cache_key).await
    {
      cache_hits += 1;
      (Some(cached), false)
    } else {
      // Fetch from API
      match fetch_crypto_overview(&client, sid, &symbol, &name, args.cmc_api_key.as_deref()).await {
        Ok(overview) => {
          api_calls += 1;

          // Store in cache
          if cache_config.enable_cache {
            if let Err(e) = store_cached_overview(&cache_config, &cache_key, &overview).await {
              warn!("Failed to cache overview for {}: {}", symbol, e);
            }
          }

          (Some(overview), true)
        }
        Err(e) => {
          error!("Failed to fetch overview for {}: {}", symbol, e);
          progress.inc(1);
          continue;
        }
      }
    };

    if let Some(overview) = overview {
      // Fetch GitHub data if URL is available and GitHub scraping is enabled
      let github_data = if args.include_github {
        if let Some(ref github_url) = overview.github {
          debug!("Fetching GitHub data for {}", symbol);
          let gh_data =
            fetch_github_data(&client, Some(github_url), args.github_token.as_ref()).await;

          if let Some(ref gh) = gh_data {
            info!(
              "GitHub stats for {}: {} stars, {} forks, {} contributors",
              symbol,
              gh.stars.unwrap_or(0),
              gh.forks.unwrap_or(0),
              gh.contributors.unwrap_or(0)
            );
          }

          // Add GitHub-specific delay
          sleep(Duration::from_millis(github_delay_ms)).await;

          gh_data
        } else {
          None
        }
      } else {
        None
      };

      info!(
        "Successfully loaded overview for {}: Market Cap ${}, Rank #{}",
        symbol, overview.market_cap, overview.rank
      );

      all_overviews_with_github.push((overview, github_data));
    }

    progress.inc(1);

    // Rate limiting for crypto APIs - sleep after EVERY API call
    if made_api_call {
      sleep(Duration::from_millis(args.delay_ms)).await;
    }
  }

  progress.finish_with_message("Loading complete");

  info!(
    "📊 Load statistics: {} cache hits, {} API calls ({:.1}% cache hit rate)",
    cache_hits,
    api_calls,
    if (cache_hits + api_calls) > 0 {
      (cache_hits as f64 / (cache_hits + api_calls) as f64) * 100.0
    } else {
      0.0
    }
  );

  if !all_overviews_with_github.is_empty() {
    // Save to database
    let saved_count = tokio::task::spawn_blocking({
      let database_url = config.database_url.clone();
      move || save_crypto_overviews_with_github_to_db(&database_url, all_overviews_with_github)
    })
    .await??;

    info!("Successfully saved {} cryptocurrency overviews to database", saved_count);
  } else {
    warn!("No overviews to save");
  }

  Ok(())
}

/// Update GitHub data for existing cryptocurrency overviews
pub async fn update_github_data(args: UpdateGitHubArgs, config: Config) -> Result<()> {
  info!("Starting GitHub data update for cryptocurrencies");

  // Create HTTP client
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("Mozilla/5.0 (compatible; CryptoOverviewBot/1.0)")
    .build()?;

  // Check rate limit if requested
  if args.check_rate_limit {
    check_github_rate_limit(&client, args.github_token.as_ref()).await?;
  }

  // Get symbols with GitHub URLs
  let symbols_with_github = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let symbols = args.symbols.clone();
    let limit = args.limit;
    move || get_symbols_with_github(&database_url, symbols, limit)
  })
  .await??;

  if symbols_with_github.is_empty() {
    info!("No symbols found with GitHub URLs");
    return Ok(());
  }

  info!("Found {} symbols with GitHub URLs to update", symbols_with_github.len());

  let progress = ProgressBar::new(symbols_with_github.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .unwrap()
      .progress_chars("##-"),
  );

  let mut updated_count = 0;

  for (sid, symbol, github_url) in symbols_with_github {
    progress.set_message(format!("Updating GitHub data for {}", symbol));

    if let Some(github_data) =
      fetch_github_data(&client, github_url.as_ref(), args.github_token.as_ref()).await
    {
      // Update database with GitHub data
      let update_result = tokio::task::spawn_blocking({
        let database_url = config.database_url.clone();
        let gh_data = github_data.clone();
        move || {
          use av_database_postgres::schema::crypto_technical;

          let mut conn = PgConnection::establish(&database_url)?;

          diesel::update(crypto_technical::table.filter(crypto_technical::sid.eq(sid)))
            .set((
              crypto_technical::github_forks.eq(gh_data.forks),
              crypto_technical::github_stars.eq(gh_data.stars),
              crypto_technical::github_subscribers.eq(gh_data.watchers),
              crypto_technical::github_total_issues.eq(gh_data.open_issues),
              crypto_technical::github_pull_requests.eq(gh_data.pull_requests),
              crypto_technical::github_contributors.eq(gh_data.contributors),
              crypto_technical::github_commits_4_weeks.eq(gh_data.commits_30d),
              crypto_technical::m_time.eq(Utc::now().naive_utc()),
            ))
            .execute(&mut conn)?;

          Result::<(), anyhow::Error>::Ok(())
        }
      })
      .await?;

      if update_result.is_ok() {
        info!(
          "Updated GitHub data for {}: {} stars, {} forks",
          symbol,
          github_data.stars.unwrap_or(0),
          github_data.forks.unwrap_or(0)
        );
        updated_count += 1;
      }
    }

    progress.inc(1);
    sleep(Duration::from_millis(args.delay_ms)).await;
  }

  progress
    .finish_with_message(format!("Updated GitHub data for {} cryptocurrencies", updated_count));
  Ok(())
}

/// Get cryptocurrency symbols that need overviews
fn get_crypto_symbols_to_load(
  database_url: &str,
  specific_symbols: Option<Vec<String>>,
  limit: Option<usize>,
  no_priority_filter: bool,
) -> Result<Vec<(i64, String, String)>> {
  use av_database_postgres::schema::symbols::dsl::*;

  let mut conn = PgConnection::establish(database_url)
    .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

  // Query for cryptocurrency symbols where overview = false
  let mut query = symbols
    .filter(sec_type.eq("Cryptocurrency"))
    .filter(overview.eq(false))
    .select((sid, symbol, name))
    .into_boxed();

  // Apply priority filter by default (only load primary symbols)
  if !no_priority_filter {
    query = query.filter(priority.lt(9999999));
    info!("Filtering to symbols with priority < 9999999 (primary symbols only)");
  } else {
    info!("Loading all symbols (no priority filter applied)");
  }

  // Filter by specific symbols if provided
  if let Some(ref symbol_list) = specific_symbols {
    query = query.filter(symbol.eq_any(symbol_list));
  }

  // Apply limit if specified
  if let Some(limit_val) = limit {
    query = query.limit(limit_val as i64);
  }

  // Execute query
  let results = query.order(symbol.asc()).load::<(i64, String, String)>(&mut conn)?;

  if results.is_empty() && specific_symbols.is_some() {
    warn!(
      "No cryptocurrency symbols found that need overviews (or they might not be type 'Cryptocurrency')"
    );
  }

  Ok(results)
}

/// Get symbols with GitHub URLs for updating
fn get_symbols_with_github(
  database_url: &str,
  specific_symbols: Option<Vec<String>>,
  limit: Option<usize>,
) -> Result<Vec<(i64, String, Option<String>)>> {
  use av_database_postgres::schema::{crypto_social, symbols};

  let mut conn = PgConnection::establish(database_url)?;

  let mut query = symbols::table
    .inner_join(crypto_social::table)
    .filter(symbols::sec_type.eq("Cryptocurrency"))
    .select((symbols::sid, symbols::symbol, crypto_social::github_url))
    .into_boxed();

  if let Some(ref symbol_list) = specific_symbols {
    query = query.filter(symbols::symbol.eq_any(symbol_list));
  }

  if let Some(limit_val) = limit {
    query = query.limit(limit_val as i64);
  }

  let results = query.load::<(i64, String, Option<String>)>(&mut conn)?;

  // Filter out None GitHub URLs
  Ok(results.into_iter().filter(|(_, _, github)| github.is_some()).collect())
}

/// Fetch cryptocurrency overview from multiple sources
async fn fetch_crypto_overview(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
  cmc_api_key: Option<&str>,
) -> Result<CryptoOverviewData> {
  // Try multiple sources in order of preference

  // 0. Try CoinMarketCap FIRST (if API key is available)
  if let Some(api_key) = cmc_api_key {
    match fetch_from_coinmarketcap(client, sid, symbol, name, api_key).await {
      Ok(data) => {
        info!("Successfully fetched {} data from CoinMarketCap", symbol);
        return Ok(data);
      }
      Err(e) => {
        warn!("CoinMarketCap failed for {}: {}", symbol, e);
        // Small delay before trying next source
        sleep(Duration::from_millis(500)).await;
      }
    }
  }

  // 1. Try CoinGecko     todo!  Get rid of this !!
  match fetch_from_coingecko_free(client, sid, symbol, name).await {
    Ok(data) => return Ok(data),
    Err(e) => {
      debug!("CoinGecko failed for {}: {}", symbol, e);
      sleep(Duration::from_millis(500)).await;
    }
  }

  // 2. Try CoinPaprika
  match fetch_from_coinpaprika(client, sid, symbol, name).await {
    //todo: get rid of this
    Ok(data) => return Ok(data),
    Err(e) => {
      debug!("CoinPaprika failed for {}: {}", symbol, e);
      sleep(Duration::from_millis(500)).await;
    }
  }

  // 3. Try CoinCap
  match fetch_from_coincap(client, sid, symbol, name).await {
    Ok(data) => return Ok(data), //todo:: Get rid of this
    Err(e) => debug!("CoinCap failed for {}: {}", symbol, e),
  }

  // 4. If all else fails, return error
  Err(anyhow!("Failed to fetch data from all sources for {}", symbol))
}

/// Fetch from CoinGecko without API key (respects free tier limits)
async fn fetch_from_coingecko_free(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
) -> Result<CryptoOverviewData> {
  // Get coin ID mapping
  let coin_id = get_coingecko_id(symbol);

  let url = format!(
    "https://pro-api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&market_data=true&community_data=false&developer_data=false",
    coin_id
  );

  debug!("Fetching from CoinGecko: {}", url);

  let response = client.get(&url).timeout(Duration::from_secs(10)).send().await?;

  if response.status() != 200 {
    return Err(anyhow!("CoinGecko returned status: {}", response.status()));
  }

  let data: Value = response.json().await?;

  // Parse the JSON response
  let market_data = &data["market_data"];

  Ok(CryptoOverviewData {
    sid,
    symbol: symbol.to_string(),
    name: name.to_string(),
    description: data["description"]["en"].as_str().unwrap_or("").to_string(),
    market_cap: market_data["market_cap"]["usd"].as_f64().unwrap_or(0.0) as i64,
    circulating_supply: market_data["circulating_supply"].as_f64().unwrap_or(0.0),
    total_supply: market_data["total_supply"].as_f64(),
    max_supply: market_data["max_supply"].as_f64(),
    price_usd: market_data["current_price"]["usd"].as_f64().unwrap_or(0.0),
    volume_24h: market_data["total_volume"]["usd"].as_f64().unwrap_or(0.0) as i64,
    price_change_24h: market_data["price_change_percentage_24h"].as_f64().unwrap_or(0.0),
    ath: market_data["ath"]["usd"].as_f64().unwrap_or(0.0),
    ath_date: market_data["ath_date"]["usd"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
    atl: market_data["atl"]["usd"].as_f64().unwrap_or(0.0),
    atl_date: market_data["atl_date"]["usd"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
    rank: data["market_cap_rank"].as_u64().unwrap_or(9999999) as u32,
    website: data["links"]["homepage"][0].as_str().map(|s| s.to_string()).filter(|s| !s.is_empty()),
    whitepaper: data["links"]["whitepaper"]
      .as_str()
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
    github: data["links"]["repos_url"]["github"]
      .as_array()
      .and_then(|arr| arr.first())
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
  })
}

/// Fetch from CoinPaprika (no API key required)
async fn fetch_from_coinpaprika(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
) -> Result<CryptoOverviewData> {
  // Get coin ID
  let coin_id = get_coinpaprika_id(symbol);

  // Get coin details
  let coin_url = format!("https://api.coinpaprika.com/v1/coins/{}", coin_id);
  let coin_response = client.get(&coin_url).send().await?;

  if coin_response.status() != 200 {
    return Err(anyhow!("CoinPaprika coin API returned status: {}", coin_response.status()));
  }

  let coin_data: Value = coin_response.json().await?;

  // Get ticker data for price info
  let ticker_url = format!("https://api.coinpaprika.com/v1/tickers/{}", coin_id);
  let ticker_response = client.get(&ticker_url).send().await?;

  if ticker_response.status() != 200 {
    return Err(anyhow!("CoinPaprika ticker API returned status: {}", ticker_response.status()));
  }

  let ticker_data: Value = ticker_response.json().await?;

  Ok(CryptoOverviewData {
    sid,
    symbol: symbol.to_string(),
    name: name.to_string(),
    description: coin_data["description"].as_str().unwrap_or("").to_string(),
    market_cap: ticker_data["quotes"]["USD"]["market_cap"].as_f64().unwrap_or(0.0) as i64,
    circulating_supply: ticker_data["circulating_supply"].as_f64().unwrap_or(0.0),
    total_supply: ticker_data["total_supply"].as_f64(),
    max_supply: ticker_data["max_supply"].as_f64(),
    price_usd: ticker_data["quotes"]["USD"]["price"].as_f64().unwrap_or(0.0),
    volume_24h: ticker_data["quotes"]["USD"]["volume_24h"].as_f64().unwrap_or(0.0) as i64,
    price_change_24h: ticker_data["quotes"]["USD"]["percent_change_24h"].as_f64().unwrap_or(0.0),
    ath: ticker_data["quotes"]["USD"]["ath_price"].as_f64().unwrap_or(0.0),
    ath_date: ticker_data["quotes"]["USD"]["ath_date"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
    atl: 0.0, // Not provided by CoinPaprika
    atl_date: None,
    rank: ticker_data["rank"].as_u64().unwrap_or(9999999) as u32,
    website: coin_data["links"]["website"]
      .as_array()
      .and_then(|arr| arr.first())
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
    whitepaper: coin_data["whitepaper"]["link"].as_str().map(|s| s.to_string()),
    github: coin_data["links"]["source_code"]
      .as_array()
      .and_then(|arr| arr.first())
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty() && s.contains("github")),
  })
}

/// Fetch from CoinCap (no API key required)
async fn fetch_from_coincap(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
) -> Result<CryptoOverviewData> {
  let asset_id = get_coincap_id(symbol);

  let url = format!("https://api.coincap.io/v2/assets/{}", asset_id);

  let response = client.get(&url).timeout(Duration::from_secs(10)).send().await?;

  if response.status() != 200 {
    return Err(anyhow!("CoinCap returned status: {}", response.status()));
  }

  let data: Value = response.json().await?;
  let asset = &data["data"];

  Ok(CryptoOverviewData {
    sid,
    symbol: symbol.to_string(),
    name: name.to_string(),
    description: format!("{} cryptocurrency", name), // CoinCap doesn't provide descriptions
    market_cap: asset["marketCapUsd"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
      as i64,
    circulating_supply: asset["supply"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
    total_supply: None, // Not provided by CoinCap
    max_supply: asset["maxSupply"].as_str().and_then(|s| s.parse::<f64>().ok()),
    price_usd: asset["priceUsd"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
    volume_24h: asset["volumeUsd24Hr"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
      as i64,
    price_change_24h: asset["changePercent24Hr"]
      .as_str()
      .and_then(|s| s.parse::<f64>().ok())
      .unwrap_or(0.0),
    ath: 0.0, // Not provided by CoinCap
    ath_date: None,
    atl: 0.0,
    atl_date: None,
    rank: asset["rank"].as_str().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
    website: Some(format!("https://coincap.io/assets/{}", asset_id)), // Default to CoinCap page
    whitepaper: None,
    github: None,
  })
}

async fn fetch_from_coinmarketcap(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
  api_key: &str,
) -> Result<CryptoOverviewData> {
  let url = "https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest";

  debug!("Calling CMC API for {} with key: {}...", symbol, &api_key[..8.min(api_key.len())]);

  let response = client
      .get(url)
      .header("X-CMC_PRO_API_KEY", api_key)
      .header("Accept", "application/json")
      .query(&[
        ("symbol", symbol),
        ("convert", "USD"),
        ("aux", "num_market_pairs,cmc_rank,date_added,tags,platform,max_supply,circulating_supply,total_supply"),
      ])
      .timeout(Duration::from_secs(10))
      .send()
      .await?;

  debug!("CMC API response status for {}: {}", symbol, response.status());

  if response.status() != 200 {
    return Err(anyhow!("CoinMarketCap returned status: {}", response.status()));
  }

  let cmc_response: serde_json::Value = response.json().await?;

  // Check for API errors
  if let Some(status) = cmc_response.get("status") {
    if let Some(error_code) = status.get("error_code").and_then(|v| v.as_i64()) {
      if error_code != 0 {
        let error_msg =
          status.get("error_message").and_then(|v| v.as_str()).unwrap_or("Unknown CMC error");
        return Err(anyhow!("CoinMarketCap API error: {}", error_msg));
      }
    }
  }

  // Extract cryptocurrency data
  let crypto_data = cmc_response
    .get("data")
    .and_then(|d| d.get(symbol))
    .and_then(|arr| arr.as_array())
    .and_then(|arr| arr.first())
    .ok_or_else(|| anyhow!("CoinMarketCap response missing data for {}", symbol))?;

  let usd_quote = crypto_data
    .get("quote")
    .and_then(|q| q.get("USD"))
    .ok_or_else(|| anyhow!("Missing USD quote data for {}", symbol))?;

  Ok(CryptoOverviewData {
    sid,
    symbol: symbol.to_string(),
    name: crypto_data.get("name").and_then(|n| n.as_str()).unwrap_or(name).to_string(),
    description: format!("{} cryptocurrency", name),
    market_cap: usd_quote.get("market_cap").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
    circulating_supply: crypto_data
      .get("circulating_supply")
      .and_then(|v| v.as_f64())
      .unwrap_or(0.0),
    total_supply: crypto_data.get("total_supply").and_then(|v| v.as_f64()),
    max_supply: crypto_data.get("max_supply").and_then(|v| v.as_f64()),
    price_usd: usd_quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0),
    volume_24h: usd_quote.get("volume_24h").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
    price_change_24h: usd_quote.get("percent_change_24h").and_then(|v| v.as_f64()).unwrap_or(0.0),
    ath: 0.0, // Not in basic API response
    ath_date: None,
    atl: 0.0,
    atl_date: None,
    rank: crypto_data.get("cmc_rank").and_then(|v| v.as_u64()).unwrap_or(9999999) as u32,
    website: None, // Need metadata endpoint for this
    whitepaper: None,
    github: None,
  })
}

// Helper functions for API ID mapping

fn get_coingecko_id(symbol: &str) -> String {
  // todo: This is for the free access but should delete
  match symbol.to_uppercase().as_str() {
    "BTC" => "bitcoin".to_string(),
    "ETH" => "ethereum".to_string(),
    "BNB" => "binancecoin".to_string(),
    "XRP" => "ripple".to_string(),
    "ADA" => "cardano".to_string(),
    "DOGE" => "dogecoin".to_string(),
    "SOL" => "solana".to_string(),
    "TRX" => "tron".to_string(),
    "DOT" => "polkadot".to_string(),
    "MATIC" => "matic-network".to_string(),
    "AVAX" => "avalanche-2".to_string(),
    "SHIB" => "shiba-inu".to_string(),
    "DAI" => "dai".to_string(),
    "WBTC" => "wrapped-bitcoin".to_string(),
    "LTC" => "litecoin".to_string(),
    "BCH" => "bitcoin-cash".to_string(),
    "LINK" => "chainlink".to_string(),
    "LEO" => "leo-token".to_string(),
    "UNI" => "uniswap".to_string(),
    "ATOM" => "cosmos".to_string(),
    "XLM" => "stellar".to_string(),
    "OKB" => "okb".to_string(),
    "ETC" => "ethereum-classic".to_string(),
    "XMR" => "monero".to_string(),
    "ICP" => "internet-computer".to_string(),
    "FIL" => "filecoin".to_string(),
    "HBAR" => "hedera".to_string(),
    "LDO" => "lido-dao".to_string(),
    "CRO" => "crypto-com-chain".to_string(),
    "VET" => "vechain".to_string(),
    "ALGO" => "algorand".to_string(),
    "USDC" => "usd-coin".to_string(),
    "USDT" => "tether".to_string(),
    "BUSD" => "binance-usd".to_string(),
    "1ST" => "firstblood".to_string(),
    _ => symbol.to_lowercase(),
  }
}

fn get_coinpaprika_id(symbol: &str) -> String {
  //todo:: delete
  match symbol.to_uppercase().as_str() {
    "BTC" => "btc-bitcoin".to_string(),
    "ETH" => "eth-ethereum".to_string(),
    "BNB" => "bnb-binance-coin".to_string(),
    "XRP" => "xrp-xrp".to_string(),
    "ADA" => "ada-cardano".to_string(),
    "DOGE" => "doge-dogecoin".to_string(),
    "SOL" => "sol-solana".to_string(),
    "TRX" => "trx-tron".to_string(),
    "DOT" => "dot-polkadot".to_string(),
    "MATIC" => "matic-polygon".to_string(),
    "LTC" => "ltc-litecoin".to_string(),
    "SHIB" => "shib-shiba-inu".to_string(),
    "AVAX" => "avax-avalanche".to_string(),
    "LINK" => "link-chainlink".to_string(),
    "ATOM" => "atom-cosmos".to_string(),
    "XLM" => "xlm-stellar".to_string(),
    "XMR" => "xmr-monero".to_string(),
    "ETC" => "etc-ethereum-classic".to_string(),
    "BCH" => "bch-bitcoin-cash".to_string(),
    "ALGO" => "algo-algorand".to_string(),
    "VET" => "vet-vechain".to_string(),
    "ICP" => "icp-internet-computer".to_string(),
    "FIL" => "fil-filecoin".to_string(),
    "1ST" => "1st-firstblood".to_string(),
    _ => symbol.to_lowercase(),
  }
}

fn get_coincap_id(symbol: &str) -> String {
  //todo:: This should be deleted
  match symbol.to_uppercase().as_str() {
    "BTC" => "bitcoin".to_string(),
    "ETH" => "ethereum".to_string(),
    "BNB" => "binance-coin".to_string(),
    "XRP" => "xrp".to_string(),
    "ADA" => "cardano".to_string(),
    "DOGE" => "dogecoin".to_string(),
    "SOL" => "solana".to_string(),
    "DOT" => "polkadot".to_string(),
    "MATIC" => "polygon".to_string(),
    "LTC" => "litecoin".to_string(),
    "SHIB" => "shiba-inu".to_string(),
    "AVAX" => "avalanche".to_string(),
    "LINK" => "chainlink".to_string(),
    "ATOM" => "cosmos".to_string(),
    "XLM" => "stellar".to_string(),
    "XMR" => "monero".to_string(),
    "ETC" => "ethereum-classic".to_string(),
    "BCH" => "bitcoin-cash".to_string(),
    "ALGO" => "algorand".to_string(),
    "VET" => "vechain".to_string(),
    "TRX" => "tron".to_string(),
    "1ST" => "firstblood".to_string(),
    _ => symbol.to_lowercase(),
  }
}

/// Fetch GitHub repository data
async fn fetch_github_data(
  client: &reqwest::Client,
  github_url: Option<&String>,
  github_token: Option<&String>,
) -> Option<GitHubData> {
  let github_url = github_url?;

  // Extract owner and repo from GitHub URL
  let re = Regex::new(r"github\.com/([^/]+)/([^/]+)").ok()?;
  let caps = re.captures(github_url)?;
  let owner = caps.get(1)?.as_str();
  let repo = caps.get(2)?.as_str().trim_end_matches(".git");

  // Fetch repository data
  let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);

  let mut req = client.get(&repo_url).header("User-Agent", "CryptoOverviewBot");

  if let Some(token) = github_token {
    req = req.header("Authorization", format!("token {}", token));
  }

  let repo_response = req.send().await.ok()?;

  if repo_response.status() != 200 {
    warn!("GitHub API returned status {} for {}/{}", repo_response.status(), owner, repo);
    return None;
  }

  let repo_data: Value = repo_response.json().await.ok()?;

  // Fetch contributors count
  let contributors_url =
    format!("https://api.github.com/repos/{}/{}/contributors?per_page=1", owner, repo);

  let mut contrib_req = client.get(&contributors_url).header("User-Agent", "CryptoOverviewBot");

  if let Some(token) = github_token {
    contrib_req = contrib_req.header("Authorization", format!("token {}", token));
  }

  let contrib_response = contrib_req.send().await.ok()?;
  let contributors = if contrib_response.status() == 200 {
    contrib_response
      .headers()
      .get("Link")
      .and_then(|v| v.to_str().ok())
      .and_then(|link| {
        let re = Regex::new(r#"page=(\d+)>; rel="last""#).ok()?;
        let caps = re.captures(link)?;
        caps.get(1)?.as_str().parse::<i32>().ok()
      })
      .or_else(|| Some(1))
  } else {
    None
  };

  // Fetch commits in last 30 days
  let since = Utc::now() - chrono::Duration::days(30);
  let commits_url = format!(
    "https://api.github.com/repos/{}/{}/commits?since={}&per_page=1",
    owner,
    repo,
    since.to_rfc3339()
  );

  let mut commits_req = client.get(&commits_url).header("User-Agent", "CryptoOverviewBot");

  if let Some(token) = github_token {
    commits_req = commits_req.header("Authorization", format!("token {}", token));
  }

  let commits_response = commits_req.send().await.ok()?;
  let commits_30d = if commits_response.status() == 200 {
    commits_response
      .headers()
      .get("Link")
      .and_then(|v| v.to_str().ok())
      .and_then(|link| {
        let re = Regex::new(r#"page=(\d+)>; rel="last""#).ok()?;
        let caps = re.captures(link)?;
        caps.get(1)?.as_str().parse::<i32>().ok()
      })
      .or_else(|| Some(1))
  } else {
    None
  };

  Some(GitHubData {
    forks: repo_data["forks_count"].as_i64().map(|v| v as i32),
    stars: repo_data["stargazers_count"].as_i64().map(|v| v as i32),
    watchers: repo_data["subscribers_count"].as_i64().map(|v| v as i32),
    open_issues: repo_data["open_issues_count"].as_i64().map(|v| v as i32),
    contributors,
    commits_30d,
    pull_requests: None, // Would need separate API call
    last_commit_date: repo_data["pushed_at"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
  })
}

/// Check GitHub rate limit
async fn check_github_rate_limit(
  client: &reqwest::Client,
  github_token: Option<&String>,
) -> Result<()> {
  let url = "https://api.github.com/rate_limit";
  let mut req = client.get(url).header("User-Agent", "CryptoOverviewBot");

  if let Some(token) = github_token {
    req = req.header("Authorization", format!("token {}", token));
  }

  let response = req.send().await?;
  let rate_limit: Value = response.json().await?;

  let core_limit = &rate_limit["resources"]["core"];
  let remaining = core_limit["remaining"].as_u64().unwrap_or(0);
  let limit = core_limit["limit"].as_u64().unwrap_or(0);

  info!("GitHub API rate limit: {}/{} remaining", remaining, limit);

  if remaining < 10 {
    warn!("GitHub API rate limit is low. Consider waiting before continuing.");
  }

  Ok(())
}

/// Save cryptocurrency overviews with GitHub data to database
fn save_crypto_overviews_with_github_to_db(
  database_url: &str,
  overviews: Vec<(CryptoOverviewData, Option<GitHubData>)>,
) -> Result<usize> {
  use av_database_postgres::schema::{
    crypto_overview_basic, crypto_overview_metrics, crypto_social, crypto_technical, symbols,
  };

  let mut conn = PgConnection::establish(database_url)
    .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

  let mut saved_count = 0;
  let _now_t = Utc::now().naive_utc();

  // Use transaction for all inserts
  conn.transaction::<_, anyhow::Error, _>(|conn| {
    for (overview, github_data) in overviews {
      // helper for f64 to I 64 conversion with overflow protection
      let safe_f64_to_i64 = |value: f64, field_name: &str, symbol: &str| -> Option<i64> {
        if value.is_nan() || value.is_infinite() {
          warn!("{} for {} is invalid (NaN or Infinite), setting to None", field_name, symbol);
          return None;
        }

        // Check if value exceeds i64 range
        if value > i64::MAX as f64 {
          warn!("{} for {} exceeds i64::MAX ({}), capping to i64::MAX", field_name, symbol, value);
          Some(i64::MAX)
        } else if value < i64::MIN as f64 {
          warn!("{} for {} is below i64::MIN ({}), capping to i64::MIN", field_name, symbol, value);
          Some(i64::MIN)
        } else {
          Some(value as i64)
        }
      };

      // Convert numeric values to BigDecimal
      let current_price_bd =
        f64_to_price_bigdecimal(overview.price_usd, "current_price", overview.sid);
      let price_change_pct =
        f64_to_price_bigdecimal(overview.price_change_24h, "price_change_24h", overview.sid);
      let circulating_supply_bd =
        f64_to_supply_bigdecimal(overview.circulating_supply, "circulating_supply", overview.sid);
      let total_supply_bd = overview
        .total_supply
        .and_then(|ts| f64_to_supply_bigdecimal(ts, "total_supply", overview.sid));
      let max_supply_bd =
        overview.max_supply.and_then(|ms| f64_to_supply_bigdecimal(ms, "max_supply", overview.sid));
      let ath_bd = f64_to_price_bigdecimal(overview.ath, "ath", overview.sid);
      let atl_bd = f64_to_price_bigdecimal(overview.atl, "atl", overview.sid);

      // Safely convert market_cap and volume_24h with overflow protection
      let market_cap_safe =
        safe_f64_to_i64(overview.market_cap as f64, "market_cap", &overview.symbol);
      let volume_24h_safe =
        safe_f64_to_i64(overview.volume_24h as f64, "volume_24h", &overview.symbol);

      // Calculate fully diluted valuation with overflow protection
      let fully_diluted_valuation = match (&current_price_bd, &max_supply_bd) {
        (Some(_), Some(_)) => {
          let price_f64 = overview.price_usd;
          let max_supply_f64 = overview.max_supply.unwrap_or(0.0);
          let fdv = price_f64 * max_supply_f64;
          safe_f64_to_i64(fdv, "fully_diluted_valuation", &overview.symbol)
        }
        _ => None,
      };

      // Create the values that need to be borrowed
      let slug = overview.symbol.to_lowercase().replace(" ", "-");
      let market_cap_rank = if overview.rank == 0 || overview.rank == 9999999 {
        None
      } else {
        Some(overview.rank as i32)
      };
      let now = chrono::Utc::now();

      // Create basic overview
      let new_overview_basic = NewCryptoOverviewBasic {
        sid: &overview.sid,
        symbol: &overview.symbol,
        name: &overview.name,
        slug: Some(&slug),
        description: Some(overview.description.as_str()),
        market_cap_rank: market_cap_rank.as_ref(),
        market_cap: market_cap_safe.as_ref(),
        fully_diluted_valuation: fully_diluted_valuation.as_ref(),
        volume_24h: volume_24h_safe.as_ref(),
        volume_change_24h: None,
        current_price: current_price_bd.as_ref(),
        circulating_supply: circulating_supply_bd.as_ref(),
        total_supply: total_supply_bd.as_ref(),
        max_supply: max_supply_bd.as_ref(),
        last_updated: Some(&now),
      };

      // Insert basic overview
      diesel::insert_into(crypto_overview_basic::table)
        .values(&new_overview_basic)
        .on_conflict(crypto_overview_basic::sid)
        .do_nothing()
        .execute(conn)?;

      // Create overview metrics
      let ath_date_dt = overview.ath_date.map(|d| {
        DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap_or_default(), Utc)
      });
      let atl_date_dt = overview.atl_date.map(|d| {
        DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap_or_default(), Utc)
      });

      let new_overview_metrics = NewCryptoOverviewMetrics {
        sid: &overview.sid,
        price_change_24h: None,
        price_change_pct_24h: price_change_pct.as_ref(),
        price_change_pct_7d: None,
        price_change_pct_14d: None,
        price_change_pct_30d: None,
        price_change_pct_60d: None,
        price_change_pct_200d: None,
        price_change_pct_1y: None,
        ath: ath_bd.as_ref(),
        ath_date: ath_date_dt.as_ref(),
        ath_change_percentage: None,
        atl: atl_bd.as_ref(),
        atl_date: atl_date_dt.as_ref(),
        atl_change_percentage: None,
        roi_times: None,
        roi_currency: None,
        roi_percentage: None,
      };

      diesel::insert_into(crypto_overview_metrics::table)
        .values(&new_overview_metrics)
        .on_conflict(crypto_overview_metrics::sid)
        .do_nothing()
        .execute(conn)?;

      // Create social data entry
      let new_social = NewCryptoSocial {
        sid: overview.sid,
        website_url: overview.website.clone(),
        whitepaper_url: overview.whitepaper.clone(),
        github_url: overview.github.clone(),
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
        c_time: now,
        m_time: now,
      };

      diesel::insert_into(crypto_social::table)
        .values(&new_social)
        .on_conflict(crypto_social::sid)
        .do_nothing()
        .execute(conn)?;

      // Create technical data with GitHub info
      if let Some(gh) = github_data {
        let new_technical = NewCryptoTechnical {
          sid: overview.sid,
          blockchain_platform: None,
          token_standard: None,
          consensus_mechanism: None,
          hashing_algorithm: None,
          block_time_minutes: None,
          block_reward: None,
          block_height: None,
          hash_rate: None,
          difficulty: None,
          github_forks: gh.forks,
          github_stars: gh.stars,
          github_subscribers: gh.watchers,
          github_total_issues: gh.open_issues,
          github_closed_issues: None,
          github_pull_requests: gh.pull_requests,
          github_contributors: gh.contributors,
          github_commits_4_weeks: gh.commits_30d,
          is_defi: false,
          is_stablecoin: false,
          is_nft_platform: false,
          is_exchange_token: false,
          is_gaming: false,
          is_metaverse: false,
          is_privacy_coin: false,
          is_layer2: false,
          is_wrapped: false,
          genesis_date: None,
          ico_price: None,
          ico_date: None,
          c_time: now,
          m_time: now,
        };

        diesel::insert_into(crypto_technical::table)
          .values(&new_technical)
          .on_conflict(crypto_technical::sid)
          .do_nothing()
          .execute(conn)?;
      }

      // Mark symbol as having overview
      diesel::update(symbols::table.filter(symbols::sid.eq(overview.sid)))
        .set(symbols::overview.eq(true))
        .execute(conn)?;

      saved_count += 1;
    }

    Ok(saved_count)
  })
}
