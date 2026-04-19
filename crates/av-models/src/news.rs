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

//! News sentiment analysis data models.
//!
//! This module provides structs for the Alpha Vantage `NEWS_SENTIMENT`
//! endpoint response and companion types for aggregated sentiment analysis.
//!
//! # Type categories
//!
//! ## API response types (direct deserialization)
//!
//! | Type               | JSON path / purpose                                    |
//! |--------------------|--------------------------------------------------------|
//! | [`NewsSentiment`]  | Top-level response: items count, definitions, feed     |
//! | [`NewsArticle`]    | `.feed[]` — article with title, URL, sentiment, topics |
//! | [`TopicInfo`]      | `.feed[].topics[]` — topic name + relevance score      |
//! | [`TickerSentiment`]| `.feed[].ticker_sentiment[]` — per-ticker scores       |
//!
//! ## Aggregation / analytics types (client-side)
//!
//! | Type                      | Purpose                                         |
//! |---------------------------|-------------------------------------------------|
//! | [`MarketSentiment`]       | Aggregated sentiment for a time period           |
//! | [`SentimentDistribution`] | Bullish/neutral/bearish counts and percentages   |
//! | [`TickerMention`]         | Per-ticker aggregated mention stats              |
//! | [`TopicMention`]          | Per-topic aggregated mention stats               |
//! | [`NewsSource`]            | Source metadata with reliability/bias info        |
//! | [`SentimentTrend`]        | Time-series of sentiment data points             |
//! | [`SentimentDataPoint`]    | Single point in a sentiment trend                |
//! | [`CustomSentimentAnalysis`] | Analysis result with query parameters          |
//! | [`SentimentQuery`]        | Query parameters for custom analysis             |
//!
//! # Helper methods
//!
//! Rich helper methods are provided on [`NewsArticle`], [`TickerSentiment`],
//! [`TopicInfo`], and [`NewsSentiment`] for sentiment classification,
//! ticker lookup, and aggregate computation.

use serde::{Deserialize, Serialize};

// ─── API response types ─────────────────────────────────────────────────────

/// Top-level response from the `NEWS_SENTIMENT` endpoint.
///
/// Contains metadata definitions and a `feed` array of [`NewsArticle`] entries.
///
/// # Helper methods
///
/// - [`average_sentiment`](NewsSentiment::average_sentiment) — mean overall sentiment across all articles.
/// - [`sentiment_distribution`](NewsSentiment::sentiment_distribution) — bullish/neutral/bearish breakdown.
/// - [`top_tickers`](NewsSentiment::top_tickers) — most-mentioned tickers ranked by frequency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsSentiment {
  /// Number of articles
  pub items: String,

  /// Sentiment score statistics
  pub sentiment_score_definition: String,

  /// Relevance score statistics  
  pub relevance_score_definition: String,

  /// List of news articles with sentiment
  pub feed: Vec<NewsArticle>,
}

/// A single news article with content, metadata, and sentiment analysis.
///
/// The `overall_sentiment_score` is a numeric value in `[-1.0, 1.0]` and
/// `overall_sentiment_label` is `"Bullish"`, `"Bearish"`, `"Neutral"`, or
/// a more granular label like `"Somewhat-Bullish"`.
///
/// # Helper methods
///
/// - [`is_bullish`](NewsArticle::is_bullish) / [`is_bearish`](NewsArticle::is_bearish) / [`is_neutral`](NewsArticle::is_neutral) — label checks.
/// - [`sentiment_for_ticker`](NewsArticle::sentiment_for_ticker) — find a specific ticker's sentiment.
/// - [`mentioned_tickers`](NewsArticle::mentioned_tickers) — list all ticker symbols mentioned.
/// - [`topic_relevance`](NewsArticle::topic_relevance) — get a topic's relevance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsArticle {
  pub title: String,

  pub url: String,

  pub time_published: String,

  pub authors: Vec<String>,

  pub summary: String,

  pub banner_image: Option<String>,

  pub source: String,

  pub category_within_source: String,

  pub source_domain: String,

  pub topics: Vec<TopicInfo>,

  pub overall_sentiment_score: f64,

  pub overall_sentiment_label: String,

  pub ticker_sentiment: Vec<TickerSentiment>,
}

/// A topic tag associated with a news article, with a relevance score.
///
/// `relevance_score` is a string in `["0.0", "1.0"]` — use
/// [`relevance_as_f64`](TopicInfo::relevance_as_f64) to parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicInfo {
  pub topic: String,

  pub relevance_score: String,
}

