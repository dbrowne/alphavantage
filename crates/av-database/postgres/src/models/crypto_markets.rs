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

        let results = markets.iter().map(|market| {
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
        }).collect::<Result<Vec<_>, _>>();

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
        use diesel::dsl::{count, count_distinct, max};

        let (total, active, exchanges, pairs, last_update) = crypto_markets
            .select((
                count(id),
                count(id.nullable()).filter(is_active.eq(Some(true))),
                count_distinct(exchange),
                count_distinct((base, target)),
                max(last_fetch_at),
            ))
            .first::<(i64, i64, i64, i64, Option<DateTime<Utc>>)>(conn)?;

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
        use diesel::dsl::now;

        let threshold_time = now - diesel::dsl::sql::<diesel::sql_types::Interval>(
            &format!("interval '{} hours'", threshold_hours)
        );

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