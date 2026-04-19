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

//! Diesel models for cryptocurrency fundamental data.
//!
//! This module contains database models for storing and querying cryptocurrency
//! information across five normalized tables:
//!
//! | Table                    | Model                    | Purpose                                        |
//! |--------------------------|--------------------------|------------------------------------------------|
//! | `crypto_overview_basic`  | [`CryptoOverviewBasic`]  | Core identity and market data (price, cap, supply) |
//! | `crypto_overview_metrics`| [`CryptoOverviewMetrics`]| Price change percentages, ATH/ATL, ROI           |
//! | `crypto_technical`       | [`CryptoTechnical`]      | Blockchain parameters and GitHub activity        |
//! | `crypto_social`          | [`CryptoSocial`]         | Community links, follower counts, scores         |
//! | `crypto_api_map`         | [`CryptoApiMap`]         | External API identifier mapping (CoinGecko, CoinPaprika) |
//!
//! # Architecture
//!
//! The data is split across multiple tables to separate concerns and allow
//! independent update cadences — e.g., social metrics may be refreshed less
//! frequently than price data. All tables share the same `sid` (security ID)
//! primary key, which references the `symbols` table.
//!
//! # Struct conventions
//!
//! - **Query types** (e.g., `CryptoOverviewBasic`) derive `Queryable`, `Selectable`,
//!   `Identifiable`, `Serialize`, `Deserialize` for ORM reads and JSON serialization.
//! - **Insertable types** come in two flavors:
//!   - *Borrowed* (e.g., `NewCryptoOverviewBasic<'a>`) — borrows fields by reference
//!     for efficient batch inserts.
//!   - *Owned* (e.g., `NewCryptoTechnical`) — owns all data for use when the struct
//!     must outlive the source.
//! - [`CryptoOverviewFull`] is a convenience wrapper that pairs `Basic` + `Metrics`
//!   for presentation without a database table of its own.
//!
//! # Helper functions
//!
//! Two async helper functions are provided for external API discovery:
//! - [`discover_coingecko_id`] — resolves a ticker symbol to a CoinGecko coin ID.
//! - [`discover_coinpaprika_id`] — resolves a ticker symbol to a CoinPaprika coin ID.
//!
//! These are used by the data ingestion pipeline to populate the `crypto_api_map`
//! table.

