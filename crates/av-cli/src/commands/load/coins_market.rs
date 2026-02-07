/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 * CoinGecko /coins/markets loader
 *
 * This loader fetches market data from CoinGecko's /coins/markets endpoint,
 * creates new symbols for coins not in the database, and updates market data.
 */

use anyhow::Result;
use av_loaders::error::{LoaderError, LoaderResult};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use clap::Args;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use av_core::types::market::{SecurityIdentifier, SecurityType};
use av_database_postgres::establish_connection;
use av_database_postgres::models::crypto::{
  NewCryptoApiMap, NewCryptoOverviewBasic, NewCryptoOverviewMetrics,
};
use av_database_postgres::models::security::NewSymbolOwned;
use av_database_postgres::repository::{CacheRepository, DatabaseContext};
use av_database_postgres::schema::{
  crypto_api_map, crypto_overview_basic, crypto_overview_metrics, symbols,
};

use crate::config::Config;

const NO_PRIORITY: i32 = 9_999_999;
const PROVIDER: &str = "CoinGecko";

#[derive(Args, Debug)]
pub struct CoinsMarketArgs {
  /// Number of results per page (1-250)
  #[arg(long, default_value = "250")]
  pub per_page: u32,

  /// Number of pages to fetch (0 = all available)
  #[arg(long, default_value = "0")]
  pub pages: u32,

  /// Starting page number
  #[arg(long, default_value = "1")]
  pub start_page: u32,

  /// CoinGecko API key for higher rate limits
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub api_key: Option<String>,

  /// Delay between API calls in milliseconds
  #[arg(long, default_value = "1500")]
  pub delay_ms: u64,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  pub dry_run: bool,

  /// Only update existing symbols, don't create new ones
  #[arg(long)]
  pub update_only: bool,

  /// Include price change percentages (1h, 24h, 7d, 14d, 30d, 200d, 1y)
  #[arg(long, default_value = "24h,7d")]
  pub price_change_percentage: String,

  /// Sort order (market_cap_desc, market_cap_asc, volume_desc, volume_asc)
  #[arg(long, default_value = "market_cap_desc")]
  pub order: String,

  /// Show verbose output
  #[arg(short, long)]
  pub verbose: bool,

  /// Enable response caching
  #[arg(long, default_value = "true")]
  pub enable_cache: bool,

  /// Cache TTL in hours
  #[arg(long, default_value = "1")]
  pub cache_ttl_hours: i64,

  /// Force refresh - bypass cache
  #[arg(long)]
  pub force_refresh: bool,
}

/// CoinGecko /coins/markets response structure
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct CoinMarketData {
  pub id: String,
  pub symbol: String,
  pub name: String,
  pub image: Option<String>,
  pub current_price: Option<f64>,
  pub market_cap: Option<f64>,
  pub market_cap_rank: Option<i32>,
  pub fully_diluted_valuation: Option<f64>,
  pub total_volume: Option<f64>,
  pub high_24h: Option<f64>,
  pub low_24h: Option<f64>,
  pub price_change_24h: Option<f64>,
  pub price_change_percentage_24h: Option<f64>,
  pub market_cap_change_24h: Option<f64>,
  pub market_cap_change_percentage_24h: Option<f64>,
  pub circulating_supply: Option<f64>,
  pub total_supply: Option<f64>,
  pub max_supply: Option<f64>,
  pub ath: Option<f64>,
  pub ath_change_percentage: Option<f64>,
  pub ath_date: Option<String>,
  pub atl: Option<f64>,
  pub atl_change_percentage: Option<f64>,
  pub atl_date: Option<String>,
  pub last_updated: Option<String>,
  // Additional price change fields when requested
  pub price_change_percentage_1h_in_currency: Option<f64>,
  pub price_change_percentage_7d_in_currency: Option<f64>,
  pub price_change_percentage_14d_in_currency: Option<f64>,
  pub price_change_percentage_30d_in_currency: Option<f64>,
  pub price_change_percentage_200d_in_currency: Option<f64>,
  pub price_change_percentage_1y_in_currency: Option<f64>,
}

/// SID generator for cryptocurrency symbols
struct CryptoSidGenerator {
  next_raw_id: u32,
}

