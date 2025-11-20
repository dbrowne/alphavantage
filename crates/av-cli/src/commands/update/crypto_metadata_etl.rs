/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
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

use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use std::str::FromStr;
use tracing::info;

use av_database_postgres::connection::establish_connection;

pub fn execute_metadata_etl(database_url: &str) -> Result<()> {
  dotenvy::dotenv().ok();
  info!("Starting crypto metadata ETL");

  let mut conn = establish_connection(database_url)?;
  let mut stats = ProcessingStats::default();

  let batch_size = 1000;
  let mut offset = 0;

  loop {
    let query = format!(
      r#"
            SELECT
                cm.sid,
                s.symbol,
                s.name,
                cm.market_cap_rank,
                cm.additional_data::text as json_data,
                cm.last_updated
            FROM crypto_metadata cm
            INNER JOIN symbols s ON cm.sid = s.sid
            WHERE cm.additional_data IS NOT NULL
            ORDER BY cm.sid
            LIMIT {} OFFSET {}
        "#,
      batch_size, offset
    );

    let results = diesel::sql_query(&query).load::<MetadataRow>(&mut conn)?;

    if results.is_empty() {
      break; // No more records
    }

    info!("Processing batch: {} records (offset: {})", results.len(), offset);

    for row in &results {
      if let Some(json_str) = row.json_data.as_ref() {
        if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
          // Process each table
          if process_overview_basic(&mut conn, row, &json_value).is_ok() {
            stats.basic_updated += 1;
          }

          if process_social(&mut conn, row.sid, &json_value).is_ok() {
            stats.social_updated += 1;
          }

          if process_technical(&mut conn, row.sid, &json_value).is_ok() {
            stats.technical_updated += 1;
          }
        }
      }
      stats.total_processed += 1;
    }

    offset += batch_size;
  }

  info!(
    "ETL Complete: processed={}, basic={}, social={}, technical={}, errors={}",
    stats.total_processed,
    stats.basic_updated,
    stats.social_updated,
    stats.technical_updated,
    stats.errors
  );

  Ok(())
}

#[derive(QueryableByName, Debug)]
struct MetadataRow {
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  sid: i64,
  #[diesel(sql_type = diesel::sql_types::Text)]
  symbol: String,
  #[diesel(sql_type = diesel::sql_types::Text)]
  name: String,
  #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
  market_cap_rank: Option<i32>,
  #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
  json_data: Option<String>,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  last_updated: DateTime<Utc>,
}