/// Per-ticker sentiment scores within a news article.
///
/// Each article may mention multiple tickers; this struct captures the
/// relevance and sentiment for one ticker mention.
///
/// # Helper methods
///
/// - [`sentiment_as_f64`](TickerSentiment::sentiment_as_f64) / [`relevance_as_f64`](TickerSentiment::relevance_as_f64) — parse string scores.
/// - [`is_bullish`](TickerSentiment::is_bullish) / [`is_bearish`](TickerSentiment::is_bearish) / [`is_neutral`](TickerSentiment::is_neutral) — label checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerSentiment {
  pub ticker: String,

  pub relevance_score: String,

  pub ticker_sentiment_score: String,

  pub ticker_sentiment_label: String,
}

// ─── Aggregation / analytics types ──────────────────────────────────────────

/// Aggregated market sentiment over a time period.
///
/// Client-side type that combines overall score, sentiment distribution,
/// top tickers, and top topics into a single analytics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSentiment {
  pub time_period: String,

  pub overall_sentiment_score: f64,

  pub overall_sentiment_label: String,

  pub article_count: u32,

  pub sentiment_distribution: SentimentDistribution,

  pub top_tickers: Vec<TickerMention>,

  pub top_topics: Vec<TopicMention>,
}

/// Breakdown of article sentiment labels into counts and percentages.
///
/// Computed by [`NewsSentiment::sentiment_distribution`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDistribution {
  pub bullish_count: u32,

  pub neutral_count: u32,

  pub bearish_count: u32,

  pub bullish_percentage: f64,

  pub neutral_percentage: f64,

  pub bearish_percentage: f64,
}

/// Aggregated mention and sentiment statistics for a single ticker.
///
/// Computed by [`NewsSentiment::top_tickers`]. `dominant_sentiment` is
/// `"Bullish"` if avg > 0.1, `"Bearish"` if avg < -0.1, else `"Neutral"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerMention {
  pub ticker: String,

  pub mention_count: u32,

  pub average_sentiment: f64,

  pub average_relevance: f64,

  pub dominant_sentiment: String,
}

/// Aggregated mention and sentiment statistics for a single topic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicMention {
  pub topic: String,

  pub mention_count: u32,

  pub average_relevance: f64,

  pub associated_sentiment: f64,
}

/// Metadata about a news source, including optional reliability and bias ratings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsSource {
  pub name: String,

  pub domain: String,

  pub reliability_score: Option<f64>,

  pub bias_rating: Option<String>,

  pub article_count: u32,
}

/// A time-series of sentiment measurements over a named period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentTrend {
  pub time_period: String,

  pub data_points: Vec<SentimentDataPoint>,
}

/// A single data point in a [`SentimentTrend`], with a confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDataPoint {
  pub timestamp: String,

  pub sentiment_score: f64,

  pub article_count: u32,

  pub confidence: f64,
}

/// A saved custom sentiment analysis result, bundling query parameters
/// with computed [`MarketSentiment`] results and a creation timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomSentimentAnalysis {
  pub analysis_id: String,

  pub query_params: SentimentQuery,

  pub results: MarketSentiment,

  pub created_at: String,
}

/// Query parameters for a custom sentiment analysis request.
///
/// All fields are optional — omitted fields apply no filter for that dimension.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentQuery {
  pub topics: Option<Vec<String>>,

  pub tickers: Option<Vec<String>>,

  pub time_from: Option<String>,

  pub time_to: Option<String>,

  pub sort: Option<String>,

  pub limit: Option<u32>,
}

// ─── Helper methods ─────────────────────────────────────────────────────────

