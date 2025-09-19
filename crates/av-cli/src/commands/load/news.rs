use anyhow::{anyhow, Result};
use av_client::AlphaVantageClient;
use av_database_postgres::models::news::ProcessedNewsStats;
use av_loaders::{
    NewsLoader, NewsLoaderConfig, NewsLoaderInput,
    LoaderContext, LoaderConfig, DataLoader,
    NewsSymbolInfo,
};
use chrono::{Duration, Utc};
use clap::Args;
use diesel::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::config::Config;

#[derive(Args, Clone, Debug)]
pub struct NewsArgs {
    /// Load for all equity symbols with overview=true
    #[arg(long)]
    all_equity: bool,

    /// Comma-separated list of specific tickers
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Number of days back to fetch news
    #[arg(short = 'd', long, default_value = "7")]
    days_back: u32,

    /// Topics to filter by (comma-separated)
    #[arg(short = 't', long, value_delimiter = ',')]
    topics: Option<Vec<String>>,

    /// Sort order (LATEST, EARLIEST, RELEVANCE)
    #[arg(long, default_value = "LATEST")]
    sort_order: String,

    /// Maximum number of articles to fetch (default: 100, max: 1000)
    #[arg(short, long, default_value = "100")]
    limit: u32,

    /// Disable caching
    #[arg(long)]
    no_cache: bool,

    /// Force refresh (bypass cache)
    #[arg(long)]
    force_refresh: bool,

    /// Cache TTL in hours
    #[arg(long, default_value = "24")]
    cache_ttl_hours: u32,

    /// Continue on error instead of stopping
    #[arg(long)]
    continue_on_error: bool,

    /// Dry run - fetch but don't save to database
    #[arg(long)]
    dry_run: bool,
}

/// Main execute function with inline persistence
pub async fn execute(args: NewsArgs, config: Config) -> Result<()> {
    info!("Starting news sentiment loader");

    // Validate limit
    if args.limit > 1000 {
        return Err(anyhow!("Limit cannot exceed 1000 (API maximum)"));
    }
    if args.limit < 1 {
        return Err(anyhow!("Limit must be at least 1"));
    }

    // Create API client
    let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Configure news loader
    let news_config = NewsLoaderConfig {
        days_back: Some(args.days_back),
        topics: args.topics.clone(),
        sort_order: Some(args.sort_order.clone()),
        limit: Some(args.limit),
        enable_cache: !args.no_cache,
        cache_ttl_hours: args.cache_ttl_hours,
        force_refresh: args.force_refresh,
        database_url: config.database_url.clone(),
    };

    info!("📰 News Loader Configuration:");
    info!("  Days back: {}", args.days_back);
    info!("  Limit: {} articles per request", args.limit);
    info!("  Sort order: {}", args.sort_order);
    info!("  Cache: {}", if args.no_cache { "disabled" } else { "enabled" });

    // Get symbols to process
    let symbols = if args.all_equity {
        info!("Loading all equity symbols with overview=true");
        NewsLoader::get_equity_symbols_with_overview(&config.database_url)?
    } else if let Some(ref symbol_list) = args.symbols {
        info!("Loading specific symbols: {:?}", symbol_list);
        get_specific_symbols(&config.database_url, symbol_list)?
    } else {
        return Err(anyhow!("Must specify either --all-equity or --symbols"));
    };

    if symbols.is_empty() {
        warn!("No symbols found to process");
        return Ok(());
    }

    info!("Processing {} symbols", symbols.len());

    // Create loader
    let loader = NewsLoader::new(5).with_config(news_config);

    // Create input
    let input = NewsLoaderInput {
        symbols: symbols.clone(),
        time_from: Some(Utc::now() - Duration::days(args.days_back as i64)),
        time_to: Some(Utc::now()),
    };

    // Create context
    let context = LoaderContext::new(
        client,
        LoaderConfig::default(),
    );

    // Load data from API
    info!("📡 Fetching news from AlphaVantage API...");
    let output = match loader.load(&context, input).await {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to load news: {}", e);
            if !args.continue_on_error {
                return Err(e.into());
            }
            return Ok(());
        }
    };

    info!(
        "✅ API fetch complete:\n  \
        - {} articles processed\n  \
        - {} data batches created\n  \
        - {} cache hits\n  \
        - {} API calls made",
        output.articles_processed,
        output.loaded_count,
        output.cache_hits,
        output.api_calls
    );

    // Save to database
    if !args.dry_run && !output.data.is_empty() {
        info!("💾 Saving news to database...");

        let stats = save_news_to_database(&config.database_url, output.data, args.continue_on_error).await?;

        info!(
            "✅ Database persistence complete:\n  \
            - {} news overviews\n  \
            - {} feeds\n  \
            - {} articles\n  \
            - {} ticker sentiments\n  \
            - {} topics",
            stats.news_overviews,
            stats.feeds,
            stats.articles,
            stats.sentiments,
            stats.topics
        );
    } else if args.dry_run {
        info!("🔍 Dry run mode - no database updates performed");
        info!("Would have saved {} news data batches", output.loaded_count);
    } else if output.data.is_empty() {
        warn!("⚠️ No data to save to database");
    }

    // Report loader errors
    if !output.errors.is_empty() {
        error!("❌ Errors during news loading:");
        for error in &output.errors {
            error!("  - {}", error);
        }
        if !args.continue_on_error {
            return Err(anyhow!("News loading completed with errors"));
        }
    }

    info!("🎉 News loading completed successfully");
    Ok(())
}

