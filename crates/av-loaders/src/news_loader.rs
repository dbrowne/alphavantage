//! News loader for AlphaVantage news sentiment data
//!
//! This module handles loading news articles with sentiment analysis
//! from the AlphaVantage API and persisting them to the database.
//!
//! Features:
//! - API response caching to minimize vendor fees
//! - Filters for equity symbols with overview=true
//! - Batch processing with configurable rate limiting
//! - Comprehensive sentiment analysis persistence
//! - Support for up to 1000 articles per API call (default: 100)

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    sql_query, sql_types, Connection, PgConnection, QueryableByName, RunQueryDsl,
    QueryDsl, ExpressionMethods, OptionalExtension,
};
use tracing::{info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
    models::{
        news::{
            NewsData, NewsItem,
            TickerSentimentData, TopicData,
        },
    },
};
use av_models::news::{NewsArticle, NewsSentiment};

use crate::{
    error::{LoaderError, LoaderResult},
    loader::{DataLoader, LoaderContext},
};

/// Cache query result structure for SQL queries
#[derive(QueryableByName, Debug)]
struct CacheQueryResult {
    #[diesel(sql_type = sql_types::Jsonb)]
    response_data: serde_json::Value,
    #[diesel(sql_type = sql_types::Timestamptz)]
    expires_at: DateTime<Utc>,
}

/// Symbol info for news loading
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub sid: i64,
    pub symbol: String,
}

/// Configuration for news loader
#[derive(Debug, Clone)]
pub struct NewsLoaderConfig {
    /// Number of days of news history to fetch
    pub days_back: Option<u32>,

    /// Specific topics to filter by
    pub topics: Option<Vec<String>>,

    /// Sort order for results (LATEST, EARLIEST, RELEVANCE)
    pub sort_order: Option<String>,

    /// Maximum number of articles per request (default: 100, max: 1000)
    pub limit: Option<u32>,

    /// Enable API response caching
    pub enable_cache: bool,

    /// Cache TTL in hours
    pub cache_ttl_hours: u32,

    /// Force refresh (bypass cache)
    pub force_refresh: bool,

    /// Database URL for caching
    pub database_url: String,
}

impl Default for NewsLoaderConfig {
    fn default() -> Self {
        Self {
            days_back: Some(7),
            topics: None,
            sort_order: Some("LATEST".to_string()),
            limit: Some(100),
            enable_cache: true,
            cache_ttl_hours: 24,
            force_refresh: false,
            database_url: "postgresql://ts_user:dev_pw@localhost:6433/sec_master".to_string(),
        }
    }
}

/// Input for news loader
#[derive(Debug, Clone)]
pub struct NewsLoaderInput {
    /// List of symbols to fetch news for
    pub symbols: Vec<SymbolInfo>,

    /// Optional time range
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,
}

/// Output from news loader
#[derive(Debug)]
pub struct NewsLoaderOutput {
    /// Processed news data
    pub data: Vec<NewsData>,

    /// Number of news overviews created
    pub news_overviews_created: usize,

    /// Number of articles processed
    pub articles_processed: usize,

    /// Number of feeds created
    pub feeds_created: usize,

    /// Number of ticker sentiments recorded
    pub sentiments_recorded: usize,

    /// Number of topics mapped
    pub topics_mapped: usize,

    /// Articles that were skipped (already exist)
    pub articles_skipped: usize,

    /// Number of cache hits
    pub cache_hits: usize,

    /// Number of API calls made
    pub api_calls: usize,

    /// Any errors encountered
    pub errors: Vec<String>,

    /// Number successfully loaded
    pub loaded_count: usize,

    /// Number with no data
    pub no_data_count: usize,
}

/// News loader implementation
pub struct NewsLoader {
    config: NewsLoaderConfig,
    concurrent_limit: usize,
}

impl NewsLoader {
    /// Create a new news loader
    pub fn new(concurrent_limit: usize) -> Self {
        Self {
            config: NewsLoaderConfig::default(),
            concurrent_limit,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: NewsLoaderConfig) -> Self {
        self.config = config;
        self
    }

    /// Get equity symbols with overview=true from database
    pub fn get_equity_symbols_with_overview(
        database_url: &str,
    ) -> LoaderResult<Vec<SymbolInfo>> {
        use av_database_postgres::schema::symbols;

        let mut conn = PgConnection::establish(database_url)
            .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect: {}", e)))?;

        let results = symbols::table
            .filter(symbols::sec_type.eq("Equity"))
            .filter(symbols::overview.eq(true))
            .select((symbols::sid, symbols::symbol))
            .load::<(i64, String)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Failed to fetch equity symbols: {}", e)))?;

        Ok(results.into_iter().map(|(sid, symbol)| SymbolInfo { sid, symbol }).collect())
    }

    /// Generate cache key for API request
    fn generate_cache_key(&self, tickers: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tickers.hash(&mut hasher);
        if let Some(topics) = &self.config.topics {
            topics.join(",").hash(&mut hasher);
        }
        format!("news_{:x}", hasher.finish())
    }

    /// Get cached response if available
    fn get_cached_response(&self, cache_key: &str) -> Option<NewsSentiment> {
        if !self.config.enable_cache || self.config.force_refresh {
            return None;
        }

        let mut conn = match PgConnection::establish(&self.config.database_url) {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Failed to connect for cache lookup: {}", e);
                return None;
            }
        };

        let cached_entry: Option<CacheQueryResult> = sql_query(
            "SELECT response_data, expires_at FROM api_response_cache
             WHERE cache_key = $1 AND expires_at > NOW() AND api_source = 'alphavantage'"
        )
            .bind::<sql_types::Text, _>(cache_key)
            .get_result(&mut conn)
            .optional()
            .unwrap_or(None);

        if let Some(cache_result) = cached_entry {
            info!("📦 Cache hit for news query (expires: {})", cache_result.expires_at);

            if let Ok(news) = serde_json::from_value::<NewsSentiment>(cache_result.response_data) {
                return Some(news);
            } else {
                warn!("Failed to parse cached news response");
            }
        }

        None
    }

