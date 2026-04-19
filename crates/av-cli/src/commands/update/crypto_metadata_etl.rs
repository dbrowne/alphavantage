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

//! Cryptocurrency metadata ETL (Extract-Transform-Load) pipeline.
//!
//! This module implements `av-cli update crypto metadata`, which reads raw
//! CoinGecko JSON metadata from the `crypto_metadata` table and normalizes it
//! into three destination tables:
//!
//! ```text
//!   Source                         Destinations
//! ┌──────────────────┐     ┌─────────────────────────────┐
//! │  crypto_metadata  │────▶│  crypto_overview_basic       │  (market data, description)
//! │  (additional_data │     ├─────────────────────────────┤
//! │   JSONB column)   │────▶│  crypto_social               │  (links, followers, scores)
//! │                   │     ├─────────────────────────────┤
//! │                   │────▶│  crypto_technical             │  (classification, GitHub stats)
//! └───��──────────────┘     └─────────────────────────────┘
//! ```
//!
//! ## ETL Pipeline Overview
//!
//! 1. **Extract** — Reads rows from `crypto_metadata` joined with `symbols`
//!    (to get `symbol` and `name`), batched in groups of 1,000, ordered by `sid`.
//!    Only rows with non-null `additional_data` are processed.
//!
//! 2. **Transform** — Parses the `additional_data` JSON column and extracts
//!    structured fields for each destination table:
//!    - **Overview basic** — description, slug, current price (USD), market cap,
//!      24h volume, circulating/total/max supply, market cap rank
//!    - **Social** — website URL, whitepaper URL, GitHub URL, Twitter handle +
//!      followers, Telegram URL + members, Discord URL, Reddit URL + subscribers,
//!      Facebook URL + likes, CoinGecko/developer/community/liquidity/public
//!      interest scores, sentiment vote percentages
//!    - **Technical** — blockchain platform detection, category-based boolean
//!      classifications (DeFi, stablecoin, NFT, exchange token, gaming, metaverse,
//!      privacy coin, layer-2, wrapped), GitHub forks/stars/contributors/commits
//!
//! 3. **Load** — Upserts each record into its destination table using
//!    `INSERT ... ON CONFLICT (sid) DO UPDATE`. The upsert uses `COALESCE` to
//!    preserve existing non-null values when the new data is null, ensuring
//!    partial updates don't erase previously loaded data.
//!
//! ## Execution Model
//!
//! Unlike most other command handlers in `av-cli`, this module runs
//! **synchronously** (no `async`). It is called from
//! [`handle_crypto_update`](super::crypto::handle_crypto_update) via:
//! ```rust,ignore
//! crypto_metadata_etl::execute_metadata_etl(&database_url)?;
//! ```
//!
//! The database URL is read from the `DATABASE_URL` environment variable by the
//! caller, since the `Metadata` variant receives `av_core::Config` (which lacks
//! a database URL).
//!
//! ## Batch Processing
//!
//! Records are processed in batches of 1,000 to manage memory usage. The loop
//! issues a `LIMIT/OFFSET` query per batch and terminates when an empty result
//! set is returned. Progress is logged via [`tracing::info`] at each batch
//! boundary and at completion.
//!
//! ## Error Handling
//!
//! - Individual row processing failures (JSON parse errors, missing fields) are
//!   **silently skipped** — the ETL continues with the next row. The
//!   [`ProcessingStats`] struct tracks counts but the `errors` field is not
//!   currently incremented.
//! - Batch-level failures (database connection loss, SQL errors in the `SELECT`
//!   query) propagate immediately as `anyhow::Error`.
//! - Upsert failures for individual records propagate via `?` and will abort
//!   the entire ETL run.
//!
//! ## Blockchain Platform Detection
//!
//! The [`determine_blockchain_platform`] helper uses a three-tier detection
//! strategy (see its documentation for details):
//! 1. Blockchain explorer URLs (e.g., etherscan.io → Ethereum)
//! 2. CoinGecko category tags (e.g., "polygon-ecosystem" → Polygon)
//! 3. Well-known native coin symbols (e.g., BTC → Bitcoin, SOL → Solana)
//!
//! Supports 18+ blockchain platforms including Ethereum, Solana, Polygon,
//! Arbitrum, Avalanche, Cosmos, and others.

use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use std::str::FromStr;
use tracing::info;

use av_database_postgres::connection::establish_connection;

