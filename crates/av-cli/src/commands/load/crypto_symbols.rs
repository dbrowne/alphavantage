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
use clap::Args;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::config::Config;

#[derive(Args, Debug)]
pub struct CryptoSymbolsArgs {
  /// Limit number of symbols to load (for testing)
  #[arg(short, long)]
  limit: Option<usize>,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,

  /// Continue on errors
  #[arg(short = 'k', long, default_value = "true")]
  continue_on_error: bool,

  /// Delay between requests in milliseconds
  #[arg(long, default_value = "1000")]
  delay_ms: u64,

  /// CoinGecko API key (optional, for pro tier)
  #[arg(long, env = "COINGECKO_API_KEY")]
  coingecko_api_key: Option<String>,

  /// CoinMarketCap API key (optional, for mapping verification)
  #[arg(long, env = "CMC_API_KEY")]
  coinmarketcap_api_key: Option<String>,

  /// Only update existing symbols (don't insert new ones)
  #[arg(long)]
  update_only: bool,

  /// Minimum market cap rank to include (e.g., top 500)
  #[arg(long)]
  max_rank: Option<u32>,
}

/// CoinGecko coin list entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoCoin {
  id: String,     // "bitcoin"
  symbol: String, // "btc"
  name: String,   // "Bitcoin"
}

/// Symbol to be inserted
#[derive(Debug, Clone)]
struct CryptoSymbol {
  symbol: String,                       // "BTC"
  name: String,                         // "Bitcoin"
  coingecko_id: String,                 // "bitcoin"
  coinmarketcap_symbol: Option<String>, // "BTC" (verified from CMC)
  sec_type: String,                     // "Cryptocurrency"
}

/// Fetch list of all cryptocurrencies from CoinGecko
async fn fetch_coingecko_coin_list(
  client: &reqwest::Client,
  api_key: Option<&str>,
) -> Result<Vec<CoinGeckoCoin>> {
  let base_url = if api_key.is_some() {
    "https://pro-api.coingecko.com/api/v3"
  } else {
    "https://api.coingecko.com/api/v3"
  };
  let url = format!("{}/coins/list", base_url);

  info!("Fetching cryptocurrency list from CoinGecko...");

  let mut request = client.get(&url).timeout(Duration::from_secs(30));

  if let Some(key) = api_key {
    request = request.header("x-cg-pro-api-key", key);
  }

  let response = request.send().await?;

  if response.status() != 200 {
    let status = response.status();
    let text = response.text().await.unwrap_or_else(|_| "Unable to read response".to_string());
    return Err(anyhow!("CoinGecko returned status {}: {}", status, text));
  }

  let coins: Vec<CoinGeckoCoin> = response.json().await?;
  info!("Fetched {} cryptocurrencies from CoinGecko", coins.len());

  Ok(coins)
}