    /// Cache API response
    fn cache_response(&self, cache_key: &str, endpoint_url: &str, response: &NewsSentiment) {
        if !self.config.enable_cache {
            return;
        }

        let mut conn = match PgConnection::establish(&self.config.database_url) {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Failed to connect for caching: {}", e);
                return;
            }
        };

        let response_json = match serde_json::to_value(response) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to serialize response for caching: {}", e);
                return;
            }
        };

        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours as i64);

        let result = sql_query(
            "INSERT INTO api_response_cache
             (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
             VALUES ($1, 'alphavantage', $2, $3, 200, $4)
             ON CONFLICT (cache_key) DO UPDATE SET
                response_data = EXCLUDED.response_data,
                status_code = EXCLUDED.status_code,
                expires_at = EXCLUDED.expires_at,
                cached_at = NOW()"
        )
            .bind::<sql_types::Text, _>(cache_key)
            .bind::<sql_types::Text, _>(endpoint_url)
            .bind::<sql_types::Jsonb, _>(&response_json)
            .bind::<sql_types::Timestamptz, _>(expires_at)
            .execute(&mut conn);

        match result {
            Ok(_) => info!("💾 Cached news response (expires: {})", expires_at),
            Err(e) => warn!("Failed to cache response: {}", e),
        }
    }

    /// Convert API news response to internal format
    fn convert_news_to_data(&self,
                            news: &NewsSentiment,
                            symbols: &[SymbolInfo],
    ) -> Vec<NewsData> {
        let symbol_map: HashMap<String, i64> = symbols.iter()
            .map(|s| (s.symbol.clone(), s.sid))
            .collect();

        let mut news_data_map: HashMap<i64, Vec<NewsItem>> = HashMap::new();

        for article in &news.feed {
            for ticker_sentiment in &article.ticker_sentiment {
                if let Some(&sid) = symbol_map.get(&ticker_sentiment.ticker) {
                    let news_item = self.convert_article_to_item(article, &symbol_map);
                    news_data_map.entry(sid).or_insert_with(Vec::new).push(news_item);
                }
            }
        }

        news_data_map.into_iter().map(|(sid, items)| {
            let hash_id = format!("news_{}_{}", sid, Utc::now().timestamp());

            NewsData {
                sid,
                hash_id,
                timestamp: Utc::now(),
                items,
            }
        }).collect()
    }

    /// Convert API article to internal news item
    fn convert_article_to_item(&self,
                               article: &NewsArticle,
                               symbol_map: &HashMap<String, i64>,
    ) -> NewsItem {
        let published_time = self.parse_time_published(&article.time_published);

        let ticker_sentiments = article.ticker_sentiment.iter()
            .filter_map(|ts| {
                symbol_map.get(&ts.ticker).map(|&sid| {
                    TickerSentimentData {
                        sid,
                        relevance_score: ts.relevance_score.parse().unwrap_or(0.0),
                        sentiment_score: ts.ticker_sentiment_score.parse().unwrap_or(0.0),
                        sentiment_label: ts.ticker_sentiment_label.clone(),
                    }
                })
            })
            .collect();

        let topics = article.topics.iter().map(|topic| {
            TopicData {
                name: topic.topic.clone(),
                relevance_score: topic.relevance_score.parse().unwrap_or(0.0),
            }
        }).collect();

        let author_name = article.authors.first()
            .map(|s| s.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        NewsItem {
            source_name: article.source.clone(),
            source_domain: article.source_domain.clone(),
            author_name,
            article_hash: format!("{}_{}", article.url, article.time_published),
            category: article.category_within_source.clone(),
            title: article.title.clone(),
            url: article.url.clone(),
            summary: article.summary.clone(),
            banner_url: article.banner_image.clone().unwrap_or_default(),
            published_time,
            overall_sentiment_score: article.overall_sentiment_score as f32,
            overall_sentiment_label: article.overall_sentiment_label.clone(),
            ticker_sentiments,
            topics,
            source_link: Some(article.url.clone()),
            release_time: Some(published_time.and_utc().timestamp()),
            author_description: None,
            author_avatar_url: None,
            feature_image: article.banner_image.clone(),
            author_nick_name: None,
        }
    }

    /// Parse time published string to NaiveDateTime
    fn parse_time_published(&self, time_str: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(time_str, "%Y%m%dT%H%M%S")
            .unwrap_or_else(|e| {
                warn!("Failed to parse time '{}': {}", time_str, e);
                DateTime::from_timestamp(0, 0)
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_else(|| NaiveDateTime::default())
            })
    }
}

