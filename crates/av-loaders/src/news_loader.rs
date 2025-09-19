use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{Connection, PgConnection, RunQueryDsl, QueryDsl, ExpressionMethods};
use tracing::{debug, info, warn};
use std::collections::HashMap;

use av_client::AlphaVantageClient;
use av_database_postgres::{
    models::news::{NewsData, NewsItem, TickerSentimentData, TopicData},
    schema::symbols,
};
use av_models::news::NewsSentiment;

use crate::{
    error::{LoaderError, LoaderResult},
    loader::{DataLoader, LoaderContext},
};

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
    /// Maximum number of articles per request
    pub limit: Option<u32>,
    /// Enable caching
    pub enable_cache: bool,
    /// Cache TTL in hours
    pub cache_ttl_hours: u32,
    /// Force refresh (bypass cache)
    pub force_refresh: bool,
    /// Database URL for cache and symbol lookup
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
            database_url: String::new(),
        }
    }
}

/// Input for news loader
#[derive(Debug, Clone)]
pub struct NewsLoaderInput {
    pub symbols: Vec<SymbolInfo>,
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,
}

/// Output from news loader
#[derive(Debug)]
pub struct NewsLoaderOutput {
    pub data: Vec<NewsData>,
    pub loaded_count: usize,
    pub articles_processed: usize,
    pub cache_hits: usize,
    pub api_calls: usize,
    pub errors: Vec<String>,
    pub no_data_count: usize,
}

impl Default for NewsLoaderOutput {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            loaded_count: 0,
            articles_processed: 0,
            cache_hits: 0,
            api_calls: 0,
            errors: Vec::new(),
            no_data_count: 0,
        }
    }
}

/// News loader implementation
pub struct NewsLoader {
    config: NewsLoaderConfig,
    concurrent_requests: usize,
}

impl NewsLoader {
    /// Create a new news loader
    pub fn new(concurrent_requests: usize) -> Self {
        Self {
            config: NewsLoaderConfig::default(),
            concurrent_requests,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: NewsLoaderConfig) -> Self {
        self.config = config;
        self
    }

    /// Load all symbols from database for sentiment mapping
    fn load_all_symbols(&self) -> LoaderResult<HashMap<String, i64>> {
        let mut conn = PgConnection::establish(&self.config.database_url)
            .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

        let results: Vec<(String, i64)> = symbols::table
            .select((symbols::symbol, symbols::sid))
            .load::<(String, i64)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(results.into_iter().collect())
    }

    /// Get equity symbols with overview=true from database
    pub fn get_equity_symbols_with_overview(database_url: &str) -> LoaderResult<Vec<SymbolInfo>> {
        let mut conn = PgConnection::establish(database_url)
            .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

        let results = symbols::table
            .filter(symbols::overview.eq(true))
            .filter(symbols::sec_type.eq("Equity"))
            .select((symbols::sid, symbols::symbol))
            .load::<(i64, String)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(results.into_iter().map(|(sid, symbol)| SymbolInfo { sid, symbol }).collect())
    }

    /// Convert API response to database-ready structure, capturing ALL ticker sentiments
    fn convert_news_to_data(&self, news: &NewsSentiment, symbols: &[SymbolInfo]) -> Vec<NewsData> {
        // Load ALL symbols from database for sentiment mapping
        let symbol_to_sid: HashMap<String, i64> = match self.load_all_symbols() {
            Ok(map) => {
                info!("Loaded {} symbols for sentiment mapping", map.len());
                map
            },
            Err(e) => {
                warn!("Failed to load symbol mapping: {}", e);
                HashMap::new()
            }
        };

        let mut result = Vec::new();
        let mut global_missed_tickers: Vec<String> = Vec::new();

        for symbol_info in symbols {
            let mut news_items: Vec<NewsItem> = Vec::new();

            for article in &news.feed {
                // Check if article mentions this primary symbol
                let is_relevant = article.ticker_sentiment.iter()
                    .any(|ts| ts.ticker == symbol_info.symbol);

                if !is_relevant {
                    continue;
                }

                // Parse published time
                let published_time = parse_article_time(&article.time_published);

                // Extract domain from URL
                let source_domain = extract_domain(&article.url);

                // Capture ALL ticker sentiments from the article, not just the primary symbol
                let mut ticker_sentiments: Vec<TickerSentimentData> = Vec::new();
                let mut article_missed_tickers: Vec<String> = Vec::new();

                for ts in &article.ticker_sentiment {
                    // Look up the SID for each ticker mentioned
                    match symbol_to_sid.get(&ts.ticker) {
                        Some(&sid) => {
                            ticker_sentiments.push(TickerSentimentData {
                                sid,
                                relevance_score: ts.relevance_score.parse::<f32>().unwrap_or(0.0),
                                sentiment_score: ts.ticker_sentiment_score.parse::<f32>().unwrap_or(0.0),
                                sentiment_label: ts.ticker_sentiment_label.clone(),
                            });
                        }
                        None => {
                            if !article_missed_tickers.contains(&ts.ticker) {
                                article_missed_tickers.push(ts.ticker.clone());
                            }
                            if !global_missed_tickers.contains(&ts.ticker) {
                                global_missed_tickers.push(ts.ticker.clone());
                            }
                        }
                    }
                }

                if !article_missed_tickers.is_empty() {
                    debug!("Article '{}' mentions tickers not in database: {:?}",
                           article.title, article_missed_tickers);
                }

                // Build topics
                let topics: Vec<TopicData> = article.topics
                    .iter()
                    .map(|topic| TopicData {
                        name: topic.topic.clone(),
                        relevance_score: topic.relevance_score.parse::<f32>().unwrap_or(0.0),
                    })
                    .collect();

                // Create NewsItem with ALL ticker sentiments
                let news_item = NewsItem {
                    source_name: article.source.clone(),
                    source_domain: source_domain.clone(),
                    author_name: article.authors.first()
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string()),
                    article_hash: generate_article_hash(&article.url),
                    category: if article.category_within_source.is_empty() ||
                        article.category_within_source == "n/a" {
                        "General".to_string()
                    } else {
                        article.category_within_source.clone()
                    },
                    title: article.title.clone(),
                    url: article.url.clone(),
                    summary: article.summary.clone(),
                    banner_url: article.banner_image.clone().unwrap_or_default(),
                    published_time,
                    overall_sentiment_score: article.overall_sentiment_score as f32,
                    overall_sentiment_label: article.overall_sentiment_label.clone(),
                    ticker_sentiments,  // This now contains ALL mentioned tickers with resolved SIDs
                    topics,
                    // Optional fields
                    source_link: Some(source_domain.clone()),
                    release_time: Some(published_time.and_utc().timestamp()),
                    author_description: None,
                    author_avatar_url: None,
                    feature_image: article.banner_image.clone(),
                    author_nick_name: None,
                };

                news_items.push(news_item);
            }

            // Only create NewsData if we have items for this symbol
            if !news_items.is_empty() {
                let news_data = NewsData {
                    sid: symbol_info.sid,
                    hash_id: generate_batch_hash(symbol_info.sid, &news_items),
                    timestamp: Utc::now(),
                    items: news_items,
                };

                result.push(news_data);
            }
        }

        // Log summary of missed tickers
        if !global_missed_tickers.is_empty() {
            info!("Tickers mentioned but not in database: {} unique - {:?}",
                  global_missed_tickers.len(), global_missed_tickers);
            info!("To capture sentiments for these tickers, add them to the symbols table");
        }

        result
    }
}

