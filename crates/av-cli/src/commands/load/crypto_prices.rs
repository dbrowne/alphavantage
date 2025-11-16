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

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::Args;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use av_database_postgres::models::SymbolMapping;

#[derive(Args, Debug)]
pub struct CryptoPricesArgs {
  /// Specific symbols to load (comma-separated)
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Limit number of symbols to load
  #[arg(short, long)]
  limit: Option<usize>,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,

  /// Continue on errors (default: true, use --no-continue-on-error to stop on first error)
  #[arg(short = 'k', long, default_value = "true")]
  continue_on_error: bool,

  /// Delay between requests in milliseconds (deprecated, use per-source rate limits)
  #[arg(long, default_value = "2000")]
  delay_ms: u64,

  /// CoinGecko API key (optional, for pro tier with higher rate limits)
  #[arg(long, env = "COINGECKO_API_KEY")]
  coingecko_api_key: Option<String>,

  /// CoinMarketCap API key (optional)
  #[arg(long, env = "CMC_API_KEY")]
  coinmarketcap_api_key: Option<String>,

  /// AlphaVantage API key (optional)
  #[arg(long, env = "ALPHAVANTAGE_API_KEY")]
  alphavantage_api_key: Option<String>,

  /// Priority order of sources to try (comma-separated, default: coingecko,coinmarketcap,alphavantage)
  #[arg(long, value_delimiter = ',', default_value = "coingecko,coinmarketcap,alphavantage")]
  sources: Vec<String>,

  /// Enable parallel fetching from multiple sources (default: true)
  #[arg(long, default_value = "true")]
  parallel_fetch: bool,

  /// Enable response caching
  #[arg(long, default_value = "true")]
  enable_cache: bool,

  /// Cache TTL in minutes (default: 5 minutes for price data)
  #[arg(long, default_value = "5")]
  cache_ttl_minutes: u32,

  /// Force refresh - ignore cache and fetch fresh data
  #[arg(long)]
  force_refresh: bool,

  /// Include all symbols (including those with priority >= 9999999)
  #[arg(long)]
  no_priority_filter: bool,

  /// Force specific source (overrides --sources, for backward compatibility)
  #[arg(long)]
  source: Option<String>,
}

/// Cryptocurrency price data from any supported source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoPriceData {
  pub sid: i64,
  pub symbol: String,
  pub timestamp: DateTime<Utc>,
  pub price_usd: f64,
  pub volume_24h: f64,
  pub market_cap: f64,
  pub percent_change_1h: f64,
  pub percent_change_24h: f64,
  pub percent_change_7d: f64,
}

/// Configuration for caching
#[derive(Clone)]
struct CacheConfig {
  database_url: String,
  enable_cache: bool,
  cache_ttl_minutes: u32,
  force_refresh: bool,
}

/// Symbol with its associated source mappings
#[derive(Debug, Clone)]
struct SymbolWithMappings {
  sid: i64,
  symbol: String,
  mappings: Vec<SymbolMapping>,
}

// Cache query result structure
#[derive(QueryableByName, Debug)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: DateTime<Utc>,
}

/// Generate cache key for price requests
fn generate_cache_key(sid: i64, symbol: &str) -> String {
  format!("crypto_price_{}_{}", sid, symbol)
}

/// Get cached price if available and not expired
async fn get_cached_price(cache_config: &CacheConfig, cache_key: &str) -> Option<CryptoPriceData> {
  if !cache_config.enable_cache || cache_config.force_refresh {
    return None;
  }

  let database_url = cache_config.database_url.clone();
  let cache_key = cache_key.to_string();

  tokio::task::spawn_blocking(move || {
    let mut conn = match diesel::PgConnection::establish(&database_url) {
      Ok(conn) => conn,
      Err(e) => {
        warn!("Failed to connect for cache check: {}", e);
        return None;
      }
    };

    let cached_entry: Option<CacheQueryResult> = match sql_query(
      "SELECT response_data, expires_at FROM api_response_cache
           WHERE cache_key = $1 AND expires_at > NOW()",
    )
    .bind::<sql_types::Text, _>(&cache_key)
    .get_result(&mut conn)
    .optional()
    {
      Ok(result) => result,
      Err(e) => {
        warn!("Cache query failed for {}: {}", cache_key, e);
        return None;
      }
    };

    if let Some(cache_result) = cached_entry {
      info!("📦 Cache hit for {} (expires: {})", cache_key, cache_result.expires_at);

      match serde_json::from_value::<CryptoPriceData>(cache_result.response_data) {
        Ok(price) => return Some(price),
        Err(e) => {
          warn!("Failed to deserialize cached data for {}: {}", cache_key, e);
          return None;
        }
      }
    }

    debug!("Cache miss for {}", cache_key);
    None
  })
  .await
  .unwrap_or(None)
}