impl CryptoSidGenerator {
  fn new(conn: &mut PgConnection) -> LoaderResult<Self> {
    let crypto_sids: Vec<i64> = symbols::table
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .select(symbols::sid)
      .load(conn)
      .map_err(|e| LoaderError::DatabaseError(format!("Failed to load crypto SIDs: {}", e)))?;

    let mut max_raw_id: u32 = 0;
    for sid_val in crypto_sids {
      if let Some(identifier) = SecurityIdentifier::decode(sid_val) {
        if identifier.security_type == SecurityType::Cryptocurrency
          && identifier.raw_id > max_raw_id
        {
          max_raw_id = identifier.raw_id;
        }
      }
    }

    Ok(Self { next_raw_id: max_raw_id + 1 })
  }

  fn next_sid(&mut self) -> i64 {
    let sid = SecurityType::encode(SecurityType::Cryptocurrency, self.next_raw_id);
    self.next_raw_id += 1;
    sid
  }
}

/// Public execute function - converts LoaderError to anyhow::Error at the boundary
pub async fn execute(args: CoinsMarketArgs, config: Config) -> Result<()> {
  execute_internal(args, config).await.map_err(|e| anyhow::anyhow!("{}", e))
}

/// Internal execute function using typed errors
async fn execute_internal(args: CoinsMarketArgs, config: Config) -> LoaderResult<()> {
  info!("Starting CoinGecko /coins/markets loader");

  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| LoaderError::ConfigurationError(format!("Failed to build HTTP client: {}", e)))?;

  let base_url = if args.api_key.is_some() {
    "https://pro-api.coingecko.com/api/v3"
  } else {
    "https://api.coingecko.com/api/v3"
  };

  // Set up cache repository (enabled even during dry-run to cache API responses)
  let cache_repo: Option<Arc<dyn CacheRepository>> = if args.enable_cache {
    let db_context = DatabaseContext::new(&config.database_url).map_err(|e| {
      LoaderError::DatabaseError(format!("Failed to create database context: {}", e))
    })?;
    Some(Arc::new(db_context.cache_repository()))
  } else {
    None
  };

  let mut page = args.start_page;
  let mut total_processed = 0;
  let mut total_created = 0;
  let mut total_updated = 0;
  let mut cache_hits = 0;

  // Build existing CoinGecko ID -> SID mapping from database
  let existing_mappings = if !args.dry_run {
    load_existing_coingecko_mappings(&config.database_url)?
  } else {
    HashMap::new()
  };
  info!("Loaded {} existing CoinGecko mappings from database", existing_mappings.len());

  loop {
    let cache_key =
      format!("coins_market_page_{}_per_{}_order_{}", page, args.per_page, args.order);
    let endpoint_url = format!(
      "{}/coins/markets?vs_currency=usd&order={}&per_page={}&page={}&sparkline=false&price_change_percentage={}",
      base_url, args.order, args.per_page, page, args.price_change_percentage
    );

    // Try to get from cache first
    let coins: Vec<CoinMarketData> = if let Some(ref cache) = cache_repo {
      if !args.force_refresh {
        if let Ok(Some(cached_data)) = cache.get_json(&cache_key, "coingecko").await {
          match serde_json::from_value::<Vec<CoinMarketData>>(cached_data) {
            Ok(coins) => {
              info!("Cache hit for page {} ({} coins)", page, coins.len());
              cache_hits += 1;
              coins
            }
            Err(e) => {
              debug!("Cache data invalid: {}, fetching from API", e);
              fetch_page(&client, &endpoint_url, &args.api_key).await?
            }
          }
        } else {
          fetch_and_cache_page(
            &client,
            &endpoint_url,
            &args.api_key,
            cache,
            &cache_key,
            args.cache_ttl_hours,
          )
          .await?
        }
      } else {
        fetch_and_cache_page(
          &client,
          &endpoint_url,
          &args.api_key,
          cache,
          &cache_key,
          args.cache_ttl_hours,
        )
        .await?
      }
    } else {
      info!("Fetching page {} from CoinGecko...", page);
      fetch_page(&client, &endpoint_url, &args.api_key).await?
    };

    if coins.is_empty() {
      info!("No more coins to fetch (empty page)");
      break;
    }

    info!("Processing {} coins from page {}", coins.len(), page);

    if !args.dry_run {
      let (created, updated) = process_coins(
        &config.database_url,
        &coins,
        &existing_mappings,
        args.update_only,
        args.verbose,
      )?;
      total_created += created;
      total_updated += updated;
    } else {
      info!("Dry run: would process {} coins", coins.len());
    }

    total_processed += coins.len();

    // Check if we've fetched enough pages
    if args.pages > 0 && page >= args.start_page + args.pages - 1 {
      info!("Reached requested page limit ({})", args.pages);
      break;
    }

    // Check if we got fewer results than requested (last page)
    if coins.len() < args.per_page as usize {
      info!("Reached last page (got {} < {} requested)", coins.len(), args.per_page);
      break;
    }

    page += 1;

    // Rate limiting delay
    if args.delay_ms > 0 {
      tokio::time::sleep(Duration::from_millis(args.delay_ms)).await;
    }
  }

  println!("\n╔════════════════════════════════════════════╗");
  println!("║     COINS MARKET LOADER SUMMARY            ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Total Processed:    {:<22} ║", total_processed);
  println!("║ Symbols Created:    {:<22} ║", total_created);
  println!("║ Symbols Updated:    {:<22} ║", total_updated);
  println!("║ Cache Hits:         {:<22} ║", cache_hits);
  println!("╚════════════════════════════════════════════╝");

  Ok(())
}