use crate::schema::{
  crypto_api_map, crypto_overview_basic, crypto_overview_metrics, crypto_social, crypto_technical,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// ─── Crypto Overview (Basic) ────────────────────────────────────────────────

/// Core cryptocurrency overview data: identity, market capitalization, price,
/// and supply information.
///
/// Maps to the `crypto_overview_basic` table. Primary key is `sid` (security ID),
/// linking back to the `symbols` table.
///
/// # Key fields
///
/// | Field                    | Type                 | Description                                    |
/// |--------------------------|----------------------|------------------------------------------------|
/// | `sid`                    | `i64`                | Security ID (FK to `symbols`)                  |
/// | `symbol` / `name`        | `String`             | Ticker symbol and full project name            |
/// | `slug`                   | `Option<String>`     | URL-friendly identifier (e.g., `"bitcoin"`)    |
/// | `market_cap_rank`        | `Option<i32>`        | Global ranking by market capitalization         |
/// | `market_cap`             | `Option<i64>`        | Total market cap in USD                        |
/// | `fully_diluted_valuation`| `Option<i64>`        | Market cap assuming max supply is circulating  |
/// | `current_price`          | `Option<BigDecimal>` | Latest price in USD                            |
/// | `circulating_supply`     | `Option<BigDecimal>` | Coins currently in circulation                 |
/// | `total_supply`           | `Option<BigDecimal>` | Total coins that exist (incl. locked/reserved) |
/// | `max_supply`             | `Option<BigDecimal>` | Hard cap on total coins (e.g., 21M for BTC)    |
/// | `volume_24h`             | `Option<i64>`        | 24-hour trading volume in USD                  |
/// | `c_time` / `m_time`      | `DateTime<Utc>`      | Row creation and last-modification timestamps  |
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

// ─── Crypto Overview (Metrics) ──────────────────────────────────────────────

/// Price-change performance metrics, all-time highs/lows, and ROI data.
///
/// Maps to the `crypto_overview_metrics` table. Keyed by `sid` and typically
/// joined with [`CryptoOverviewBasic`] via [`CryptoOverviewFull`].
///
/// # Key fields
///
/// | Field                    | Type                 | Description                                     |
/// |--------------------------|----------------------|-------------------------------------------------|
/// | `price_change_24h`       | `Option<BigDecimal>` | Absolute price change in the last 24 hours      |
/// | `price_change_pct_*`     | `Option<BigDecimal>` | Percentage change over 24h, 7d, 14d, 30d, 60d, 200d, 1y |
/// | `ath` / `ath_date`       | `Option<BigDecimal>` / `Option<DateTime<Utc>>` | All-time high price and when it occurred |
/// | `ath_change_percentage`  | `Option<BigDecimal>` | Percentage change from ATH to current price     |
/// | `atl` / `atl_date`       | `Option<BigDecimal>` / `Option<DateTime<Utc>>` | All-time low price and when it occurred  |
/// | `atl_change_percentage`  | `Option<BigDecimal>` | Percentage change from ATL to current price     |
/// | `roi_times` / `roi_percentage` | `Option<BigDecimal>` | Return on investment (multiple and percent) |
/// | `roi_currency`           | `Option<String>`     | Currency the ROI is denominated in              |
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

/// Combined view joining [`CryptoOverviewBasic`] and [`CryptoOverviewMetrics`].
///
/// This is a **presentation-only struct** — it does not map to a database table.
/// Construct it by querying both tables separately and pairing the results.
/// `metrics` is `Option` because a basic record may exist before metrics are
/// first ingested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoOverviewFull {
  /// Core identity and market data.
  pub basic: CryptoOverviewBasic,
  /// Performance metrics (may be absent for newly-added coins).
  pub metrics: Option<CryptoOverviewMetrics>,
}

// ─── Insertable structs for Overview tables ─────────────────────────────────

/// Insertable (borrowed) form of [`CryptoOverviewBasic`].
///
/// All string and `BigDecimal` fields are borrowed references (`&'a`) for
/// zero-copy batch inserts. Timestamps (`c_time`, `m_time`) are omitted and
/// defaulted by the database.
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

/// Insertable (borrowed) form of [`CryptoOverviewMetrics`].
///
/// Timestamps are omitted; the database defaults them on insert.
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

// ─── Crypto Technical ───────────────────────────────────────────────────────

/// Blockchain technical parameters, GitHub development activity, and
/// categorical classification flags.
///
/// Maps to the `crypto_technical` table. This data changes infrequently
/// (primarily GitHub stats and block height) and is typically refreshed on
/// a daily or weekly cadence.
///
/// # Field groups
///
/// - **Blockchain parameters:** `blockchain_platform`, `token_standard`,
///   `consensus_mechanism`, `hashing_algorithm`, `block_time_minutes`,
///   `block_reward`, `block_height`, `hash_rate`, `difficulty`.
/// - **GitHub activity:** `github_forks`, `github_stars`, `github_subscribers`,
///   `github_total_issues`, `github_closed_issues`, `github_pull_requests`,
///   `github_contributors`, `github_commits_4_weeks`.
/// - **Category flags:** `is_defi`, `is_stablecoin`, `is_nft_platform`,
///   `is_exchange_token`, `is_gaming`, `is_metaverse`, `is_privacy_coin`,
///   `is_layer2`, `is_wrapped`.
/// - **ICO / genesis info:** `genesis_date`, `ico_price`, `ico_date`.
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

