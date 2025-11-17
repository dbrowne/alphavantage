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

use crate::schema::{
  crypto_api_map, crypto_overview_basic, crypto_overview_metrics, crypto_social, crypto_technical,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_overview_basic)]
#[diesel(primary_key(sid))]
pub struct CryptoOverviewBasic {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub slug: Option<String>,
  pub description: Option<String>,
  pub market_cap_rank: Option<i32>,
  pub market_cap: Option<i64>,
  pub fully_diluted_valuation: Option<i64>,
  pub volume_24h: Option<i64>,
  pub volume_change_24h: Option<BigDecimal>,
  pub current_price: Option<BigDecimal>,
  pub circulating_supply: Option<BigDecimal>,
  pub total_supply: Option<BigDecimal>,
  pub max_supply: Option<BigDecimal>,
  pub last_updated: Option<DateTime<Utc>>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_overview_metrics)]
#[diesel(primary_key(sid))]
pub struct CryptoOverviewMetrics {
  pub sid: i64,
  pub price_change_24h: Option<BigDecimal>,
  pub price_change_pct_24h: Option<BigDecimal>,
  pub price_change_pct_7d: Option<BigDecimal>,
  pub price_change_pct_14d: Option<BigDecimal>,
  pub price_change_pct_30d: Option<BigDecimal>,
  pub price_change_pct_60d: Option<BigDecimal>,
  pub price_change_pct_200d: Option<BigDecimal>,
  pub price_change_pct_1y: Option<BigDecimal>,
  pub ath: Option<BigDecimal>,
  pub ath_date: Option<DateTime<Utc>>,
  pub ath_change_percentage: Option<BigDecimal>,
  pub atl: Option<BigDecimal>,
  pub atl_date: Option<DateTime<Utc>>,
  pub atl_change_percentage: Option<BigDecimal>,
  pub roi_times: Option<BigDecimal>,
  pub roi_currency: Option<String>,
  pub roi_percentage: Option<BigDecimal>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

// Combined view for convenience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoOverviewFull {
  pub basic: CryptoOverviewBasic,
  pub metrics: Option<CryptoOverviewMetrics>,
}

// Insertable structs
#[derive(Insertable, Debug)]
#[diesel(table_name = crypto_overview_basic)]
pub struct NewCryptoOverviewBasic<'a> {
  pub sid: &'a i64,
  pub symbol: &'a str,
  pub name: &'a str,
  pub slug: Option<&'a str>,
  pub description: Option<&'a str>,
  pub market_cap_rank: Option<&'a i32>,
  pub market_cap: Option<&'a i64>,
  pub fully_diluted_valuation: Option<&'a i64>,
  pub volume_24h: Option<&'a i64>,
  pub volume_change_24h: Option<&'a BigDecimal>,
  pub current_price: Option<&'a BigDecimal>,
  pub circulating_supply: Option<&'a BigDecimal>,
  pub total_supply: Option<&'a BigDecimal>,
  pub max_supply: Option<&'a BigDecimal>,
  pub last_updated: Option<&'a DateTime<Utc>>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crypto_overview_metrics)]
pub struct NewCryptoOverviewMetrics<'a> {
  pub sid: &'a i64,
  pub price_change_24h: Option<&'a BigDecimal>,
  pub price_change_pct_24h: Option<&'a BigDecimal>,
  pub price_change_pct_7d: Option<&'a BigDecimal>,
  pub price_change_pct_14d: Option<&'a BigDecimal>,
  pub price_change_pct_30d: Option<&'a BigDecimal>,
  pub price_change_pct_60d: Option<&'a BigDecimal>,
  pub price_change_pct_200d: Option<&'a BigDecimal>,
  pub price_change_pct_1y: Option<&'a BigDecimal>,
  pub ath: Option<&'a BigDecimal>,
  pub ath_date: Option<&'a DateTime<Utc>>,
  pub ath_change_percentage: Option<&'a BigDecimal>,
  pub atl: Option<&'a BigDecimal>,
  pub atl_date: Option<&'a DateTime<Utc>>,
  pub atl_change_percentage: Option<&'a BigDecimal>,
  pub roi_times: Option<&'a BigDecimal>,
  pub roi_currency: Option<&'a str>,
  pub roi_percentage: Option<&'a BigDecimal>,
}

