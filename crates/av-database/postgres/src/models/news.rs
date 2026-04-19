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

//! Diesel models for news articles, feeds, sentiment, and topic data.
//!
//! This module implements a normalized news storage schema that ingests data
//! from the Alpha Vantage `NEWS_SENTIMENT` endpoint and stores it across
//! nine interrelated tables. Unlike most other model modules in this crate,
//! all query methods here are **async** (`&mut AsyncPgConnection`).
//!
//! # Database schema (ER overview)
//!
//! ```text
//! newsoverviews ──1:N──► feeds ──1:N──► tickersentiments
//!                          │
//!                          ├──N:1──► articles ──N:1──► sources
//!                          ├──1:N──► authormaps ──N:1──► authors
//!                          └──1:N──► topicmaps  ──N:1──► topicrefs
//! ```
//!
//! # Table / model mapping
//!
//! | Table              | Query model          | Insertable (borrowed)     | Insertable (owned)         |
//! |--------------------|----------------------|---------------------------|----------------------------|
//! | `newsoverviews`    | [`NewsOverview`]     | [`NewNewsOverview`]       | [`NewNewsOverviewOwned`]   |
//! | `feeds`            | [`Feed`]             | [`NewFeed`]               | [`NewFeedOwned`]           |
//! | `articles`         | [`Article`]          | [`NewArticle`]            | [`NewArticleOwned`]        |
//! | `authors`          | [`Author`]           | [`NewAuthor`]             | [`NewAuthorOwned`]         |
//! | `authormaps`       | [`AuthorMap`]        | [`NewAuthorMap`]          | [`NewAuthorMapOwned`]      |
//! | `sources`          | [`Source`]           | [`NewSource`]             | [`NewSourceOwned`]         |
//! | `tickersentiments` | [`TickerSentiment`]  | [`NewTickerSentiment`]    | [`NewTickerSentimentOwned`]|
//! | `topicrefs`        | [`TopicRef`]         | [`NewTopicRef`]           | [`NewTopicRefOwned`]       |
//! | `topicmaps`        | [`TopicMap`]         | [`NewTopicMap`]           | [`NewTopicMapOwned`]       |
//!
//! # Analytics query-result types
//!
//! | Type                   | Purpose                                                    |
//! |------------------------|------------------------------------------------------------|
//! | [`SentimentSummary`]   | Aggregated sentiment for a symbol over a time window       |
//! | [`SentimentTrend`]     | Time-bucketed sentiment trend (uses TimescaleDB `time_bucket`) |
//! | [`TrendingTopic`]      | Topics ranked by mention count with sentiment stats        |
//! | [`ProcessedNewsStats`] | Counters returned by [`process_news_batch`]                |
//!
//! # Ingestion DTOs
//!
//! | Type                    | Purpose                                                   |
//! |-------------------------|-----------------------------------------------------------|
//! | [`NewsData`]            | Top-level ingestion input: one symbol's news batch        |
//! | [`NewsItem`]            | A single news article with sentiment and topic data       |
//! | [`TickerSentimentData`] | Per-ticker sentiment score within a news item             |
//! | [`TopicData`]           | Topic tag with relevance score within a news item         |
//!
//! # Key operations
//!
//! - **Batch ingestion:** [`process_news_batch`] — transactional insert of a
//!   full news batch, deduplicating by `hashid`.
//! - **Sentiment analytics:** [`NewsOverview::with_sentiment_summary`],
//!   [`TickerSentiment::get_sentiment_trend`].
//! - **Topic analytics:** [`TopicMap::get_trending_topics`],
//!   [`TopicMap::get_topic_correlation`].
//! - **Find-or-create:** [`Author::find_or_create`], [`Source::find_or_create`],
//!   [`TopicRef::find_or_create`].
//! - **Bulk insert:** [`NewFeed::bulk_insert_refs`],
//!   [`NewTickerSentiment::bulk_insert_refs`], [`NewTopicMap::bulk_insert_refs`].

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::schema::{
  articles, authormaps, authors, feeds, newsoverviews, sources, tickersentiments, topicmaps,
  topicrefs,
};

// ─── Analytics query-result types ───────────────────────────────────────────

/// Aggregated sentiment statistics for a single symbol over a time window.
///
/// Returned by [`NewsOverview::with_sentiment_summary`] via a raw SQL CTE query.
/// Not backed by a database table.
///
/// # Fields
///
/// - `sid` — the security ID that was queried.
/// - `avg_sentiment` — mean overall sentiment score across matching feeds.
/// - `article_count` — number of distinct news overview records.
/// - `bullish_pct` / `bearish_pct` / `neutral_pct` — percentage distribution
///   of sentiment labels.
/// - `topics` — JSONB array of `{topic, avg_relevance}` objects for topics
///   mentioned in the matching articles.
#[derive(QueryableByName, Debug, Serialize)]
pub struct SentimentSummary {
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub sid: i64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub avg_sentiment: f64,
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub article_count: i64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub bullish_pct: f64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub bearish_pct: f64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub neutral_pct: f64,
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  pub topics: serde_json::Value,
}