fn process_overview_basic(conn: &mut PgConnection, row: &MetadataRow, data: &Value) -> Result<()> {
  let market_data = data.get("market_data");

  // Extract description - handle both direct string and nested object formats
  let description = data
    .get("description")
    .and_then(|d| {
      // First try to get it as a direct string
      if let Some(desc_str) = d.as_str() {
        Some(desc_str.to_string())
      } else if d.is_object() {
        // If it's an object, try to get the "en" field
        d.get("en").and_then(|v| v.as_str()).map(|s| s.to_string())
      } else {
        None
      }
    })
    .filter(|s| !s.is_empty());

  // Generate slug from name if not provided
  let slug = data
    .get("id")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string())
    .unwrap_or_else(|| row.name.to_lowercase().replace(' ', "-"));

  let current_price = market_data
    .and_then(|md| md.get("current_price"))
    .and_then(|cp| cp.get("usd"))
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let market_cap = market_data
    .and_then(|md| md.get("market_cap"))
    .and_then(|mc| mc.get("usd"))
    .and_then(|v| v.as_f64())
    .map(|v| v as i64);

  let volume_24h = market_data
    .and_then(|md| md.get("total_volume"))
    .and_then(|tv| tv.get("usd"))
    .and_then(|v| v.as_f64())
    .map(|v| v as i64);

  let circulating_supply = market_data
    .and_then(|md| md.get("circulating_supply"))
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let total_supply = market_data
    .and_then(|md| md.get("total_supply"))
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let max_supply = market_data
    .and_then(|md| md.get("max_supply"))
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  // Execute the upsert query with proper description field
  diesel::sql_query(
        "INSERT INTO crypto_overview_basic
         (sid, symbol, name, slug, description, market_cap_rank, market_cap, volume_24h,
          current_price, circulating_supply, total_supply, max_supply, last_updated)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (sid) DO UPDATE SET
            symbol = EXCLUDED.symbol,
            name = EXCLUDED.name,
            slug = EXCLUDED.slug,
            description = COALESCE(EXCLUDED.description, crypto_overview_basic.description),
            market_cap_rank = COALESCE(EXCLUDED.market_cap_rank, crypto_overview_basic.market_cap_rank),
            market_cap = COALESCE(EXCLUDED.market_cap, crypto_overview_basic.market_cap),
            volume_24h = COALESCE(EXCLUDED.volume_24h, crypto_overview_basic.volume_24h),
            current_price = COALESCE(EXCLUDED.current_price, crypto_overview_basic.current_price),
            circulating_supply = COALESCE(EXCLUDED.circulating_supply, crypto_overview_basic.circulating_supply),
            total_supply = COALESCE(EXCLUDED.total_supply, crypto_overview_basic.total_supply),
            max_supply = COALESCE(EXCLUDED.max_supply, crypto_overview_basic.max_supply),
            last_updated = EXCLUDED.last_updated"
    )
        .bind::<diesel::sql_types::BigInt, _>(row.sid)
        .bind::<diesel::sql_types::Text, _>(&row.symbol)
        .bind::<diesel::sql_types::Text, _>(&row.name)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(Some(&slug))
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(description.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(row.market_cap_rank)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::BigInt>, _>(market_cap)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::BigInt>, _>(volume_24h)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(current_price.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(circulating_supply.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(total_supply.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(max_supply.as_ref())
        .bind::<diesel::sql_types::Timestamptz, _>(row.last_updated)
        .execute(conn)?;

  Ok(())
}

fn process_social(conn: &mut PgConnection, sid: i64, data: &Value) -> Result<()> {
  let links = data.get("links");

  // Extract website URL from homepage array
  let website_url = links
    .and_then(|l| l.get("homepage"))
    .and_then(|h| h.as_array())
    .and_then(|arr| arr.first())
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());

  // Extract whitepaper URL
  let whitepaper_url = links
    .and_then(|l| l.get("whitepaper"))
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());

  // Extract GitHub URL from repos_url
  let github_url = links
    .and_then(|l| l.get("repos_url"))
    .and_then(|repos| repos.get("github"))
    .and_then(|github| github.as_array())
    .and_then(|arr| arr.first())
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());

  // Extract Twitter handle
  let twitter_handle = links
    .and_then(|l| l.get("twitter_screen_name"))
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());

  // Extract Telegram URL
  let telegram_url = links
    .and_then(|l| l.get("telegram_channel_identifier"))
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| format!("https://t.me/{}", s));

  // Extract Reddit URL
  let reddit_url = links
    .and_then(|l| l.get("subreddit_url"))
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());

  // Extract Discord URL
  let discord_url = links
    .and_then(|l| l.get("chat_url"))
    .and_then(|urls| urls.as_array())
    .and_then(|arr| arr.iter().find(|url| url.as_str().is_some_and(|s| s.contains("discord"))))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  // Extract Facebook username
  let facebook_url = links
    .and_then(|l| l.get("facebook_username"))
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())
    .map(|s| format!("https://facebook.com/{}", s));

  // Extract community data
  let community_data = data.get("community_data");

  let twitter_followers = community_data
    .and_then(|cd| cd.get("twitter_followers"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  let telegram_members = community_data
    .and_then(|cd| cd.get("telegram_channel_user_count"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  let reddit_subscribers = community_data
    .and_then(|cd| cd.get("reddit_subscribers"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  let facebook_likes = community_data
    .and_then(|cd| cd.get("facebook_likes"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  // Extract scores
  let coingecko_score = data
    .get("coingecko_score")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let developer_score = data
    .get("developer_score")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let community_score = data
    .get("community_score")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let liquidity_score = data
    .get("liquidity_score")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let public_interest_score = data
    .get("public_interest_score")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  // Extract sentiment percentages
  let sentiment_votes_up_pct = data
    .get("sentiment_votes_up_percentage")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  let sentiment_votes_down_pct = data
    .get("sentiment_votes_down_percentage")
    .and_then(|v| v.as_f64())
    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

  // Only update if we have at least one non-null value
  if website_url.is_some()
    || whitepaper_url.is_some()
    || github_url.is_some()
    || twitter_handle.is_some()
    || telegram_url.is_some()
    || reddit_url.is_some()
  {
    diesel::sql_query(
            "INSERT INTO crypto_social
             (sid, website_url, whitepaper_url, github_url, twitter_handle, twitter_followers,
              telegram_url, telegram_members, discord_url, reddit_url, reddit_subscribers,
              facebook_url, facebook_likes, coingecko_score, developer_score, community_score,
              liquidity_score, public_interest_score, sentiment_votes_up_pct, sentiment_votes_down_pct)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
             ON CONFLICT (sid) DO UPDATE SET
                website_url = COALESCE(EXCLUDED.website_url, crypto_social.website_url),
                whitepaper_url = COALESCE(EXCLUDED.whitepaper_url, crypto_social.whitepaper_url),
                github_url = COALESCE(EXCLUDED.github_url, crypto_social.github_url),
                twitter_handle = COALESCE(EXCLUDED.twitter_handle, crypto_social.twitter_handle),
                twitter_followers = COALESCE(EXCLUDED.twitter_followers, crypto_social.twitter_followers),
                telegram_url = COALESCE(EXCLUDED.telegram_url, crypto_social.telegram_url),
                telegram_members = COALESCE(EXCLUDED.telegram_members, crypto_social.telegram_members),
                discord_url = COALESCE(EXCLUDED.discord_url, crypto_social.discord_url),
                reddit_url = COALESCE(EXCLUDED.reddit_url, crypto_social.reddit_url),
                reddit_subscribers = COALESCE(EXCLUDED.reddit_subscribers, crypto_social.reddit_subscribers),
                facebook_url = COALESCE(EXCLUDED.facebook_url, crypto_social.facebook_url),
                facebook_likes = COALESCE(EXCLUDED.facebook_likes, crypto_social.facebook_likes),
                coingecko_score = COALESCE(EXCLUDED.coingecko_score, crypto_social.coingecko_score),
                developer_score = COALESCE(EXCLUDED.developer_score, crypto_social.developer_score),
                community_score = COALESCE(EXCLUDED.community_score, crypto_social.community_score),
                liquidity_score = COALESCE(EXCLUDED.liquidity_score, crypto_social.liquidity_score),
                public_interest_score = COALESCE(EXCLUDED.public_interest_score, crypto_social.public_interest_score),
                sentiment_votes_up_pct = COALESCE(EXCLUDED.sentiment_votes_up_pct, crypto_social.sentiment_votes_up_pct),
                sentiment_votes_down_pct = COALESCE(EXCLUDED.sentiment_votes_down_pct, crypto_social.sentiment_votes_down_pct)"
        )
            .bind::<diesel::sql_types::BigInt, _>(sid)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(website_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(whitepaper_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(github_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(twitter_handle)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(twitter_followers)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(telegram_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(telegram_members)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(discord_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(reddit_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(reddit_subscribers)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(facebook_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(facebook_likes)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(coingecko_score.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(developer_score.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(community_score.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(liquidity_score.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(public_interest_score.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(sentiment_votes_up_pct.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(sentiment_votes_down_pct.as_ref())
            .execute(conn)?;
  }

  Ok(())
}

fn process_technical(conn: &mut PgConnection, sid: i64, data: &Value) -> Result<()> {
  // Extract categories for classification
  let categories = data
    .get("categories")
    .and_then(|c| c.as_array())
    .map(|arr| {
      arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_lowercase()).collect::<Vec<String>>()
    })
    .unwrap_or_default();

  // Determine blockchain platform from various sources
  let blockchain_platform = determine_blockchain_platform(data, &categories);

  // Classification based on categories
  let is_defi =
    categories.iter().any(|c| c.contains("defi") || c.contains("decentralized-finance"));

  let is_stablecoin = categories.iter().any(|c| c.contains("stablecoin") || c.contains("stable"));

  let is_nft_platform = categories.iter().any(|c| c.contains("nft") || c.contains("non-fungible"));

  let is_exchange_token = categories
    .iter()
    .any(|c| c.contains("exchange") || c.contains("cex-token") || c.contains("dex-token"));

  let is_gaming = categories.iter().any(|c| c.contains("gaming") || c.contains("game"));

  let is_metaverse = categories.iter().any(|c| c.contains("metaverse") || c.contains("virtual"));

  let is_privacy_coin = categories.iter().any(|c| c.contains("privacy"));

  let is_layer2 = categories.iter().any(|c| c.contains("layer-2") || c.contains("l2"));

  let is_wrapped = categories.iter().any(|c| c.contains("wrapped"));

  // Extract developer data
  let developer_data = data.get("developer_data");

  let github_forks =
    developer_data.and_then(|dd| dd.get("forks")).and_then(|v| v.as_i64()).map(|v| v as i32);

  let github_stars =
    developer_data.and_then(|dd| dd.get("stars")).and_then(|v| v.as_i64()).map(|v| v as i32);

  let github_contributors = developer_data
    .and_then(|dd| dd.get("pull_request_contributors"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  let github_commits_4_weeks = developer_data
    .and_then(|dd| dd.get("commit_count_4_weeks"))
    .and_then(|v| v.as_i64())
    .map(|v| v as i32);

  diesel::sql_query(
        "INSERT INTO crypto_technical
         (sid, blockchain_platform, is_defi, is_stablecoin, is_nft_platform, is_exchange_token,
          is_gaming, is_metaverse, is_privacy_coin, is_layer2, is_wrapped,
          github_forks, github_stars, github_contributors, github_commits_4_weeks)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         ON CONFLICT (sid) DO UPDATE SET
            blockchain_platform = COALESCE(EXCLUDED.blockchain_platform, crypto_technical.blockchain_platform),
            is_defi = EXCLUDED.is_defi,
            is_stablecoin = EXCLUDED.is_stablecoin,
            is_nft_platform = EXCLUDED.is_nft_platform,
            is_exchange_token = EXCLUDED.is_exchange_token,
            is_gaming = EXCLUDED.is_gaming,
            is_metaverse = EXCLUDED.is_metaverse,
            is_privacy_coin = EXCLUDED.is_privacy_coin,
            is_layer2 = EXCLUDED.is_layer2,
            is_wrapped = EXCLUDED.is_wrapped,
            github_forks = COALESCE(EXCLUDED.github_forks, crypto_technical.github_forks),
            github_stars = COALESCE(EXCLUDED.github_stars, crypto_technical.github_stars),
            github_contributors = COALESCE(EXCLUDED.github_contributors, crypto_technical.github_contributors),
            github_commits_4_weeks = COALESCE(EXCLUDED.github_commits_4_weeks, crypto_technical.github_commits_4_weeks)"
    )
        .bind::<diesel::sql_types::BigInt, _>(sid)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(blockchain_platform)
        .bind::<diesel::sql_types::Bool, _>(is_defi)
        .bind::<diesel::sql_types::Bool, _>(is_stablecoin)
        .bind::<diesel::sql_types::Bool, _>(is_nft_platform)
        .bind::<diesel::sql_types::Bool, _>(is_exchange_token)
        .bind::<diesel::sql_types::Bool, _>(is_gaming)
        .bind::<diesel::sql_types::Bool, _>(is_metaverse)
        .bind::<diesel::sql_types::Bool, _>(is_privacy_coin)
        .bind::<diesel::sql_types::Bool, _>(is_layer2)
        .bind::<diesel::sql_types::Bool, _>(is_wrapped)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(github_forks)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(github_stars)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(github_contributors)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(github_commits_4_weeks)
        .execute(conn)?;

  Ok(())
}

/// Helper function to determine blockchain platform from various data sources
fn determine_blockchain_platform(data: &Value, categories: &[String]) -> Option<String> {
  // Check blockchain_site URLs for platform hints
  if let Some(blockchain_sites) =
    data.get("links").and_then(|l| l.get("blockchain_site")).and_then(|bs| bs.as_array())
  {
    for site in blockchain_sites {
      if let Some(url) = site.as_str() {
        let url_lower = url.to_lowercase();

        // Check for common blockchain explorers
        if url_lower.contains("etherscan") || url_lower.contains("ethereum") {
          return Some("Ethereum".to_string());
        } else if url_lower.contains("bscscan") || url_lower.contains("binance") {
          return Some("Binance Smart Chain".to_string());
        } else if url_lower.contains("polygonscan") || url_lower.contains("polygon") {
          return Some("Polygon".to_string());
        } else if url_lower.contains("arbiscan") || url_lower.contains("arbitrum") {
          return Some("Arbitrum".to_string());
        } else if url_lower.contains("optimistic") || url_lower.contains("optimism") {
          return Some("Optimism".to_string());
        } else if url_lower.contains("snowtrace") || url_lower.contains("avalanche") {
          return Some("Avalanche".to_string());
        } else if url_lower.contains("ftmscan") || url_lower.contains("fantom") {
          return Some("Fantom".to_string());
        } else if url_lower.contains("solscan") || url_lower.contains("solana") {
          return Some("Solana".to_string());
        } else if url_lower.contains("cardanoscan") || url_lower.contains("cardano") {
          return Some("Cardano".to_string());
        } else if url_lower.contains("polkascan") || url_lower.contains("polkadot") {
          return Some("Polkadot".to_string());
        } else if url_lower.contains("cosmos") || url_lower.contains("mintscan") {
          return Some("Cosmos".to_string());
        } else if url_lower.contains("near") {
          return Some("Near".to_string());
        } else if url_lower.contains("tron") || url_lower.contains("tronscan") {
          return Some("Tron".to_string());
        } else if url_lower.contains("zklink") {
          return Some("zkLink Nova".to_string());
        } else if url_lower.contains("zksync") {
          return Some("zkSync".to_string());
        } else if url_lower.contains("base") {
          return Some("Base".to_string());
        } else if url_lower.contains("cronos") {
          return Some("Cronos".to_string());
        } else if url_lower.contains("algorand") || url_lower.contains("algoexplorer") {
          return Some("Algorand".to_string());
        }
      }
    }
  }

  // Check categories for blockchain ecosystem mentions
  for category in categories {
    if category.contains("ethereum") {
      return Some("Ethereum".to_string());
    } else if category.contains("binance") || category.contains("bsc") || category.contains("bnb") {
      return Some("Binance Smart Chain".to_string());
    } else if category.contains("polygon") || category.contains("matic") {
      return Some("Polygon".to_string());
    } else if category.contains("arbitrum") {
      return Some("Arbitrum".to_string());
    } else if category.contains("optimism") {
      return Some("Optimism".to_string());
    } else if category.contains("avalanche") {
      return Some("Avalanche".to_string());
    } else if category.contains("solana") {
      return Some("Solana".to_string());
    } else if category.contains("cardano") {
      return Some("Cardano".to_string());
    } else if category.contains("polkadot") {
      return Some("Polkadot".to_string());
    } else if category.contains("cosmos") {
      return Some("Cosmos".to_string());
    } else if category.contains("near") {
      return Some("Near".to_string());
    } else if category.contains("tron") {
      return Some("Tron".to_string());
    } else if category.contains("algorand") {
      return Some("Algorand".to_string());
    } else if category.contains("fantom") {
      return Some("Fantom".to_string());
    } else if category.contains("zklink") {
      return Some("zkLink Nova".to_string());
    } else if category.contains("zksync") {
      return Some("zkSync".to_string());
    } else if category.contains("base") {
      return Some("Base".to_string());
    } else if category.contains("cronos") {
      return Some("Cronos".to_string());
    }
  }

  // Check if it's a native blockchain coin (usually these don't have platform info)
  // This would need to be expanded based on known native coins
  let symbol = data.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
  match symbol.to_uppercase().as_str() {
    "BTC" => return Some("Bitcoin".to_string()),
    "ETH" => return Some("Ethereum".to_string()),
    "BNB" => return Some("Binance Smart Chain".to_string()),
    "ADA" => return Some("Cardano".to_string()),
    "SOL" => return Some("Solana".to_string()),
    "DOT" => return Some("Polkadot".to_string()),
    "AVAX" => return Some("Avalanche".to_string()),
    "MATIC" => return Some("Polygon".to_string()),
    "ATOM" => return Some("Cosmos".to_string()),
    "NEAR" => return Some("Near".to_string()),
    "TRX" => return Some("Tron".to_string()),
    "ALGO" => return Some("Algorand".to_string()),
    "FTM" => return Some("Fantom".to_string()),
    "XRP" => return Some("XRP Ledger".to_string()),
    "XLM" => return Some("Stellar".to_string()),
    "XTZ" => return Some("Tezos".to_string()),
    "EOS" => return Some("EOS".to_string()),
    "HBAR" => return Some("Hedera".to_string()),
    "FLOW" => return Some("Flow".to_string()),
    "ICP" => return Some("Internet Computer".to_string()),
    "EGLD" => return Some("MultiversX".to_string()),
    _ => {}
  }

  None
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
  pub total_processed: usize,
  pub basic_updated: usize,
  pub social_updated: usize,
  pub technical_updated: usize,
  pub errors: usize,
}
