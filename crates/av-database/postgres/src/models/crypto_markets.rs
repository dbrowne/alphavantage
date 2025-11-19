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

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};

use crate::schema::crypto_markets;

// Helper struct for raw SQL queries
#[derive(QueryableByName, Debug)]
struct CountResult {
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  count: i64,
}

/// Database model for crypto_markets table
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

/// New crypto market for database insertion
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

/// Updateable fields for crypto markets
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

/// Market summary statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoMarketsSummary {
  pub total_markets: i64,
  pub active_markets: i64,
  pub unique_exchanges: i64,
  pub unique_trading_pairs: i64,
  pub last_updated: Option<DateTime<Utc>>,
}

/// Market statistics by exchange
#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeStats {
  pub exchange: String,
  pub market_count: i64,
  pub active_markets: i64,
  pub total_volume_24h: Option<BigDecimal>,
  pub average_trust_score: Option<String>,
}

/// Input data structure for creating crypto markets (replaces loader dependency)
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

impl CryptoMarket {
  /// Insert a new crypto market
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

  /// Insert multiple crypto markets in a transaction
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

  /// Upsert (insert or update) crypto markets
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

  /// Get market summary statistics
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

  /// Clean up stale market entries
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

  /// Get markets with their symbol information
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

  /// Get markets by exchange with active filter
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

  /// Get markets by symbol (sid) with active filter
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

  /// Update market status (active/inactive/stale)
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

  /// Mark markets as stale if not updated within threshold
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

  /// Get exchange statistics
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

  /// Upsert batch - alternative name for compatibility
  pub fn upsert_batch(
    conn: &mut PgConnection,
    markets: &[NewCryptoMarket],
  ) -> Result<usize, DieselError> {
    Self::upsert_markets(conn, markets).map(|v| v.len())
  }
}

// Conversion from input data to database model
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