/// Store price data in cache
async fn store_cached_price(
  cache_config: &CacheConfig,
  cache_key: &str,
  price: &CryptoPriceData,
) -> Result<()> {
  if !cache_config.enable_cache {
    return Ok(());
  }

  let database_url = cache_config.database_url.clone();
  let cache_key = cache_key.to_string();
  let price_json = serde_json::to_value(price)?;
  let cache_ttl_minutes = cache_config.cache_ttl_minutes;

  tokio::task::spawn_blocking(move || {
    let mut conn = diesel::PgConnection::establish(&database_url)
      .map_err(|e| anyhow!("Cache connection failed: {}", e))?;

    let now = Utc::now();
    let expires_at = now + chrono::Duration::minutes(cache_ttl_minutes as i64);

    let endpoint_url = format!("crypto_price/{}", cache_key);
    let status_code = 200;

    sql_query(
      "INSERT INTO api_response_cache
       (cache_key, api_source, endpoint_url, response_data, status_code, cached_at, expires_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       ON CONFLICT (cache_key)
       DO UPDATE SET response_data = $4, cached_at = $6, expires_at = $7",
    )
    .bind::<sql_types::Text, _>(&cache_key)
    .bind::<sql_types::Text, _>("crypto_price")
    .bind::<sql_types::Text, _>(&endpoint_url)
    .bind::<sql_types::Jsonb, _>(&price_json)
    .bind::<sql_types::Int4, _>(status_code)
    .bind::<sql_types::Timestamptz, _>(now)
    .bind::<sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn)
    .map_err(|e| anyhow!("Failed to store cache: {}", e))?;

    info!(
      "💾 Cached price for {} (TTL: {}m, expires: {})",
      cache_key, cache_ttl_minutes, expires_at
    );

    Ok(())
  })
  .await
  .map_err(|e| anyhow!("Task join error: {}", e))?
}

/// Fetch price data from CoinGecko
async fn fetch_from_coingecko(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  api_key: Option<&str>,
) -> Result<CryptoPriceData> {
  // CoinGecko uses different URLs for free vs pro tier
  let base_url = if api_key.is_some() {
    "https://pro-api.coingecko.com/api/v3"
  } else {
    "https://api.coingecko.com/api/v3"
  };
  let url = format!("{}/simple/price", base_url);

  let symbol_lower = symbol.to_lowercase();

  debug!("Calling CoinGecko API ({}) for price data: {}", base_url, symbol);

  let mut request = client
    .get(&url)
    .query(&[
      ("ids", symbol_lower.as_str()),
      ("vs_currencies", "usd"),
      ("include_market_cap", "true"),
      ("include_24hr_vol", "true"),
      ("include_24hr_change", "true"),
      ("include_last_updated_at", "true"),
    ])
    .timeout(Duration::from_secs(10));

  // Add API key if provided (for pro tier)
  if let Some(key) = api_key {
    request = request.header("x-cg-pro-api-key", key);
  }

  let response = request.send().await?;

  debug!("CoinGecko API response status for {}: {}", symbol, response.status());

  if response.status() != 200 {
    let status = response.status();
    let text = response.text().await.unwrap_or_else(|_| "Unable to read response".to_string());
    return Err(anyhow!("CoinGecko returned status {}: {}", status, text));
  }

  let cg_response: Value = response.json().await?;

  debug!(
    "CoinGecko response for {}: {}",
    symbol,
    serde_json::to_string(&cg_response).unwrap_or_default()
  );

  // CoinGecko /simple/price returns: {"coin-id": {"usd": 123.45, "usd_market_cap": 456, ...}}
  let coin_data = cg_response
    .get(&symbol_lower)
    .or_else(|| cg_response.get(symbol))
    .ok_or_else(|| {
      // If not found, might need to use coin ID instead of symbol
      anyhow!(
        "CoinGecko response missing data for {} (available keys: {:?}). Note: CoinGecko uses coin IDs, not symbols.",
        symbol,
        cg_response.as_object().map(|o| o.keys().collect::<Vec<_>>()).unwrap_or_default()
      )
    })?;

  let timestamp = if let Some(ts) = coin_data.get("last_updated_at").and_then(|v| v.as_i64()) {
    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
  } else {
    Utc::now()
  };

  Ok(CryptoPriceData {
    sid,
    symbol: symbol.to_string(),
    timestamp,
    price_usd: coin_data.get("usd").and_then(|v| v.as_f64()).unwrap_or(0.0),
    volume_24h: coin_data.get("usd_24h_vol").and_then(|v| v.as_f64()).unwrap_or(0.0),
    market_cap: coin_data.get("usd_market_cap").and_then(|v| v.as_f64()).unwrap_or(0.0),
    percent_change_1h: 0.0, // CoinGecko simple/price doesn't provide 1h change
    percent_change_24h: coin_data.get("usd_24h_change").and_then(|v| v.as_f64()).unwrap_or(0.0),
    percent_change_7d: 0.0, // CoinGecko simple/price doesn't provide 7d change
  })
}