/// Fetch a page from CoinGecko API
async fn fetch_page(
  client: &reqwest::Client,
  url: &str,
  api_key: &Option<String>,
) -> LoaderResult<Vec<CoinMarketData>> {
  let mut request = client.get(url);
  if let Some(ref key) = api_key {
    request = request.header("x-cg-pro-api-key", key);
  }

  let response = request
    .send()
    .await
    .map_err(|e| LoaderError::ApiError(format!("{} request failed: {}", PROVIDER, e)))?;

  if response.status() == 429 {
    return Err(LoaderError::RateLimitExceeded { retry_after: 60 });
  }

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(LoaderError::ApiError(format!("{} API error {}: {}", PROVIDER, status, body)));
  }

  let coins: Vec<CoinMarketData> = response.json().await.map_err(|e| {
    LoaderError::SerializationError(format!("Failed to parse {} response: {}", PROVIDER, e))
  })?;

  Ok(coins)
}

/// Fetch a page and cache the result
async fn fetch_and_cache_page(
  client: &reqwest::Client,
  url: &str,
  api_key: &Option<String>,
  cache: &Arc<dyn CacheRepository>,
  cache_key: &str,
  cache_ttl_hours: i64,
) -> LoaderResult<Vec<CoinMarketData>> {
  info!("Fetching from CoinGecko API...");
  let coins = fetch_page(client, url, api_key).await?;

  // Cache the response
  if !coins.is_empty() {
    let json_value = serde_json::to_value(&coins).map_err(|e| {
      LoaderError::SerializationError(format!("Failed to serialize for cache: {}", e))
    })?;
    if let Err(e) = cache.set_json(cache_key, "coingecko", url, json_value, cache_ttl_hours).await {
      warn!("Failed to cache response: {}", e);
    } else {
      info!("Cached {} coins (TTL: {}h)", coins.len(), cache_ttl_hours);
    }
  }

  Ok(coins)
}

/// Load existing CoinGecko ID -> SID mappings from database
fn load_existing_coingecko_mappings(database_url: &str) -> LoaderResult<HashMap<String, i64>> {
  let mut conn = establish_connection(database_url)
    .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect to database: {}", e)))?;

  let mappings: Vec<(String, i64)> = crypto_api_map::table
    .filter(crypto_api_map::api_source.eq(PROVIDER))
    .select((crypto_api_map::api_id, crypto_api_map::sid))
    .load(&mut conn)
    .map_err(|e| {
      LoaderError::DatabaseError(format!("Failed to load {} mappings: {}", PROVIDER, e))
    })?;

  Ok(mappings.into_iter().collect())
}

