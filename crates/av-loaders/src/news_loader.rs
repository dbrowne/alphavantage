use async_trait::async_trait;
use av_client::AlphaVantageClient;
use av_database_postgres::{
  models::news::{NewsData, NewsItem, TickerSentimentData, TopicData},
  schema::symbols,
};
use av_models::news::NewsSentiment;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::sql_query;
use diesel::sql_types;
use diesel::{
  Connection, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl, QueryableByName,
  RunQueryDsl,
};
use serde_json;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

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
  /// Continue processing on error
  pub continue_on_error: bool,
  /// Delay between API calls in milliseconds
  pub api_delay_ms: u64,
  /// Show progress every N symbols
  pub progress_interval: usize,
}

impl Default for NewsLoaderConfig {
  fn default() -> Self {
    Self {
      days_back: Some(7),
      topics: None,
      sort_order: Some("LATEST".to_string()),
      limit: Some(1000), // Higher limit per symbol
      enable_cache: true,
      cache_ttl_hours: 24,
      force_refresh: false,
      database_url: String::new(),
      continue_on_error: true,
      api_delay_ms: 800, // Default for standard tier (~75 calls per minute)
      progress_interval: 10,
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

// Cache query result structure
#[derive(Debug, Clone, QueryableByName)]
struct CacheQueryResult {
  #[diesel(sql_type = diesel::sql_types::Jsonb)]
  response_data: serde_json::Value,
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  expires_at: chrono::DateTime<chrono::Utc>,
}

/// News loader implementation
pub struct NewsLoader {
  config: NewsLoaderConfig,
  concurrent_requests: usize,
}

impl NewsLoader {
  /// Create a new news loader
  pub fn new(concurrent_requests: usize) -> Self {
    Self { config: NewsLoaderConfig::default(), concurrent_requests }
  }

  /// Set configuration
  pub fn with_config(mut self, config: NewsLoaderConfig) -> Self {
    self.config = config;
    self
  }

  /// Normalize ticker symbol by removing known prefixes
  /// This handles tickers like "CRYPTO:BTC" -> "BTC", "FOREX:USD" -> "USD"
  fn normalize_ticker(ticker: &str) -> String {
    // Remove common prefixes
    if let Some(stripped) = ticker.strip_prefix("CRYPTO:") {
      return stripped.to_string();
    }
    if let Some(stripped) = ticker.strip_prefix("FOREX:") {
      return stripped.to_string();
    }
    if let Some(stripped) = ticker.strip_prefix("INDEX:") {
      return stripped.to_string();
    }
    if let Some(stripped) = ticker.strip_prefix("COMMODITY:") {
      return stripped.to_string();
    }
    // Return as-is if no prefix
    ticker.to_string()
  }

  /// Generate cache key for news requests
  fn generate_cache_key(
    &self,
    symbol: &str,
    time_from: Option<&str>,
    time_to: Option<&str>,
    topics: Option<&str>,
  ) -> String {
    let mut hasher = DefaultHasher::new();

    // Include all parameters that affect the API response
    symbol.hash(&mut hasher);
    time_from.unwrap_or("").hash(&mut hasher);
    time_to.unwrap_or("").hash(&mut hasher);
    topics.unwrap_or("").hash(&mut hasher);
    self.config.sort_order.as_deref().unwrap_or("").hash(&mut hasher);
    self.config.limit.unwrap_or(0).hash(&mut hasher);

    format!("news_sentiment_{:x}", hasher.finish())
  }

  /// Get cached response if available and not expired
  async fn get_cached_response(&self, cache_key: &str) -> Option<NewsSentiment> {
    if !self.config.enable_cache || self.config.force_refresh {
      return None;
    }

    let mut conn = match PgConnection::establish(&self.config.database_url) {
      Ok(conn) => conn,
      Err(e) => {
        debug!("Failed to connect for cache check: {}", e);
        return None;
      }
    };

    let cached_entry: Option<CacheQueryResult> = sql_query(
      "SELECT response_data, expires_at FROM api_response_cache
             WHERE cache_key = $1 AND expires_at > NOW() AND api_source = 'alphavantage'",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .get_result(&mut conn)
    .optional()
    .unwrap_or(None);

    if let Some(cache_result) = cached_entry {
      info!("📦 Cache hit for key: {} (expires: {})", cache_key, cache_result.expires_at);

      // Try to parse the cached JSON into NewsSentiment
      match serde_json::from_value::<NewsSentiment>(cache_result.response_data) {
        Ok(news) => {
          debug!("Successfully parsed cached news sentiment");
          return Some(news);
        }
        Err(e) => {
          warn!("Failed to parse cached news sentiment: {}", e);
          return None;
        }
      }
    }

    debug!("Cache miss for key: {}", cache_key);
    None
  }

  /// Cache the API response
  async fn cache_response(&self, cache_key: &str, news: &NewsSentiment, endpoint_url: &str) {
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

    let response_json = match serde_json::to_value(news) {
      Ok(json) => json,
      Err(e) => {
        warn!("Failed to serialize news for caching: {}", e);
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
                cached_at = NOW()",
    )
    .bind::<sql_types::Text, _>(cache_key)
    .bind::<sql_types::Text, _>(endpoint_url)
    .bind::<sql_types::Jsonb, _>(&response_json)
    .bind::<sql_types::Timestamptz, _>(expires_at)
    .execute(&mut conn);

    match result {
      Ok(_) => info!("💾 Cached news response for {} (expires: {})", cache_key, expires_at),
      Err(e) => warn!("Failed to cache news response: {}", e),
    }
  }

  /// Clean expired cache entries
  pub async fn cleanup_expired_cache(&self) -> Result<usize, LoaderError> {
    let mut conn = PgConnection::establish(&self.config.database_url)
      .map_err(|e| LoaderError::DatabaseError(format!("Connection failed: {}", e)))?;

    let deleted_count = sql_query(
      "DELETE FROM api_response_cache
             WHERE expires_at < NOW() AND api_source = 'alphavantage'",
    )
    .execute(&mut conn)
    .map_err(|e| LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e)))?;

    if deleted_count > 0 {
      info!("🧹 Cleaned up {} expired news cache entries", deleted_count);
    }

    Ok(deleted_count)
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
      }
      Err(e) => {
        warn!("Failed to load symbol mapping, sentiment data may be incomplete: {}", e);
        HashMap::new()
      }
    };

    let mut result = Vec::new();
    let mut global_missed_tickers: std::collections::HashSet<String> =
      std::collections::HashSet::new();

    // Process news for each symbol
    for symbol_info in symbols {
      let mut news_items = Vec::new();

      // Process each article
      for article in &news.feed {
        let published_time = parse_article_time(&article.time_published);
        let source_domain = extract_domain(&article.url);

        // Track which tickers were not found
        let mut article_missed_tickers = Vec::new();

        // Process ALL ticker sentiments mentioned in the article
        let mut ticker_sentiments = Vec::new();
        for ticker_sent in &article.ticker_sentiment {
          // Normalize the ticker (remove CRYPTO:, FOREX: prefixes)
          let normalized_ticker = Self::normalize_ticker(&ticker_sent.ticker);

          // Try to find the SID for this ticker
          let sid = if let Some(&found_sid) = symbol_to_sid.get(&normalized_ticker) {
            found_sid
          } else if let Some(&found_sid) = symbol_to_sid.get(&ticker_sent.ticker) {
            // Also try the original ticker in case it's in the DB with prefix
            found_sid
          } else {
            // Ticker not found in database
            article_missed_tickers.push(ticker_sent.ticker.clone());
            global_missed_tickers.insert(ticker_sent.ticker.clone());
            continue; // Skip this ticker sentiment
          };

          let sentiment_data = TickerSentimentData {
            sid,
            relevance_score: ticker_sent.relevance_score.parse::<f32>().unwrap_or(0.0),
            sentiment_score: ticker_sent.ticker_sentiment_score.parse::<f32>().unwrap_or(0.0),
            sentiment_label: ticker_sent.ticker_sentiment_label.clone(),
          };
          ticker_sentiments.push(sentiment_data);
        }

        if !article_missed_tickers.is_empty() {
          debug!(
            "Article '{}' mentions tickers not in database: {:?}",
            article.title, article_missed_tickers
          );
        }

        // Build topics
        let topics: Vec<TopicData> = article
          .topics
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
          author_name: article.authors.first().cloned().unwrap_or_else(|| "Unknown".to_string()),
          article_hash: generate_article_hash(&article.url),
          category: if article.category_within_source.is_empty()
            || article.category_within_source == "n/a"
          {
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
          ticker_sentiments, // This now contains ALL mentioned tickers with resolved SIDs
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

    // Log summary of missed tickers (with normalized check)
    if !global_missed_tickers.is_empty() {
      let mut still_missing = Vec::new();
      for ticker in global_missed_tickers {
        let normalized = Self::normalize_ticker(&ticker);
        // Only report as missing if the normalized version is also not in the database
        if !symbol_to_sid.contains_key(&normalized) {
          still_missing.push(ticker);
        }
      }

      if !still_missing.is_empty() {
        info!(
          "Tickers mentioned but not in database: {} unique - {:?}",
          still_missing.len(),
          still_missing
        );
        info!("To capture sentiments for these tickers, add them to the symbols table");
      }
    }

    result
  }
}

#[async_trait]
impl DataLoader for NewsLoader {
  type Input = NewsLoaderInput;
  type Output = NewsLoaderOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    let mut output = NewsLoaderOutput::default();

    if input.symbols.is_empty() {
      return Ok(output);
    }

    info!("Processing {} symbols individually for maximum news coverage", input.symbols.len());

    let mut all_news_data = Vec::new();
    let total_symbols = input.symbols.len();

    for (idx, symbol_info) in input.symbols.iter().enumerate() {
      info!("📡 Fetching news for symbol {}/{}: {}", idx + 1, total_symbols, symbol_info.symbol);

      // Build topic string if provided
      let topics_str = self.config.topics.as_ref().map(|t| t.join(","));

      // Format time parameters
      let time_from = input.time_from.map(|t| t.format("%Y%m%dT%H%M").to_string());
      let time_to = input.time_to.map(|t| t.format("%Y%m%dT%H%M").to_string());

      // Generate cache key
      let cache_key = self.generate_cache_key(
        &symbol_info.symbol,
        time_from.as_deref(),
        time_to.as_deref(),
        topics_str.as_deref(),
      );

      // Check cache first
      if let Some(cached_news) = self.get_cached_response(&cache_key).await {
        info!("  Using cached data for {} (no API call needed)", symbol_info.symbol);
        output.cache_hits += 1;

        // Process cached data
        if cached_news.feed.is_empty() {
          debug!("  No news in cache for {}", symbol_info.symbol);
          output.no_data_count += 1;
        } else {
          info!("  Found {} cached articles for {}", cached_news.feed.len(), symbol_info.symbol);
          let batch_data = self.convert_news_to_data(&cached_news, &[symbol_info.clone()]);
          output.articles_processed += cached_news.feed.len();
          output.loaded_count += batch_data.len();
          all_news_data.extend(batch_data);
        }
      } else {
        // Cache miss - need to call API
        info!("  Cache miss - calling API for {}", symbol_info.symbol);
        output.api_calls += 1;

        // Fetch from API for this single symbol
        match context
          .client
          .news()
          .news_sentiment(
            Some(&symbol_info.symbol),
            topics_str.as_deref(),
            time_from.as_deref(),
            time_to.as_deref(),
            self.config.sort_order.as_deref(),
            self.config.limit,
          )
          .await
        {
          Ok(news) => {
            // Cache the successful response
            let endpoint_url = format!(
              "https://www.alphavantage.co/query?function=NEWS_SENTIMENT&tickers={}",
              symbol_info.symbol
            );
            self.cache_response(&cache_key, &news, &endpoint_url).await;

            if news.feed.is_empty() {
              debug!("  No news found for {}", symbol_info.symbol);
              output.no_data_count += 1;
            } else {
              info!("  Found {} articles for {}", news.feed.len(), symbol_info.symbol);

              // Convert to internal format - pass as single-element slice
              let batch_data = self.convert_news_to_data(&news, &[symbol_info.clone()]);

              output.articles_processed += news.feed.len();
              output.loaded_count += batch_data.len();
              all_news_data.extend(batch_data);
            }
          }
          Err(e) => {
            // Log the error but continue processing
            let error_msg = e.to_string();

            // Check if it's an "invalid input" error from the API
            let is_invalid_input =
              error_msg.contains("Invalid inputs") || error_msg.contains("Invalid API call");

            if is_invalid_input {
              debug!("  Symbol {} not available in news API", symbol_info.symbol);
              output.no_data_count += 1;
            } else {
              warn!("Failed to fetch news for {}: {}", symbol_info.symbol, error_msg);
              output.errors.push(format!("{}: {}", symbol_info.symbol, error_msg));

              // Only stop if continue_on_error is false AND it's not an API "invalid input" error
              if !self.config.continue_on_error {
                error!("Stopping due to error (not an invalid symbol error)");
                break;
              }
            }
          }
        }

        // Add delay between API calls to respect rate limits
        if idx < total_symbols - 1 {
          let delay_ms = self.config.api_delay_ms;

          // Show progress every N symbols
          if (idx + 1) % self.config.progress_interval == 0 {
            let elapsed_minutes = (idx + 1) as f64 * (delay_ms as f64 / 1000.0) / 60.0;
            let remaining_minutes =
              (total_symbols - idx - 1) as f64 * (delay_ms as f64 / 1000.0) / 60.0;
            info!(
              "Progress: {}/{} symbols processed. Time elapsed: {:.1}min, Remaining: {:.1}min",
              idx + 1,
              total_symbols,
              elapsed_minutes,
              remaining_minutes
            );
          }

          tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
      }
    }

    output.data = all_news_data;

    info!("✅ News loading complete:");
    info!("  - {} symbols processed", total_symbols);
    info!("  - {} total articles fetched", output.articles_processed);
    info!("  - {} symbols with no news or not in API", output.no_data_count);
    info!("  - {} API calls made", output.api_calls);
    info!("  - {} cache hits", output.cache_hits);
    if !output.errors.is_empty() {
      info!("  - {} actual errors encountered", output.errors.len());
    }

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
  let context = LoaderContext::new(client, crate::LoaderConfig::default());

  // Load data
  loader.load(&context, input).await
}

// Helper functions
fn extract_domain(url: &str) -> String {
  url
    .parse::<url::Url>()
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
  let mut hasher = DefaultHasher::new();
  sid.hash(&mut hasher);

  // Hash the article hashes to make it deterministic
  let mut article_hashes: Vec<&str> = items.iter().map(|item| item.article_hash.as_str()).collect();
  article_hashes.sort(); // Sort to ensure consistent ordering

  for hash in article_hashes {
    hash.hash(&mut hasher);
  }

  format!("{:x}", hasher.finish())
}

/// Parse article publication time from AlphaVantage format
/// Format: "YYYYMMDDTHHMMSS" or fallback formats
fn parse_article_time(time_str: &str) -> NaiveDateTime {
  // Try the primary format first: YYYYMMDDTHHMMSS
  if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y%m%dT%H%M%S") {
    return dt;
  }

  // Try alternative format with dashes: YYYY-MM-DDTHH:MM:SS
  if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S") {
    return dt;
  }

  // Try ISO format: YYYY-MM-DD HH:MM:SS
  if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
    return dt;
  }

  // If all parsing fails, return current time
  warn!("Failed to parse article time: '{}', using current time", time_str);
  Utc::now().naive_utc()
}
