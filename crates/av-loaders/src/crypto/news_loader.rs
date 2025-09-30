use crate::{
    error::{LoaderError, LoaderResult},
    loader::{DataLoader, LoaderContext},
};
use async_trait::async_trait;
use av_models::news::NewsSentiment;
use av_database_postgres::models::news::{NewsData, NewsItem, TickerSentimentData, TopicData};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Configuration for crypto news loader
#[derive(Debug, Clone)]
pub struct CryptoNewsConfig {
    pub days_back: Option<u32>,
    pub topics: Option<Vec<String>>,
    pub sort_order: Option<String>,
    pub limit: Option<u32>,
    pub enable_cache: bool,
    pub cache_ttl_hours: u32,
    pub force_refresh: bool,
    pub database_url: String,
    pub continue_on_error: bool,
    pub api_delay_ms: u64,
    pub progress_interval: usize,
    pub include_forex: bool, // Include FOREX:USD or other forex pairs
}

impl Default for CryptoNewsConfig {
    fn default() -> Self {
        Self {
            days_back: Some(7),
            topics: None,
            sort_order: Some("LATEST".to_string()),
            limit: Some(1000),
            enable_cache: true,
            cache_ttl_hours: 24,
            force_refresh: false,
            database_url: String::new(),
            continue_on_error: true,
            api_delay_ms: 800,
            progress_interval: 10,
            include_forex: false,
        }
    }
}

/// Crypto symbol information for news
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForNews {
    pub sid: i64,
    pub symbol: String,
    pub api_symbol: String, // The symbol to use in API calls (e.g., "CRYPTO:BTC")
}

/// Input for crypto news loader
#[derive(Debug, Clone)]
pub struct CryptoNewsInput {
    pub symbols: Vec<CryptoSymbolForNews>,
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,
    pub include_market_pairs: bool, // Include related trading pairs like COIN stock
}

/// Crypto news loader implementation
pub struct CryptoNewsLoader {
    config: CryptoNewsConfig,
    #[allow(dead_code)]
    concurrent_requests: usize,
}