/// Verify symbol exists on CoinMarketCap and get its identifier
async fn verify_coinmarketcap_symbol(
  client: &reqwest::Client,
  symbol: &str,
  api_key: &str,
) -> Result<String> {
  let url = "https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest";

  let response = client
    .get(url)
    .header("X-CMC_PRO_API_KEY", api_key)
    .header("Accept", "application/json")
    .query(&[("symbol", symbol), ("convert", "USD")])
    .timeout(Duration::from_secs(10))
    .send()
    .await?;

  if response.status() != 200 {
    return Err(anyhow!("CoinMarketCap returned status {}", response.status()));
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

  // Extract the symbol from response
  let data_obj = cmc_response
    .get("data")
    .and_then(|d| d.as_object())
    .ok_or_else(|| anyhow!("CoinMarketCap response missing 'data' object"))?;

  // Find matching key (case-insensitive)
  let symbol_key = data_obj
    .keys()
    .find(|k| k.eq_ignore_ascii_case(symbol))
    .ok_or_else(|| anyhow!("Symbol {} not found on CoinMarketCap", symbol))?;

  Ok(symbol_key.clone())
}

/// Insert or update symbol in database, return SID
fn insert_symbol(conn: &mut PgConnection, crypto: &CryptoSymbol, dry_run: bool) -> Result<i64> {
  use diesel::sql_query;
  use diesel::sql_types::{BigInt, Text};

  if dry_run {
    info!("Would insert symbol: {} ({})", crypto.symbol, crypto.name);
    return Ok(-1); // Dummy SID for dry run
  }

  #[derive(QueryableByName, Debug)]
  struct SymbolId {
    #[diesel(sql_type = BigInt)]
    sid: i64,
  }

  // Try to find existing symbol
  let existing: Option<SymbolId> = sql_query("SELECT sid FROM symbols WHERE symbol = $1")
    .bind::<Text, _>(&crypto.symbol)
    .get_result(conn)
    .optional()?;

  if let Some(existing_symbol) = existing {
    // Update existing
    sql_query(
      "UPDATE symbols SET name = $1, sec_type = $2, updated_at = NOW()
       WHERE sid = $3",
    )
    .bind::<Text, _>(&crypto.name)
    .bind::<Text, _>(&crypto.sec_type)
    .bind::<BigInt, _>(existing_symbol.sid)
    .execute(conn)?;

    debug!("Updated existing symbol: {} (sid: {})", crypto.symbol, existing_symbol.sid);
    Ok(existing_symbol.sid)
  } else {
    // Insert new
    let result: SymbolId = sql_query(
      "INSERT INTO symbols (symbol, name, sec_type, created_at, updated_at)
       VALUES ($1, $2, $3, NOW(), NOW())
       RETURNING sid",
    )
    .bind::<Text, _>(&crypto.symbol)
    .bind::<Text, _>(&crypto.name)
    .bind::<Text, _>(&crypto.sec_type)
    .get_result(conn)?;

    info!("Inserted new symbol: {} (sid: {})", crypto.symbol, result.sid);
    Ok(result.sid)
  }
}

/// Insert or update symbol mapping
fn insert_mapping(
  conn: &mut PgConnection,
  sid: i64,
  source_name: &str,
  source_identifier: &str,
  dry_run: bool,
) -> Result<()> {
  if dry_run {
    info!("Would insert mapping: sid={} → {}:{}", sid, source_name, source_identifier);
    return Ok(());
  }

  use av_database_postgres::schema::symbol_mappings;
  use diesel::prelude::*;

  diesel::insert_into(symbol_mappings::table)
    .values((
      symbol_mappings::sid.eq(sid),
      symbol_mappings::source_name.eq(source_name),
      symbol_mappings::source_identifier.eq(source_identifier),
      symbol_mappings::verified.eq(true),
      symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
    ))
    .on_conflict((symbol_mappings::sid, symbol_mappings::source_name))
    .do_update()
    .set((
      symbol_mappings::source_identifier.eq(source_identifier),
      symbol_mappings::verified.eq(true),
      symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
    ))
    .execute(conn)?;

  debug!("Inserted mapping: sid={} → {}:{}", sid, source_name, source_identifier);
  Ok(())
}

/// Main execute function
///
/// DEPRECATED: This command is now redundant. Use `load crypto` instead, which automatically
/// populates both crypto_api_map AND symbol_mappings tables.
pub async fn execute(args: CryptoSymbolsArgs, config: Config) -> Result<()> {
  warn!("DEPRECATED: 'crypto-symbols' command is deprecated. Use 'load crypto' instead.");
  warn!("The 'load crypto' command now automatically populates symbol_mappings table.");

  info!("Starting cryptocurrency symbol loader");

  // Create HTTP client
  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("Mozilla/5.0 (compatible; CryptoSymbolLoader/1.0)")
    .build()?;

  // Fetch coin list from CoinGecko
  let mut coins = fetch_coingecko_coin_list(&client, args.coingecko_api_key.as_deref()).await?;

  // Apply limit if specified
  if let Some(limit) = args.limit {
    coins.truncate(limit);
    info!("Limited to {} symbols for processing", limit);
  }

  if coins.is_empty() {
    warn!("No symbols to process");
    return Ok(());
  }

  info!("Processing {} cryptocurrencies", coins.len());

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
  }

  // Process symbols
  let progress = ProgressBar::new(coins.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
      .expect("Invalid progress bar template")
      .progress_chars("##-"),
  );

  let mut inserted_count = 0;
  let mut updated_count = 0;
  let mut mapping_count = 0;
  let mut error_count = 0;

  for coin in coins {
    let symbol_upper = coin.symbol.to_uppercase();
    progress.set_message(format!("Processing {}", symbol_upper));

    // Verify on CoinMarketCap if API key is provided
    let cmc_symbol = if let Some(ref api_key) = args.coinmarketcap_api_key {
      sleep(Duration::from_millis(args.delay_ms)).await;

      match verify_coinmarketcap_symbol(&client, &symbol_upper, api_key).await {
        Ok(verified_symbol) => {
          debug!("Verified {} on CoinMarketCap as {}", symbol_upper, verified_symbol);
          Some(verified_symbol)
        }
        Err(e) => {
          debug!("Could not verify {} on CoinMarketCap: {}", symbol_upper, e);
          None
        }
      }
    } else {
      // Assume symbol is the same on CMC if no API key
      Some(symbol_upper.clone())
    };

    // Create crypto symbol struct
    let crypto = CryptoSymbol {
      symbol: symbol_upper.clone(),
      name: coin.name.clone(),
      coingecko_id: coin.id.clone(),
      coinmarketcap_symbol: cmc_symbol.clone(),
      sec_type: "Cryptocurrency".to_string(),
    };

    // Insert symbol and mappings
    if !args.dry_run {
      let mut conn = match diesel::PgConnection::establish(&config.database_url) {
        Ok(conn) => conn,
        Err(e) => {
          error!("Failed to connect to database: {}", e);
          error_count += 1;
          progress.inc(1);
          continue;
        }
      };

      match insert_symbol(&mut conn, &crypto, args.dry_run) {
        Ok(sid) => {
          inserted_count += 1;

          // Insert CoinGecko mapping
          if let Err(e) =
            insert_mapping(&mut conn, sid, "coingecko", &crypto.coingecko_id, args.dry_run)
          {
            error!("Failed to insert CoinGecko mapping for {}: {}", symbol_upper, e);
          } else {
            mapping_count += 1;
          }

          // Insert CoinMarketCap mapping if verified
          if let Some(ref cmc_sym) = crypto.coinmarketcap_symbol {
            if let Err(e) = insert_mapping(&mut conn, sid, "coinmarketcap", cmc_sym, args.dry_run) {
              error!("Failed to insert CoinMarketCap mapping for {}: {}", symbol_upper, e);
            } else {
              mapping_count += 1;
            }
          }
        }
        Err(e) => {
          error!("Failed to insert symbol {}: {}", symbol_upper, e);
          error_count += 1;

          if !args.continue_on_error {
            progress.finish_with_message("Stopped due to error");
            return Err(e);
          }
        }
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message("Symbol loading complete");

  info!("📊 Load statistics:");
  info!("  - Symbols inserted/updated: {}", inserted_count);
  info!("  - Mappings created: {}", mapping_count);
  info!("  - Errors: {}", error_count);

  Ok(())
}
