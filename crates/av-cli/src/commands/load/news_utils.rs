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

use anyhow::{Result, anyhow};
use av_database_postgres::models::news::{NewsData, ProcessedNewsStats};
use chrono::Utc;
use diesel::{
  Connection, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl, RunQueryDsl,
};

/// Save news data to database using synchronous diesel with symbol mapping
/// This is copied EXACTLY from the original news.rs file
pub async fn save_news_to_database(
  database_url: &str,
  news_data: Vec<NewsData>,
  _continue_on_error: bool,
) -> Result<ProcessedNewsStats> {
  use av_database_postgres::schema::*;
  use diesel::insert_into;

  let database_url = database_url.to_string();

  tokio::task::spawn_blocking(move || {
    let mut conn = PgConnection::establish(&database_url)
      .map_err(|e| anyhow!("Database connection failed: {}", e))?;

    let mut stats = ProcessedNewsStats::default();

    // Use transaction for atomicity
    conn
      .transaction::<_, diesel::result::Error, _>(|conn| {
        for data in news_data {
          // Check if we've already processed this batch
          let existing = newsoverviews::table
            .filter(newsoverviews::hashid.eq(&data.hash_id))
            .select(newsoverviews::id)
            .first::<i32>(conn)
            .optional()?;

          if existing.is_some() {
            continue;
          }

          // Insert new newsoverview
          let overview_id = insert_into(newsoverviews::table)
            .values((
              newsoverviews::creation.eq(data.timestamp),
              newsoverviews::sid.eq(data.sid),
              newsoverviews::items.eq(data.items.len() as i32),
              newsoverviews::hashid.eq(&data.hash_id),
            ))
            .returning(newsoverviews::id)
            .get_result::<i32>(conn)?;

          stats.news_overviews += 1;

          // Process each news item
          for item in data.items {
            // Insert or get source
            let source_id = match sources::table
              .filter(sources::domain.eq(&item.source_domain))
              .select(sources::id)
              .first::<i32>(conn)
              .optional()?
            {
              Some(id) => id,
              None => insert_into(sources::table)
                .values((
                  sources::source_name.eq(&item.source_name),
                  sources::domain.eq(&item.source_domain),
                ))
                .returning(sources::id)
                .get_result::<i32>(conn)?,
            };

            // Insert or get author
            let author_id = insert_into(authors::table)
              .values(authors::author_name.eq(&item.author_name))
              .on_conflict(authors::author_name)
              .do_nothing()
              .returning(authors::id)
              .get_result::<i32>(conn)
              .or_else(|_| {
                authors::table
                  .filter(authors::author_name.eq(&item.author_name))
                  .select(authors::id)
                  .first::<i32>(conn)
              })?;

            // Check if article exists
            let article_exists = articles::table
              .filter(articles::hashid.eq(&item.article_hash))
              .select(articles::hashid)
              .first::<String>(conn)
              .optional()?;

            if article_exists.is_none() {
              // Insert article
              insert_into(articles::table)
                .values((
                  articles::hashid.eq(&item.article_hash),
                  articles::sourceid.eq(source_id),
                  articles::category.eq(&item.category),
                  articles::title.eq(&item.title),
                  articles::url.eq(&item.url),
                  articles::summary.eq(&item.summary),
                  articles::banner.eq(&item.banner_url),
                  articles::author.eq(author_id),
                  articles::ct.eq(item.published_time),
                  articles::source_link.eq(item.source_link.as_ref()),
                  articles::release_time.eq(item.release_time),
                  articles::author_description.eq(item.author_description.as_ref()),
                  articles::author_avatar_url.eq(item.author_avatar_url.as_ref()),
                  articles::feature_image.eq(item.feature_image.as_ref()),
                  articles::author_nick_name.eq(item.author_nick_name.as_ref()),
                ))
                .execute(conn)?;
              stats.articles += 1;
            }

            // Create feed entry
            let feed_id = insert_into(feeds::table)
              .values((
                feeds::sid.eq(data.sid),
                feeds::newsoverviewid.eq(overview_id),
                feeds::articleid.eq(&item.article_hash),
                feeds::sourceid.eq(source_id),
                feeds::osentiment.eq(item.overall_sentiment_score),
                feeds::sentlabel.eq(&item.overall_sentiment_label),
                feeds::created_at.eq(Utc::now()),
              ))
              .returning(feeds::id)
              .get_result::<i32>(conn)?;

            stats.feeds += 1;

            // Create authormap linking feed and author
            insert_into(authormaps::table)
              .values((authormaps::feedid.eq(feed_id), authormaps::authorid.eq(author_id)))
              .execute(conn)?;

            // Process ticker sentiments
            for sentiment in item.ticker_sentiments {
              insert_into(tickersentiments::table)
                .values((
                  tickersentiments::sid.eq(sentiment.sid),
                  tickersentiments::feedid.eq(feed_id),
                  tickersentiments::relevance.eq(sentiment.relevance_score),
                  tickersentiments::tsentiment.eq(sentiment.sentiment_score),
                  tickersentiments::sentiment_label.eq(&sentiment.sentiment_label),
                ))
                .execute(conn)?;

              stats.sentiments += 1;
            }

            // Process topics
            for topic in item.topics {
              // Get or create topic reference
              let topic_id = match topicrefs::table
                .filter(topicrefs::name.eq(&topic.name))
                .select(topicrefs::id)
                .first::<i32>(conn)
                .optional()?
              {
                Some(id) => id,
                None => insert_into(topicrefs::table)
                  .values(topicrefs::name.eq(&topic.name))
                  .returning(topicrefs::id)
                  .get_result::<i32>(conn)?,
              };

              insert_into(topicmaps::table)
                .values((
                  topicmaps::sid.eq(data.sid),
                  topicmaps::feedid.eq(feed_id),
                  topicmaps::topicid.eq(topic_id),
                  topicmaps::relscore.eq(topic.relevance_score),
                ))
                .execute(conn)?;

              stats.topics += 1;
            }
          }
        }

        Ok(stats)
      })
      .map_err(|e: diesel::result::Error| anyhow!("Transaction failed: {}", e))
  })
  .await
  .map_err(|e| anyhow!("Task failed: {}", e))?
}