// ===== CryptoTechnical =====
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_technical)]
#[diesel(primary_key(sid))]
pub struct CryptoTechnical {
  pub sid: i64,
  pub blockchain_platform: Option<String>,
  pub token_standard: Option<String>,
  pub consensus_mechanism: Option<String>,
  pub hashing_algorithm: Option<String>,
  pub block_time_minutes: Option<BigDecimal>,
  pub block_reward: Option<BigDecimal>,
  pub block_height: Option<i64>,
  pub hash_rate: Option<BigDecimal>,
  pub difficulty: Option<BigDecimal>,
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
  pub ico_price: Option<BigDecimal>,
  pub ico_date: Option<NaiveDate>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crypto_technical)]
pub struct NewCryptoTechnical {
  pub sid: i64,
  pub blockchain_platform: Option<String>,
  pub token_standard: Option<String>,
  pub consensus_mechanism: Option<String>,
  pub hashing_algorithm: Option<String>,
  pub block_time_minutes: Option<BigDecimal>,
  pub block_reward: Option<BigDecimal>,
  pub block_height: Option<i64>,
  pub hash_rate: Option<BigDecimal>,
  pub difficulty: Option<BigDecimal>,
  pub github_forks: Option<i32>,
  pub github_stars: Option<i32>,
  pub github_subscribers: Option<i32>,
  pub github_total_issues: Option<i32>,
  pub github_closed_issues: Option<i32>,
  pub github_pull_requests: Option<i32>,
  pub github_contributors: Option<i32>,
  pub github_commits_4_weeks: Option<i32>,
  pub is_defi: bool,
  pub is_stablecoin: bool,
  pub is_nft_platform: bool,
  pub is_exchange_token: bool,
  pub is_gaming: bool,
  pub is_metaverse: bool,
  pub is_privacy_coin: bool,
  pub is_layer2: bool,
  pub is_wrapped: bool,
  pub genesis_date: Option<NaiveDate>,
  pub ico_price: Option<BigDecimal>,
  pub ico_date: Option<NaiveDate>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

// ===== CryptoSocial =====
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_social)]
#[diesel(primary_key(sid))]
pub struct CryptoSocial {
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
  pub coingecko_score: Option<BigDecimal>,
  pub developer_score: Option<BigDecimal>,
  pub community_score: Option<BigDecimal>,
  pub liquidity_score: Option<BigDecimal>,
  pub public_interest_score: Option<BigDecimal>,
  pub sentiment_votes_up_pct: Option<BigDecimal>,
  pub sentiment_votes_down_pct: Option<BigDecimal>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crypto_social)]
pub struct NewCryptoSocial {
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
  pub coingecko_score: Option<BigDecimal>,
  pub developer_score: Option<BigDecimal>,
  pub community_score: Option<BigDecimal>,
  pub liquidity_score: Option<BigDecimal>,
  pub public_interest_score: Option<BigDecimal>,
  pub sentiment_votes_up_pct: Option<BigDecimal>,
  pub sentiment_votes_down_pct: Option<BigDecimal>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

// ===== CryptoApiMap =====
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_api_map)]
#[diesel(primary_key(sid, api_source))]
pub struct CryptoApiMap {
  pub sid: i64,
  pub api_source: String,
  pub api_id: String,
  pub api_slug: Option<String>,
  pub api_symbol: Option<String>,
  pub rank: Option<i32>,
  pub is_active: Option<bool>,
  pub last_verified: Option<DateTime<Utc>>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crypto_api_map)]
pub struct NewCryptoApiMap {
  pub sid: i64,
  pub api_source: String,
  pub api_id: String,
  pub api_slug: Option<String>,
  pub api_symbol: Option<String>,
  pub rank: Option<i32>,
  pub is_active: Option<bool>,
  pub last_verified: Option<DateTime<Utc>>,
  pub c_time: DateTime<Utc>,
  pub m_time: DateTime<Utc>,
}

impl CryptoApiMap {
  /// Get API ID for a specific source and symbol (SYNC version)
  pub fn get_api_id(
    conn: &mut PgConnection,
    sid_param: i64,
    api_source_param: &str,
  ) -> Result<Option<String>, diesel::result::Error> {
    use crate::schema::crypto_api_map::dsl::*;

    crypto_api_map
      .filter(sid.eq(sid_param))
      .filter(api_source.eq(api_source_param))
      .filter(is_active.eq(Some(true)))
      .select(api_id)
      .first::<String>(conn)
      .optional()
  }

