use crate::{
    LoaderError, LoaderResult,
};

use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use bigdecimal::BigDecimal;
use rust_decimal::Decimal;

// Define the struct locally instead of importing to avoid conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForSocial {
    pub sid: i64,
    pub symbol: String,
    pub name: String,
    pub coingecko_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessedSocialData {
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
    pub coingecko_score: Option<Decimal>,
    pub developer_score: Option<Decimal>,
    pub community_score: Option<Decimal>,
    pub liquidity_score: Option<Decimal>,
    pub public_interest_score: Option<Decimal>,
    pub sentiment_votes_up_pct: Option<Decimal>,
    pub sentiment_votes_down_pct: Option<Decimal>,
}

pub struct CryptoSocialConfig {
    pub coingecko_api_key: Option<String>,
    pub github_token: Option<String>,
    pub skip_github: bool,
    pub delay_ms: u64,
    pub batch_size: usize,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

pub struct CryptoSocialInput {
    pub symbols: Option<Vec<CryptoSymbolForSocial>>,
    pub update_existing: bool,
}

#[allow(dead_code)]
pub struct CryptoSocialLoader {   //todo:: Refacor
    config: CryptoSocialConfig
}

impl CryptoSocialLoader {
    pub fn new(config: CryptoSocialConfig) -> Self {
        Self { config }
    }

    pub async fn load_data(
        &self,
        input: &CryptoSocialInput,
        _context: &crate::LoaderContext,
    ) -> LoaderResult<Vec<ProcessedSocialData>> {
        // Placeholder implementation - you'll need to implement the actual data loading logic
        let symbols = input.symbols.as_ref().ok_or_else(|| {
            LoaderError::ApiError("No symbols provided".to_string())
        })?;

        let mut results = Vec::new();

        for symbol in symbols {
            // Placeholder - implement actual API calls to fetch social data
            let social_data = ProcessedSocialData {
                sid: symbol.sid,
                website_url: None,
                whitepaper_url: None,
                github_url: None,
                twitter_handle: None,
                twitter_followers: None,
                telegram_url: None,
                telegram_members: None,
                discord_url: None,
                discord_members: None,
                reddit_url: None,
                reddit_subscribers: None,
                facebook_url: None,
                facebook_likes: None,
                coingecko_score: None,
                developer_score: None,
                community_score: None,
                liquidity_score: None,
                public_interest_score: None,
                sentiment_votes_up_pct: None,
                sentiment_votes_down_pct: None,
            };

            results.push(social_data);
        }

        Ok(results)
    }

    // Helper function to convert Decimal to BigDecimal
    fn decimal_to_bigdecimal(decimal: Option<Decimal>) -> Option<BigDecimal> {
        decimal.map(|d| {
            // Convert via string to avoid precision issues
            BigDecimal::parse_bytes(d.to_string().as_bytes(), 10)
                .unwrap_or_else(|| BigDecimal::from(0))
        })
    }