#[async_trait]
impl DataLoader for NewsLoader {
    type Input = NewsLoaderInput;
    type Output = NewsLoaderOutput;

    fn name(&self) -> &'static str {
        "NewsLoader"
    }

    async fn load(
        &self,
        context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output> {
        info!("Loading news for {} symbols", input.symbols.len());

        let mut output = NewsLoaderOutput {
            data: Vec::new(),
            news_overviews_created: 0,
            articles_processed: 0,
            feeds_created: 0,
            sentiments_recorded: 0,
            topics_mapped: 0,
            articles_skipped: 0,
            cache_hits: 0,
            api_calls: 0,
            errors: Vec::new(),
            loaded_count: 0,
            no_data_count: 0,
        };

        // Build tickers string
        let tickers: Vec<String> = input.symbols.iter()
            .map(|s| s.symbol.clone())
            .collect();
        let tickers_str = tickers.join(",");

        // Build topics string if configured
        let topics_str = self.config.topics.as_ref()
            .map(|topics| topics.join(","));

        // Format time parameters
        let time_from = input.time_from
            .map(|dt| dt.format("%Y%m%dT%H%M").to_string());
        let time_to = input.time_to
            .map(|dt| dt.format("%Y%m%dT%H%M").to_string());

        // Validate and set limit
        let limit = self.config.limit.map(|l| {
            if l > 1000 {
                warn!("Limit {} exceeds API maximum of 1000, capping at 1000", l);
                1000
            } else if l < 1 {
                warn!("Invalid limit {}, using default of 100", l);
                100
            } else {
                l
            }
        });

        // Generate cache key
        let cache_key = self.generate_cache_key(&tickers_str);

        // Try to get cached response first
        let news = if let Some(cached_news) = self.get_cached_response(&cache_key) {
            output.cache_hits += 1;
            cached_news
        } else {
            info!("🌐 Fetching news from API (limit: {})", limit.unwrap_or(100));
            output.api_calls += 1;

            // Build endpoint URL for caching
            let endpoint_url = format!(
                "https://www.alphavantage.co/query?function=NEWS_SENTIMENT&tickers={}&limit={}",
                tickers_str,
                limit.unwrap_or(100)
            );

            // Fetch from API
            match context.client.news().news_sentiment(
                Some(&tickers_str),
                topics_str.as_deref(),
                time_from.as_deref(),
                time_to.as_deref(),
                self.config.sort_order.as_deref(),
                limit,
            ).await {
                Ok(news) => {
                    // Cache the response
                    self.cache_response(&cache_key, &endpoint_url, &news);
                    news
                }
                Err(e) => {
                    output.errors.push(format!("Failed to fetch news: {}", e));
                    return Ok(output);
                }
            }
        };

        info!("Processing {} news articles", news.feed.len());

        // Convert to internal format
        let news_data = self.convert_news_to_data(&news, &input.symbols);

        output.loaded_count = news_data.len();
        output.articles_processed = news.feed.len();
        output.data = news_data;

        Ok(output)
    }
}

/// Helper function to load news for all equity symbols with overview=true
pub async fn load_news_for_equity_symbols(
    client: Arc<AlphaVantageClient>,
    database_url: &str,
    config: NewsLoaderConfig,
) -> LoaderResult<NewsLoaderOutput> {
    // Get equity symbols with overview=true
    let symbols = NewsLoader::get_equity_symbols_with_overview(database_url)?;

    info!("Found {} equity symbols with overview=true", symbols.len());

    if symbols.is_empty() {
        return Ok(NewsLoaderOutput {
            data: Vec::new(),
            news_overviews_created: 0,
            articles_processed: 0,
            feeds_created: 0,
            sentiments_recorded: 0,
            topics_mapped: 0,
            articles_skipped: 0,
            cache_hits: 0,
            api_calls: 0,
            errors: Vec::new(),
            loaded_count: 0,
            no_data_count: 0,
        });
    }

    // Create loader
    let loader = NewsLoader::new(5).with_config(config);

    // Create input
    let input = NewsLoaderInput {
        symbols,
        time_from: Some(Utc::now() - chrono::Duration::days(7)),
        time_to: Some(Utc::now()),
    };

    // Create context
    let context = LoaderContext::new(
        client,
        crate::LoaderConfig::default(),
    );

    // Load data
    loader.load(&context, input).await
}