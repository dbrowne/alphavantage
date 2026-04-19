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

//! Diesel models for cryptocurrency exchange and trading-pair market data.
//!
//! This module tracks **where** each cryptocurrency is traded, capturing
//! per-exchange, per-trading-pair volume, liquidity, and trust metrics.
//! It complements the [`crypto`](super::crypto) module, which stores
//! fundamental *what* data (price, supply, technicals).
//!
//! # Database table
//!
//! All types map to the `crypto_markets` table, which uses a
//! **natural composite unique constraint** on `(sid, exchange, base, target)` —
//! one row per coin per exchange per trading pair.
//!
//! # Struct inventory
//!
//! | Type                   | Role                                                          |
//! |------------------------|---------------------------------------------------------------|
//! | [`CryptoMarket`]       | Queryable row from `crypto_markets`                           |
//! | [`NewCryptoMarket`]    | Insertable struct for new/upserted market records             |
//! | [`UpdateCryptoMarket`] | `AsChangeset` struct for partial field updates                 |
//! | [`CryptoMarketInput`]  | Ingestion DTO; converts to `NewCryptoMarket` via `From`       |
//! | [`CryptoMarketsSummary`] | Aggregate stats (total/active markets, exchanges, pairs)    |
//! | [`ExchangeStats`]      | Per-exchange aggregate metrics                                |
//!
//! # Key operations
//!
//! All query methods on [`CryptoMarket`] are **synchronous** (`&mut PgConnection`).
//! Major operations include:
//!
//! - **Insert / upsert:** [`insert`](CryptoMarket::insert),
//!   [`insert_batch`](CryptoMarket::insert_batch),
//!   [`upsert_markets`](CryptoMarket::upsert_markets).
//! - **Query:** [`get_by_exchange`](CryptoMarket::get_by_exchange),
//!   [`get_by_symbol`](CryptoMarket::get_by_symbol),
//!   [`get_markets_with_symbols`](CryptoMarket::get_markets_with_symbols).
//! - **Lifecycle:** [`mark_stale_markets`](CryptoMarket::mark_stale_markets),
//!   [`cleanup_stale_markets`](CryptoMarket::cleanup_stale_markets).
//! - **Analytics:** [`get_summary`](CryptoMarket::get_summary),
//!   [`get_exchange_stats`](CryptoMarket::get_exchange_stats).

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};

use crate::schema::crypto_markets;

/// Helper for extracting a single `i64` count from a raw SQL query.
///
/// Used internally by [`CryptoMarket::get_summary`] to count distinct
/// `(base, target)` pairs, which Diesel's DSL cannot express natively.
#[derive(QueryableByName, Debug)]
struct CountResult {
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  count: i64,
}

// ─── Queryable model ────────────────────────────────────────────────────────

/// A single trading-pair listing on a cryptocurrency exchange.
///
/// Maps to one row of the `crypto_markets` table. Each row represents a
/// specific `base/target` trading pair (e.g., `BTC/USDT`) on a specific
/// exchange (e.g., `"binance"`).
///
/// # Key fields
///
/// | Field               | Type                 | Description                                      |
/// |---------------------|----------------------|--------------------------------------------------|
/// | `id`                | `i32`                | Auto-increment primary key                       |
/// | `sid`               | `i64`                | Security ID (FK to `symbols`)                    |
/// | `exchange`          | `String`             | Exchange name (e.g., `"binance"`, `"coinbase"`)  |
/// | `base` / `target`   | `String`             | Trading pair (e.g., base=`"BTC"`, target=`"USDT"`) |
/// | `market_type`       | `Option<String>`     | Market type (e.g., `"spot"`, `"futures"`)        |
/// | `volume_24h`        | `Option<BigDecimal>` | 24-hour trading volume in target currency        |
/// | `volume_percentage` | `Option<BigDecimal>` | This pair's share of the coin's total volume     |
/// | `bid_ask_spread_pct`| `Option<BigDecimal>` | Bid-ask spread as a percentage                   |
/// | `liquidity_score`   | `Option<String>`     | Qualitative liquidity rating                     |
/// | `trust_score`       | `Option<String>`     | Exchange trust/reliability score                 |
/// | `is_active`         | `Option<bool>`       | Whether this market is currently trading         |
/// | `is_anomaly`        | `Option<bool>`       | Flagged for suspicious volume/price data         |
/// | `is_stale`          | `Option<bool>`       | Not updated within the freshness threshold       |
/// | `last_traded_at`    | `Option<DateTime<Utc>>` | Timestamp of the most recent trade            |
/// | `last_fetch_at`     | `Option<DateTime<Utc>>` | When this row was last refreshed from the API |
/// | `c_time`            | `DateTime<Utc>`      | Row creation timestamp                           |
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crypto_markets)]
pub struct CryptoMarket {
  pub id: i32,
  pub sid: i64,
  pub exchange: String,
  pub base: String,
  pub target: String,
  pub market_type: Option<String>,
  pub volume_24h: Option<BigDecimal>,
  pub volume_percentage: Option<BigDecimal>,
  pub bid_ask_spread_pct: Option<BigDecimal>,
  pub liquidity_score: Option<String>,
  pub is_active: Option<bool>,
  pub is_anomaly: Option<bool>,
  pub is_stale: Option<bool>,
  pub trust_score: Option<String>,
  pub last_traded_at: Option<DateTime<Utc>>,
  pub last_fetch_at: Option<DateTime<Utc>>,
  pub c_time: DateTime<Utc>,
}