/// Process coins from the API response
fn process_coins(
  database_url: &str,
  coins: &[CoinMarketData],
  existing_mappings: &HashMap<String, i64>,
  update_only: bool,
  verbose: bool,
) -> LoaderResult<(usize, usize)> {
  let mut conn = establish_connection(database_url)
    .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect to database: {}", e)))?;
  let mut sid_generator = CryptoSidGenerator::new(&mut conn)?;

  // Load existing crypto symbols by (symbol, name) to avoid duplicates
  let existing_symbols = load_existing_crypto_symbols(&mut conn)?;

  let mut created = 0;
  let mut updated = 0;
  let mut linked = 0;
  let mut skipped = 0;

  let progress = if verbose {
    let pb = ProgressBar::new(coins.len() as u64);
    pb.set_style(
      ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("##-"),
    );
    Some(pb)
  } else {
    None
  };

  for coin in coins {
    if let Some(ref pb) = progress {
      pb.set_message(format!("{}", coin.symbol.to_uppercase()));
    }

    let result = if let Some(&existing_sid) = existing_mappings.get(&coin.id) {
      // CoinGecko ID already mapped, update market data
      match update_market_data(&mut conn, existing_sid, coin) {
        Ok(()) => {
          updated += 1;
          existing_sid
        }
        Err(e) => {
          warn!(
            "Failed to update {} ({}): {} - market_cap={:?}, price={:?}",
            coin.symbol, coin.id, e, coin.market_cap, coin.current_price
          );
          skipped += 1;
          if let Some(ref pb) = progress {
            pb.inc(1);
          }
          continue;
        }
      }
    } else if !update_only {
      // No CoinGecko mapping exists - check if symbol with same ticker+name exists
      let symbol_upper = coin.symbol.to_uppercase();
      let lookup_key = (symbol_upper.clone(), coin.name.clone());

      if let Some(&existing_sid) = existing_symbols.get(&lookup_key) {
        // Symbol exists with same ticker+name, just add CoinGecko mapping and update data
        match link_existing_symbol(&mut conn, existing_sid, coin) {
          Ok(()) => {
            linked += 1;
            info!(
              "Linked {} '{}' to existing SID {} (CoinGecko: {})",
              symbol_upper, coin.name, existing_sid, coin.id
            );
            existing_sid
          }
          Err(e) => {
            warn!("Failed to link {} ({}): {} - SID={}", coin.symbol, coin.id, e, existing_sid);
            skipped += 1;
            if let Some(ref pb) = progress {
              pb.inc(1);
            }
            continue;
          }
        }
      } else {
        // No existing symbol, create new one
        match create_new_symbol(&mut conn, &mut sid_generator, coin) {
          Ok(new_sid) => {
            created += 1;
            new_sid
          }
          Err(e) => {
            warn!(
              "Failed to create {} ({}): {} - market_cap={:?}, price={:?}",
              coin.symbol, coin.id, e, coin.market_cap, coin.current_price
            );
            skipped += 1;
            if let Some(ref pb) = progress {
              pb.inc(1);
            }
            continue;
          }
        }
      }
    } else {
      if let Some(ref pb) = progress {
        pb.inc(1);
      }
      continue;
    };

    debug!("Processed {} ({}) -> SID {}", coin.symbol, coin.id, result);

    if let Some(ref pb) = progress {
      pb.inc(1);
    }
  }

  if let Some(pb) = progress {
    pb.finish_with_message("Done");
  }

  if skipped > 0 {
    warn!("Skipped {} coins due to errors", skipped);
  }
  if linked > 0 {
    info!("Linked {} coins to existing symbols", linked);
  }

  Ok((created, updated))
}

/// Load existing crypto symbols by (symbol, name) -> SID
fn load_existing_crypto_symbols(
  conn: &mut PgConnection,
) -> LoaderResult<HashMap<(String, String), i64>> {
  let results: Vec<(String, String, i64)> = symbols::table
    .filter(symbols::sec_type.eq("Cryptocurrency"))
    .select((symbols::symbol, symbols::name, symbols::sid))
    .load(conn)
    .map_err(|e| LoaderError::DatabaseError(format!("Failed to load crypto symbols: {}", e)))?;

  Ok(results.into_iter().map(|(sym, name, sid)| ((sym, name), sid)).collect())
}