/// Helper function to get specific symbols from database
fn get_specific_symbols(database_url: &str, symbols: &[String]) -> Result<Vec<NewsSymbolInfo>> {
    use diesel::prelude::*;
    use av_database_postgres::schema::symbols;

    let mut conn = PgConnection::establish(database_url)?;

    let results = symbols::table
        .filter(symbols::symbol.eq_any(symbols))
        .select((symbols::sid, symbols::symbol))
        .load::<(i64, String)>(&mut conn)?;

    Ok(results.into_iter().map(|(sid, symbol)| NewsSymbolInfo {
        sid,
        symbol,
    }).collect())
}

/// Save news data to database using synchronous diesel with symbol mapping
async fn save_news_to_database(
    database_url: &str,
    news_data: Vec<av_database_postgres::models::news::NewsData>,
    _continue_on_error: bool,
) -> Result<ProcessedNewsStats> {
    use av_database_postgres::schema::*;
    use diesel::insert_into;

    // Clone database_url for the spawned task
    let database_url = database_url.to_string();

    // Run in blocking task since we're using synchronous diesel
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url)
            .map_err(|e| anyhow!("Database connection failed: {}", e))?;

        // Load symbol to SID mapping for all symbols in the database
        let symbol_to_sid: HashMap<String, i64> = {
            let results: Vec<(String, i64)> = symbols::table
                .select((symbols::symbol, symbols::sid))
                .load(&mut conn)
                .map_err(|e| anyhow!("Failed to load symbol mapping: {}", e))?;

            let mut mapping = HashMap::new();
            for (symbol, sid) in results {
                mapping.insert(symbol, sid);
            }
            mapping
        };

        info!("Loaded {} symbols for sentiment mapping", symbol_to_sid.len());

        let mut stats = ProcessedNewsStats::default();
        let mut missed_symbols: Vec<String> = Vec::new();

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
                    // Already processed this batch, skip it entirely
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

                    // Process ALL ticker sentiments - they're already resolved to SIDs
                    for sentiment in &item.ticker_sentiments {
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

                    // Process topics
                    for topic in &item.topics {
                        // Get or create topic reference
                        let topic_id = insert_into(topicrefs::table)
                            .values(topicrefs::name.eq(&topic.name))
                            .on_conflict(topicrefs::name)
                            .do_nothing()
                            .returning(topicrefs::id)
                            .get_result::<i32>(conn)
                            .or_else(|_| {
                                topicrefs::table
                                    .filter(topicrefs::name.eq(&topic.name))
                                    .select(topicrefs::id)
                                    .first::<i32>(conn)
                            })?;

                        // Insert topic mapping
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

                    // Insert author mapping for this feed
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
        }).map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(stats)
    }).await?
}