use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::{crypto_overview_basic, crypto_overview_metrics};
use bigdecimal::BigDecimal;

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