/// Runs the full cryptocurrency metadata ETL pipeline.
///
/// This is the main entry point for `av-cli update crypto metadata`. It connects
/// to the database, iterates through all `crypto_metadata` rows with non-null
/// `additional_data` in batches of 1,000, and upserts the extracted fields into
/// `crypto_overview_basic`, `crypto_social`, and `crypto_technical`.
///
/// ## Processing Flow
///
/// For each row with parseable JSON in `additional_data`:
/// 1. [`process_overview_basic`] — Extracts market data and description,
///    upserts into `crypto_overview_basic`
/// 2. [`process_social`] — Extracts links, community data, and scores,
///    upserts into `crypto_social` (skipped if no social data is present)
/// 3. [`process_technical`] — Extracts category classifications and developer
///    data, upserts into `crypto_technical`
///
/// Each processor runs independently — a failure in one does not prevent the
/// others from executing for the same row. Success counts are tracked in
/// [`ProcessingStats`] and logged at completion.
///
/// # Arguments
///
/// * `database_url` — PostgreSQL connection string (e.g.,
///   `postgres://user:pass@localhost/alphavantage`).
///
/// # Errors
///
/// Returns [`anyhow::Error`] on:
/// - Database connection failure
/// - SQL query execution failure (batch SELECT or individual upsert)
///
/// # Example
///
/// ```rust,ignore
/// crypto_metadata_etl::execute_metadata_etl("postgres://localhost/alphavantage")?;
/// ```
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

/// A single row from the ETL source query, mapped via Diesel's `QueryableByName`.
///
/// Represents the join of `crypto_metadata` and `symbols` tables:
///
/// ```sql
/// SELECT cm.sid, s.symbol, s.name, cm.market_cap_rank,
///        cm.additional_data::text AS json_data, cm.last_updated
/// FROM crypto_metadata cm
/// INNER JOIN symbols s ON cm.sid = s.sid
/// WHERE cm.additional_data IS NOT NULL
/// ```
///
/// # Fields
///
/// - `sid` — Unique symbol identifier, foreign key linking `crypto_metadata`
///   to `symbols` and used as the primary key in all destination tables.
/// - `symbol` — Ticker symbol (e.g., `"BTC"`, `"ETH"`), from the `symbols` table.
/// - `name` — Full name (e.g., `"Bitcoin"`, `"Ethereum"`), from the `symbols` table.
/// - `market_cap_rank` — CoinGecko market cap ranking, nullable (may be absent
///   for low-cap or delisted coins).
/// - `json_data` — The `additional_data` JSONB column cast to text for parsing
///   with `serde_json`. Contains the full CoinGecko coin detail response.
///   `None` when `additional_data` is SQL NULL (filtered out by the WHERE clause,
///   but Diesel still requires the type to be `Option`).
/// - `last_updated` — Timestamp of the last metadata update, propagated to
///   destination tables as `last_updated`.
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

/// Extracts market overview data from CoinGecko JSON and upserts into `crypto_overview_basic`.
///
/// Parses the following fields from the JSON `additional_data`:
///
/// | Field                | JSON Path                              | DB Column            | Type         |
/// |----------------------|----------------------------------------|----------------------|--------------|
/// | Description          | `.description` (string or `.description.en`) | `description`    | `TEXT`       |
/// | Slug                 | `.id` (fallback: name slugified)       | `slug`               | `TEXT`       |
/// | Current price (USD)  | `.market_data.current_price.usd`       | `current_price`      | `NUMERIC`    |
/// | Market cap (USD)     | `.market_data.market_cap.usd`          | `market_cap`         | `BIGINT`     |
/// | 24h volume (USD)     | `.market_data.total_volume.usd`        | `volume_24h`         | `BIGINT`     |
/// | Circulating supply   | `.market_data.circulating_supply`      | `circulating_supply` | `NUMERIC`    |
/// | Total supply         | `.market_data.total_supply`            | `total_supply`       | `NUMERIC`    |
/// | Max supply           | `.market_data.max_supply`              | `max_supply`         | `NUMERIC`    |
///
/// The `sid`, `symbol`, `name`, `market_cap_rank`, and `last_updated` fields
/// come from the [`MetadataRow`] (the source query), not from the JSON.
///
/// ## Description Handling
///
/// The description field supports two CoinGecko JSON formats:
/// - Direct string: `"description": "A peer-to-peer..."`
/// - Localized object: `"description": {"en": "A peer-to-peer...", "de": "..."}`
///
/// Empty strings are filtered out and treated as `None`.
///
/// ## Upsert Strategy
///
/// Uses `INSERT ... ON CONFLICT (sid) DO UPDATE` with `COALESCE` on nullable
/// fields, so existing non-null values are preserved when the new data is null.
/// The `symbol`, `name`, and `slug` fields are always overwritten. `last_updated`
/// is always set to the source row's timestamp.
///
/// # Errors
///
/// Returns [`anyhow::Error`] if the SQL upsert query fails (e.g., connection
/// loss, constraint violation).
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