/// Insertable (owned) form of [`CryptoTechnical`].
///
/// Unlike the borrowed `NewCrypto*<'a>` structs, this variant owns all data
/// and includes explicit `c_time` / `m_time` fields for caller-controlled
/// timestamps. The `is_*` category flags are non-optional `bool` (defaulting
/// to `false` on insert).
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

// ─── Crypto Social ──────────────────────────────────────────────────────────

/// Community presence, social media metrics, and composite scoring data.
///
/// Maps to the `crypto_social` table. Captures links to official project
/// channels and follower/subscriber counts from major platforms.
///
/// # Field groups
///
/// - **URLs:** `website_url`, `whitepaper_url`, `github_url`, `twitter_handle`,
///   `telegram_url`, `discord_url`, `reddit_url`, `facebook_url`.
/// - **Follower counts:** `twitter_followers`, `telegram_members`,
///   `discord_members`, `reddit_subscribers`, `facebook_likes`.
/// - **Composite scores:** `coingecko_score`, `developer_score`,
///   `community_score`, `liquidity_score`, `public_interest_score`.
/// - **Sentiment:** `sentiment_votes_up_pct`, `sentiment_votes_down_pct`.
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

/// Insertable (owned) form of [`CryptoSocial`].
///
/// Includes caller-controlled `c_time` / `m_time` timestamps.
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

// ─── Crypto API Mapping ─────────────────────────────────────────────────────

/// Maps an internal security ID (`sid`) to an external API's identifier.
///
/// Maps to the `crypto_api_map` table with a **composite primary key**
/// `(sid, api_source)`, allowing one mapping per external API per coin.
///
/// Currently supported API sources:
/// - `"CoinGecko"` — uses the CoinGecko coin list API.
/// - `"CoinPaprika"` — uses the CoinPaprika coins API.
///
/// # Key fields
///
/// | Field           | Type                 | Description                                |
/// |-----------------|----------------------|--------------------------------------------|
/// | `sid`           | `i64`                | Internal security ID (FK to `symbols`)     |
/// | `api_source`    | `String`             | External API name (e.g., `"CoinGecko"`)    |
/// | `api_id`        | `String`             | The ID used by the external API            |
/// | `api_slug`      | `Option<String>`     | URL slug on the external platform          |
/// | `api_symbol`    | `Option<String>`     | Symbol as represented by the external API  |
/// | `rank`          | `Option<i32>`        | Market-cap rank from the external API      |
/// | `is_active`     | `Option<bool>`       | Whether this mapping is currently valid    |
/// | `last_verified` | `Option<DateTime<Utc>>` | When the mapping was last confirmed     |
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

/// Insertable (owned) form of [`CryptoApiMap`].
///
/// Includes caller-controlled `c_time` / `m_time` and `last_verified` fields.
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

/// Synchronous query methods for [`CryptoApiMap`].
///
/// All methods take a `&mut PgConnection` and execute synchronously. These are
/// intended for CLI tools and migration scripts; for async service code, use
/// the corresponding `diesel-async` repository layer instead.
impl CryptoApiMap {
  /// Returns the external API ID for a given security and API source.
  ///
  /// Filters to active mappings only (`is_active = true`). Returns `None`
  /// if no active mapping exists for the `(sid, api_source)` pair.
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

  /// Returns all active `(api_source, api_id)` pairs for a given security.
  ///
  /// Useful for iterating over all external APIs that have a mapping for
  /// a particular coin.
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

  /// Inserts a new API mapping or updates an existing one (upsert).
  ///
  /// Uses Diesel's `on_conflict(...).do_update()` on the `(sid, api_source)`
  /// composite key. On conflict, all fields except `sid`, `api_source`, and
  /// `c_time` are updated, and `m_time` is set to `Utc::now()`.
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

