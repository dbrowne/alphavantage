use diesel::prelude::*;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

use crate::schema::crypto_markets;

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

    /// Upsert crypto market (insert or update on conflict)
    pub fn upsert(
        conn: &mut PgConnection,
        new_market: &NewCryptoMarket,
    ) -> Result<Self, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        diesel::insert_into(crypto_markets)
            .values(new_market)
            .on_conflict((sid, exchange, base, target))
            .do_update()
            .set((
                volume_24h.eq(&new_market.volume_24h),
                volume_percentage.eq(&new_market.volume_percentage),
                bid_ask_spread_pct.eq(&new_market.bid_ask_spread_pct),
                liquidity_score.eq(&new_market.liquidity_score),
                is_active.eq(&new_market.is_active),
                is_anomaly.eq(&new_market.is_anomaly),
                is_stale.eq(&new_market.is_stale),
                trust_score.eq(&new_market.trust_score),
                last_traded_at.eq(&new_market.last_traded_at),
                last_fetch_at.eq(&new_market.last_fetch_at),
            ))
            .returning(CryptoMarket::as_returning())
            .get_result(conn)
    }

    /// Upsert multiple crypto markets
    pub fn upsert_batch(
        conn: &mut PgConnection,
        new_markets: &[NewCryptoMarket],
    ) -> Result<usize, DieselError> {
        conn.transaction(|conn| {
            let mut count = 0;
            for market in new_markets {
                match Self::upsert(conn, market) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        log::error!("Failed to upsert market for sid {}: {}", market.sid, e);
                        return Err(e);
                    }
                }
            }
            Ok(count)
        })
    }

    /// Find markets by symbol ID
    pub fn find_by_sid(
        conn: &mut PgConnection,
        symbol_id: i64,
    ) -> Result<Vec<Self>, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        crypto_markets
            .filter(sid.eq(symbol_id))
            .order(volume_24h.desc().nulls_last())
            .load::<Self>(conn)
    }

    /// Find active markets by symbol ID
    pub fn find_active_by_sid(
        conn: &mut PgConnection,
        symbol_id: i64,
    ) -> Result<Vec<Self>, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        crypto_markets
            .filter(sid.eq(symbol_id))
            .filter(is_active.eq(Some(true)))
            .filter(is_stale.eq(Some(false)).or(is_stale.is_null()))
            .order(volume_24h.desc().nulls_last())
            .load::<Self>(conn)
    }

    /// Find markets by exchange
    pub fn find_by_exchange(
        conn: &mut PgConnection,
        exchange_name: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        let mut query = crypto_markets
            .filter(exchange.eq(exchange_name))
            .filter(is_active.eq(Some(true)))
            .order(volume_24h.desc().nulls_last())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<Self>(conn)
    }

    /// Find top markets by volume
    pub fn find_top_by_volume(
        conn: &mut PgConnection,
        limit: i64,
    ) -> Result<Vec<Self>, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        crypto_markets
            .filter(is_active.eq(Some(true)))
            .filter(volume_24h.is_not_null())
            .order(volume_24h.desc())
            .limit(limit)
            .load::<Self>(conn)
    }

    /// Update market activity status
    pub fn update_activity_status(
        conn: &mut PgConnection,
        market_id: i32,
        active: bool,
        stale: bool,
    ) -> Result<Self, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        diesel::update(crypto_markets.filter(id.eq(market_id)))
            .set((
                is_active.eq(Some(active)),
                is_stale.eq(Some(stale)),
                last_fetch_at.eq(Some(Utc::now())),
            ))
            .returning(CryptoMarket::as_returning())
            .get_result(conn)
    }

    /// Mark stale markets (not updated recently)
    pub fn mark_stale_markets(
        conn: &mut PgConnection,
        hours_threshold: i32,
    ) -> Result<usize, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        let threshold_time = Utc::now() - chrono::Duration::hours(hours_threshold as i64);

        diesel::update(
            crypto_markets.filter(
                last_fetch_at.lt(threshold_time)
                    .or(last_fetch_at.is_null())
            )
        )
            .set(is_stale.eq(Some(true)))
            .execute(conn)
    }

    /// Get crypto markets summary
    pub fn get_summary(
        conn: &mut PgConnection,
    ) -> Result<CryptoMarketsSummary, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        let total_markets: i64 = crypto_markets.count().get_result(conn)?;

        let active_markets: i64 = crypto_markets
            .filter(is_active.eq(Some(true)))
            .count()
            .get_result(conn)?;

        // Get unique exchanges count
        let unique_exchanges: i64 = crypto_markets
            .select(exchange)
            .distinct()
            .load::<String>(conn)?
            .len() as i64;

        // Get unique trading pairs count
        let trading_pairs = crypto_markets
            .select((base, target))
            .distinct()
            .load::<(String, String)>(conn)?;
        let unique_trading_pairs = trading_pairs.len() as i64;

        // Get last updated timestamp
        let last_updated: Option<DateTime<Utc>> = crypto_markets
            .select(last_fetch_at)
            .filter(last_fetch_at.is_not_null())
            .order(last_fetch_at.desc())
            .first::<Option<DateTime<Utc>>>(conn)
            .optional()?
            .flatten();

        Ok(CryptoMarketsSummary {
            total_markets,
            active_markets,
            unique_exchanges,
            unique_trading_pairs,
            last_updated,
        })
    }

    /// Get exchange statistics
    pub fn get_exchange_stats(
        conn: &mut PgConnection,
        limit: Option<i64>,
    ) -> Result<Vec<ExchangeStats>, DieselError> {
        use crate::schema::crypto_markets::dsl::*;
        use diesel::dsl::{count, sum};

        let mut query = crypto_markets
            .group_by(exchange)
            .select((
                exchange,
                count(id),
                count(id.nullable()).filter(is_active.eq(Some(true))),
                sum(volume_24h),
            ))
            .order(count(id).desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        let results: Vec<(String, i64, i64, Option<BigDecimal>)> = query.load(conn)?;

        Ok(results
            .into_iter()
            .map(|(exch, total, active, vol)| ExchangeStats {
                exchange: exch,
                market_count: total,
                active_markets: active,
                total_volume_24h: vol,
                average_trust_score: None, // Could be calculated separately
            })
            .collect())
    }

    /// Delete markets for a specific symbol
    pub fn delete_by_sid(
        conn: &mut PgConnection,
        symbol_id: i64,
    ) -> Result<usize, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        diesel::delete(crypto_markets.filter(sid.eq(symbol_id)))
            .execute(conn)
    }

    /// Clean up old or inactive markets
    pub fn cleanup_old_markets(
        conn: &mut PgConnection,
        days_threshold: i32,
    ) -> Result<usize, DieselError> {
        use crate::schema::crypto_markets::dsl::*;

        let threshold_time = Utc::now() - chrono::Duration::days(days_threshold as i64);

        diesel::delete(
            crypto_markets.filter(
                is_stale.eq(Some(true))
                    .and(last_fetch_at.lt(threshold_time))
            )
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
            .select((
                CryptoMarket::as_select(),
                symbols::symbol,
                symbols::name,
            ))
            .order(crypto_markets::volume_24h.desc().nulls_last())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<(Self, String, String)>(conn)
    }
}

// Conversion from loader types to database types
impl From<crate::loaders::crypto::markets_loader::CryptoMarketData> for NewCryptoMarket {
    fn from(market: crate::loaders::crypto::markets_loader::CryptoMarketData) -> Self {
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