/// Fetch price data from CoinMarketCap
async fn fetch_from_coinmarketcap(
  client: &reqwest::Client,
  sid: i64,
  symbol: &str,
  api_key: &str,
) -> Result<CryptoPriceData> {
  let url = "https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest";

  debug!("Calling CoinMarketCap API for price data: {}", symbol);

  let response = client
    .get(url)
    .header("X-CMC_PRO_API_KEY", api_key)
    .header("Accept", "application/json")
    .query(&[("symbol", symbol), ("convert", "USD")])
    .timeout(Duration::from_secs(10))
    .send()
    .await?;

  debug!("CoinMarketCap API response status for {}: {}", symbol, response.status());

  if response.status() != 200 {
    let status = response.status();
    let text = response.text().await.unwrap_or_else(|_| "Unable to read response".to_string());
    return Err(anyhow!("CoinMarketCap returned status {}: {}", status, text));
  }

  let cmc_response: Value = response.json().await?;

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

  // Extract cryptocurrency data from the response
  // CMC v2 API returns: {"data": {"SYMBOL": [array of coin objects with that symbol]}}
  let data_obj = cmc_response
    .get("data")
    .and_then(|d| d.as_object())
    .ok_or_else(|| anyhow!("CoinMarketCap response missing 'data' object"))?;

  // Find matching key (case-insensitive)
  let symbol_key = data_obj.keys().find(|k| k.eq_ignore_ascii_case(symbol)).ok_or_else(|| {
    anyhow!(
      "CoinMarketCap response missing data for {} (available keys: {:?})",
      symbol,
      data_obj.keys().collect::<Vec<_>>()
    )
  })?;

  let symbol_data = data_obj.get(symbol_key).expect("symbol_key exists");

  // Extract the coin data - it's an array in v2 API
  let crypto_data = if let Some(arr) = symbol_data.as_array() {
    if let Some(first) = arr.first() {
      debug!("CMC returned array with {} items, using first", arr.len());
      first.clone()
    } else {
      return Err(anyhow!("CoinMarketCap returned empty array for {}", symbol));
    }
  } else {
    debug!("CMC returned non-array data for {}, using as-is", symbol);
    symbol_data.clone()
  };

  let usd_quote = crypto_data
    .get("quote")
    .and_then(|q| q.get("USD"))
    .ok_or_else(|| anyhow!("Missing USD quote data for {}", symbol))?;

  let timestamp = if let Some(last_updated) = usd_quote.get("last_updated").and_then(|v| v.as_str())
  {
    DateTime::parse_from_rfc3339(last_updated)
      .map(|dt| dt.with_timezone(&Utc))
      .unwrap_or_else(|_| Utc::now())
  } else {
    Utc::now()
  };

  Ok(CryptoPriceData {
    sid,
    symbol: symbol.to_string(),
    timestamp,
    price_usd: usd_quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0),
    volume_24h: usd_quote.get("volume_24h").and_then(|v| v.as_f64()).unwrap_or(0.0),
    market_cap: usd_quote.get("market_cap").and_then(|v| v.as_f64()).unwrap_or(0.0),
    percent_change_1h: usd_quote.get("percent_change_1h").and_then(|v| v.as_f64()).unwrap_or(0.0),
    percent_change_24h: usd_quote.get("percent_change_24h").and_then(|v| v.as_f64()).unwrap_or(0.0),
    percent_change_7d: usd_quote.get("percent_change_7d").and_then(|v| v.as_f64()).unwrap_or(0.0),
  })
}