#[async_trait]
impl DataLoader for NewsLoader {
    type Input = NewsLoaderInput;
    type Output = NewsLoaderOutput;

    async fn load(
        &self,
        context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output> {
        let mut output = NewsLoaderOutput::default();

        if input.symbols.is_empty() {
            return Ok(output);
        }

        // Build ticker string
        let tickers: Vec<String> = input.symbols.iter().map(|s| s.symbol.clone()).collect();
        let tickers_str = tickers.join(",");

        // Build topic string if provided
        let topics_str = self.config.topics.as_ref().map(|t| t.join(","));

        // Format time parameters - AlphaVantage expects YYYYMMDDTHHMM format
        let time_from = input.time_from.map(|t| t.format("%Y%m%dT%H%M").to_string());
        let time_to = input.time_to.map(|t| t.format("%Y%m%dT%H%M").to_string());

        // Get limit
        let limit = self.config.limit;

        info!("📡 Fetching news from API for {} symbols", input.symbols.len());
        output.api_calls += 1;

        // Fetch from API
        let news = match context.client.news().news_sentiment(
            Some(&tickers_str),
            topics_str.as_deref(),
            time_from.as_deref(),
            time_to.as_deref(),
            self.config.sort_order.as_deref(),
            limit,
        ).await {
            Ok(news) => news,
            Err(e) => {
                output.errors.push(format!("Failed to fetch news: {}", e));
                return Ok(output);
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

    fn name(&self) -> &'static str {
        "NewsLoader"
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
        return Ok(NewsLoaderOutput::default());
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

// Helper functions
fn extract_domain(url: &str) -> String {
    url.parse::<url::Url>()
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_else(|| "unknown".to_string())
}

fn generate_article_hash(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn generate_batch_hash(sid: i64, items: &[NewsItem]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    sid.hash(&mut hasher);
    items.len().hash(&mut hasher);
    Utc::now().timestamp().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn parse_article_time(time_str: &str) -> NaiveDateTime {
    // Parse format: "20240315T123456"
    NaiveDateTime::parse_from_str(time_str, "%Y%m%dT%H%M%S")
        .unwrap_or_else(|_| {
            // Fallback to current time if parsing fails
            Utc::now().naive_utc()
        })
}