/// Link an existing symbol to a CoinGecko ID and update its data
fn link_existing_symbol(
  conn: &mut PgConnection,
  sid: i64,
  coin: &CoinMarketData,
) -> LoaderResult<()> {
  let symbol_upper = coin.symbol.to_uppercase();

  // Check if this CoinGecko ID is already mapped (to a different SID)
  let existing_mapping: Option<i64> = crypto_api_map::table
    .filter(crypto_api_map::api_source.eq(PROVIDER))
    .filter(crypto_api_map::api_id.eq(&coin.id))
    .select(crypto_api_map::sid)
    .first(conn)
    .optional()
    .map_err(|e| LoaderError::DatabaseError(format!("Failed to check existing mapping: {}", e)))?;

  if existing_mapping.is_some() {
    // Already mapped, skip
    return Ok(());
  }

  // Add CoinGecko mapping for this existing SID
  let api_mapping = NewCryptoApiMap {
    sid,
    api_source: PROVIDER.to_string(),
    api_id: coin.id.clone(),
    api_slug: Some(coin.id.clone()),
    api_symbol: Some(symbol_upper),
    rank: coin.market_cap_rank,
    is_active: Some(true),
    last_verified: Some(Utc::now()),
    c_time: Utc::now(),
    m_time: Utc::now(),
  };

  diesel::insert_into(crypto_api_map::table)
    .values(&api_mapping)
    .on_conflict((crypto_api_map::sid, crypto_api_map::api_source))
    .do_nothing()
    .execute(conn)
    .map_err(|e| LoaderError::DatabaseError(format!("Failed to insert API mapping: {}", e)))?;

  // Check if overview data exists
  let has_overview: bool = crypto_overview_basic::table
    .filter(crypto_overview_basic::sid.eq(sid))
    .select(diesel::dsl::count_star())
    .first::<i64>(conn)
    .map(|c| c > 0)
    .unwrap_or(false);

  if has_overview {
    // Update existing overview data
    update_market_data(conn, sid, coin)?;
  } else {
    // Insert new overview data
    insert_market_data(conn, sid, coin)?;
  }

  Ok(())
}

/// Create a new cryptocurrency symbol
fn create_new_symbol(
  conn: &mut PgConnection,
  sid_generator: &mut CryptoSidGenerator,
  coin: &CoinMarketData,
) -> LoaderResult<i64> {
  let new_sid = sid_generator.next_sid();
  let symbol_upper = coin.symbol.to_uppercase();

  // Determine priority based on market cap rank
  let priority = coin.market_cap_rank.unwrap_or(NO_PRIORITY);

  // Insert into symbols table
  let new_symbol = NewSymbolOwned::from_symbol_data(
    &symbol_upper,
    priority,
    &coin.name,
    "Cryptocurrency",
    "Global",
    "USD",
    new_sid,
  );

  diesel::insert_into(symbols::table).values(&new_symbol).execute(conn).map_err(|e| {
    LoaderError::DatabaseError(format!("Failed to insert symbol {}: {}", symbol_upper, e))
  })?;

  // Insert into crypto_api_map
  let api_mapping = NewCryptoApiMap {
    sid: new_sid,
    api_source: PROVIDER.to_string(),
    api_id: coin.id.clone(),
    api_slug: Some(coin.id.clone()),
    api_symbol: Some(symbol_upper.clone()),
    rank: coin.market_cap_rank,
    is_active: Some(true),
    last_verified: Some(Utc::now()),
    c_time: Utc::now(),
    m_time: Utc::now(),
  };

  diesel::insert_into(crypto_api_map::table).values(&api_mapping).execute(conn).map_err(|e| {
    LoaderError::DatabaseError(format!("Failed to insert API mapping for {}: {}", symbol_upper, e))
  })?;

  // Insert market data
  insert_market_data(conn, new_sid, coin)?;

  info!(
    "Created new symbol: {} '{}' (SID: {}, CoinGecko: {})",
    symbol_upper, coin.name, new_sid, coin.id
  );

  Ok(new_sid)
}