// ─── Insertable model ───────────────────────────────────────────────────────

/// Insertable form of [`CryptoMarket`].
///
/// Omits the auto-increment `id` and `c_time` (database-defaulted).
/// All fields are owned; use [`CryptoMarketInput`] and its `From` impl
/// for ergonomic construction from ingestion pipeline data.
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crypto_markets)]
pub struct NewCryptoMarket {
  pub sid: i64,
  pub exchange: String,
  pub base: String,
  pub target: String,
  pub market_type: Option<String>,
  pub volume_24h: Option<BigDecimal>,
  pub volume_percentage: Option<BigDecimal>,
  pub bid_ask_spread_pct: Option<BigDecimal>,
  pub liquidity_score: Option<String>,
  pub is_active: Option<bool>,
  pub is_anomaly: Option<bool>,
  pub is_stale: Option<bool>,
  pub trust_score: Option<String>,
  pub last_traded_at: Option<DateTime<Utc>>,
  pub last_fetch_at: Option<DateTime<Utc>>,
}

// ─── Changeset model ────────────────────────────────────────────────────────

/// Partial-update changeset for [`CryptoMarket`].
///
/// Contains only the mutable metric and status fields — identity fields
/// (`id`, `sid`, `exchange`, `base`, `target`, `c_time`) are excluded.
/// Used by [`CryptoMarket::update_status`].
#[derive(AsChangeset, Debug)]
#[diesel(table_name = crypto_markets)]
pub struct UpdateCryptoMarket {
  pub volume_24h: Option<BigDecimal>,
  pub volume_percentage: Option<BigDecimal>,
  pub bid_ask_spread_pct: Option<BigDecimal>,
  pub liquidity_score: Option<String>,
  pub is_active: Option<bool>,
  pub is_anomaly: Option<bool>,
  pub is_stale: Option<bool>,
  pub trust_score: Option<String>,
  pub last_traded_at: Option<DateTime<Utc>>,
  pub last_fetch_at: Option<DateTime<Utc>>,
}

// ─── Analytics / presentation structs ────────────────────────────────────────

/// Aggregate summary of the entire `crypto_markets` table.
///
/// Returned by [`CryptoMarket::get_summary`]. Not a database-backed model.
///
/// # Fields
///
/// - `total_markets` — total row count in `crypto_markets`.
/// - `active_markets` — rows where `is_active = true`.
/// - `unique_exchanges` — `COUNT(DISTINCT exchange)`.
/// - `unique_trading_pairs` — `COUNT(DISTINCT (base, target))`.
/// - `last_updated` — `MAX(last_fetch_at)` across all rows.
#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoMarketsSummary {
  pub total_markets: i64,
  pub active_markets: i64,
  pub unique_exchanges: i64,
  pub unique_trading_pairs: i64,
  pub last_updated: Option<DateTime<Utc>>,
}

/// Per-exchange aggregate metrics.
///
/// Returned by [`CryptoMarket::get_exchange_stats`]. Not a database-backed model.
///
/// Note: `average_trust_score` is currently always `None` because trust scores
/// are stored as text and cannot be averaged natively.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeStats {
  /// Exchange name (e.g., `"binance"`).
  pub exchange: String,
  /// Total market listings on this exchange.
  pub market_count: i64,
  /// Active market listings on this exchange.
  pub active_markets: i64,
  /// Sum of `volume_24h` across active markets.
  pub total_volume_24h: Option<BigDecimal>,
  /// Average trust score (currently unimplemented — always `None`).
  pub average_trust_score: Option<String>,
}

// ─── Ingestion DTO ──────────────────────────────────────────────────────────