/// Time-bucketed sentiment trend for a symbol.
///
/// Returned by [`TickerSentiment::get_sentiment_trend`] using TimescaleDB's
/// `time_bucket()` function. Each row represents one time bucket.
///
/// - `bucket` — the start of the time bucket.
/// - `avg_sentiment` / `avg_relevance` — mean values within the bucket.
/// - `mention_count` — number of ticker-sentiment records in the bucket.
/// - `bullish_ratio` — fraction of mentions labeled `"Bullish"`.
#[derive(QueryableByName, Debug, Serialize)]
pub struct SentimentTrend {
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  pub bucket: chrono::DateTime<chrono::Utc>,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub avg_sentiment: f64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub avg_relevance: f64,
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub mention_count: i64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub bullish_ratio: f64,
}

/// A topic ranked by mention count with associated sentiment statistics.
///
/// Returned by [`TopicMap::get_trending_topics`].
///
/// - `topic_name` — the topic label (from `topicrefs`).
/// - `mention_count` — total appearances across feeds in the time window.
/// - `avg_relevance` / `avg_sentiment` — mean scores across mentions.
/// - `unique_symbols` — number of distinct securities associated with this topic.
#[derive(QueryableByName, Debug, Serialize)]
pub struct TrendingTopic {
  #[diesel(sql_type = diesel::sql_types::Text)]
  pub topic_name: String,
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub mention_count: i64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub avg_relevance: f64,
  #[diesel(sql_type = diesel::sql_types::Float8)]
  pub avg_sentiment: f64,
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub unique_symbols: i64,
}

/// Counters tracking how many records were inserted during a
/// [`process_news_batch`] call.
///
/// All fields start at `0` (via `Default`) and are incremented as each
/// entity is successfully inserted.
#[derive(Debug, Default)]
pub struct ProcessedNewsStats {
  pub news_overviews: usize,
  pub feeds: usize,
  pub articles: usize,
  pub sentiments: usize,
  pub topics: usize,
}

// ─── NewsOverview ───────────────────────────────────────────────────────────

/// A news-query snapshot for a single symbol at a point in time.
///
/// Maps to `newsoverviews` with composite PK `(creation, id)`.
/// Each row represents one API call to `NEWS_SENTIMENT` for a given `sid`.
///
/// - `hashid` — deterministic hash of the query parameters, used for
///   deduplication (see [`find_by_hashid`](NewsOverview::find_by_hashid)).
/// - `items` — number of news items returned by the query.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = newsoverviews)]
#[diesel(primary_key(creation, id))]
pub struct NewsOverview {
  pub id: i32,
  pub creation: chrono::DateTime<chrono::Utc>,
  pub sid: i64,
  pub items: i32,
  pub hashid: String,
}

/// Insertable (borrowed) form of [`NewsOverview`].
#[derive(Insertable, Debug)]
#[diesel(table_name = newsoverviews)]
pub struct NewNewsOverview<'a> {
  pub creation: &'a chrono::DateTime<chrono::Utc>,
  pub sid: &'a i64,
  pub items: &'a i32,
  pub hashid: &'a str,
}

/// Insertable (owned) form of [`NewsOverview`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = newsoverviews)]
pub struct NewNewsOverviewOwned {
  pub creation: chrono::DateTime<chrono::Utc>,
  pub sid: i64,
  pub items: i32,
  pub hashid: String,
}

/// Async query methods for [`NewsOverview`].
impl NewsOverview {
  /// Finds a news overview by its deterministic hash ID.
  /// Used for deduplication — if a result is returned, the batch was already ingested.
  pub async fn find_by_hashid(
    conn: &mut diesel_async::AsyncPgConnection,
    hashid: &str,
  ) -> Result<Option<Self>, diesel::result::Error> {
    newsoverviews::table.filter(newsoverviews::hashid.eq(hashid)).first(conn).await.optional()
  }

  /// Returns news overviews for a symbol within the last `days` days,
  /// ordered by creation time descending (most recent first).
  pub async fn get_recent_by_symbol(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
    days: i32,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    newsoverviews::table
      .filter(newsoverviews::sid.eq(sid))
      .filter(newsoverviews::creation.ge(chrono::Utc::now() - chrono::Duration::days(days as i64)))
      .order(newsoverviews::creation.desc())
      .load(conn)
      .await
  }

  /// Returns aggregated sentiment statistics for a symbol over the last `days` days.
  ///
  /// Executes a raw SQL CTE that joins `newsoverviews → feeds → topicmaps → topicrefs`
  /// to compute average sentiment, bullish/bearish/neutral percentages, and a JSONB
  /// array of associated topics. Returns a [`SentimentSummary`].
  pub async fn with_sentiment_summary(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
    days: i32,
  ) -> Result<Vec<SentimentSummary>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{BigInt, Integer};

