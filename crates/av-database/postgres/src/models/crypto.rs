use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use bigdecimal::BigDecimal;
use crate::schema::{crypto_overview_basic, crypto_overview_metrics, crypto_technical, crypto_social};
use chrono::{DateTime, NaiveDate, Utc};

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

#[derive(Insertable, Debug)]
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

#[derive(Insertable, Debug)]
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