/// Input DTO for the data ingestion pipeline.
///
/// Provides a clean API boundary between the data loader and the database
/// layer. Convert to [`NewCryptoMarket`] via the [`From`] implementation.
///
/// Key differences from [`NewCryptoMarket`]:
/// - Boolean flags (`is_active`, `is_anomaly`, `is_stale`) are non-optional
///   `bool` rather than `Option<bool>`.
/// - `last_fetch_at` is a required `DateTime<Utc>` rather than `Option`.
///
/// The `From<CryptoMarketInput>` impl wraps booleans in `Some(…)` and
/// `last_fetch_at` in `Some(…)` to match the database schema.
#[derive(Debug, Clone)]
pub struct CryptoMarketInput {
  pub sid: i64,
  pub exchange: String,
  pub base: String,
  pub target: String,
  pub market_type: Option<String>,
  pub volume_24h: Option<BigDecimal>,
  pub volume_percentage: Option<BigDecimal>,
  pub bid_ask_spread_pct: Option<BigDecimal>,
  pub liquidity_score: Option<String>,
  pub is_active: bool,
  pub is_anomaly: bool,
  pub is_stale: bool,
  pub trust_score: Option<String>,
  pub last_traded_at: Option<DateTime<Utc>>,
  pub last_fetch_at: DateTime<Utc>,
}