/// Extracts social and community data from CoinGecko JSON and upserts into `crypto_social`.
///
/// Parses two categories of data from the JSON:
///
/// ### Link Data (from `.links`)
///
/// | Field            | JSON Path                           | Notes                            |
/// |------------------|-------------------------------------|----------------------------------|
/// | Website URL      | `.links.homepage[0]`                | First non-empty homepage entry   |
/// | Whitepaper URL   | `.links.whitepaper`                 | Direct string                    |
/// | GitHub URL       | `.links.repos_url.github[0]`        | First GitHub repo URL            |
/// | Twitter handle   | `.links.twitter_screen_name`        | Handle only, no URL prefix       |
/// | Telegram URL     | `.links.telegram_channel_identifier` | Prefixed with `https://t.me/`   |
/// | Reddit URL       | `.links.subreddit_url`              | Full URL                         |
/// | Discord URL      | `.links.chat_url[]` (containing "discord") | First Discord match     |
/// | Facebook URL     | `.links.facebook_username`          | Prefixed with `https://facebook.com/` |
///
/// ### Community & Score Data
///
/// | Field                  | JSON Path                          | DB Type    |
/// |------------------------|------------------------------------|------------|
/// | Twitter followers      | `.community_data.twitter_followers` | `INTEGER` |
/// | Telegram members       | `.community_data.telegram_channel_user_count` | `INTEGER` |
/// | Reddit subscribers     | `.community_data.reddit_subscribers` | `INTEGER` |
/// | Facebook likes         | `.community_data.facebook_likes`   | `INTEGER`  |
/// | CoinGecko score        | `.coingecko_score`                 | `NUMERIC`  |
/// | Developer score        | `.developer_score`                 | `NUMERIC`  |
/// | Community score        | `.community_score`                 | `NUMERIC`  |
/// | Liquidity score        | `.liquidity_score`                 | `NUMERIC`  |
/// | Public interest score  | `.public_interest_score`           | `NUMERIC`  |
/// | Sentiment up %         | `.sentiment_votes_up_percentage`   | `NUMERIC`  |
/// | Sentiment down %       | `.sentiment_votes_down_percentage` | `NUMERIC`  |
///
/// ## Skip Condition
///
/// The upsert is **skipped entirely** if none of the six primary link fields
/// (website, whitepaper, GitHub, Twitter, Telegram, Reddit) have a value. This
/// avoids creating empty rows for coins with no social presence.
///
/// ## Upsert Strategy
///
/// Uses `INSERT ... ON CONFLICT (sid) DO UPDATE` with `COALESCE` on all fields,
/// preserving existing non-null values when new data is null.
///
/// # Errors
///
/// Returns [`anyhow::Error`] if the SQL upsert query fails.
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

/// Extracts technical classification and developer data from CoinGecko JSON
/// and upserts into `crypto_technical`.
///
/// This processor performs two distinct extraction tasks:
///
/// ### Category-Based Classification
///
/// Reads the `.categories` array from the JSON and sets boolean classification
/// flags by checking for keyword matches (case-insensitive):
///
/// | Flag               | Matching Keywords                              |
/// |--------------------|------------------------------------------------|
/// | `is_defi`          | "defi", "decentralized-finance"                |
/// | `is_stablecoin`    | "stablecoin", "stable"                         |
/// | `is_nft_platform`  | "nft", "non-fungible"                          |
/// | `is_exchange_token`| "exchange", "cex-token", "dex-token"           |
/// | `is_gaming`        | "gaming", "game"                               |
/// | `is_metaverse`     | "metaverse", "virtual"                         |
/// | `is_privacy_coin`  | "privacy"                                      |
/// | `is_layer2`        | "layer-2", "l2"                                |
/// | `is_wrapped`       | "wrapped"                                      |
///
/// The `blockchain_platform` field is determined by
/// [`determine_blockchain_platform`], which uses a multi-tier detection
/// strategy (see its documentation).
///
/// ### Developer / GitHub Statistics (from `.developer_data`)
///
/// | Field                   | JSON Path                                | DB Type   |
/// |-------------------------|------------------------------------------|-----------|
/// | GitHub forks            | `.developer_data.forks`                  | `INTEGER` |
/// | GitHub stars            | `.developer_data.stars`                  | `INTEGER` |
/// | GitHub contributors     | `.developer_data.pull_request_contributors` | `INTEGER` |
/// | GitHub commits (4 weeks)| `.developer_data.commit_count_4_weeks`   | `INTEGER` |
///
/// ## Upsert Strategy
///
/// Uses `INSERT ... ON CONFLICT (sid) DO UPDATE`. Boolean classification flags
/// are **always overwritten** (no `COALESCE`) since they are deterministically
/// derived from the current category data. Nullable fields (`blockchain_platform`,
/// GitHub stats) use `COALESCE` to preserve existing values.
///
/// # Errors
///
/// Returns [`anyhow::Error`] if the SQL upsert query fails.
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