  /// Returns actively-traded crypto symbols that lack an API mapping for
  /// the given source.
  ///
  /// Performs a three-table join: `symbols` → `crypto_markets` (inner, to
  /// confirm the coin is actively traded) → `crypto_api_map` (left, to
  /// find gaps). Only rows where the left join produces `NULL` (i.e., no
  /// mapping exists) are returned.
  ///
  /// Returns `Vec<(sid, symbol, name)>`.
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

  /// Looks up an active mapping by ticker symbol string and API source.
  ///
  /// Joins `crypto_api_map` → `symbols` to resolve the ticker to an `sid`,
  /// then filters by `api_source` and `is_active`. Returns `None` if no
  /// active mapping is found.
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

  /// Returns active mappings whose `last_verified` timestamp is older than
  /// `days_threshold` days ago (or is `NULL`).
  ///
  /// Use this to build a re-verification queue that keeps external API
  /// mappings fresh.
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

  /// Returns all actively-traded crypto symbols alongside their API mapping
  /// (if one exists) for a given source.
  ///
  /// Returns `Vec<(sid, symbol, name, Option<api_id>)>`. Rows where the
  /// `Option<api_id>` is `None` represent coins that still need mapping.
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

  /// Builds an aggregate [`CryptoSummary`] with counts of total crypto symbols,
  /// actively-traded coins, and mappings per external API source.
  ///
  /// Executes four `COUNT` queries against `symbols`, `crypto_markets`, and
  /// `crypto_api_map`.
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

// ─── Summary ────────────────────────────────────────────────────────────────

/// Aggregate statistics for the crypto API mapping pipeline.
///
/// Returned by [`CryptoApiMap::get_crypto_summary`]. This is a
/// presentation-only struct — it does not map to a database table.
///
/// # Fields
///
/// - `total_cryptos` — number of symbols with `sec_type = "Cryptocurrency"`.
/// - `active_cryptos` — subset of `total_cryptos` that have at least one
///   active market in the `crypto_markets` table.
/// - `mapped_coingecko` — active CoinGecko mappings in `crypto_api_map`.
/// - `mapped_coinpaprika` — active CoinPaprika mappings in `crypto_api_map`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoSummary {
  /// Total cryptocurrency symbols in the `symbols` table.
  pub total_cryptos: i64,
  /// Cryptocurrencies with at least one active market.
  pub active_cryptos: i64,
  /// Active CoinGecko API mappings.
  pub mapped_coingecko: i64,
  /// Active CoinPaprika API mappings.
  pub mapped_coinpaprika: i64,
}

// ─── External API discovery helpers ──────────────────────────────────────────

/// Resolves a cryptocurrency ticker symbol to its CoinGecko coin ID.
///
/// Fetches the full CoinGecko coin list (`/api/v3/coins/list`) and performs
/// a case-insensitive exact match on the `symbol` field. Returns the
/// CoinGecko `id` string (e.g., `"bitcoin"` for `BTC`).
///
/// # Arguments
///
/// - `client` — a reusable `reqwest::Client` (connection pooling recommended).
/// - `symbol` — the ticker to search for (e.g., `"BTC"`). Compared lowercase.
/// - `api_key` — optional CoinGecko Pro API key. If `None`, the free
///   (rate-limited) endpoint is used.
///
/// # Errors
///
/// Returns `Err` on HTTP 429 (rate limit) or any non-success status code.
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

/// Resolves a cryptocurrency ticker symbol to its CoinPaprika coin ID.
///
/// Fetches the full CoinPaprika coin list (`/v1/coins`) and performs a
/// case-insensitive exact match on the `symbol` field. Returns the
/// CoinPaprika `id` string (e.g., `"btc-bitcoin"` for `BTC`).
///
/// CoinPaprika's public API does **not** require an API key but is
/// rate-limited. Returns `Err` on HTTP 429.
///
/// # Arguments
///
/// - `client` — a reusable `reqwest::Client`.
/// - `symbol` — the ticker to search for (e.g., `"BTC"`). Compared uppercase.
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