/// Insert market data for a new symbol
fn insert_market_data(
  conn: &mut PgConnection,
  sid: i64,
  coin: &CoinMarketData,
) -> LoaderResult<()> {
  let symbol_upper = coin.symbol.to_uppercase();
  let slug = coin.id.clone();

  // Parse last_updated timestamp
  let last_updated = coin
    .last_updated
    .as_ref()
    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&Utc));

  // Convert f64 to i64 for database columns
  let market_cap = coin.market_cap.map(|v| v as i64);
  let fully_diluted_valuation = coin.fully_diluted_valuation.map(|v| v as i64);
  let volume_24h = coin.total_volume.map(|v| v as i64);

  // Convert to BigDecimal
  let current_price =
    coin.current_price.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let circulating_supply =
    coin.circulating_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let total_supply =
    coin.total_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let max_supply =
    coin.max_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());

  let new_basic = NewCryptoOverviewBasic {
    sid: &sid,
    symbol: &symbol_upper,
    name: &coin.name,
    slug: Some(&slug),
    description: None,
    market_cap_rank: coin.market_cap_rank.as_ref(),
    market_cap: market_cap.as_ref(),
    fully_diluted_valuation: fully_diluted_valuation.as_ref(),
    volume_24h: volume_24h.as_ref(),
    volume_change_24h: None,
    current_price: current_price.as_ref(),
    circulating_supply: circulating_supply.as_ref(),
    total_supply: total_supply.as_ref(),
    max_supply: max_supply.as_ref(),
    last_updated: last_updated.as_ref(),
    image_url: coin.image.as_deref(),
    market_cap_rank_rehyp: None,
  };

  diesel::insert_into(crypto_overview_basic::table)
    .values(&new_basic)
    .on_conflict(crypto_overview_basic::sid)
    .do_nothing()
    .execute(conn)
    .map_err(|e| {
      LoaderError::DatabaseError(format!("Failed to insert basic data for {}: {}", symbol_upper, e))
    })?;

  // Insert metrics
  insert_metrics(conn, sid, coin)?;

  Ok(())
}

/// Update market data for an existing symbol
fn update_market_data(
  conn: &mut PgConnection,
  sid: i64,
  coin: &CoinMarketData,
) -> LoaderResult<()> {
  let last_updated = coin
    .last_updated
    .as_ref()
    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&Utc));

  // Convert f64 to i64 for database columns
  let market_cap = coin.market_cap.map(|v| v as i64);
  let fully_diluted_valuation = coin.fully_diluted_valuation.map(|v| v as i64);
  let volume_24h = coin.total_volume.map(|v| v as i64);

  let current_price =
    coin.current_price.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let circulating_supply =
    coin.circulating_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let total_supply =
    coin.total_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let max_supply =
    coin.max_supply.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());

  // Update crypto_overview_basic
  diesel::update(crypto_overview_basic::table.filter(crypto_overview_basic::sid.eq(sid)))
    .set((
      crypto_overview_basic::market_cap_rank.eq(coin.market_cap_rank),
      crypto_overview_basic::market_cap.eq(market_cap),
      crypto_overview_basic::fully_diluted_valuation.eq(fully_diluted_valuation),
      crypto_overview_basic::volume_24h.eq(volume_24h),
      crypto_overview_basic::current_price.eq(&current_price),
      crypto_overview_basic::circulating_supply.eq(&circulating_supply),
      crypto_overview_basic::total_supply.eq(&total_supply),
      crypto_overview_basic::max_supply.eq(&max_supply),
      crypto_overview_basic::last_updated.eq(&last_updated),
      crypto_overview_basic::image_url.eq(&coin.image),
      crypto_overview_basic::m_time.eq(Utc::now()),
    ))
    .execute(conn)
    .map_err(|e| {
      LoaderError::DatabaseError(format!("Failed to update basic data for SID {}: {}", sid, e))
    })?;

  // Update or insert metrics
  insert_metrics(conn, sid, coin)?;

  // Update crypto_api_map last_verified and rank - MUST filter by api_id to avoid updating wrong entry
  diesel::update(
    crypto_api_map::table
      .filter(crypto_api_map::sid.eq(sid))
      .filter(crypto_api_map::api_id.eq(&coin.id)),
  )
  .set((
    crypto_api_map::rank.eq(coin.market_cap_rank),
    crypto_api_map::last_verified.eq(Utc::now()),
    crypto_api_map::m_time.eq(Utc::now()),
  ))
  .execute(conn)
  .map_err(|e| {
    LoaderError::DatabaseError(format!("Failed to update API mapping for SID {}: {}", sid, e))
  })?;

  Ok(())
}

