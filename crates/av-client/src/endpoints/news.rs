use super::EndpointBase;
use crate::impl_endpoint_base;
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::news::*;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// News sentiment endpoints
pub struct NewsEndpoints {
  transport: Arc<Transport>,
  rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl NewsEndpoints {
  /// Create a new news endpoints instance
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
  ) -> Self {
    Self { transport, rate_limiter }
  }

  /// Get news sentiment data
  ///
  /// # Arguments
  ///
  /// * `tickers` - Optional comma-separated list of stock tickers
  /// * `topics` - Optional topics to filter by
  /// * `time_from` - Optional start time in YYYYMMDDTHHMM format
  /// * `time_to` - Optional end time in YYYYMMDDTHHMM format
  /// * `sort` - Optional sort order ("LATEST", "EARLIEST", "RELEVANCE")
  /// * `limit` - Optional limit on number of results (default 50, max 1000)
  ///
  /// # Examples
  ///
  /// ```ignore
  /// # use av_client::NewsEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = NewsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// // Get news for Apple and Microsoft
  /// let news = endpoints.news_sentiment(
  ///     Some("AAPL,MSFT"),
  ///     None,
  ///     None,
  ///     None,
  ///     Some("LATEST"),
  ///     Some(20)
  /// ).await?;
  ///
  /// for article in &news.feed {
  ///     println!("Title: {}", article.title);
  ///     println!("Sentiment: {:.2}", article.overall_sentiment_score);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self), fields(tickers, topics, time_from, time_to, sort, limit))]
  pub async fn news_sentiment(
    &self,
    tickers: Option<&str>,
    topics: Option<&str>,
    time_from: Option<&str>,
    time_to: Option<&str>,
    sort: Option<&str>,
    limit: Option<u32>,
  ) -> Result<NewsSentiment> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();

    if let Some(tickers) = tickers {
      params.insert("tickers".to_string(), tickers.to_string());
    }
    if let Some(topics) = topics {
      params.insert("topics".to_string(), topics.to_string());
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
}

impl_endpoint_base!(NewsEndpoints);