    sql_query(
      r#"
            WITH recent_news AS (
                SELECT n.*, f.osentiment, f.sentlabel
                FROM newsoverviews n
                JOIN feeds f ON n.id = f.newsoverviewid
                WHERE n.sid = $1
                    AND n.creation >= NOW() - INTERVAL '1 day' * $2
            ),
            sentiment_counts AS (
                SELECT 
                    COUNT(*) as total,
                    COUNT(CASE WHEN sentlabel = 'Bullish' THEN 1 END) as bullish,
                    COUNT(CASE WHEN sentlabel = 'Bearish' THEN 1 END) as bearish,
                    COUNT(CASE WHEN sentlabel = 'Neutral' THEN 1 END) as neutral
                FROM recent_news
            )
            SELECT 
                $1 as sid,
                AVG(rn.osentiment) as avg_sentiment,
                COUNT(DISTINCT rn.id) as article_count,
                (sc.bullish::float8 / NULLIF(sc.total, 0) * 100) as bullish_pct,
                (sc.bearish::float8 / NULLIF(sc.total, 0) * 100) as bearish_pct,
                (sc.neutral::float8 / NULLIF(sc.total, 0) * 100) as neutral_pct,
                COALESCE(
                    jsonb_agg(DISTINCT 
                        jsonb_build_object(
                            'topic', tr.name,
                            'avg_relevance', tm.avg_relevance
                        )
                    ) FILTER (WHERE tr.name IS NOT NULL),
                    '[]'::jsonb
                ) as topics
            FROM recent_news rn
            CROSS JOIN sentiment_counts sc
            LEFT JOIN (
                SELECT 
                    f.newsoverviewid,
                    tm.topicid,
                    AVG(tm.relscore) as avg_relevance
                FROM feeds f
                JOIN topicmaps tm ON f.id = tm.feedid
                WHERE f.newsoverviewid IN (SELECT id FROM recent_news)
                GROUP BY f.newsoverviewid, tm.topicid
            ) tm ON rn.id = tm.newsoverviewid
            LEFT JOIN topicrefs tr ON tm.topicid = tr.id
            GROUP BY sc.bullish, sc.bearish, sc.neutral, sc.total
            "#,
    )
    .bind::<BigInt, _>(sid)
    .bind::<Integer, _>(days)
    .load::<SentimentSummary>(conn)
    .await
  }
}

// ─── Feed ───────────────────────────────────────────────────────────────────

/// A single news feed entry linking a [`NewsOverview`] to an [`Article`].
///
/// Maps to `feeds`. Each feed record captures the overall sentiment score
/// and label for one article within a news overview batch. The `belongs_to`
/// association enables Diesel's `grouped_by` pattern for eager loading.
///
/// - `osentiment` — overall sentiment score (`f32`, range `[-1.0, 1.0]`).
/// - `sentlabel` — sentiment label string (`"Bullish"`, `"Neutral"`, `"Bearish"`).
#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone, Serialize)]
#[diesel(table_name = feeds)]
#[diesel(belongs_to(NewsOverview, foreign_key = newsoverviewid))]
pub struct Feed {
  pub id: i32,
  pub sid: i64,
  pub newsoverviewid: i32,
  pub articleid: String,
  pub sourceid: i32,
  pub osentiment: f32,
  pub sentlabel: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Insertable (borrowed) form of [`Feed`].
#[derive(Insertable, Debug)]
#[diesel(table_name = feeds)]
pub struct NewFeed<'a> {
  pub sid: &'a i64,
  pub newsoverviewid: &'a i32,
  pub articleid: &'a str,
  pub sourceid: &'a i32,
  pub osentiment: &'a f32,
  pub sentlabel: &'a str,
}

/// Insertable (owned) form of [`Feed`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = feeds)]
pub struct NewFeedOwned {
  pub sid: i64,
  pub newsoverviewid: i32,
  pub articleid: String,
  pub sourceid: i32,
  pub osentiment: f32,
  pub sentlabel: String,
}

impl<'a> NewFeed<'a> {
  /// Inserts feeds in chunks of 500, returning all generated `id` values.
  pub async fn bulk_insert_refs(
    conn: &mut diesel_async::AsyncPgConnection,
    records: &[NewFeed<'a>],
  ) -> Result<Vec<i32>, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::new();

    for chunk in records.chunks(BATCH_SIZE) {
      let ids: Vec<i32> =
        insert_into(feeds::table).values(chunk).returning(feeds::id).get_results(conn).await?;
      all_ids.extend(ids);
    }

    Ok(all_ids)
  }
}