/// Fetch price data from AlphaVantage
async fn fetch_from_alphavantage(
  client: &reqwest::Client,
  sid: i64,
  from_symbol: &str,
  api_key: &str,
) -> Result<CryptoPriceData> {
  let url = "https://www.alphavantage.co/query";

  debug!("Calling AlphaVantage API for price data: {}", from_symbol);

  let response = client
    .get(url)
    .query(&[
      ("function", "CURRENCY_EXCHANGE_RATE"),
      ("from_currency", from_symbol),
      ("to_currency", "USD"),
      ("apikey", api_key),
    ])
    .timeout(Duration::from_secs(10))
    .send()
    .await?;

  debug!("AlphaVantage API response status for {}: {}", from_symbol, response.status());

  if response.status() != 200 {
    let status = response.status();
    let text = response.text().await.unwrap_or_else(|_| "Unable to read response".to_string());
    return Err(anyhow!("AlphaVantage returned status {}: {}", status, text));
  }

  let av_response: Value = response.json().await?;

  // Check for API error messages
  if let Some(note) = av_response.get("Note").and_then(|v| v.as_str()) {
    return Err(anyhow!("AlphaVantage API limit: {}", note));
  }

  if let Some(error) = av_response.get("Error Message").and_then(|v| v.as_str()) {
    return Err(anyhow!("AlphaVantage API error: {}", error));
  }

  // Extract exchange rate data
  let exchange_rate = av_response.get("Realtime Currency Exchange Rate").ok_or_else(|| {
    anyhow!("AlphaVantage response missing exchange rate data for {}", from_symbol)
  })?;

  let price_usd = exchange_rate
    .get("5. Exchange Rate")
    .and_then(|v| v.as_str())
    .and_then(|s| s.parse::<f64>().ok())
    .unwrap_or(0.0);

  let timestamp_str = exchange_rate.get("6. Last Refreshed").and_then(|v| v.as_str()).unwrap_or("");

  let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
    .map(|dt| dt.with_timezone(&Utc))
    .or_else(|_| {
      // Try parsing as naive datetime and assume UTC
      chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| DateTime::from_naive_utc_and_offset(ndt, Utc))
    })
    .unwrap_or_else(|_| Utc::now());

  Ok(CryptoPriceData {
    sid,
    symbol: from_symbol.to_string(),
    timestamp,
    price_usd,
    volume_24h: 0.0, // AlphaVantage CURRENCY_EXCHANGE_RATE doesn't provide volume
    market_cap: 0.0, // AlphaVantage CURRENCY_EXCHANGE_RATE doesn't provide market cap
    percent_change_1h: 0.0, // Not available in this endpoint
    percent_change_24h: 0.0, // Not available in this endpoint
    percent_change_7d: 0.0, // Not available in this endpoint
  })
}