/// Sentiment classification and ticker/topic lookup helpers for [`NewsArticle`].
impl NewsArticle {
  /// Returns the overall sentiment score (already `f64`, wrapped in `Ok`
  /// for API consistency with string-based parsing methods).
  pub fn overall_sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    Ok(self.overall_sentiment_score)
  }

  /// Returns `true` if the overall sentiment label is `"Bullish"` (case-insensitive).
  pub fn is_bullish(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "bullish"
  }

  /// Returns `true` if the overall sentiment label is `"Bearish"` (case-insensitive).
  pub fn is_bearish(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "bearish"
  }

  /// Returns `true` if the overall sentiment label is `"Neutral"` (case-insensitive).
  pub fn is_neutral(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "neutral"
  }

  /// Finds the [`TickerSentiment`] entry for a specific ticker (case-insensitive).
  /// Returns `None` if the ticker is not mentioned in this article.
  pub fn sentiment_for_ticker(&self, ticker: &str) -> Option<&TickerSentiment> {
    self.ticker_sentiment.iter().find(|ts| ts.ticker.eq_ignore_ascii_case(ticker))
  }

  /// Returns the list of all ticker symbols mentioned in this article.
  pub fn mentioned_tickers(&self) -> Vec<&str> {
    self.ticker_sentiment.iter().map(|ts| ts.ticker.as_str()).collect()
  }

  /// Returns the relevance score for a specific topic (case-insensitive),
  /// or `None` if the topic is not associated with this article.
  pub fn topic_relevance(&self, topic: &str) -> Option<f64> {
    self
      .topics
      .iter()
      .find(|t| t.topic.eq_ignore_ascii_case(topic))
      .and_then(|t| t.relevance_score.parse().ok())
  }
}

/// Parsing and classification helpers for [`TickerSentiment`].
impl TickerSentiment {
  /// Parses the ticker sentiment score string as `f64`.
  pub fn sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.ticker_sentiment_score.parse()
  }

  /// Parses the relevance score string as `f64`.
  pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.relevance_score.parse()
  }

  /// Returns `true` if the ticker sentiment label is `"Bullish"`.
  pub fn is_bullish(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "bullish"
  }

  /// Returns `true` if the ticker sentiment label is `"Bearish"`.
  pub fn is_bearish(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "bearish"
  }

  /// Returns `true` if the ticker sentiment label is `"Neutral"`.
  pub fn is_neutral(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "neutral"
  }
}

/// Parsing helper for [`TopicInfo`].
impl TopicInfo {
  /// Parses the topic relevance score string as `f64`.
  pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.relevance_score.parse()
  }
}

/// Aggregate analysis methods for [`NewsSentiment`].
impl NewsSentiment {
  /// Computes the mean `overall_sentiment_score` across all articles in the feed.
  ///
  /// Returns `0.0` if the feed is empty.
  pub fn average_sentiment(&self) -> Result<f64, std::num::ParseFloatError> {
    let sentiments: Result<Vec<f64>, _> =
      self.feed.iter().map(|article| article.overall_sentiment_as_f64()).collect();

    let sentiments = sentiments?;
    if sentiments.is_empty() {
      Ok(0.0)
    } else {
      Ok(sentiments.iter().sum::<f64>() / sentiments.len() as f64)
    }
  }

  /// Computes a [`SentimentDistribution`] by counting bullish/neutral/bearish
  /// labels across all articles and converting to percentages.
  pub fn sentiment_distribution(&self) -> SentimentDistribution {
    let mut bullish_count = 0;
    let mut neutral_count = 0;
    let mut bearish_count = 0;

    for article in &self.feed {
      match article.overall_sentiment_label.to_lowercase().as_str() {
        "bullish" => bullish_count += 1,
        "neutral" => neutral_count += 1,
        "bearish" => bearish_count += 1,
        _ => {} // Unknown sentiment
      }
    }

    let total = self.feed.len() as f64;

    SentimentDistribution {
      bullish_count,
      neutral_count,
      bearish_count,
      bullish_percentage: if total > 0.0 { bullish_count as f64 / total * 100.0 } else { 0.0 },
      neutral_percentage: if total > 0.0 { neutral_count as f64 / total * 100.0 } else { 0.0 },
      bearish_percentage: if total > 0.0 { bearish_count as f64 / total * 100.0 } else { 0.0 },
    }
  }