  /// Get all API mappings for a symbol (SYNC version)
  pub fn get_all_mappings(
    conn: &mut PgConnection,
    sid_param: i64,
  ) -> Result<Vec<(String, String)>, diesel::result::Error> {
    use crate::schema::crypto_api_map::dsl::*;

    crypto_api_map
      .filter(sid.eq(sid_param))
      .filter(is_active.eq(Some(true)))
      .select((api_source, api_id))
      .load::<(String, String)>(conn)
  }

  /// Update or insert API mapping (SYNC version)
  pub fn upsert_mapping(
    conn: &mut PgConnection,
    sid_param: i64,
    api_source_param: &str,
    api_id_param: &str,
    api_slug_param: Option<&str>,
    api_symbol_param: Option<&str>,
    rank_param: Option<i32>,
  ) -> Result<(), diesel::result::Error> {
    use crate::schema::crypto_api_map;
    use diesel::insert_into;

    let new_mapping = NewCryptoApiMap {
      sid: sid_param,
      api_source: api_source_param.to_string(),
      api_id: api_id_param.to_string(),
      api_slug: api_slug_param.map(|s| s.to_string()),
      api_symbol: api_symbol_param.map(|s| s.to_string()),
      rank: rank_param,
      is_active: Some(true),
      last_verified: Some(Utc::now()),
      c_time: Utc::now(),
      m_time: Utc::now(),
    };

    insert_into(crypto_api_map::table)
      .values(&new_mapping)
      .on_conflict((crypto_api_map::sid, crypto_api_map::api_source))
      .do_update()
      .set((
        crypto_api_map::api_id.eq(&new_mapping.api_id),
        crypto_api_map::api_slug.eq(&new_mapping.api_slug),
        crypto_api_map::api_symbol.eq(&new_mapping.api_symbol),
        crypto_api_map::rank.eq(&new_mapping.rank),
        crypto_api_map::is_active.eq(&new_mapping.is_active),
        crypto_api_map::last_verified.eq(&new_mapping.last_verified),
        crypto_api_map::m_time.eq(&new_mapping.m_time),
      ))
      .execute(conn)?;

    Ok(())
  }

  /// Get cryptocurrencies that need API mapping for a specific source
  /// Uses crypto_markets to determine if symbol is actively traded
  pub fn get_symbols_needing_mapping(
    conn: &mut PgConnection,
    api_source_param: &str,
  ) -> Result<Vec<(i64, String, String)>, diesel::result::Error> {
    use crate::schema::{crypto_api_map, crypto_markets, symbols};

    // Get crypto symbols that:
    // 1. Are cryptocurrency type
    // 2. Have active markets (are actively traded)
    // 3. Don't have API mapping for this source
    symbols::table
      .inner_join(crypto_markets::table.on(crypto_markets::sid.eq(symbols::sid)))
      .left_join(
        crypto_api_map::table.on(
          crypto_api_map::sid
            .eq(symbols::sid)
            .and(crypto_api_map::api_source.eq(api_source_param))
            .and(crypto_api_map::is_active.eq(Some(true))),
        ),
      )
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(crypto_markets::is_active.eq(Some(true)))
      .filter(crypto_api_map::sid.is_null()) // No mapping exists
      .select((symbols::sid, symbols::symbol, symbols::name))
      .distinct()
      .load::<(i64, String, String)>(conn)
  }

  /// Find mapping by symbol and source
  pub fn find_by_symbol_and_source(
    conn: &mut PgConnection,
    symbol_param: &str,
    api_source_param: &str,
  ) -> Result<Option<Self>, diesel::result::Error> {
    use crate::schema::{crypto_api_map, symbols};

    crypto_api_map::table
      .inner_join(symbols::table)
      .filter(symbols::symbol.eq(symbol_param))
      .filter(crypto_api_map::api_source.eq(api_source_param))
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .select(CryptoApiMap::as_select())
      .first::<Self>(conn)
      .optional()
  }

  /// Get stale mappings that need verification
  pub fn get_stale_mappings(
    conn: &mut PgConnection,
    api_source_param: &str,
    days_threshold: i32,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    use crate::schema::crypto_api_map::dsl::*;

    let threshold_date = Utc::now() - chrono::Duration::days(days_threshold as i64);

    crypto_api_map
      .filter(api_source.eq(api_source_param))
      .filter(is_active.eq(Some(true)))
      .filter(last_verified.is_null().or(last_verified.lt(threshold_date)))
      .load::<Self>(conn)
  }