/// Determines the blockchain platform for a cryptocurrency using a three-tier
/// detection strategy.
///
/// Returns the first match found across the following tiers (checked in order):
///
/// ## Tier 1: Blockchain Explorer URLs
///
/// Examines `.links.blockchain_site` (an array of explorer URLs) and matches
/// against known explorer domains:
///
/// | Domain Pattern       | Platform            |
/// |----------------------|---------------------|
/// | `etherscan`          | Ethereum            |
/// | `bscscan`, `binance` | Binance Smart Chain |
/// | `polygonscan`        | Polygon             |
/// | `arbiscan`           | Arbitrum            |
/// | `optimistic`         | Optimism            |
/// | `snowtrace`          | Avalanche           |
/// | `ftmscan`            | Fantom              |
/// | `solscan`            | Solana              |
/// | `cardanoscan`        | Cardano             |
/// | `polkascan`          | Polkadot            |
/// | `mintscan`, `cosmos` | Cosmos              |
/// | `near`               | Near                |
/// | `tronscan`           | Tron                |
/// | `zklink`             | zkLink Nova         |
/// | `zksync`             | zkSync              |
/// | `base`               | Base                |
/// | `cronos`             | Cronos              |
/// | `algoexplorer`       | Algorand            |
///
/// ## Tier 2: CoinGecko Category Tags
///
/// Falls through to examine the `categories` slice (already lowercased by the
/// caller) for ecosystem keywords (e.g., `"ethereum"`, `"polygon"`, `"solana"`).
/// Matches the same set of 18+ platforms as Tier 1.
///
/// ## Tier 3: Well-Known Native Coin Symbols
///
/// As a final fallback, matches the coin's `.symbol` field against a hardcoded
/// list of native blockchain coins:
///
/// `BTC` → Bitcoin, `ETH` → Ethereum, `BNB` → Binance Smart Chain,
/// `ADA` → Cardano, `SOL` → Solana, `DOT` → Polkadot, `AVAX` → Avalanche,
/// `MATIC` → Polygon, `ATOM` → Cosmos, `NEAR` → Near, `TRX` → Tron,
/// `ALGO` → Algorand, `FTM` → Fantom, `XRP` → XRP Ledger, `XLM` → Stellar,
/// `XTZ` → Tezos, `EOS` → EOS, `HBAR` → Hedera, `FLOW` → Flow,
/// `ICP` → Internet Computer, `EGLD` → MultiversX
///
/// ## Returns
///
/// `Some(platform_name)` if a platform was identified, `None` otherwise.
/// Tokens on unrecognized platforms or with insufficient metadata will
/// return `None`.
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

/// Tracks processing counts across the ETL pipeline run.
///
/// Accumulated during [`execute_metadata_etl`] and logged at completion via
/// [`tracing::info`].
///
/// # Fields
///
/// - `total_processed` — Number of `crypto_metadata` rows iterated (regardless
///   of whether JSON parsing or upserts succeeded).
/// - `basic_updated` — Number of successful upserts to `crypto_overview_basic`.
/// - `social_updated` — Number of successful upserts to `crypto_social`.
/// - `technical_updated` — Number of successful upserts to `crypto_technical`.
/// - `errors` — Intended for error counting, but **not currently incremented**
///   anywhere in the pipeline. Individual processor failures are silently
///   skipped without incrementing this counter.
#[derive(Debug, Default)]
pub struct ProcessingStats {
  pub total_processed: usize,
  pub basic_updated: usize,
  pub social_updated: usize,
  pub technical_updated: usize,
  pub errors: usize,
}