  /// Returns the `limit` most-mentioned tickers across all articles,
  /// ranked by mention count descending.
  ///
  /// For each ticker, computes `average_sentiment` from all its mentions
  /// and assigns `dominant_sentiment` based on the ±0.1 threshold.
  /// Note: `average_relevance` is currently set to `0.0` (not computed).
  pub fn top_tickers(&self, limit: usize) -> Vec<TickerMention> {
    use std::collections::HashMap;

    let mut ticker_stats: HashMap<String, Vec<f64>> = HashMap::new();

    // Collect all ticker sentiments
    for article in &self.feed {
      for ticker_sentiment in &article.ticker_sentiment {
        if let Ok(sentiment) = ticker_sentiment.sentiment_as_f64() {
          ticker_stats.entry(ticker_sentiment.ticker.clone()).or_default().push(sentiment);
        }
      }
    }

    // Calculate statistics and sort by mention count
    let mut ticker_mentions: Vec<TickerMention> = ticker_stats
      .into_iter()
      .map(|(ticker, sentiments)| {
        let mention_count = sentiments.len() as u32;
        let average_sentiment = sentiments.iter().sum::<f64>() / sentiments.len() as f64;
        let dominant_sentiment = if average_sentiment > 0.1 {
          "Bullish".to_string()
        } else if average_sentiment < -0.1 {
          "Bearish".to_string()
        } else {
          "Neutral".to_string()
        };

        TickerMention {
          ticker,
          mention_count,
          average_sentiment,
          average_relevance: 0.0, // Would need to calculate from relevance scores
          dominant_sentiment,
        }
      })
      .collect();

    ticker_mentions.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
    ticker_mentions.truncate(limit);
    ticker_mentions
  }
}
#[cfg(test)]
use av_core::test_utils::DEFAULT_TOLERANCE;

#[cfg(test)]
mod tests {
  use super::*;
  use av_core::test_utils;

  #[test]
  fn test_news_article_sentiment() {
    let article = NewsArticle {
      title: "Apple Stock Rises".to_string(),
      url: "https://example.com".to_string(),
      time_published: "20240115T120000".to_string(),
      authors: vec!["John Doe".to_string()],
      summary: "Apple stock rises on strong earnings".to_string(),
      banner_image: None,
      source: "Financial News".to_string(),
      category_within_source: "Technology".to_string(),
      source_domain: "financialnews.com".to_string(),
      topics: vec![],
      overall_sentiment_score: 0.5,
      overall_sentiment_label: "Bullish".to_string(),
      ticker_sentiment: vec![TickerSentiment {
        ticker: "AAPL".to_string(),
        relevance_score: "0.8".to_string(),
        ticker_sentiment_score: "0.6".to_string(),
        ticker_sentiment_label: "Bullish".to_string(),
      }],
    };

    assert!(article.is_bullish());
    assert!(!article.is_bearish());
    assert_eq!(article.overall_sentiment_as_f64().unwrap(), 0.5);

    let aapl_sentiment = article.sentiment_for_ticker("AAPL").unwrap();
    assert_eq!(aapl_sentiment.sentiment_as_f64().unwrap(), 0.6);
    assert!(aapl_sentiment.is_bullish());
  }

  #[test]
  fn test_sentiment_distribution() {
    let news = NewsSentiment {
      items: "3".to_string(),
      sentiment_score_definition: "".to_string(),
      relevance_score_definition: "".to_string(),
      feed: vec![
        NewsArticle {
          title: "Bullish Article".to_string(),
          url: "".to_string(),
          time_published: "".to_string(),
          authors: vec![],
          summary: "".to_string(),
          banner_image: None,
          source: "".to_string(),
          category_within_source: "".to_string(),
          source_domain: "".to_string(),
          topics: vec![],
          overall_sentiment_score: 0.5,
          overall_sentiment_label: "Bullish".to_string(),
          ticker_sentiment: vec![],
        },
        NewsArticle {
          title: "Bearish Article".to_string(),
          url: "".to_string(),
          time_published: "".to_string(),
          authors: vec![],
          summary: "".to_string(),
          banner_image: None,
          source: "".to_string(),
          category_within_source: "".to_string(),
          source_domain: "".to_string(),
          topics: vec![],
          overall_sentiment_score: -0.3,
          overall_sentiment_label: "Bearish".to_string(),
          ticker_sentiment: vec![],
        },
        NewsArticle {
          title: "Neutral Article".to_string(),
          url: "".to_string(),
          time_published: "".to_string(),
          authors: vec![],
          summary: "".to_string(),
          banner_image: None,
          source: "".to_string(),
          category_within_source: "".to_string(),
          source_domain: "".to_string(),
          topics: vec![],
          overall_sentiment_score: 0.0,
          overall_sentiment_label: "Neutral".to_string(),
          ticker_sentiment: vec![],
        },
      ],
    };

    let distribution = news.sentiment_distribution();
    assert_eq!(distribution.bullish_count, 1);
    assert_eq!(distribution.bearish_count, 1);
    assert_eq!(distribution.neutral_count, 1);
    test_utils::assert_approx_eq(distribution.bullish_percentage, 100.0 / 3.0, DEFAULT_TOLERANCE);
  }
}