  /// Get active crypto symbols with their API mappings
  pub fn get_active_cryptos_with_mappings(
    conn: &mut PgConnection,
    api_source_param: &str,
  ) -> Result<Vec<(i64, String, String, Option<String>)>, diesel::result::Error> {
    use crate::schema::{crypto_api_map, crypto_markets, symbols};

    symbols::table
      .inner_join(crypto_markets::table.on(crypto_markets::sid.eq(symbols::sid)))
      .left_join(
        crypto_api_map::table.on(
          crypto_api_map::sid
            .eq(symbols::sid)
            .and(crypto_api_map::api_source.eq(api_source_param))
            .and(crypto_api_map::is_active.eq(Some(true))),
        ),
      )
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(crypto_markets::is_active.eq(Some(true)))
      .select((symbols::sid, symbols::symbol, symbols::name, crypto_api_map::api_id.nullable()))
      .distinct()
      .load::<(i64, String, String, Option<String>)>(conn)
  }

  /// Get cryptocurrency summary with market activity
  pub fn get_crypto_summary(
    conn: &mut PgConnection,
  ) -> Result<CryptoSummary, diesel::result::Error> {
    use crate::schema::{crypto_markets, symbols};

    let total_cryptos: i64 =
      symbols::table.filter(symbols::sec_type.eq("Cryptocurrency")).count().get_result(conn)?;

    let active_cryptos: i64 = symbols::table
      .inner_join(crypto_markets::table.on(crypto_markets::sid.eq(symbols::sid)))
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(crypto_markets::is_active.eq(Some(true)))
      .count()
      .get_result(conn)?;

    let mapped_coingecko: i64 = crypto_api_map::table
      .filter(crypto_api_map::api_source.eq("CoinGecko"))
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .count()
      .get_result(conn)?;

    let mapped_coinpaprika: i64 = crypto_api_map::table
      .filter(crypto_api_map::api_source.eq("CoinPaprika"))
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .count()
      .get_result(conn)?;

    Ok(CryptoSummary { total_cryptos, active_cryptos, mapped_coingecko, mapped_coinpaprika })
  }
}

/// Summary statistics for crypto mappings
#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoSummary {
  pub total_cryptos: i64,
  pub active_cryptos: i64,
  pub mapped_coingecko: i64,
  pub mapped_coinpaprika: i64,
}

// Helper function to discover CoinGecko ID using their API
pub async fn discover_coingecko_id(
  client: &reqwest::Client,
  symbol: &str,
  api_key: Option<&str>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
  let mut url = "https://pro-api.coingecko.com/api/v3/coins/list".to_string();
  if let Some(key) = api_key {
    url = format!("{}?x_cg_pro_api_key={}", url, key);
  }

  let response: reqwest::Response = client.get(&url).send().await?;

  if response.status() == 429 {
    return Err("Rate limit exceeded".into());
  }

  if !response.status().is_success() {
    return Err(format!("HTTP {}", response.status()).into());
  }

  let coins: Vec<serde_json::Value> = response.json().await?;

  // Look for exact symbol match
  for coin in coins {
    if let (Some(id), Some(coin_symbol)) = (coin.get("id"), coin.get("symbol")) {
      if coin_symbol.as_str() == Some(&symbol.to_lowercase()) {
        return Ok(Some(id.as_str().unwrap().to_string()));
      }
    }
  }

  Ok(None)
}

// Helper function to discover CoinPaprika ID using their API
pub async fn discover_coinpaprika_id(
  client: &reqwest::Client,
  symbol: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
  let url = "https://api.coinpaprika.com/v1/coins";
  let response: reqwest::Response = client.get(url).send().await?;

  if response.status() == 429 {
    return Err("Rate limit exceeded".into());
  }

  if !response.status().is_success() {
    return Err(format!("HTTP {}", response.status()).into());
  }

  let coins: Vec<serde_json::Value> = response.json().await?;

  // Look for exact symbol match
  for coin in coins {
    if let Some(coin_symbol) = coin.get("symbol") {
      if coin_symbol.as_str() == Some(&symbol.to_uppercase()) {
        if let Some(id) = coin.get("id") {
          return Ok(Some(id.as_str().unwrap().to_string()));
        }
      }
    }
  }

  Ok(None)
}