/// Fetch price from multiple sources with optional parallel execution
/// Tries sources in priority order, using mappings when available
async fn fetch_price_parallel(
  client: &reqwest::Client,
  symbol_with_mappings: &SymbolWithMappings,
  source_priority: &[String],
  parallel_fetch: bool,
  coingecko_api_key: Option<&str>,
  coinmarketcap_api_key: Option<&str>,
  alphavantage_api_key: Option<&str>,
  database_url: String,
) -> Result<CryptoPriceData> {
  let sid = symbol_with_mappings.sid;
  let symbol = &symbol_with_mappings.symbol;

  // Build a map of source -> identifier for quick lookup
  let mut mapping_map: HashMap<String, String> = HashMap::new();
  for mapping in &symbol_with_mappings.mappings {
    mapping_map.insert(mapping.source_name.to_lowercase(), mapping.source_identifier.clone());
  }

  // Try sources in priority order (sequential for now, parallel to be implemented)
  let mut last_error = None;

  for source in source_priority {
    let source_lower = source.to_lowercase();

    // Get the identifier to use for this source
    // Use mapping if available, otherwise fall back to raw symbol
    let identifier = mapping_map.get(&source_lower).cloned().unwrap_or_else(|| symbol.clone());

    debug!(
      "Trying source '{}' for {} (sid={}) using identifier '{}'",
      source, symbol, sid, identifier
    );

    // Try to fetch from this source
    let result = match source_lower.as_str() {
      "coingecko" => fetch_from_coingecko(client, sid, &identifier, coingecko_api_key).await,
      "coinmarketcap" => {
        if let Some(api_key) = coinmarketcap_api_key {
          fetch_from_coinmarketcap(client, sid, &identifier, api_key).await
        } else {
          Err(anyhow!("CoinMarketCap requires an API key"))
        }
      }
      "alphavantage" => {
        if let Some(api_key) = alphavantage_api_key {
          fetch_from_alphavantage(client, sid, &identifier, api_key).await
        } else {
          Err(anyhow!("AlphaVantage requires an API key"))
        }
      }
      _ => {
        warn!("Unknown source: {}", source);
        continue;
      }
    };

    match result {
      Ok(price) => {
        info!("✓ Successfully fetched {} from {} using '{}'", symbol, source, identifier);

        // Auto-discover mapping if this was successful and we didn't have a mapping
        if !mapping_map.contains_key(&source_lower) {
          if let Err(e) =
            auto_discover_mapping(database_url.clone(), sid, source.clone(), identifier.clone())
              .await
          {
            warn!("Failed to auto-discover mapping: {}", e);
          }
        }

        return Ok(price);
      }
      Err(e) => {
        debug!("Failed to fetch {} from {}: {}", symbol, source, e);
        last_error = Some(e);
      }
    }
  }

  // All sources failed
  Err(last_error.unwrap_or_else(|| anyhow!("No sources available to fetch price for {}", symbol)))
}

