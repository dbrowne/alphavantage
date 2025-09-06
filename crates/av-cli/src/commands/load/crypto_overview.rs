use anyhow::{Result, anyhow};
use av_database_postgres::models::crypto::{
  NewCryptoOverviewBasic, NewCryptoOverviewMetrics, NewCryptoSocial, NewCryptoTechnical,
};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use clap::Args;
use diesel::PgConnection;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

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

/// Main execute function
pub async fn execute(args: CryptoOverviewArgs, config: Config) -> Result<()> {
  info!("Starting cryptocurrency overview loader");

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
    move || get_crypto_symbols_to_load(&database_url, symbols, limit)
  })
  .await??;

  if symbols_to_load.is_empty() {
    info!("No cryptocurrency symbols need overview data");
    return Ok(());
  }

  info!("Found {} cryptocurrency symbols to load overviews for", symbols_to_load.len());
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

  for (sid, symbol, name) in symbols_to_load {
    progress.set_message(format!("Loading {}", symbol));

    match fetch_crypto_overview(&client, sid, &symbol, &name,args.cmc_api_key.as_deref()).await {
      Ok(overview) => {
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
      Err(e) => {
        error!("Failed to fetch overview for {}: {}", symbol, e);
        progress.inc(1);
        continue; // skip the failure for now
      }
    }

    progress.inc(1);

    // Rate limiting for crypto APIs
    sleep(Duration::from_millis(args.delay_ms)).await;
  }

  progress.finish_with_message("Loading complete");

  if !all_overviews_with_github.is_empty() {
    // Save to database
    let saved_count = tokio::task::spawn_blocking({
      let database_url = config.database_url.clone();
      move || save_crypto_overviews_with_github_to_db(&database_url, all_overviews_with_github)
    })
    .await??;

    info!("Saved {} cryptocurrency overviews to database", saved_count);
  }

  Ok(())
}

