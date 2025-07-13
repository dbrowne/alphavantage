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
    /// Article title
    pub title: String,
    
    /// Article URL
    pub url: String,
    
    /// Publication time
    pub time_published: String,
    
    /// List of authors
    pub authors: Vec<String>,
    
    /// Article summary
    pub summary: String,
    
    /// Banner image URL
    pub banner_image: Option<String>,
    
    /// News source
    pub source: String,
    
    /// Category within source
    pub category_within_source: String,
    
    /// Source domain
    pub source_domain: String,
    
    /// Topics mentioned in the article
    pub topics: Vec<TopicInfo>,
    
    /// Overall sentiment score (-1 to 1)
    pub overall_sentiment_score: String,
    
    /// Overall sentiment label (Bearish/Neutral/Bullish)
    pub overall_sentiment_label: String,
    
    /// Ticker-specific sentiment analysis
    pub ticker_sentiment: Vec<TickerSentiment>,
}

/// Topic information in news articles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicInfo {
    /// Topic name
    pub topic: String,
    
    /// Relevance score (0 to 1)
    pub relevance_score: String,
}

/// Sentiment analysis for a specific ticker mentioned in the article
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerSentiment {
    /// Stock ticker symbol
    pub ticker: String,
    
    /// Relevance score for this ticker (0 to 1)
    pub relevance_score: String,
    
    /// Sentiment score for this ticker (-1 to 1)
    pub ticker_sentiment_score: String,
    
    /// Sentiment label for this ticker (Bearish/Neutral/Bullish)
    pub ticker_sentiment_label: String,
}

/// Market sentiment aggregation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSentiment {
    /// Time period for sentiment analysis
    pub time_period: String,
    
    /// Overall market sentiment score
    pub overall_sentiment_score: f64,
    
    /// Overall market sentiment label
    pub overall_sentiment_label: String,
    
    /// Number of articles analyzed
    pub article_count: u32,
    
    /// Sentiment distribution
    pub sentiment_distribution: SentimentDistribution,
    
    /// Top mentioned tickers
    pub top_tickers: Vec<TickerMention>,
    
    /// Most discussed topics
    pub top_topics: Vec<TopicMention>,
}

/// Distribution of sentiment across articles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDistribution {
    /// Number of bullish articles
    pub bullish_count: u32,
    
    /// Number of neutral articles
    pub neutral_count: u32,
    
    /// Number of bearish articles
    pub bearish_count: u32,
    
    /// Percentage bullish
    pub bullish_percentage: f64,
    
    /// Percentage neutral
    pub neutral_percentage: f64,
    
    /// Percentage bearish
    pub bearish_percentage: f64,
}

/// Ticker mention statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickerMention {
    /// Stock ticker
    pub ticker: String,
    
    /// Number of mentions
    pub mention_count: u32,
    
    /// Average sentiment score
    pub average_sentiment: f64,
    
    /// Average relevance score
    pub average_relevance: f64,
    
    /// Dominant sentiment label
    pub dominant_sentiment: String,
}

/// Topic mention statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicMention {
    /// Topic name
    pub topic: String,
    
    /// Number of mentions
    pub mention_count: u32,
    
    /// Average relevance score
    pub average_relevance: f64,
    
    /// Associated sentiment
    pub associated_sentiment: f64,
}

/// News source information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsSource {
    /// Source name
    pub name: String,
    
    /// Source domain
    pub domain: String,
    
    /// Source reliability score
    pub reliability_score: Option<f64>,
    
    /// Source bias rating
    pub bias_rating: Option<String>,
    
    /// Articles published count
    pub article_count: u32,
}

/// Sentiment trend over time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentTrend {
    /// Time period
    pub time_period: String,
    
    /// Data points over time
    pub data_points: Vec<SentimentDataPoint>,
}

/// Individual sentiment data point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentDataPoint {
    /// Timestamp
    pub timestamp: String,
    
    /// Sentiment score at this time
    pub sentiment_score: f64,
    
    /// Number of articles
    pub article_count: u32,
    
    /// Confidence level
    pub confidence: f64,
}

/// Custom sentiment analysis result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomSentimentAnalysis {
    /// Analysis ID
    pub analysis_id: String,
    
    /// Query parameters used
    pub query_params: SentimentQuery,
    
    /// Analysis results
    pub results: MarketSentiment,
    
    /// Analysis timestamp
    pub created_at: String,
}