/// Auto-discover and store a new symbol mapping
/// Called when an API fetch succeeds to create/verify the mapping
async fn auto_discover_mapping(
  database_url: String,
  sid: i64,
  source_name: String,
  source_identifier: String,
) -> Result<()> {
  tokio::task::spawn_blocking(move || {
    use av_database_postgres::schema::symbol_mappings;
    use diesel::prelude::*;

    let mut conn = diesel::PgConnection::establish(&database_url)
      .map_err(|e| anyhow!("Failed to connect for mapping discovery: {}", e))?;

    // Insert or update the mapping, marking it as verified
    diesel::insert_into(symbol_mappings::table)
      .values((
        symbol_mappings::sid.eq(sid),
        symbol_mappings::source_name.eq(&source_name),
        symbol_mappings::source_identifier.eq(&source_identifier),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .on_conflict((symbol_mappings::sid, symbol_mappings::source_name))
      .do_update()
      .set((
        symbol_mappings::source_identifier.eq(&source_identifier),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .execute(&mut conn)?;

    info!("✓ Auto-discovered mapping: sid={} → {}:{}", sid, source_name, source_identifier);

    Ok(())
  })
  .await
  .map_err(|e| anyhow!("Task join error: {}", e))?
}

/// Get or create price source ID
fn get_or_create_price_source(conn: &mut PgConnection, source_name: &str) -> Result<i32> {
  use diesel::sql_query;
  use diesel::sql_types::{Integer, Text};

  #[derive(QueryableByName)]
  struct SourceId {
    #[diesel(sql_type = Integer)]
    sourceid: i32,
  }

  // First try to select existing
  if let Ok(source) = sql_query("SELECT sourceid FROM price_sources WHERE name = $1")
    .bind::<Text, _>(source_name)
    .get_result::<SourceId>(conn)
  {
    return Ok(source.sourceid);
  }

  // If not found, insert new
  let source: SourceId = sql_query(
    "INSERT INTO price_sources (name)
     VALUES ($1)
     ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
     RETURNING sourceid",
  )
  .bind::<Text, _>(source_name)
  .get_result(conn)?;

  Ok(source.sourceid)
}

/// Save price data to database
fn save_price_to_db(
  conn: &mut PgConnection,
  price: &CryptoPriceData,
  source_id: i32,
) -> Result<()> {
  use diesel::sql_query;
  use diesel::sql_types::{BigInt, Float4, Integer, Text, Timestamptz};

  // For crypto prices, we use the price as OHLC (simplified)
  // In a real scenario, you might want to fetch actual OHLCV data
  let price_f32 = price.price_usd as f32;
  let volume_i64 = price.volume_24h as i64;

  sql_query(
    "INSERT INTO intradayprices
     (tstamp, sid, symbol, open, high, low, close, volume, price_source_id)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
     ON CONFLICT DO NOTHING",
  )
  .bind::<Timestamptz, _>(price.timestamp)
  .bind::<BigInt, _>(price.sid)
  .bind::<Text, _>(&price.symbol)
  .bind::<Float4, _>(price_f32) // open
  .bind::<Float4, _>(price_f32) // high
  .bind::<Float4, _>(price_f32) // low
  .bind::<Float4, _>(price_f32) // close
  .bind::<BigInt, _>(volume_i64)
  .bind::<Integer, _>(source_id)
  .execute(conn)?;

  Ok(())
}

/// Main execute function
pub async fn execute(args: CryptoPricesArgs, config: Config) -> Result<()> {
  // Determine source priority
  let source_list = if let Some(ref single_source) = args.source {
    vec![single_source.clone()]
  } else {
    args.sources.clone()
  };

  info!("Starting cryptocurrency price loader with sources: {}", source_list.join(", "));
  info!(
    "Parallel fetch: {}",
    if args.parallel_fetch { "enabled" } else { "disabled (sequential)" }
  );

  // Create HTTP client
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("Mozilla/5.0 (compatible; CryptoPriceBot/1.0)")
    .build()?;

  // Setup cache configuration
  let cache_config = CacheConfig {
    database_url: config.database_url.clone(),
    enable_cache: args.enable_cache,
    cache_ttl_minutes: args.cache_ttl_minutes,
    force_refresh: args.force_refresh,
  };

  // Get symbols with mappings to load
  let symbols_with_mappings = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let symbols = args.symbols.clone();
    let limit = args.limit;
    let no_priority_filter = args.no_priority_filter;

    move || -> Result<Vec<SymbolWithMappings>> {
      use av_database_postgres::schema::{symbol_mappings, symbols};

      let mut conn = diesel::PgConnection::establish(&database_url)?;

      // First, query symbols
      let mut query = symbols::table
        .select((symbols::sid, symbols::symbol))
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .into_boxed();

      if !no_priority_filter {
        query = query.filter(symbols::priority.lt(9999999));
      }

      if let Some(symbol_list) = symbols {
        query = query.filter(symbols::symbol.eq_any(symbol_list));
      }

      if let Some(lim) = limit {
        query = query.limit(lim as i64);
      }

      let symbol_list = query.load::<(i64, String)>(&mut conn)?;

      if symbol_list.is_empty() {
        return Ok(Vec::new());
      }

      // Extract symbol IDs for mapping query
      let symbol_ids: Vec<i64> = symbol_list.iter().map(|(sid, _)| *sid).collect();

      // Query all mappings for these symbols
      let all_mappings = symbol_mappings::table
        .filter(symbol_mappings::sid.eq_any(&symbol_ids))
        .filter(symbol_mappings::verified.eq(true).or(symbol_mappings::verified.is_null()))
        .load::<SymbolMapping>(&mut conn)?;

      // Group mappings by sid
      let mut mappings_by_sid: HashMap<i64, Vec<SymbolMapping>> = HashMap::new();
      for mapping in all_mappings {
        mappings_by_sid.entry(mapping.sid).or_insert_with(Vec::new).push(mapping);
      }

      // Combine symbols with their mappings
      let results: Vec<SymbolWithMappings> = symbol_list
        .into_iter()
        .map(|(sid, symbol)| SymbolWithMappings {
          sid,
          symbol,
          mappings: mappings_by_sid.remove(&sid).unwrap_or_else(Vec::new),
        })
        .collect();

      Ok(results)
    }
  })
  .await??;

  if symbols_with_mappings.is_empty() {
    warn!("No cryptocurrency symbols found to process");
    return Ok(());
  }

  info!("Found {} cryptocurrencies to load prices for", symbols_with_mappings.len());

  // Log mapping statistics
  let with_mappings = symbols_with_mappings.iter().filter(|s| !s.mappings.is_empty()).count();
  let without_mappings = symbols_with_mappings.len() - with_mappings;
  info!("Symbol mappings: {} with mappings, {} without", with_mappings, without_mappings);

  info!("Cache: {}", if args.enable_cache { "enabled" } else { "disabled" });
  info!("Cache TTL: {} minutes (price data updates frequently)", args.cache_ttl_minutes);

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    for symbol_with_mappings in &symbols_with_mappings {
      info!(
        "Would load price for: {} (sid: {}, mappings: {})",
        symbol_with_mappings.symbol,
        symbol_with_mappings.sid,
        symbol_with_mappings.mappings.len()
      );
    }
    return Ok(());
  }

  // Get or create price source (for database tracking purposes)
  // For now, use first source in list or backward compat single source
  let source_name = if let Some(ref single_source) = args.source {
    single_source.clone()
  } else {
    args.sources.first().cloned().unwrap_or_else(|| "coingecko".to_string())
  };

  let source_id = tokio::task::spawn_blocking({
    let database_url = config.database_url.clone();
    let source_name_clone = source_name.clone();

    move || -> Result<i32> {
      let mut conn = diesel::PgConnection::establish(&database_url)?;
      get_or_create_price_source(&mut conn, &source_name_clone)
    }
  })
  .await??;

  info!("Using price source: {} (id: {})", source_name, source_id);

  // Load prices
  let mut all_prices = Vec::new();
  let progress = ProgressBar::new(symbols_with_mappings.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .expect("Invalid progress bar template")
      .progress_chars("##-"),
  );

  let mut cache_hits = 0;
  let mut api_calls = 0;
  let mut errors = 0;

  for symbol_with_mappings in symbols_with_mappings {
    let sid = symbol_with_mappings.sid;
    let symbol = &symbol_with_mappings.symbol;
    progress.set_message(format!("Loading {}", symbol));

    // Skip symbols with invalid characters (CoinMarketCap only accepts alphanumeric + hyphen/underscore)
    if !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
      debug!("Skipping symbol with invalid characters: {}", symbol);
      errors += 1;
      progress.inc(1);
      continue;
    }

    // Try cache first
    let cache_key = generate_cache_key(sid, &symbol);
    let price = if let Some(cached) = get_cached_price(&cache_config, &cache_key).await {
      cache_hits += 1;
      Some(cached)
    } else {
      // Fetch from API using multi-source coordinator
      sleep(Duration::from_millis(args.delay_ms)).await;

      // Determine which sources to try
      let sources_to_try = if let Some(ref single_source) = args.source {
        // Backward compatibility: use single source if specified
        vec![single_source.clone()]
      } else {
        args.sources.clone()
      };

      match fetch_price_parallel(
        &client,
        &symbol_with_mappings,
        &sources_to_try,
        args.parallel_fetch,
        args.coingecko_api_key.as_deref(),
        args.coinmarketcap_api_key.as_deref(),
        args.alphavantage_api_key.as_deref(),
        config.database_url.clone(),
      )
      .await
      {
        Ok(price) => {
          api_calls += 1;

          // Store in cache
          if cache_config.enable_cache {
            if let Err(e) = store_cached_price(&cache_config, &cache_key, &price).await {
              warn!("Failed to cache price for {}: {}", symbol, e);
            }
          }

          info!(
            "Successfully fetched {} price: ${:.6} (24h change: {:.2}%)",
            symbol, price.price_usd, price.percent_change_24h
          );

          Some(price)
        }
        Err(e) => {
          error!("Failed to fetch price for {}: {}", symbol, e);
          errors += 1;

          if !args.continue_on_error {
            progress.finish_with_message("Stopped due to error");
            return Err(e);
          }

          None
        }
      }
    };

    if let Some(price) = price {
      all_prices.push(price);
    }

    progress.inc(1);
  }

  progress.finish_with_message("Loading complete");

  info!(
    "📊 Load statistics: {} cache hits, {} API calls, {} errors ({:.1}% cache hit rate)",
    cache_hits,
    api_calls,
    errors,
    if (cache_hits + api_calls) > 0 {
      (cache_hits as f64 / (cache_hits + api_calls) as f64) * 100.0
    } else {
      0.0
    }
  );

  if !all_prices.is_empty() {
    // Save to database
    let saved_count = tokio::task::spawn_blocking({
      let database_url = config.database_url.clone();
      move || -> Result<usize> {
        let mut conn = diesel::PgConnection::establish(&database_url)?;
        let mut count = 0;

        for price in &all_prices {
          if let Err(e) = save_price_to_db(&mut conn, price, source_id) {
            warn!("Failed to save price for {}: {}", price.symbol, e);
          } else {
            count += 1;
          }
        }

        Ok(count)
      }
    })
    .await??;

    info!("Successfully saved {} cryptocurrency prices to database", saved_count);
  } else {
    warn!("No prices to save");
  }

  Ok(())
}