/// Update only GitHub data for existing cryptocurrencies
pub async fn update_github_data(args: UpdateGitHubArgs, config: Config) -> Result<()> {
  info!("Updating GitHub data for cryptocurrencies");

  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("Mozilla/5.0 (compatible; CryptoGitHubBot/1.0)")
    .build()?;

  // Check rate limit if requested
  if args.check_rate_limit {
    check_github_rate_limit(&client, args.github_token.as_ref()).await?;
  }

  // Query for cryptos with GitHub URLs
  let cryptos_with_github = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let symbols = args.symbols.clone();
    let limit = args.limit;
    move || {
      use av_database_postgres::schema::{crypto_social, symbols};

      let mut conn = PgConnection::establish(&database_url)?;

      let mut query = symbols::table
        .inner_join(crypto_social::table)
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .filter(crypto_social::github_url.is_not_null())
        .select((symbols::sid, symbols::symbol, crypto_social::github_url))
        .into_boxed();

      if let Some(ref symbol_list) = symbols {
        query = query.filter(symbols::symbol.eq_any(symbol_list));
      }

      if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
      }

      Ok::<Vec<(i64, String, Option<String>)>, anyhow::Error>(
        query.load::<(i64, String, Option<String>)>(&mut conn)?,
      )
    }
  })
  .await??;

  if cryptos_with_github.is_empty() {
    info!("No cryptocurrencies found with GitHub URLs");
    return Ok(());
  }

  info!("Found {} cryptocurrencies with GitHub URLs", cryptos_with_github.len());

  let progress = ProgressBar::new(cryptos_with_github.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .unwrap()
      .progress_chars("##-"),
  );

  let mut updated_count = 0;

  for (sid, symbol, github_url) in cryptos_with_github {
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
        }
      }
    }
  // 1. Try CoinGecko     todo!  Get rid of this !!
  match fetch_from_coingecko_free(client, sid, symbol, name).await {
    Ok(data) => return Ok(data),
    Err(e) => debug!("CoinGecko failed for {}: {}", symbol, e),
  }

  // 2. Try CoinPaprika
  match fetch_from_coinpaprika(client, sid, symbol, name).await {  //todo: get rid of this
    Ok(data) => return Ok(data),
    Err(e) => debug!("CoinPaprika failed for {}: {}", symbol, e),
  }

  // 3. Try CoinCap
  match fetch_from_coincap(client, sid, symbol, name).await {
    Ok(data) => return Ok(data),  //todo:: Get rid of this
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
    "https://api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&market_data=true&community_data=false&developer_data=false",
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
    rank: data["market_cap_rank"].as_u64().unwrap_or(0) as u32,
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

  let quotes = &ticker_data["quotes"]["USD"];

  Ok(CryptoOverviewData {
    sid,
    symbol: symbol.to_string(),
    name: name.to_string(),
    description: coin_data["description"].as_str().unwrap_or("").to_string(),
    market_cap: quotes["market_cap"].as_f64().unwrap_or(0.0) as i64,
    circulating_supply: ticker_data["circulating_supply"].as_f64().unwrap_or(0.0),
    total_supply: ticker_data["total_supply"].as_f64(),
    max_supply: ticker_data["max_supply"].as_f64(),
    price_usd: quotes["price"].as_f64().unwrap_or(0.0),
    volume_24h: quotes["volume_24h"].as_f64().unwrap_or(0.0) as i64,
    price_change_24h: quotes["percent_change_24h"].as_f64().unwrap_or(0.0),
    ath: quotes["ath_price"].as_f64().unwrap_or(0.0),
    ath_date: quotes["ath_date"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
    atl: 0.0, // CoinPaprika doesn't provide ATL in free tier
    atl_date: None,
    rank: ticker_data["rank"].as_u64().unwrap_or(0) as u32,
    website: coin_data["links"]["website"]
      .as_array()
      .and_then(|arr| arr.first())
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
    whitepaper: coin_data["whitepaper"]["link"]
      .as_str()
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
    github: coin_data["links"]["source_code"]
      .as_array()
      .and_then(|arr| arr.first())
      .and_then(|v| v.as_str())
      .map(|s| s.to_string())
      .filter(|s| !s.is_empty()),
  })
}

/// Fetch from CoinCap (no API key required)
async fn fetch_from_coincap(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  name: &str,
) -> Result<CryptoOverviewData> {
  // Get asset ID
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

    if response.status() != 200 {
        return Err(anyhow!("CoinMarketCap returned status: {}", response.status()));
    }

    let cmc_response: serde_json::Value = response.json().await?;

    // Check for API errors
    if let Some(status) = cmc_response.get("status") {
        if let Some(error_code) = status.get("error_code").and_then(|v| v.as_i64()) {
            if error_code != 0 {
                let error_msg = status.get("error_message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown CMC error");
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
        .ok_or_else(|| anyhow!("No data found for symbol {} in CMC response", symbol))?;

    let usd_quote = crypto_data
        .get("quote")
        .and_then(|q| q.get("USD"))
        .ok_or_else(|| anyhow!("No USD quote data found for {}", symbol))?;

    // Extract values with proper error handling
    let market_cap = usd_quote.get("market_cap")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i64;

    let current_price = usd_quote.get("price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let volume_24h = usd_quote.get("volume_24h")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i64;

    let price_change_24h = usd_quote.get("percent_change_24h")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let circulating_supply = crypto_data.get("circulating_supply")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let total_supply = crypto_data.get("total_supply")
        .and_then(|v| v.as_f64());

    let max_supply = crypto_data.get("max_supply")
        .and_then(|v| v.as_f64());

    let rank = crypto_data.get("cmc_rank")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    Ok(CryptoOverviewData {
        sid,
        symbol: symbol.to_string(),
        name: name.to_string(),
        description: "".to_string(), // CMC quotes endpoint doesn't provide descriptions
        market_cap,
        circulating_supply,
        total_supply,
        max_supply,
        price_usd: current_price,
        volume_24h,
        price_change_24h,
        ath: 0.0, // Requires separate endpoint
        ath_date: None,
        atl: 0.0, // Requires separate endpoint
        atl_date: None,
        rank,
        website: None, // Requires metadata endpoint
        whitepaper: None,
        github: None,
    })
}

/// Fetch GitHub data with authentication support
async fn fetch_github_data(
  client: &reqwest::Client,
  github_url: Option<&String>,
  github_token: Option<&String>,
) -> Option<GitHubData> {
  let github_url = github_url?;

  // Extract owner and repo from GitHub URL
  let re = Regex::new(r"github\.com/([^/]+)/([^/\s]+)").ok()?;
  let captures = re.captures(github_url)?;
  let owner = captures.get(1)?.as_str();
  let repo = captures.get(2)?.as_str().trim_end_matches(".git");

  debug!("Fetching GitHub data for {}/{}", owner, repo);

  // Build request with optional authentication
  let mut request = client
    .get(&format!("https://api.github.com/repos/{}/{}", owner, repo))
    .header("Accept", "application/vnd.github.v3+json");

  // Add authentication if token provided
  if let Some(token) = github_token {
    request = request.header("Authorization", format!("Bearer {}", token));
    debug!("Using GitHub authentication");
  } else {
    debug!("No GitHub authentication - limited to 60 requests/hour");
  }

  let repo_response = request.send().await.ok()?;

  // Check rate limit headers
  if let Some(remaining) = repo_response.headers().get("x-ratelimit-remaining") {
    if let Ok(remaining_str) = remaining.to_str() {
      if let Ok(remaining_count) = remaining_str.parse::<i32>() {
        if remaining_count < 10 {
          warn!("GitHub API rate limit low: {} requests remaining", remaining_count);
        }
      }
    }
  }

  if repo_response.status() == 401 {
    error!("GitHub authentication failed - check your token");
    return None;
  } else if repo_response.status() == 403 {
    error!("GitHub rate limit exceeded or forbidden");
    return None;
  } else if repo_response.status() != 200 {
    warn!("GitHub API returned status {} for {}/{}", repo_response.status(), owner, repo);
    return None;
  }

  let repo_data: Value = repo_response.json().await.ok()?;

  // Get contributor count (separate API call)
  let contributors_url =
    format!("https://api.github.com/repos/{}/{}/contributors?per_page=1", owner, repo);
  let mut contributors_request =
    client.get(&contributors_url).header("Accept", "application/vnd.github.v3+json");

  if let Some(token) = github_token {
    contributors_request =
      contributors_request.header("Authorization", format!("Bearer {}", token));
  }

  let contributors_response = contributors_request.send().await.ok()?;

  let contributor_count = if let Some(link_header) = contributors_response.headers().get("link") {
    // Parse the Link header to get the last page number
    let link_str = link_header.to_str().ok()?;
    let re = Regex::new(r#"page=(\d+)>; rel="last""#).ok()?;
    re.captures(link_str).and_then(|cap| cap.get(1)).and_then(|m| m.as_str().parse::<i32>().ok())
  } else {
    // If no pagination, count the results
    let contributors: Vec<Value> = contributors_response.json().await.ok()?;
    Some(contributors.len() as i32)
  };

  // Get recent commit activity (last 30 days)
  let since_date = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
  let commits_url = format!(
    "https://api.github.com/repos/{}/{}/commits?since={}&per_page=1",
    owner, repo, since_date
  );

  let mut commits_request =
    client.get(&commits_url).header("Accept", "application/vnd.github.v3+json");

  if let Some(token) = github_token {
    commits_request = commits_request.header("Authorization", format!("Bearer {}", token));
  }

  let commits_response = commits_request.send().await.ok()?;

  let commits_30d = if let Some(link_header) = commits_response.headers().get("link") {
    let link_str = link_header.to_str().ok()?;
    let re = Regex::new(r#"page=(\d+)>; rel="last""#).ok()?;
    re.captures(link_str).and_then(|cap| cap.get(1)).and_then(|m| m.as_str().parse::<i32>().ok())
  } else {
    let commits: Vec<Value> = commits_response.json().await.ok().unwrap_or_default();
    Some(commits.len() as i32)
  };

  Some(GitHubData {
    forks: repo_data["forks_count"].as_i64().map(|v| v as i32),
    stars: repo_data["stargazers_count"].as_i64().map(|v| v as i32),
    watchers: repo_data["subscribers_count"].as_i64().map(|v| v as i32),
    open_issues: repo_data["open_issues_count"].as_i64().map(|v| v as i32),
    contributors: contributor_count,
    commits_30d,
    pull_requests: None, // Skip PR count to save API calls
    last_commit_date: repo_data["pushed_at"]
      .as_str()
      .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok()),
  })
}

/// Check GitHub API rate limit status
async fn check_github_rate_limit(
  client: &reqwest::Client,
  github_token: Option<&String>,
) -> Result<()> {
  let mut request = client
    .get("https://api.github.com/rate_limit")
    .header("Accept", "application/vnd.github.v3+json");

  if let Some(token) = github_token {
    request = request.header("Authorization", format!("Bearer {}", token));
  }

  let response = request.send().await?;
  let data: Value = response.json().await?;

  let core = &data["rate"];
  let _limit = core["limit"].as_i64().unwrap_or(0);
  let remaining = core["remaining"].as_i64().unwrap_or(0);
  let reset = core["reset"].as_i64().unwrap_or(0);

  let reset_time = DateTime::<Utc>::from_timestamp(reset, 0).map(|dt| dt.naive_utc());

  if let Some(time) = reset_time {
    info!("  Resets at: {}", time);
  } else {
    info!("  Resets at: unknown");
  }

  if remaining == 0 {
    if let Some(time) = reset_time {
      return Err(anyhow!("GitHub rate limit exceeded. Resets at {}", time));
    } else {
      return Err(anyhow!("GitHub rate limit exceeded. Reset time unknown"));
    }
  }

  Ok(())
}

// Symbol to ID mapping functions
fn get_coingecko_id(symbol: &str) -> String {
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
  let now_t = Utc::now().naive_utc();

  // Use transaction for all inserts
  conn.transaction::<_, anyhow::Error, _>(|conn| {
    for (overview, github_data) in overviews {
      // Convert numeric values to BigDecimal
      let current_price_bd = BigDecimal::from_str(&overview.price_usd.to_string()).ok();
      let price_change_pct = BigDecimal::from_str(&overview.price_change_24h.to_string()).ok();
      let circulating_supply_bd =
        BigDecimal::from_str(&overview.circulating_supply.to_string()).ok();
      let total_supply_bd =
        overview.total_supply.and_then(|ts| BigDecimal::from_str(&ts.to_string()).ok());
      let max_supply_bd =
        overview.max_supply.and_then(|ms| BigDecimal::from_str(&ms.to_string()).ok());
      let ath_bd = BigDecimal::from_str(&overview.ath.to_string()).ok();
      let atl_bd = BigDecimal::from_str(&overview.atl.to_string()).ok();

      // Calculate fully diluted valuation
      let fully_diluted_valuation = match (&current_price_bd, &max_supply_bd) {
        (Some(_), Some(_)) => {
          let price_f64 = overview.price_usd;
          let max_supply_f64 = overview.max_supply.unwrap_or(0.0);
          Some((price_f64 * max_supply_f64) as i64)
        }
        _ => None,
      };

      // Create the values that need to be borrowed
      let slug = overview.symbol.to_lowercase().replace(" ", "-");
      let market_cap_rank = overview.rank as i32;
      let now = chrono::Utc::now();

      // Create basic overview
      let new_overview_basic = NewCryptoOverviewBasic {
        sid: &overview.sid,
        symbol: &overview.symbol,
        name: &overview.name,
        slug: Some(&slug),
        description: Some(overview.description.as_str()),
        market_cap_rank: Some(&market_cap_rank),
        market_cap: Some(&overview.market_cap),
        fully_diluted_valuation: fully_diluted_valuation.as_ref(),
        volume_24h: Some(&overview.volume_24h),
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

      // Convert dates for metrics
      let ath_datetime = overview.ath_date.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
      let atl_datetime = overview.atl_date.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());

      // Create metrics overview
      let new_overview_metrics = NewCryptoOverviewMetrics {
        sid: &overview.sid,
        price_change_24h: current_price_bd.as_ref(),
        price_change_pct_24h: price_change_pct.as_ref(),
        price_change_pct_7d: None,
        price_change_pct_14d: None,
        price_change_pct_30d: None,
        price_change_pct_60d: None,
        price_change_pct_200d: None,
        price_change_pct_1y: None,
        ath: ath_bd.as_ref(),
        ath_date: ath_datetime.as_ref(),
        ath_change_percentage: None,
        atl: atl_bd.as_ref(),
        atl_date: atl_datetime.as_ref(),
        atl_change_percentage: None,
        roi_times: None,
        roi_currency: None,
        roi_percentage: None,
      };

      // Insert metrics overview
      diesel::insert_into(crypto_overview_metrics::table)
        .values(&new_overview_metrics)
        .on_conflict(crypto_overview_metrics::sid)
        .do_nothing()
        .execute(conn)?;

      let mut new_technical = NewCryptoTechnical {
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
        github_forks: None,
        github_stars: None,
        github_subscribers: None,
        github_total_issues: None,
        github_closed_issues: None,
        github_pull_requests: None,
        github_contributors: None,
        github_commits_4_weeks: None,
        is_defi: false,
        is_stablecoin: overview.symbol.contains("USD")
          || overview.symbol.contains("USDT")
          || overview.symbol.contains("USDC"),
        is_nft_platform: false,
        is_exchange_token: overview.symbol == "BNB"
          || overview.symbol == "CRO"
          || overview.symbol == "OKB",
        is_gaming: false,
        is_metaverse: false,
        is_privacy_coin: overview.symbol == "XMR" || overview.symbol == "ZEC",
        is_layer2: false,
        is_wrapped: overview.symbol.starts_with("W"),
        genesis_date: None,
        ico_price: None,
        ico_date: None,
        c_time: chrono::Utc::now(),
        m_time: chrono::Utc::now(),
      };

      // Add GitHub data if available
      if let Some(gh) = &github_data {
        new_technical.github_forks = gh.forks;
        new_technical.github_stars = gh.stars;
        new_technical.github_subscribers = gh.watchers;
        new_technical.github_total_issues = gh.open_issues;
        new_technical.github_pull_requests = gh.pull_requests;
        new_technical.github_contributors = gh.contributors;
        new_technical.github_commits_4_weeks = gh.commits_30d;
      }
      // Check if technical record already exists
      let technical_exists: bool = diesel::select(diesel::dsl::exists(
        crypto_technical::table.filter(crypto_technical::sid.eq(overview.sid)),
      ))
      .get_result(conn)?;

      if technical_exists && github_data.is_some() {
        // Update only if we have new GitHub data
        diesel::update(crypto_technical::table.filter(crypto_technical::sid.eq(overview.sid)))
          .set((
            crypto_technical::github_forks.eq(new_technical.github_forks),
            crypto_technical::github_stars.eq(new_technical.github_stars),
            crypto_technical::github_subscribers.eq(new_technical.github_subscribers),
            crypto_technical::github_total_issues.eq(new_technical.github_total_issues),
            crypto_technical::github_pull_requests.eq(new_technical.github_pull_requests),
            crypto_technical::github_contributors.eq(new_technical.github_contributors),
            crypto_technical::github_commits_4_weeks.eq(new_technical.github_commits_4_weeks),
            crypto_technical::m_time.eq(now_t),
          ))
          .execute(conn)?;
      } else if !technical_exists {
        diesel::insert_into(crypto_technical::table).values(&new_technical).execute(conn)?;
      }

      // Insert into crypto_social
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
        c_time: chrono::Utc::now(),
        m_time: chrono::Utc::now(),
      };

      diesel::insert_into(crypto_social::table)
        .values(&new_social)
        .on_conflict(crypto_social::sid)
        .do_nothing() // Changed from .do_update().set()
        .execute(conn)?;

      // Update symbols table to mark overview as loaded
      diesel::update(symbols::table.filter(symbols::sid.eq(overview.sid)))
        .set(symbols::overview.eq(true))
        .execute(conn)?;

      saved_count += 1;
      debug!("Saved crypto overview for {} (SID {})", overview.symbol, overview.sid);
    }

    Ok(())
  })?;

  Ok(saved_count)
}