impl CryptoNewsLoader {
    /// Create a new crypto news loader
    pub fn new(concurrent_requests: usize) -> Self {
        Self {
            config: CryptoNewsConfig::default(),
            concurrent_requests,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: CryptoNewsConfig) -> Self {
        self.config = config;
        self
    }

    /// Get crypto symbols from database
    pub fn get_crypto_symbols_from_database(database_url: &str) -> LoaderResult<Vec<CryptoSymbolForNews>> {
        use av_database_postgres::schema::{symbols, crypto_api_map};

        let mut conn = PgConnection::establish(database_url)
            .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

        // Get all active crypto symbols
        let results = symbols::table
            .left_join(crypto_api_map::table.on(
                crypto_api_map::sid.eq(symbols::sid)
                    .and(crypto_api_map::api_source.eq("AlphaVantage"))
            ))
            .filter(symbols::sec_type.eq("Cryptocurrency"))
            .filter(symbols::overview.eq(true))
            .select((
                symbols::sid,
                symbols::symbol,
                crypto_api_map::api_symbol.nullable(),
            ))
            .load::<(i64, String, Option<String>)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(results.into_iter().map(|(sid, symbol, api_symbol)| {
            // Use mapped API symbol if available, otherwise construct it
            let api_sym = api_symbol.unwrap_or_else(|| format!("CRYPTO:{}", symbol));
            CryptoSymbolForNews {
                sid,
                symbol: symbol.clone(),
                api_symbol: api_sym,
            }
        }).collect())
    }

    /// Load all symbols from database for sentiment mapping (including crypto)
    fn load_all_symbols(&self) -> LoaderResult<HashMap<String, i64>> {
        use av_database_postgres::schema::symbols;

        let mut conn = PgConnection::establish(&self.config.database_url)
            .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

        let results: Vec<(String, i64)> = symbols::table
            .select((symbols::symbol, symbols::sid))
            .load::<(String, i64)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Query failed: {}", e)))?;

        // Also add CRYPTO: prefixed versions for crypto symbols
        let mut symbol_map: HashMap<String, i64> = results.iter().cloned().collect();

        // Add CRYPTO: prefixed mappings
        let crypto_symbols: Vec<(String, i64)> = symbols::table
            .filter(symbols::sec_type.eq("Cryptocurrency"))
            .select((symbols::symbol, symbols::sid))
            .load::<(String, i64)>(&mut conn)
            .map_err(|e| LoaderError::DatabaseError(format!("Query failed: {}", e)))?;

        for (symbol, sid) in crypto_symbols {
            symbol_map.insert(format!("CRYPTO:{}", symbol), sid);
        }

        Ok(symbol_map)
    }

    /// Convert API response to database-ready structure
    fn convert_news_to_data(&self, news: &NewsSentiment, symbols: &[CryptoSymbolForNews]) -> Vec<NewsData> {
        // Load ALL symbols from database for sentiment mapping
        let symbol_to_sid: HashMap<String, i64> = match self.load_all_symbols() {
            Ok(map) => {
                info!("Loaded {} symbols for sentiment mapping (including crypto)", map.len());
                map
            },
            Err(e) => {
                warn!("Failed to load symbols for sentiment mapping: {}", e);
                HashMap::new()
            }
        };

        let mut result = Vec::new();
        let mut global_missed_tickers = HashSet::new();

        // Process news for each symbol
        for symbol_info in symbols {
            let mut news_items = Vec::new();

            for article in &news.feed {
                // Extract domain from URL
                let source_domain = extract_domain(&article.url);

                // Parse publication time
                let published_time = parse_article_time(&article.time_published);

                // Build ticker sentiments with SID mapping for ALL mentioned tickers
                let mut ticker_sentiments = Vec::new();
                let mut article_missed_tickers = Vec::new();

                for ts in &article.ticker_sentiment {
                    // Try multiple formats for matching
                    let sid = symbol_to_sid.get(&ts.ticker)
                        .or_else(|| symbol_to_sid.get(&format!("CRYPTO:{}", ts.ticker)))
                        .or_else(|| {
                            // Remove CRYPTO: prefix if present and try again
                            if ts.ticker.starts_with("CRYPTO:") {
                                let without_prefix = &ts.ticker[7..];
                                symbol_to_sid.get(without_prefix)
                            } else {
                                None
                            }
                        })
                        .copied();

                    if sid.is_none() {
                        article_missed_tickers.push(ts.ticker.clone());
                        global_missed_tickers.insert(ts.ticker.clone());
                    }

                    // TickerSentimentData only stores the sid and scores, not the ticker string
                    if let Some(sid) = sid {
                        ticker_sentiments.push(TickerSentimentData {
                            sid,
                            relevance_score: ts.relevance_score.parse::<f32>().unwrap_or(0.0),
                            sentiment_score: ts.ticker_sentiment_score.parse::<f32>().unwrap_or(0.0),
                            sentiment_label: ts.ticker_sentiment_label.clone(),
                        });
                    } else {
                        article_missed_tickers.push(ts.ticker.clone());
                        global_missed_tickers.insert(ts.ticker.clone());
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
                        "Crypto".to_string() // Default to Crypto category
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
                    ticker_sentiments,
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
            info!("Crypto tickers mentioned but not in database: {} unique",
                  global_missed_tickers.len());
            debug!("Missing tickers: {:?}", global_missed_tickers);
        }

        result
    }
}

#[async_trait]
impl DataLoader for CryptoNewsLoader {
    type Input = CryptoNewsInput;
    type Output = NewsLoaderOutput;  // Use the local type

    async fn load(
        &self,
        context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output> {
        let mut output = Self::Output::default();

        if input.symbols.is_empty() {
            return Ok(output);
        }

        info!("Processing {} crypto symbols for news coverage", input.symbols.len());

        let mut all_news_data = Vec::new();
        let total_symbols = input.symbols.len();
        let delay_ms = self.config.api_delay_ms;

        // Process in batches to avoid rate limits
        for (idx, symbol_info) in input.symbols.iter().enumerate() {
            // Progress logging
            if idx > 0 && idx % self.config.progress_interval == 0 {
                let elapsed = (idx as f64 * delay_ms as f64) / 1000.0 / 60.0;
                let remaining = ((total_symbols - idx) as f64 * delay_ms as f64) / 1000.0 / 60.0;
                info!("Progress: {}/{} symbols. Time elapsed: {:.1}min, Remaining: {:.1}min",
                      idx, total_symbols, elapsed, remaining);
            }

            info!("📡 Fetching news for crypto {}/{}: {} ({})",
                  idx + 1,
                  total_symbols,
                  symbol_info.symbol,
                  symbol_info.api_symbol);

            // Build ticker string - can include multiple symbols
            let mut tickers = vec![symbol_info.api_symbol.clone()];

            // Optionally add forex pairs
            if self.config.include_forex {
                tickers.push("FOREX:USD".to_string());
            }

            // Add market pairs if requested (e.g., COIN stock for Coinbase)
            if input.include_market_pairs {
                // This could be expanded to include known related stocks
                if symbol_info.symbol == "BTC" {
                    tickers.extend(vec!["COIN".to_string(), "MSTR".to_string()]);
                }
            }

            let tickers_str = tickers.join(",");

            // Build topic string if provided
            let topics_str = self.config.topics.as_ref().map(|t| t.join(","));

            // Format time parameters
            let time_from = input.time_from.map(|t| t.format("%Y%m%dT%H%M").to_string());
            let time_to = input.time_to.map(|t| t.format("%Y%m%dT%H%M").to_string());

            output.api_calls += 1;

            // Helper function to try fetching news
            let try_fetch_news = |tickers_param: &str| async {
                context.client.news().news_sentiment(
                    Some(tickers_param),
                    topics_str.as_deref(),
                    time_from.as_deref(),
                    time_to.as_deref(),
                    self.config.sort_order.as_deref(),
                    self.config.limit,
                ).await
            };

            // Try with the API symbol first
            let news_result = match try_fetch_news(&tickers_str).await {
                Ok(news) => Ok(news),
                Err(e) if e.to_string().contains("Invalid inputs") => {
                    // Try without CRYPTO: prefix as fallback
                    if symbol_info.api_symbol.starts_with("CRYPTO:") {
                        debug!("Retrying {} without CRYPTO: prefix", symbol_info.symbol);
                        output.api_calls += 1;
                        try_fetch_news(&symbol_info.symbol).await
                    } else {
                        Err(e)
                    }
                },
                Err(e) => Err(e),
            };

            // Process the result
            match news_result {
                Ok(news) => {
                    if news.feed.is_empty() {
                        debug!("No news found for {}", symbol_info.symbol);
                        output.no_data_count += 1;
                    } else {
                        info!("Found {} articles for {}", news.feed.len(), symbol_info.symbol);
                        output.articles_processed += news.feed.len();

                        // Convert to NewsData
                        let news_data = self.convert_news_to_data(&news, &[symbol_info.clone()]);

                        if !news_data.is_empty() {
                            output.loaded_count += news_data.len();
                            all_news_data.extend(news_data);
                        }
                    }
                },
                Err(e) => {
                    // Check if it's an "Invalid inputs" error from Alpha Vantage
                    let error_str = e.to_string();
                    if error_str.contains("Invalid inputs") || error_str.contains("missing field `items`") {
                        warn!("Symbol {} not supported by Alpha Vantage news API", symbol_info.symbol);
                        output.no_data_count += 1;
                    } else {
                        warn!("Failed to fetch news for {}: {}", symbol_info.symbol, e);
                        output.errors.push(format!("Failed to fetch news for {}: {}", symbol_info.symbol, e));
                    }

                    if !self.config.continue_on_error {
                        return Err(LoaderError::ApiError(e.to_string()));
                    }
                }
            }

            // Rate limiting delay
            if idx < total_symbols - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }

        output.data = all_news_data;

        info!("✅ Crypto news loading complete:");
        info!("  - {} symbols processed", total_symbols);
        info!("  - {} total articles fetched", output.articles_processed);
        info!("  - {} symbols with no news", output.no_data_count);
        info!("  - {} API calls made", output.api_calls);
        if !output.errors.is_empty() {
            info!("  - {} errors encountered", output.errors.len());
        }

        Ok(output)
    }

    fn name(&self) -> &'static str {
        "CryptoNewsLoader"
    }
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