impl NewFeedOwned {
  /// Converts to a borrowed [`NewFeed`] reference for use with bulk-insert APIs.
  pub fn as_ref(&self) -> NewFeed<'_> {
    NewFeed {
      sid: &self.sid,
      newsoverviewid: &self.newsoverviewid,
      articleid: &self.articleid,
      sourceid: &self.sourceid,
      osentiment: &self.osentiment,
      sentlabel: &self.sentlabel,
    }
  }

  /// Inserts owned feeds in chunks of 500, returning all generated `id` values.
  pub async fn bulk_insert(
    conn: &mut diesel_async::AsyncPgConnection,
    records: Vec<Self>,
  ) -> Result<Vec<i32>, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::new();

    for chunk in records.chunks(BATCH_SIZE) {
      let ids: Vec<i32> =
        insert_into(feeds::table).values(chunk).returning(feeds::id).get_results(conn).await?;
      all_ids.extend(ids);
    }

    Ok(all_ids)
  }
}

// ─── Article ────────────────────────────────────────────────────────────────

/// A news article with content metadata, authorship, and media fields.
///
/// Maps to `articles` with PK `hashid` (a content-addressable hash of the
/// article URL/title). Deduplicated on insert — the same article appearing
/// in multiple feed batches is stored only once.
///
/// Fields like `source_link`, `release_time`, `author_description`,
/// `author_avatar_url`, `feature_image`, and `author_nick_name` are optional
/// extensions populated when the upstream API provides them.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = articles)]
#[diesel(primary_key(hashid))]
pub struct Article {
  pub hashid: String,
  pub sourceid: i32,
  pub category: String,
  pub title: String,
  pub url: String,
  pub summary: String,
  pub banner: String,
  pub author: i32,
  pub ct: NaiveDateTime,
  pub source_link: Option<String>,
  pub release_time: Option<i64>,
  pub author_description: Option<String>,
  pub author_avatar_url: Option<String>,
  pub feature_image: Option<String>,
  pub author_nick_name: Option<String>,
}

/// Insertable (borrowed) form of [`Article`].
#[derive(Insertable, Debug)]
#[diesel(table_name = articles)]
pub struct NewArticle<'a> {
  pub hashid: &'a str,
  pub sourceid: &'a i32,
  pub category: &'a str,
  pub title: &'a str,
  pub url: &'a str,
  pub summary: &'a str,
  pub banner: &'a str,
  pub author: &'a i32,
  pub ct: &'a NaiveDateTime,
  pub source_link: Option<&'a str>,
  pub release_time: Option<&'a i64>,
  pub author_description: Option<&'a str>,
  pub author_avatar_url: Option<&'a str>,
  pub feature_image: Option<&'a str>,
  pub author_nick_name: Option<&'a str>,
}

/// Insertable (owned) form of [`Article`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = articles)]
pub struct NewArticleOwned {
  pub hashid: String,
  pub sourceid: i32,
  pub category: String,
  pub title: String,
  pub url: String,
  pub summary: String,
  pub banner: String,
  pub author: i32,
  pub ct: NaiveDateTime,
  pub source_link: Option<String>,
  pub release_time: Option<i64>,
  pub author_description: Option<String>,
  pub author_avatar_url: Option<String>,
  pub feature_image: Option<String>,
  pub author_nick_name: Option<String>,
}

impl Article {
  /// Finds an article by its content-addressable hash ID. Returns `None` if not found.
  pub async fn find_by_hashid(
    conn: &mut diesel_async::AsyncPgConnection,
    hashid: &str,
  ) -> Result<Option<Self>, diesel::result::Error> {
    articles::table.find(hashid).first(conn).await.optional()
  }

  /// Returns up to `limit` articles in a given category, ordered by publish time descending.
  pub async fn get_by_category(
    conn: &mut diesel_async::AsyncPgConnection,
    category: &str,
    limit: i64,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    articles::table
      .filter(articles::category.eq(category))
      .order(articles::ct.desc())
      .limit(limit)
      .load(conn)
      .await
  }
}

// ─── Author ─────────────────────────────────────────────────────────────────

/// A news article author. Deduplicated by `author_name`.
///
/// Maps to `authors`. Linked to feeds via the [`AuthorMap`] junction table.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = authors)]
pub struct Author {
  pub id: i32,
  pub author_name: String,
}

/// Insertable (borrowed) form of [`Author`].
#[derive(Insertable, Debug)]
#[diesel(table_name = authors)]
pub struct NewAuthor<'a> {
  pub author_name: &'a str,
}

/// Insertable (owned) form of [`Author`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = authors)]
pub struct NewAuthorOwned {
  pub author_name: String,
}

impl Author {
  /// Returns the `id` of the author with the given name, creating a new
  /// record if one doesn't exist (find-or-create pattern).
  pub async fn find_or_create(
    conn: &mut diesel_async::AsyncPgConnection,
    name: &str,
  ) -> Result<i32, diesel::result::Error> {
    use diesel::insert_into;

    // Try to find existing
    if let Some(author) = authors::table
      .filter(authors::author_name.eq(name))
      .select(authors::id)
      .first::<i32>(conn)
      .await
      .optional()?
    {
      return Ok(author);
    }

    // Create new
    insert_into(authors::table)
      .values(NewAuthor { author_name: name })
      .returning(authors::id)
      .get_result(conn)
      .await
  }
}

