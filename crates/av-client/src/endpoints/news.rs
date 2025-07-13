//! News and sentiment analysis endpoints
//!
//! This module provides access to AlphaVantage's news sentiment analysis:
//! - Real-time and historical market news
//! - Sentiment analysis with scores and labels
//! - News filtering by topics, symbols, and time ranges
//! - Market sentiment indicators

use super::{impl_endpoint_base, EndpointBase};
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::news::*;
use governor::RateLimiter;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// News and sentiment analysis endpoints
pub struct NewsEndpoints {
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
}

impl NewsEndpoints {
    /// Create a new news endpoints instance
    pub fn new(
        transport: Arc<Transport>,
        rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
    ) -> Self {
        Self { transport, rate_limiter }
    }

    /// Get news sentiment analysis
    ///
    /// Returns market news with sentiment analysis scores, topics, and relevance.
    /// Can be filtered by topics, tickers, time range, and other criteria.
    ///
    /// # Arguments
    ///
    /// * `topics` - Optional topics to filter by (e.g., "earnings", "ipo", "merger", "financial_markets")
    /// * `tickers` - Optional list of stock symbols to filter by
    /// * `time_from` - Optional start time in YYYYMMDDTHHMM format
    /// * `time_to` - Optional end time in YYYYMMDDTHHMM format
    /// * `sort` - Optional sort order: "LATEST", "EARLIEST", "RELEVANCE"
    /// * `limit` - Optional limit on number of results (1-1000, default 50)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::NewsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = NewsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get general market news sentiment
    /// let news = endpoints.news_sentiment(None, None).await?;
    /// 
    /// // Get news for specific symbols
    /// let aapl_news = endpoints.news_sentiment(
    ///     None, 
    ///     Some(vec!["AAPL".to_string(), "MSFT".to_string()])
    /// ).await?;
    /// 
    /// // Get earnings-related news
    /// let earnings_news = endpoints.news_sentiment(
    ///     Some(vec!["earnings".to_string()]), 
    ///     None
    /// ).await?;
    /// 
    /// for article in &news.feed {
    ///     println!("Title: {}", article.title);
    ///     println!("Sentiment: {} ({})", 
    ///              article.overall_sentiment_label, 
    ///              article.overall_sentiment_score);
    ///     for ticker_sentiment in &article.ticker_sentiment {
    ///         println!("  {}: {} ({})", 
    ///                  ticker_sentiment.ticker,
    ///                  ticker_sentiment.ticker_sentiment_label,
    ///                  ticker_sentiment.ticker_sentiment_score);
    ///     }
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(topics_count = topics.as_ref().map(|t| t.len()).unwrap_or(0), tickers_count = tickers.as_ref().map(|t| t.len()).unwrap_or(0)))]
    pub async fn news_sentiment(
        &self,
        topics: Option<Vec<String>>,
        tickers: Option<Vec<String>>,
    ) -> Result<NewsSentiment> {
        self.news_sentiment_with_options(topics, tickers, None, None, None, None).await
    }

    /// Get news sentiment with full filtering options
    #[instrument(skip(self), fields(topics_count = topics.as_ref().map(|t| t.len()).unwrap_or(0), tickers_count = tickers.as_ref().map(|t| t.len()).unwrap_or(0), time_from, time_to, sort, limit))]
    pub async fn news_sentiment_with_options(
        &self,
        topics: Option<Vec<String>>,
        tickers: Option<Vec<String>>,
        time_from: Option<&str>,
        time_to: Option<&str>,
        sort: Option<&str>,
        limit: Option<u32>,
    ) -> Result<NewsSentiment> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();

        if let Some(topics) = topics {
            if !topics.is_empty() {
                params.insert("topics".to_string(), topics.join(","));
            }
        }

        if let Some(tickers) = tickers {
            if !tickers.is_empty() {
                params.insert("tickers".to_string(), tickers.join(","));
            }
        }

        if let Some(time_from) = time_from {
            params.insert("time_from".to_string(), time_from.to_string());
        }

        if let Some(time_to) = time_to {
            params.insert("time_to".to_string(), time_to.to_string());
        }

        if let Some(sort) = sort {
            params.insert("sort".to_string(), sort.to_string());
        }

        if let Some(limit) = limit {
            params.insert("limit".to_string(), limit.to_string());
        }

        self.transport.get(FuncType::NewsSentiment, params).await
    }

    /// Get news sentiment for a specific time range
    ///
    /// # Arguments
    ///
    /// * `time_from` - Start time in YYYYMMDDTHHMM format
    /// * `time_to` - End time in YYYYMMDDTHHMM format
    /// * `tickers` - Optional list of stock symbols to filter by
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::NewsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = NewsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get news from last 24 hours for Apple
    /// let news = endpoints.news_sentiment_time_range(
    ///     "20240101T0000",
    ///     "20240102T0000",
    ///     Some(vec!["AAPL".to_string()])
    /// ).await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(time_from, time_to, tickers_count = tickers.as_ref().map(|t| t.len()).unwrap_or(0)))]
    pub async fn news_sentiment_time_range(
        &self,
        time_from: &str,
        time_to: &str,
        tickers: Option<Vec<String>>,
    ) -> Result<NewsSentiment> {
        self.news_sentiment_with_options(
            None,
            tickers,
            Some(time_from),
            Some(time_to),
            None,
            None,
        ).await
    }

    /// Get latest news with sentiment for specific topics
    ///
    /// # Arguments
    ///
    /// * `topics` - List of topics to filter by
    /// * `limit` - Maximum number of articles to return (1-1000)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::NewsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = NewsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get latest earnings and merger news
    /// let news = endpoints.news_sentiment_by_topics(
    ///     vec!["earnings".to_string(), "mergers_and_acquisitions".to_string()],
    ///     100
    /// ).await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(topics_count = topics.len(), limit))]
    pub async fn news_sentiment_by_topics(
        &self,
        topics: Vec<String>,
        limit: u32,
    ) -> Result<NewsSentiment> {
        self.news_sentiment_with_options(
            Some(topics),
            None,
            None,
            None,
            Some("LATEST"),
            Some(limit),
        ).await
    }

    /// Get latest news with sentiment for specific tickers
    ///
    /// # Arguments
    ///
    /// * `tickers` - List of stock symbols to get news for
    /// * `limit` - Maximum number of articles to return (1-1000)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::NewsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = NewsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get latest news for tech giants
    /// let news = endpoints.news_sentiment_by_tickers(
    ///     vec!["AAPL".to_string(), "GOOGL".to_string(), "MSFT".to_string()],
    ///     50
    /// ).await?;
    /// 
    /// // Calculate average sentiment for each ticker
    /// let mut ticker_sentiments = std::collections::HashMap::new();
    /// for article in &news.feed {
    ///     for ticker_sentiment in &article.ticker_sentiment {
    ///         let scores = ticker_sentiments
    ///             .entry(ticker_sentiment.ticker.clone())
    ///             .or_insert_with(Vec::new);
    ///         if let Ok(score) = ticker_sentiment.ticker_sentiment_score.parse::<f64>() {
    ///             scores.push(score);
    ///         }
    ///     }
    /// }
    /// 
    /// for (ticker, scores) in ticker_sentiments {
    ///     let avg_sentiment: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    ///     println!("{}: Average sentiment = {:.3}", ticker, avg_sentiment);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(tickers_count = tickers.len(), limit))]
    pub async fn news_sentiment_by_tickers(
        &self,
        tickers: Vec<String>,
        limit: u32,
    ) -> Result<NewsSentiment> {
        self.news_sentiment_with_options(
            None,
            Some(tickers),
            None,
            None,
            Some("RELEVANCE"),
            Some(limit),
        ).await
    }
}

impl_endpoint_base!(NewsEndpoints);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;

    fn create_test_endpoints() -> NewsEndpoints {
        let transport = Arc::new(Transport::new_mock());
        let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        NewsEndpoints::new(transport, rate_limiter)
    }

    #[test]
    fn test_endpoints_creation() {
        let endpoints = create_test_endpoints();
        assert_eq!(endpoints.transport.base_url(), "https://mock.alphavantage.co");
    }

    #[tokio::test]
    async fn test_rate_limit_wait() {
        let endpoints = create_test_endpoints();
        let result = endpoints.wait_for_rate_limit().await;
        assert!(result.is_ok());
    }
}