/// Query parameters for sentiment analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentQuery {
    /// Topics to filter by
    pub topics: Option<Vec<String>>,
    
    /// Tickers to filter by
    pub tickers: Option<Vec<String>>,
    
    /// Time range start
    pub time_from: Option<String>,
    
    /// Time range end
    pub time_to: Option<String>,
    
    /// Sort order
    pub sort: Option<String>,
    
    /// Result limit
    pub limit: Option<u32>,
}

impl NewsArticle {
    /// Parse overall sentiment score as f64
    pub fn overall_sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.overall_sentiment_score.parse()
    }
    
    /// Check if the article is bullish
    pub fn is_bullish(&self) -> bool {
        self.overall_sentiment_label.to_lowercase() == "bullish"
    }
    
    /// Check if the article is bearish
    pub fn is_bearish(&self) -> bool {
        self.overall_sentiment_label.to_lowercase() == "bearish"
    }
    
    /// Check if the article is neutral
    pub fn is_neutral(&self) -> bool {
        self.overall_sentiment_label.to_lowercase() == "neutral"
    }
    
    /// Get sentiment for a specific ticker
    pub fn sentiment_for_ticker(&self, ticker: &str) -> Option<&TickerSentiment> {
        self.ticker_sentiment.iter()
            .find(|ts| ts.ticker.eq_ignore_ascii_case(ticker))
    }
    
    /// Get all mentioned tickers
    pub fn mentioned_tickers(&self) -> Vec<&str> {
        self.ticker_sentiment.iter()
            .map(|ts| ts.ticker.as_str())
            .collect()
    }
    
    /// Get relevance score for a specific topic
    pub fn topic_relevance(&self, topic: &str) -> Option<f64> {
        self.topics.iter()
            .find(|t| t.topic.eq_ignore_ascii_case(topic))
            .and_then(|t| t.relevance_score.parse().ok())
    }
}

impl TickerSentiment {
    /// Parse sentiment score as f64
    pub fn sentiment_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.ticker_sentiment_score.parse()
    }
    
    /// Parse relevance score as f64
    pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.relevance_score.parse()
    }
    
    /// Check if sentiment is bullish
    pub fn is_bullish(&self) -> bool {
        self.ticker_sentiment_label.to_lowercase() == "bullish"
    }
    
    /// Check if sentiment is bearish
    pub fn is_bearish(&self) -> bool {
        self.ticker_sentiment_label.to_lowercase() == "bearish"
    }
    
    /// Check if sentiment is neutral
    pub fn is_neutral(&self) -> bool {
        self.ticker_sentiment_label.to_lowercase() == "neutral"
    }
}

impl TopicInfo {
    /// Parse relevance score as f64
    pub fn relevance_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.relevance_score.parse()
    }
}

impl NewsSentiment {
    /// Calculate average sentiment across all articles
    pub fn average_sentiment(&self) -> Result<f64, std::num::ParseFloatError> {
        let sentiments: Result<Vec<f64>, _> = self.feed.iter()
            .map(|article| article.overall_sentiment_as_f64())
            .collect();
        
        let sentiments = sentiments?;
        if sentiments.is_empty() {
            Ok(0.0)
        } else {
            Ok(sentiments.iter().sum::<f64>() / sentiments.len() as f64)
        }
    }
    
    /// Get sentiment distribution
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
                    ticker_stats.entry(ticker_sentiment.ticker.clone())
                        .or_default()
                        .push(sentiment);
                }
            }
        }
        
        // Calculate statistics and sort by mention count
        let mut ticker_mentions: Vec<TickerMention> = ticker_stats.into_iter()
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
mod tests {
    use super::*;
    
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
            overall_sentiment_score: "0.5".to_string(),
            overall_sentiment_label: "Bullish".to_string(),
            ticker_sentiment: vec![
                TickerSentiment {
                    ticker: "AAPL".to_string(),
                    relevance_score: "0.8".to_string(),
                    ticker_sentiment_score: "0.6".to_string(),
                    ticker_sentiment_label: "Bullish".to_string(),
                }
            ],
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
                    overall_sentiment_score: "0.5".to_string(),
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
                    overall_sentiment_score: "-0.3".to_string(),
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
                    overall_sentiment_score: "0.0".to_string(),
                    overall_sentiment_label: "Neutral".to_string(),
                    ticker_sentiment: vec![],
                },
            ],
        };
        
        let distribution = news.sentiment_distribution();
        assert_eq!(distribution.bullish_count, 1);
        assert_eq!(distribution.bearish_count, 1);
        assert_eq!(distribution.neutral_count, 1);
        assert_eq!(distribution.bullish_percentage, 100.0 / 3.0);
    }
}