// ─── AuthorMap ──────────────────────────────────────────────────────────────

/// Junction table linking [`Feed`] records to [`Author`] records (many-to-many).
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = authormaps)]
pub struct AuthorMap {
  pub id: i32,
  pub feedid: i32,
  pub authorid: i32,
}

/// Insertable (borrowed) form of [`AuthorMap`].
#[derive(Insertable, Debug)]
#[diesel(table_name = authormaps)]
pub struct NewAuthorMap<'a> {
  pub feedid: &'a i32,
  pub authorid: &'a i32,
}

/// Insertable (owned) form of [`AuthorMap`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = authormaps)]
pub struct NewAuthorMapOwned {
  pub feedid: i32,
  pub authorid: i32,
}

// ─── Source ─────────────────────────────────────────────────────────────────

/// A news source (publisher). Deduplicated by `(source_name, domain)`.
///
/// Maps to `sources`. Each source has a human-readable `source_name`
/// (e.g., `"Bloomberg"`) and a `domain` (e.g., `"bloomberg.com"`).
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = sources)]
pub struct Source {
  pub id: i32,
  pub source_name: String,
  pub domain: String,
}

/// Insertable (borrowed) form of [`Source`].
#[derive(Insertable, Debug)]
#[diesel(table_name = sources)]
pub struct NewSource<'a> {
  pub source_name: &'a str,
  pub domain: &'a str,
}

/// Insertable (owned) form of [`Source`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = sources)]
pub struct NewSourceOwned {
  pub source_name: String,
  pub domain: String,
}

impl Source {
  /// Returns the `id` of the source matching `(name, domain)`, creating a
  /// new record if one doesn't exist (find-or-create pattern).
  pub async fn find_or_create(
    conn: &mut diesel_async::AsyncPgConnection,
    name: &str,
    domain: &str,
  ) -> Result<i32, diesel::result::Error> {
    use diesel::insert_into;

    // Try to find existing
    if let Some(source) = sources::table
      .filter(sources::source_name.eq(name))
      .filter(sources::domain.eq(domain))
      .select(sources::id)
      .first::<i32>(conn)
      .await
      .optional()?
    {
      return Ok(source);
    }

    // Create new
    insert_into(sources::table)
      .values(NewSource { source_name: name, domain })
      .returning(sources::id)
      .get_result(conn)
      .await
  }
}

// ─── TickerSentiment ────────────────────────────────────────────────────────

/// Per-ticker sentiment score within a news feed entry.
///
/// Maps to `tickersentiments`. Each row links a [`Feed`] to a security
/// (`sid`) with a relevance weight and sentiment score/label.
///
/// - `relevance` — how closely the article relates to this ticker (`0.0`–`1.0`).
/// - `tsentiment` — sentiment score for this ticker mention (`-1.0`–`1.0`).
/// - `sentiment_label` — `"Bullish"`, `"Neutral"`, or `"Bearish"`.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = tickersentiments)]
pub struct TickerSentiment {
  pub id: i32,
  pub feedid: i32,
  pub sid: i64,
  pub relevance: f32,
  pub tsentiment: f32,
  pub sentiment_label: String,
}

/// Insertable (borrowed) form of [`TickerSentiment`].
#[derive(Insertable, Debug)]
#[diesel(table_name = tickersentiments)]
pub struct NewTickerSentiment<'a> {
  pub feedid: &'a i32,
  pub sid: &'a i64,
  pub relevance: &'a f32,
  pub tsentiment: &'a f32,
  pub sentiment_label: &'a str,
}

/// Insertable (owned) form of [`TickerSentiment`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = tickersentiments)]
pub struct NewTickerSentimentOwned {
  pub feedid: i32,
  pub sid: i64,
  pub relevance: f32,
  pub tsentiment: f32,
  pub sentiment_label: String,
}