/// Insert or update metrics data
fn insert_metrics(conn: &mut PgConnection, sid: i64, coin: &CoinMarketData) -> LoaderResult<()> {
  // Parse ATH/ATL dates
  let ath_date = coin
    .ath_date
    .as_ref()
    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&Utc));

  let atl_date = coin
    .atl_date
    .as_ref()
    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&Utc));

  // Convert to BigDecimal
  let price_change_24h =
    coin.price_change_24h.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_24h = coin
    .price_change_percentage_24h
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_7d = coin
    .price_change_percentage_7d_in_currency
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_14d = coin
    .price_change_percentage_14d_in_currency
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_30d = coin
    .price_change_percentage_30d_in_currency
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_200d = coin
    .price_change_percentage_200d_in_currency
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let price_change_pct_1y = coin
    .price_change_percentage_1y_in_currency
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let ath = coin.ath.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let ath_change_pct =
    coin.ath_change_percentage.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let atl = coin.atl.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let atl_change_pct =
    coin.atl_change_percentage.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let high_24h = coin.high_24h.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let low_24h = coin.low_24h.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let market_cap_change_24h =
    coin.market_cap_change_24h.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let market_cap_change_pct_24h = coin
    .market_cap_change_percentage_24h
    .map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());

  let new_metrics = NewCryptoOverviewMetrics {
    sid: &sid,
    price_change_24h: price_change_24h.as_ref(),
    price_change_pct_24h: price_change_pct_24h.as_ref(),
    price_change_pct_7d: price_change_pct_7d.as_ref(),
    price_change_pct_14d: price_change_pct_14d.as_ref(),
    price_change_pct_30d: price_change_pct_30d.as_ref(),
    price_change_pct_60d: None,
    price_change_pct_200d: price_change_pct_200d.as_ref(),
    price_change_pct_1y: price_change_pct_1y.as_ref(),
    ath: ath.as_ref(),
    ath_date: ath_date.as_ref(),
    ath_change_percentage: ath_change_pct.as_ref(),
    atl: atl.as_ref(),
    atl_date: atl_date.as_ref(),
    atl_change_percentage: atl_change_pct.as_ref(),
    roi_times: None,
    roi_currency: None,
    roi_percentage: None,
    high_24h: high_24h.as_ref(),
    low_24h: low_24h.as_ref(),
    market_cap_change_24h: market_cap_change_24h.as_ref(),
    market_cap_change_pct_24h: market_cap_change_pct_24h.as_ref(),
  };

  diesel::insert_into(crypto_overview_metrics::table)
    .values(&new_metrics)
    .on_conflict(crypto_overview_metrics::sid)
    .do_update()
    .set((
      crypto_overview_metrics::price_change_24h.eq(&price_change_24h),
      crypto_overview_metrics::price_change_pct_24h.eq(&price_change_pct_24h),
      crypto_overview_metrics::price_change_pct_7d.eq(&price_change_pct_7d),
      crypto_overview_metrics::price_change_pct_14d.eq(&price_change_pct_14d),
      crypto_overview_metrics::price_change_pct_30d.eq(&price_change_pct_30d),
      crypto_overview_metrics::price_change_pct_200d.eq(&price_change_pct_200d),
      crypto_overview_metrics::price_change_pct_1y.eq(&price_change_pct_1y),
      crypto_overview_metrics::ath.eq(&ath),
      crypto_overview_metrics::ath_date.eq(&ath_date),
      crypto_overview_metrics::ath_change_percentage.eq(&ath_change_pct),
      crypto_overview_metrics::atl.eq(&atl),
      crypto_overview_metrics::atl_date.eq(&atl_date),
      crypto_overview_metrics::atl_change_percentage.eq(&atl_change_pct),
      crypto_overview_metrics::high_24h.eq(&high_24h),
      crypto_overview_metrics::low_24h.eq(&low_24h),
      crypto_overview_metrics::market_cap_change_24h.eq(&market_cap_change_24h),
      crypto_overview_metrics::market_cap_change_pct_24h.eq(&market_cap_change_pct_24h),
      crypto_overview_metrics::m_time.eq(Utc::now()),
    ))
    .execute(conn)
    .map_err(|e| {
      LoaderError::DatabaseError(format!("Failed to upsert metrics for SID {}: {}", sid, e))
    })?;

  Ok(())
}
