use anyhow::{anyhow, Result};
use av_database_postgres::models::news::ProcessedNewsStats;
use diesel::prelude::*;
use tracing::{debug, warn};

/// Save news data to database using synchronous diesel with symbol mapping
/// This function is shared between equity news and crypto news commands
pub async fn save_news_to_database(
    database_url: &str,
    news_data: Vec<av_database_postgres::models::news::NewsData>,
    continue_on_error: bool,
) -> Result<ProcessedNewsStats> {
    use av_database_postgres::schema::*;
    use diesel::{insert_into, PgConnection, Connection, RunQueryDsl, QueryDsl, ExpressionMethods};

    let database_url = database_url.to_string();

    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url)
            .map_err(|e| anyhow!("Database connection failed: {}", e))?;

        let mut stats = ProcessedNewsStats::default();

        // Use transaction for atomicity
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            for data in news_data {
                // Check if we've already processed this batch
                let existing = newsoverviews::table
                    .filter(newsoverviews::hashid.eq(&data.hash_id))
                    .select(newsoverviews::id)
                    .first::<i32>(conn)
                    .optional()?;

                if existing.is_some() {
                    debug!("Skipping duplicate news batch with hash: {}", data.hash_id);
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
                        None => {
                            insert_into(sources::table)
                                .values((
                                    sources::source_name.eq(&item.source_name),
                                    sources::domain.eq(&item.source_domain),
                                ))
                                .returning(sources::id)
                                .get_result::<i32>(conn)?
                        }
                    };

                    // Insert or get author
                    let author_id = match authors::table
                        .filter(authors::author_name.eq(&item.author_name))
                        .select(authors::id)
                        .first::<i32>(conn)
                        .optional()?
                    {
                        Some(id) => id,
                        None => {
                            insert_into(authors::table)
                                .values(authors::author_name.eq(&item.author_name))
                                .returning(authors::id)
                                .get_result::<i32>(conn)?
                        }
                    };

                    // Check if article already exists
                    let existing_article = articles::table
                        .filter(articles::hashid.eq(&item.article_hash))
                        .select(articles::hashid.clone())
                        .first::<String>(conn)
                        .optional()?;

                    let article_id = match existing_article {
                        Some(_) => {
                            debug!("Article already exists with hash: {}", item.article_hash);
                            // For existing articles, we'll use the hashid as the article reference
                            item.article_hash.clone()
                        },
                        None => {
                            // Insert new article
                            let new_article_hashid = insert_into(articles::table)
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
                                ))
                                .returning(articles::hashid)
                                .get_result::<String>(conn)?;

                            stats.articles += 1;
                            new_article_hashid
                        }
                    };

                    // Insert feed entry
                    let feed_id = insert_into(feeds::table)
                        .values((
                            feeds::sid.eq(data.sid),
                            feeds::newsoverviewid.eq(overview_id),
                            feeds::articleid.eq(&article_id),
                            feeds::sourceid.eq(source_id),
                            feeds::osentiment.eq(item.overall_sentiment_score),
                            feeds::sentlabel.eq(&item.overall_sentiment_label),
                        ))
                        .returning(feeds::id)
                        .get_result::<i32>(conn)?;

                    stats.feeds += 1;

                    // Insert ticker sentiments
                    for sentiment in &item.ticker_sentiments {
                        // SID is required and always populated in TickerSentimentData
                        insert_into(tickersentiments::table)
                            .values((
                                tickersentiments::feedid.eq(feed_id),
                                tickersentiments::sid.eq(sentiment.sid),
                                tickersentiments::relevance.eq(sentiment.relevance_score),
                                tickersentiments::tsentiment.eq(sentiment.sentiment_score),
                                tickersentiments::sentiment_label.eq(&sentiment.sentiment_label),
                            ))
                            .execute(conn)?;

                        stats.sentiments += 1;
                    }

                    // Insert topics
                    for topic in &item.topics {
                        // Get or create topic reference
                        let topic_id = match topicrefs::table
                            .filter(topicrefs::name.eq(&topic.name))
                            .select(topicrefs::id)
                            .first::<i32>(conn)
                            .optional()?
                        {
                            Some(id) => id,
                            None => {
                                insert_into(topicrefs::table)
                                    .values(topicrefs::name.eq(&topic.name))
                                    .returning(topicrefs::id)
                                    .get_result::<i32>(conn)?
                            }
                        };

                        // Insert topic mapping
                        insert_into(topicmaps::table)
                            .values((
                                topicmaps::feedid.eq(feed_id),
                                topicmaps::sid.eq(data.sid),
                                topicmaps::topicid.eq(topic_id),
                                topicmaps::relscore.eq(topic.relevance_score),
                            ))
                            .execute(conn)?;

                        stats.topics += 1;
                    }

                    // Insert author mapping
                    insert_into(authormaps::table)
                        .values((
                            authormaps::feedid.eq(feed_id),
                            authormaps::authorid.eq(author_id),
                        ))
                        .on_conflict((authormaps::feedid, authormaps::authorid))
                        .do_nothing()
                        .execute(conn)?;
                }
            }

            Ok(())
        })
            .map_err(|e| {
                if continue_on_error {
                    warn!("Database transaction failed but continuing: {}", e);
                    anyhow!("Database error (continuing): {}", e)
                } else {
                    anyhow!("Database transaction failed: {}", e)
                }
            })?;

        Ok(stats)
    })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
}