    // Make this method public and fix type conversion issues
    pub async fn save_social_data(
        &self,
        conn: &mut PgConnection,
        social_data: &[ProcessedSocialData],
        update_existing: bool,
    ) -> LoaderResult<(usize, usize)> {
        use av_database_postgres::models::crypto::NewCryptoSocial;
        use av_database_postgres::schema::crypto_social;
        use chrono::Utc;

        let mut inserted = 0;
        let mut updated = 0;

        for data in social_data {
            let new_social = NewCryptoSocial {
                sid: data.sid,
                website_url: data.website_url.clone(),
                whitepaper_url: data.whitepaper_url.clone(),
                github_url: data.github_url.clone(),
                twitter_handle: data.twitter_handle.clone(),
                twitter_followers: data.twitter_followers,
                telegram_url: data.telegram_url.clone(),
                telegram_members: data.telegram_members,
                discord_url: data.discord_url.clone(),
                discord_members: data.discord_members,
                reddit_url: data.reddit_url.clone(),
                reddit_subscribers: data.reddit_subscribers,
                facebook_url: data.facebook_url.clone(),
                facebook_likes: data.facebook_likes,
                // Convert Decimal to BigDecimal
                coingecko_score: Self::decimal_to_bigdecimal(data.coingecko_score),
                developer_score: Self::decimal_to_bigdecimal(data.developer_score),
                community_score: Self::decimal_to_bigdecimal(data.community_score),
                liquidity_score: Self::decimal_to_bigdecimal(data.liquidity_score),
                public_interest_score: Self::decimal_to_bigdecimal(data.public_interest_score),
                sentiment_votes_up_pct: Self::decimal_to_bigdecimal(data.sentiment_votes_up_pct),
                sentiment_votes_down_pct: Self::decimal_to_bigdecimal(data.sentiment_votes_down_pct),
                c_time: Utc::now(),
                m_time: Utc::now(),
            };

            if update_existing {
                // Try update first, then insert if no rows affected
                use crypto_social::dsl as social_dsl;

                let rows_affected = diesel::update(crypto_social::table.filter(social_dsl::sid.eq(data.sid)))
                    .set((
                        social_dsl::website_url.eq(&new_social.website_url),
                        social_dsl::whitepaper_url.eq(&new_social.whitepaper_url),
                        social_dsl::github_url.eq(&new_social.github_url),
                        social_dsl::twitter_handle.eq(&new_social.twitter_handle),
                        social_dsl::twitter_followers.eq(&new_social.twitter_followers),
                        social_dsl::telegram_url.eq(&new_social.telegram_url),
                        social_dsl::telegram_members.eq(&new_social.telegram_members),
                        social_dsl::discord_url.eq(&new_social.discord_url),
                        social_dsl::discord_members.eq(&new_social.discord_members),
                        social_dsl::reddit_url.eq(&new_social.reddit_url),
                        social_dsl::reddit_subscribers.eq(&new_social.reddit_subscribers),
                        social_dsl::facebook_url.eq(&new_social.facebook_url),
                        social_dsl::facebook_likes.eq(&new_social.facebook_likes),
                        social_dsl::coingecko_score.eq(&new_social.coingecko_score),
                        social_dsl::developer_score.eq(&new_social.developer_score),
                        social_dsl::community_score.eq(&new_social.community_score),
                        social_dsl::liquidity_score.eq(&new_social.liquidity_score),
                        social_dsl::public_interest_score.eq(&new_social.public_interest_score),
                        social_dsl::sentiment_votes_up_pct.eq(&new_social.sentiment_votes_up_pct),
                        social_dsl::sentiment_votes_down_pct.eq(&new_social.sentiment_votes_down_pct),
                        social_dsl::m_time.eq(&new_social.m_time),
                    ))
                    .execute(conn)
                    .map_err(|e| LoaderError::ApiError(e.to_string()))?;

                if rows_affected > 0 {
                    updated += 1;
                } else {
                    // Insert if update didn't affect any rows
                    match diesel::insert_into(crypto_social::table)
                        .values(&new_social)
                        .execute(conn)
                    {
                        Ok(_) => inserted += 1,
                        Err(e) => {
                            error!("Failed to insert social data for sid {}: {}", data.sid, e);
                            return Err(LoaderError::ApiError(e.to_string()));
                        }
                    }
                }
            } else {
                // Insert only mode
                match diesel::insert_into(crypto_social::table)
                    .values(&new_social)
                    .execute(conn)
                {
                    Ok(_) => inserted += 1,
                    Err(diesel::result::Error::DatabaseError(
                            diesel::result::DatabaseErrorKind::UniqueViolation,
                            _,
                        )) => {
                        debug!("Skipping duplicate social data for sid {}", data.sid);
                        // Skip duplicates in insert-only mode
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to insert social data for sid {}: {}", data.sid, e);
                        return Err(LoaderError::ApiError(e.to_string()));
                    }
                }
            }
        }

        Ok((inserted, updated))
    }
}