impl<'a> NewTickerSentiment<'a> {
  /// Inserts ticker sentiments in chunks of 1000. Returns total rows inserted.
  pub async fn bulk_insert_refs(
    conn: &mut diesel_async::AsyncPgConnection,
    records: &[NewTickerSentiment<'a>],
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 1000;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(tickersentiments::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl TickerSentiment {
  /// Returns time-bucketed sentiment trends for a symbol.
  ///
  /// Uses TimescaleDB's `time_bucket()` to aggregate ticker sentiments into
  /// uniform time intervals. Each bucket contains average sentiment, average
  /// relevance, mention count, and bullish ratio.
  ///
  /// # Arguments
  ///
  /// - `sid` — security ID to query.
  /// - `bucket_size` — TimescaleDB interval string (e.g., `"1 hour"`, `"1 day"`).
  /// - `days` — look-back window in days.
  pub async fn get_sentiment_trend(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
    bucket_size: &str,
    days: i32,
  ) -> Result<Vec<SentimentTrend>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{BigInt, Integer};

    sql_query(format!(
      r#"
            SELECT 
                time_bucket('{}', n.creation) AS bucket,
                AVG(ts.tsentiment) as avg_sentiment,
                AVG(ts.relevance) as avg_relevance,
                COUNT(*)::bigint as mention_count,
                SUM(CASE WHEN ts.sentiment_label = 'Bullish' THEN 1 ELSE 0 END)::float8 / 
                    NULLIF(COUNT(*), 0) as bullish_ratio
            FROM tickersentiments ts
            JOIN feeds f ON ts.feedid = f.id
            JOIN newsoverviews n ON f.newsoverviewid = n.id
            WHERE ts.sid = $1
                AND n.creation >= NOW() - INTERVAL '1 day' * $2
            GROUP BY bucket
            ORDER BY bucket DESC
            "#,
      bucket_size
    ))
    .bind::<BigInt, _>(sid)
    .bind::<Integer, _>(days)
    .load::<SentimentTrend>(conn)
    .await
  }
}

// ─── TopicRef ───────────────────────────────────────────────────────────────

/// A topic label in the topic taxonomy. Deduplicated by `name`.
///
/// Maps to `topicrefs`. Topics are linked to feeds via the [`TopicMap`]
/// junction table.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = topicrefs)]
pub struct TopicRef {
  pub id: i32,
  pub name: String,
}

/// Insertable (borrowed) form of [`TopicRef`].
#[derive(Insertable, Debug)]
#[diesel(table_name = topicrefs)]
pub struct NewTopicRef<'a> {
  pub name: &'a str,
}

/// Insertable (owned) form of [`TopicRef`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = topicrefs)]
pub struct NewTopicRefOwned {
  pub name: String,
}

impl TopicRef {
  /// Returns the `id` of the topic with the given name, creating a new
  /// record if one doesn't exist (find-or-create pattern).
  pub async fn find_or_create(
    conn: &mut diesel_async::AsyncPgConnection,
    name: &str,
  ) -> Result<i32, diesel::result::Error> {
    use diesel::insert_into;

    if let Some(topic) = topicrefs::table
      .filter(topicrefs::name.eq(name))
      .select(topicrefs::id)
      .first::<i32>(conn)
      .await
      .optional()?
    {
      return Ok(topic);
    }

    insert_into(topicrefs::table)
      .values(NewTopicRef { name })
      .returning(topicrefs::id)
      .get_result(conn)
      .await
  }
}

// ─── TopicMap ───────────────────────────────────────────────────────────────

/// Junction table linking a [`Feed`] to a [`TopicRef`] with a relevance score.
///
/// Maps to `topicmaps`. Each row associates a feed entry with a topic and
/// stores `relscore` — how relevant the topic is to that feed item
/// (`0.0`–`1.0`).
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = topicmaps)]
pub struct TopicMap {
  pub id: i32,
  pub sid: i64,
  pub feedid: i32,
  pub topicid: i32,
  pub relscore: f32,
}

/// Insertable (borrowed) form of [`TopicMap`].
#[derive(Insertable, Debug)]
#[diesel(table_name = topicmaps)]
pub struct NewTopicMap<'a> {
  pub sid: &'a i64,
  pub feedid: &'a i32,
  pub topicid: &'a i32,
  pub relscore: &'a f32,
}

/// Insertable (owned) form of [`TopicMap`].
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = topicmaps)]
pub struct NewTopicMapOwned {
  pub sid: i64,
  pub feedid: i32,
  pub topicid: i32,
  pub relscore: f32,
}

