/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Social data loader wrapper for av-loaders integration.
//!
//! This module provides a wrapper around the crypto-loaders SocialLoader
//! that integrates with the av-database-postgres and av-loaders context.

use crate::{LoaderError, LoaderResult};

use bigdecimal::BigDecimal;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use rust_decimal::Decimal;
use tracing::{debug, error};

// Re-export types from crypto-loaders for backward compatibility
pub use crypto_loaders::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSymbolForSocial, ProcessedSocialData,
  SocialLoader as BaseSocialLoader, SocialLoaderResult,
};

/// Extended CryptoSocialLoader that supports av-loaders context and database operations.
///
/// This struct wraps the base SocialLoader from crypto-loaders and adds
/// support for the LoaderContext and database persistence.
#[allow(dead_code)]
pub struct CryptoSocialLoader {
  inner: BaseSocialLoader,
}

impl CryptoSocialLoader {
  pub fn new(config: CryptoSocialConfig) -> Self {
    Self { inner: BaseSocialLoader::new(config) }
  }

  /// Load social data using the inner loader.
  pub async fn load_data(
    &self,
    input: &CryptoSocialInput,
    _context: &crate::LoaderContext,
  ) -> LoaderResult<Vec<ProcessedSocialData>> {
    self.inner.load_data(input).await.map_err(|e| LoaderError::ApiError(e.to_string()))
  }

  /// Helper function to convert Decimal to BigDecimal.
  fn decimal_to_bigdecimal(decimal: Option<Decimal>) -> Option<BigDecimal> {
    decimal.map(|d| {
      // Convert via string to avoid precision issues
      BigDecimal::parse_bytes(d.to_string().as_bytes(), 10).unwrap_or_else(|| BigDecimal::from(0))
    })
  }

  /// Save social data to the database.
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

        let rows_affected =
          diesel::update(crypto_social::table.filter(social_dsl::sid.eq(data.sid)))
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
          match diesel::insert_into(crypto_social::table).values(&new_social).execute(conn) {
            Ok(_) => inserted += 1,
            Err(e) => {
              error!("Failed to insert social data for sid {}: {}", data.sid, e);
              return Err(LoaderError::ApiError(e.to_string()));
            }
          }
        }
      } else {
        // Insert only mode
        match diesel::insert_into(crypto_social::table).values(&new_social).execute(conn) {
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

impl Clone for CryptoSocialLoader {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}