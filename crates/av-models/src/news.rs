/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

//! News sentiment analysis data models

use serde::{Deserialize, Serialize};

/// News sentiment analysis response
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

/// Individual news article with sentiment analysis
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

/// Topic information in news articles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicInfo {
  pub topic: String,

  pub relevance_score: String,
}

/// Sentiment analysis for a specific ticker mentioned in the article
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerSentiment {
  pub ticker: String,

  pub relevance_score: String,

  pub ticker_sentiment_score: String,

  pub ticker_sentiment_label: String,
}

/// Market sentiment aggregation
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

/// Distribution of sentiment across articles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDistribution {
  pub bullish_count: u32,

  pub neutral_count: u32,

  pub bearish_count: u32,

  pub bullish_percentage: f64,

  pub neutral_percentage: f64,

  pub bearish_percentage: f64,
}

/// Ticker mention statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerMention {
  pub ticker: String,

  pub mention_count: u32,

  pub average_sentiment: f64,

  pub average_relevance: f64,

  pub dominant_sentiment: String,
}

/// Topic mention statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicMention {
  pub topic: String,

  pub mention_count: u32,

  pub average_relevance: f64,

  pub associated_sentiment: f64,
}

/// News source information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsSource {
  pub name: String,

  pub domain: String,

  pub reliability_score: Option<f64>,

  pub bias_rating: Option<String>,

  pub article_count: u32,
}

/// Sentiment trend over time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentTrend {
  pub time_period: String,

  pub data_points: Vec<SentimentDataPoint>,
}

/// Individual sentiment data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDataPoint {
  pub timestamp: String,

  pub sentiment_score: f64,

  pub article_count: u32,

  pub confidence: f64,
}

/// Custom sentiment analysis result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomSentimentAnalysis {
  pub analysis_id: String,

  pub query_params: SentimentQuery,

  pub results: MarketSentiment,

  pub created_at: String,
}

/// Query parameters for sentiment analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentQuery {
  pub topics: Option<Vec<String>>,

  pub tickers: Option<Vec<String>>,

  pub time_from: Option<String>,

  pub time_to: Option<String>,

  pub sort: Option<String>,

  pub limit: Option<u32>,
}

impl NewsArticle {
  /// Parse overall sentiment score as f64
  pub fn overall_sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    Ok(self.overall_sentiment_score)
  }

  pub fn is_bullish(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "bullish"
  }

  pub fn is_bearish(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "bearish"
  }

  pub fn is_neutral(&self) -> bool {
    self.overall_sentiment_label.to_lowercase() == "neutral"
  }

  pub fn sentiment_for_ticker(&self, ticker: &str) -> Option<&TickerSentiment> {
    self.ticker_sentiment.iter().find(|ts| ts.ticker.eq_ignore_ascii_case(ticker))
  }

  pub fn mentioned_tickers(&self) -> Vec<&str> {
    self.ticker_sentiment.iter().map(|ts| ts.ticker.as_str()).collect()
  }

  pub fn topic_relevance(&self, topic: &str) -> Option<f64> {
    self
      .topics
      .iter()
      .find(|t| t.topic.eq_ignore_ascii_case(topic))
      .and_then(|t| t.relevance_score.parse().ok())
  }
}

impl TickerSentiment {
  pub fn sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.ticker_sentiment_score.parse()
  }

  pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.relevance_score.parse()
  }

  pub fn is_bullish(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "bullish"
  }

  pub fn is_bearish(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "bearish"
  }

  pub fn is_neutral(&self) -> bool {
    self.ticker_sentiment_label.to_lowercase() == "neutral"
  }
}

impl TopicInfo {
  pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
    self.relevance_score.parse()
  }
}

impl NewsSentiment {
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

  /// Get top mentioned tickers with their sentiment
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