impl<'a> NewTopicMap<'a> {
  /// Inserts topic mappings in chunks of 1000. Returns total rows inserted.
  pub async fn bulk_insert_refs(
    conn: &mut diesel_async::AsyncPgConnection,
    records: &[NewTopicMap<'a>],
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 1000;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(topicmaps::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl TopicMap {
  /// Returns the most-mentioned topics over the last `days` days, limited
  /// to `limit` results, ordered by mention count descending.
  ///
  /// Each [`TrendingTopic`] includes average relevance, average sentiment,
  /// and the number of unique securities associated with the topic.
  pub async fn get_trending_topics(
    conn: &mut diesel_async::AsyncPgConnection,
    days: i32,
    limit: i64,
  ) -> Result<Vec<TrendingTopic>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{BigInt, Integer};

    sql_query(
      r#"
            SELECT 
                tr.name as topic_name,
                COUNT(*)::bigint as mention_count,
                AVG(tm.relscore) as avg_relevance,
                AVG(f.osentiment) as avg_sentiment,
                COUNT(DISTINCT tm.sid)::bigint as unique_symbols
            FROM topicmaps tm
            JOIN topicrefs tr ON tm.topicid = tr.id
            JOIN feeds f ON tm.feedid = f.id
            JOIN newsoverviews n ON f.newsoverviewid = n.id
            WHERE n.creation >= NOW() - INTERVAL '1 day' * $1
            GROUP BY tr.name
            ORDER BY mention_count DESC
            LIMIT $2
            "#,
    )
    .bind::<Integer, _>(days)
    .bind::<BigInt, _>(limit)
    .load::<TrendingTopic>(conn)
    .await
  }

  /// Computes the Pearson correlation between two topics over the last `days` days.
  ///
  /// Measures how often the two topics co-occur in the same feed entries.
  /// Returns a value in `[-1.0, 1.0]` where `1.0` means perfect co-occurrence.
  pub async fn get_topic_correlation(
    conn: &mut diesel_async::AsyncPgConnection,
    topic1: &str,
    topic2: &str,
    days: i32,
  ) -> Result<f64, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{Float8, Integer, Text};

    #[derive(QueryableByName)]
    struct Correlation {
      #[diesel(sql_type = Float8)]
      correlation: f64,
    }

    let result = sql_query(
      r#"
            WITH topic_feeds AS (
                SELECT 
                    tm.feedid,
                    MAX(CASE WHEN tr.name = $1 THEN 1 ELSE 0 END) as has_topic1,
                    MAX(CASE WHEN tr.name = $2 THEN 1 ELSE 0 END) as has_topic2
                FROM topicmaps tm
                JOIN topicrefs tr ON tm.topicid = tr.id
                JOIN feeds f ON tm.feedid = f.id
                JOIN newsoverviews n ON f.newsoverviewid = n.id
                WHERE n.creation >= NOW() - INTERVAL '1 day' * $3
                    AND tr.name IN ($1, $2)
                GROUP BY tm.feedid
            )
            SELECT 
                CORR(has_topic1::float8, has_topic2::float8) as correlation
            FROM topic_feeds
            "#,
    )
    .bind::<Text, _>(topic1)
    .bind::<Text, _>(topic2)
    .bind::<Integer, _>(days)
    .get_result::<Correlation>(conn)
    .await?;

    Ok(result.correlation)
  }
}

// ─── Batch ingestion ────────────────────────────────────────────────────────

/// Ingests a batch of news data into all nine news tables within a single
/// database transaction.
///
/// For each [`NewsData`] item:
/// 1. Checks for an existing [`NewsOverview`] by `hash_id` (skip if duplicate).
/// 2. Inserts the overview, then iterates over its [`NewsItem`] entries.
/// 3. For each item: find-or-create the [`Source`] and [`Author`], insert
///    the [`Article`] (if not already present), create the [`Feed`] and
///    [`AuthorMap`], then insert all [`TickerSentiment`] and [`TopicMap`]
///    records.
///
/// Returns a [`ProcessedNewsStats`] with insertion counts.
///
/// # Errors
///
/// Returns `Err` if any database operation fails — the entire transaction
/// is rolled back in that case.
pub async fn process_news_batch(
  conn: &mut diesel_async::AsyncPgConnection,
  news_data: Vec<NewsData>,
) -> Result<ProcessedNewsStats, Box<dyn std::error::Error>> {
  use diesel::insert_into;
  use diesel_async::AsyncConnection;

  let mut stats = ProcessedNewsStats::default();

  // Process in a transaction
  conn
    .transaction::<_, diesel::result::Error, _>(|conn| {
      Box::pin(async move {
        for news in news_data {
          // Check if news overview exists
          if NewsOverview::find_by_hashid(conn, &news.hash_id).await?.is_some() {
            continue; // Skip if already processed
          }

          // Create news overview
          let overview_id = insert_into(newsoverviews::table)
            .values(NewNewsOverview {
              creation: &news.timestamp,
              sid: &news.sid,
              items: &(news.items.len() as i32),
              hashid: &news.hash_id,
            })
            .returning(newsoverviews::id)
            .get_result::<i32>(conn)
            .await?;
          stats.news_overviews += 1;

          // Process each feed item
          for item in news.items {
            // Get or create source
            let source_id =
              Source::find_or_create(conn, &item.source_name, &item.source_domain).await?;

            // Get or create author
            let author_id = Author::find_or_create(conn, &item.author_name).await?;

            // Create article if not exists
            if Article::find_by_hashid(conn, &item.article_hash).await?.is_none() {
              insert_into(articles::table)
                .values(NewArticle {
                  hashid: &item.article_hash,
                  sourceid: &source_id,
                  category: &item.category,
                  title: &item.title,
                  url: &item.url,
                  summary: &item.summary,
                  banner: &item.banner_url,
                  author: &author_id,
                  ct: &item.published_time,
                  // Map available data to new fields intelligently
                  source_link: item.source_link.as_deref(),
                  release_time: item.release_time.as_ref(),
                  author_description: item.author_description.as_deref(),
                  author_avatar_url: item.author_avatar_url.as_deref(),
                  feature_image: item.feature_image.as_deref().or(if !item.banner_url.is_empty() {
                    Some(&item.banner_url)
                  } else {
                    None
                  }),
                  author_nick_name: item.author_nick_name.as_deref(),
                })
                .execute(conn)
                .await?;
              stats.articles += 1;
            }

            // Create feed
            let feed_id = insert_into(feeds::table)
              .values(NewFeed {
                sid: &news.sid,
                newsoverviewid: &overview_id,
                articleid: &item.article_hash,
                sourceid: &source_id,
                osentiment: &item.overall_sentiment_score,
                sentlabel: &item.overall_sentiment_label,
              })
              .returning(feeds::id)
              .get_result::<i32>(conn)
              .await?;
            stats.feeds += 1;

            // Create authormap linking feed and author
            insert_into(authormaps::table)
              .values(NewAuthorMap { feedid: &feed_id, authorid: &author_id })
              .execute(conn)
              .await?;

            // Process ticker sentiments
            for ticker_sent in item.ticker_sentiments {
              insert_into(tickersentiments::table)
                .values(NewTickerSentiment {
                  feedid: &feed_id,
                  sid: &ticker_sent.sid,
                  relevance: &ticker_sent.relevance_score,
                  tsentiment: &ticker_sent.sentiment_score,
                  sentiment_label: &ticker_sent.sentiment_label,
                })
                .execute(conn)
                .await?;
              stats.sentiments += 1;
            }

            // Process topics
            for topic in item.topics {
              let topic_id = TopicRef::find_or_create(conn, &topic.name).await?;

              insert_into(topicmaps::table)
                .values(NewTopicMap {
                  sid: &news.sid,
                  feedid: &feed_id,
                  topicid: &topic_id,
                  relscore: &topic.relevance_score,
                })
                .execute(conn)
                .await?;
              stats.topics += 1;
            }
          }
        }

        Ok(stats)
      })
    })
    .await
    .map_err(Into::into)
}

// ─── Ingestion DTOs ─────────────────────────────────────────────────────────

/// Top-level ingestion input representing one symbol's news batch.
///
/// Consumed by [`process_news_batch`]. Not backed by a database table.
///
/// - `sid` — the security ID this news batch relates to.
/// - `hash_id` — deterministic hash for deduplication.
/// - `timestamp` — when the API query was made.
/// - `items` — the individual news articles in this batch.
#[derive(Debug)]
pub struct NewsData {
  pub sid: i64,
  pub hash_id: String,
  pub timestamp: chrono::DateTime<chrono::Utc>,
  pub items: Vec<NewsItem>,
}

/// A single news article with metadata, sentiment, topics, and optional
/// extended fields.
///
/// The required fields (`source_name`, `title`, `url`, etc.) map directly
/// to Alpha Vantage `NEWS_SENTIMENT` response fields. Optional fields
/// (`source_link`, `release_time`, `author_description`, etc.) are
/// extensions populated when the upstream API provides them.
#[derive(Debug)]
pub struct NewsItem {
  pub source_name: String,
  pub source_domain: String,
  pub author_name: String,
  pub article_hash: String,
  pub category: String,
  pub title: String,
  pub url: String,
  pub summary: String,
  pub banner_url: String,
  pub published_time: NaiveDateTime,
  pub overall_sentiment_score: f32,
  pub overall_sentiment_label: String,
  pub ticker_sentiments: Vec<TickerSentimentData>,
  pub topics: Vec<TopicData>,
  // New fields that could be populated from API data
  pub source_link: Option<String>, // Could be the source domain or original URL
  pub release_time: Option<i64>,   // Could be timestamp of published_time
  pub author_description: Option<String>, // Could be extracted from author data
  pub author_avatar_url: Option<String>, // From API if available
  pub feature_image: Option<String>, // Could use banner_url
  pub author_nick_name: Option<String>, // Could be derived from author_name
}

/// Per-ticker sentiment data within a [`NewsItem`].
///
/// - `sid` — the security this sentiment refers to.
/// - `relevance_score` — how relevant the article is to this ticker (`0.0`–`1.0`).
/// - `sentiment_score` — sentiment for this ticker (`-1.0`–`1.0`).
/// - `sentiment_label` — `"Bullish"`, `"Neutral"`, or `"Bearish"`.
#[derive(Debug)]
pub struct TickerSentimentData {
  pub sid: i64,
  pub relevance_score: f32,
  pub sentiment_score: f32,
  pub sentiment_label: String,
}

/// A topic tag with a relevance score within a [`NewsItem`].
///
/// - `name` — the topic label (e.g., `"Technology"`, `"Earnings"`).
/// - `relevance_score` — how relevant this topic is to the article (`0.0`–`1.0`).
#[derive(Debug)]
pub struct TopicData {
  pub name: String,
  pub relevance_score: f32,
}