/// Synchronous query and mutation methods for the `crypto_markets` table.
///
/// All methods take `&mut PgConnection` and block on the database call.
/// For async usage, wrap calls in `tokio::task::spawn_blocking` or use
/// the `diesel-async` repository layer.
impl CryptoMarket {
  /// Inserts a single market record, returning the inserted row.
  pub fn insert(
    conn: &mut PgConnection,
    new_market: &NewCryptoMarket,
  ) -> Result<Self, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    diesel::insert_into(crypto_markets)
      .values(new_market)
      .returning(CryptoMarket::as_returning())
      .get_result(conn)
  }

  /// Inserts multiple market records in a single statement, returning all
  /// inserted rows. Does **not** handle conflicts — use [`upsert_markets`](Self::upsert_markets)
  /// if duplicates are possible.
  pub fn insert_batch(
    conn: &mut PgConnection,
    new_markets: &[NewCryptoMarket],
  ) -> Result<Vec<Self>, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    diesel::insert_into(crypto_markets)
      .values(new_markets)
      .returning(CryptoMarket::as_returning())
      .get_results(conn)
  }

  /// Upserts (insert-or-update) a batch of market records.
  ///
  /// Uses `ON CONFLICT (sid, exchange, base, target) DO UPDATE` to update
  /// all metric and status fields when a record for the same trading pair
  /// already exists. Identity fields and `c_time` are preserved on conflict.
  ///
  /// Each record is upserted individually within the same connection (no
  /// explicit transaction wrapper — callers should wrap in a transaction if
  /// atomicity across the batch is required).
  ///
  /// Logs an error and returns `Err` if any single upsert fails.
  pub fn upsert_markets(
    conn: &mut PgConnection,
    markets: &[NewCryptoMarket],
  ) -> Result<Vec<Self>, DieselError> {
    use crate::schema::crypto_markets::dsl::*;
    use diesel::upsert::excluded;

    let results = markets
      .iter()
      .map(|market| {
        diesel::insert_into(crypto_markets)
          .values(market)
          .on_conflict((sid, exchange, base, target))
          .do_update()
          .set((
            volume_24h.eq(excluded(volume_24h)),
            volume_percentage.eq(excluded(volume_percentage)),
            bid_ask_spread_pct.eq(excluded(bid_ask_spread_pct)),
            liquidity_score.eq(excluded(liquidity_score)),
            is_active.eq(excluded(is_active)),
            is_anomaly.eq(excluded(is_anomaly)),
            is_stale.eq(excluded(is_stale)),
            trust_score.eq(excluded(trust_score)),
            last_traded_at.eq(excluded(last_traded_at)),
            last_fetch_at.eq(excluded(last_fetch_at)),
          ))
          .returning(CryptoMarket::as_returning())
          .get_result(conn)
      })
      .collect::<Result<Vec<_>, _>>();

    match results {
      Ok(markets) => Ok(markets),
      Err(e) => {
        log::error!("Failed to upsert markets: {}", e);
        Err(e)
      }
    }
  }

  /// Computes aggregate statistics across all crypto markets.
  ///
  /// Executes five queries: total count, active count, distinct exchange
  /// count, distinct trading-pair count (via raw SQL), and `MAX(last_fetch_at)`.
  /// Returns a [`CryptoMarketsSummary`].
  pub fn get_summary(conn: &mut PgConnection) -> Result<CryptoMarketsSummary, DieselError> {
    use crate::schema::crypto_markets::dsl::*;
    use diesel::dsl::{count, max};

    // Get total count
    let total = crypto_markets.select(count(id)).first::<i64>(conn)?;

    // Get active count - separate query to avoid unsupported count().filter() pattern
    let active =
      crypto_markets.filter(is_active.eq(Some(true))).select(count(id)).first::<i64>(conn)?;

    // Get exchange count
    let exchanges = crypto_markets
      .select(diesel::dsl::sql::<diesel::sql_types::BigInt>("COUNT(DISTINCT exchange)"))
      .first::<i64>(conn)?;

    // Get last update timestamp
    let last_update =
      crypto_markets.select(max(last_fetch_at)).first::<Option<DateTime<Utc>>>(conn)?;

    // Count unique trading pairs using raw SQL due to Diesel limitations with tuple count_distinct
    let pairs =
      diesel::sql_query("SELECT COUNT(DISTINCT (base, target)) as count FROM crypto_markets")
        .get_result::<CountResult>(conn)?
        .count;

    Ok(CryptoMarketsSummary {
      total_markets: total,
      active_markets: active,
      unique_exchanges: exchanges,
      unique_trading_pairs: pairs,
      last_updated: last_update,
    })
  }

  /// Deletes market rows that are both marked stale (`is_stale = true`) and
  /// have not been fetched within `threshold_hours`.
  ///
  /// Returns the number of rows deleted. Uses a PostgreSQL `interval` cast
  /// for the time comparison.
  pub fn cleanup_stale_markets(
    conn: &mut PgConnection,
    threshold_hours: i64,
  ) -> Result<usize, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    // Create the threshold time using PostgreSQL-compatible interval syntax
    // Cast to nullable timestamptz to match the column type
    let threshold_time =
      diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>>(&format!(
        "(now() - interval '{} hours')::timestamptz",
        threshold_hours
      ));

    diesel::delete(
      crypto_markets.filter(is_stale.eq(Some(true)).and(last_fetch_at.lt(threshold_time))),
    )
    .execute(conn)
  }

  /// Returns active markets joined with their symbol name and ticker.
  ///
  /// Performs an inner join `crypto_markets → symbols` filtered to
  /// `is_active = true`, ordered by `volume_24h DESC`. An optional `limit`
  /// caps the result count.
  ///
  /// Returns `Vec<(CryptoMarket, symbol_ticker, symbol_name)>`.
  pub fn get_markets_with_symbols(
    conn: &mut PgConnection,
    limit: Option<i64>,
  ) -> Result<Vec<(Self, String, String)>, DieselError> {
    use crate::schema::{crypto_markets, symbols};

    let mut query = crypto_markets::table
      .inner_join(symbols::table.on(symbols::sid.eq(crypto_markets::sid)))
      .filter(crypto_markets::is_active.eq(Some(true)))
      .select((CryptoMarket::as_select(), symbols::symbol, symbols::name))
      .order(crypto_markets::volume_24h.desc().nulls_last())
      .into_boxed();

    if let Some(limit_val) = limit {
      query = query.limit(limit_val);
    }

    query.load::<(Self, String, String)>(conn)
  }

  /// Returns markets listed on a specific exchange, ordered by volume.
  ///
  /// When `active_only` is `true`, only rows with `is_active = true` are
  /// returned.
  pub fn get_by_exchange(
    conn: &mut PgConnection,
    exchange_name: &str,
    active_only: bool,
  ) -> Result<Vec<Self>, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    let mut query = crypto_markets.filter(exchange.eq(exchange_name)).into_boxed();

    if active_only {
      query = query.filter(is_active.eq(Some(true)));
    }

    query.order(volume_24h.desc().nulls_last()).load::<Self>(conn)
  }

  /// Returns all markets for a given security ID (`sid`), ordered by volume.
  ///
  /// When `active_only` is `true`, only rows with `is_active = true` are
  /// returned.
  pub fn get_by_symbol(
    conn: &mut PgConnection,
    symbol_id: i64,
    active_only: bool,
  ) -> Result<Vec<Self>, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    let mut query = crypto_markets.filter(sid.eq(symbol_id)).into_boxed();

    if active_only {
      query = query.filter(is_active.eq(Some(true)));
    }

    query.order(volume_24h.desc().nulls_last()).load::<Self>(conn)
  }

  /// Applies a partial update to a single market row by its `id`.
  ///
  /// Only the fields present in the [`UpdateCryptoMarket`] changeset are
  /// written; all other columns are left unchanged. Returns the updated row.
  pub fn update_status(
    conn: &mut PgConnection,
    market_id: i32,
    status_update: &UpdateCryptoMarket,
  ) -> Result<Self, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    diesel::update(crypto_markets.find(market_id))
      .set(status_update)
      .returning(CryptoMarket::as_returning())
      .get_result(conn)
  }

  /// Marks markets as stale (`is_stale = true`) if their `last_fetch_at`
  /// is older than `threshold_hours` ago and they are not already stale.
  ///
  /// Returns the number of rows updated. Intended to be run periodically
  /// (e.g., via a cron job) before [`cleanup_stale_markets`](Self::cleanup_stale_markets).
  pub fn mark_stale_markets(
    conn: &mut PgConnection,
    threshold_hours: i64,
  ) -> Result<usize, DieselError> {
    use crate::schema::crypto_markets::dsl::*;

    let threshold_time =
      diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>>(&format!(
        "(now() - interval '{} hours')::timestamptz",
        threshold_hours
      ));

    diesel::update(crypto_markets.filter(
      last_fetch_at.lt(threshold_time).and(is_stale.eq(Some(false)).or(is_stale.is_null())),
    ))
    .set(is_stale.eq(Some(true)))
    .execute(conn)
  }

  /// Computes per-exchange aggregate metrics.
  ///
  /// For each distinct exchange, queries total market count, active market
  /// count, and sum of 24h volume. Returns a `Vec<ExchangeStats>`.
  ///
  /// Note: `average_trust_score` is always `None` in the current
  /// implementation because trust scores are stored as text.
  pub fn get_exchange_stats(conn: &mut PgConnection) -> Result<Vec<ExchangeStats>, DieselError> {
    use crate::schema::crypto_markets::dsl::*;
    use diesel::dsl::{count, sum};

    // Get exchange statistics using separate queries to avoid complex aggregations
    let exchanges_data: Vec<String> =
      crypto_markets.select(exchange).distinct().load::<String>(conn)?;

    let mut stats = Vec::new();

    for exch in exchanges_data {
      let total_markets =
        crypto_markets.filter(exchange.eq(&exch)).select(count(id)).first::<i64>(conn)?;

      let active_markets = crypto_markets
        .filter(exchange.eq(&exch))
        .filter(is_active.eq(Some(true)))
        .select(count(id))
        .first::<i64>(conn)?;

      let total_volume = crypto_markets
        .filter(exchange.eq(&exch))
        .filter(is_active.eq(Some(true)))
        .select(sum(volume_24h.nullable()))
        .first::<Option<BigDecimal>>(conn)?;

      stats.push(ExchangeStats {
        exchange: exch,
        market_count: total_markets,
        active_markets,
        total_volume_24h: total_volume,
        average_trust_score: None, // Would need separate calculation for text average
      });
    }

    Ok(stats)
  }

  /// Convenience alias for [`upsert_markets`](Self::upsert_markets) that returns
  /// only the count of upserted rows instead of the full row data.
  ///
  /// Retained for backward compatibility with callers that only need the count.
  pub fn upsert_batch(
    conn: &mut PgConnection,
    markets: &[NewCryptoMarket],
  ) -> Result<usize, DieselError> {
    Self::upsert_markets(conn, markets).map(|v| v.len())
  }
}

/// Converts an ingestion-layer [`CryptoMarketInput`] into a database-layer
/// [`NewCryptoMarket`].
///
/// Wraps the non-optional boolean flags (`is_active`, `is_anomaly`, `is_stale`)
/// and `last_fetch_at` in `Some(…)` to match the `Option` columns in the
/// database schema.
impl From<CryptoMarketInput> for NewCryptoMarket {
  fn from(market: CryptoMarketInput) -> Self {
    Self {
      sid: market.sid,
      exchange: market.exchange,
      base: market.base,
      target: market.target,
      market_type: market.market_type,
      volume_24h: market.volume_24h,
      volume_percentage: market.volume_percentage,
      bid_ask_spread_pct: market.bid_ask_spread_pct,
      liquidity_score: market.liquidity_score,
      is_active: Some(market.is_active),
      is_anomaly: Some(market.is_anomaly),
      is_stale: Some(market.is_stale),
      trust_score: market.trust_score,
      last_traded_at: market.last_traded_at,
      last_fetch_at: Some(market.last_fetch_at),
    }
  }
}
