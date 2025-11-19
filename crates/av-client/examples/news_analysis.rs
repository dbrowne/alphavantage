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

//! News Sentiment Analysis Example
//!
//! This example demonstrates advanced news sentiment analysis using av-client:
//! - Market-wide sentiment analysis
//! - Sector-specific sentiment tracking
//! - Individual stock sentiment monitoring
//! - Topic-based news filtering
//! - Sentiment trend analysis
//! - Real-time news monitoring

use av_client::AlphaVantageClient;
use av_core::{Config, Error};
use av_models::news::NewsSentiment;
use std::collections::HashMap;
use tokio::time::{Duration, Instant, sleep};

/// Sentiment analysis results for a group of securities
#[derive(Debug)]
struct SentimentAnalysis {
  overall_sentiment: f64,
  sentiment_distribution: SentimentDistribution,
  ticker_sentiments: HashMap<String, TickerSentimentSummary>,
  topic_analysis: Vec<TopicSentiment>,
  recent_headlines: Vec<Headline>,
}

/// Sentiment distribution breakdown
#[derive(Debug)]
struct SentimentDistribution {
  bullish_count: u32,
  neutral_count: u32,
  bearish_count: u32,
  bullish_percentage: f64,
  neutral_percentage: f64,
  bearish_percentage: f64,
}

/// Ticker-specific sentiment summary
#[derive(Debug)]
struct TickerSentimentSummary {
  ticker: String,
  average_sentiment: f64,
  sentiment_label: String,
  mention_count: u32,
  relevance_score: f64,
}

/// Topic sentiment analysis
#[derive(Debug)]
struct TopicSentiment {
  topic: String,
  relevance: f64,
  associated_sentiment: f64,
  mention_count: u32,
}

/// News headline with metadata
#[derive(Debug)]
struct Headline {
  title: String,
  source: String,
  sentiment_label: String,
  sentiment_score: f64,
  time_published: String,
  url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  // Load configuration
  let config = Config::from_env().map_err(|e| {
    eprintln!("Failed to load configuration. Make sure ALPHA_VANTAGE_API_KEY is set.");
    e
  })?;

  // Create client
  let client = AlphaVantageClient::new(config);
  println!("📰 AlphaVantage News Sentiment Analyzer");
  println!("═══════════════════════════════════════\n");

  // 1. Analyze general market sentiment
  println!("🌍 General Market Sentiment Analysis");
  analyze_market_sentiment(&client).await?;

  // 2. Analyze technology sector sentiment
  println!("\n💻 Technology Sector Sentiment");
  let tech_stocks = vec!["AAPL", "GOOGL", "MSFT", "NVDA", "TSLA", "META", "NFLX"];
  analyze_sector_sentiment(&client, "Technology", &tech_stocks).await?;

  // 3. Analyze financial sector sentiment
  println!("\n🏦 Financial Sector Sentiment");
  let financial_stocks = vec!["C", "BAC", "WFC", "GS", "MS"];
  analyze_sector_sentiment(&client, "financial_markets", &financial_stocks).await?;

  // 4. Analyze specific earnings-related news
  println!("\n📊 Earnings-Related News Analysis");
  analyze_earnings_news(&client).await?;

  // 5. Analyze cryptocurrency sentiment
  println!("\n₿ Cryptocurrency News Analysis");
  analyze_crypto_sentiment(&client).await?;

  // 6. Monitor breaking news sentiment
  println!("\n⚡ Real-time News Monitoring");
  monitor_breaking_news(&client).await?;

  println!("\n✅ News sentiment analysis complete!");
  Ok(())
}

/// Analyze general market sentiment
async fn analyze_market_sentiment(client: &AlphaVantageClient) -> Result<(), Error> {
  println!("📈 Fetching general market news...");

  let news: NewsSentiment = client
    .news()
    .news_sentiment(
      None,           // tickers
      None,           // topics
      None,           // time_from
      None,           // time_to
      Some("LATEST"), // sort
      Some(50),       // limit
    )
    .await?;
  let analysis = analyze_sentiment(&news, None);
  display_sentiment_analysis(&analysis, "General Market");

  Ok(())
}

/// Analyze sentiment for a specific sector
async fn analyze_sector_sentiment(
  client: &AlphaVantageClient,
  sector_name: &str,
  tickers: &[&str],
) -> Result<(), Error> {
  println!("📊 Fetching {} sector news...", sector_name);

  let ticker_strings: Vec<String> = tickers.iter().map(|&s| s.to_string()).collect();

  let tickers_str = ticker_strings.join(",");
  let news = client
    .news()
    .news_sentiment(
      Some(&tickers_str), // tickers
      None,               // topics
      None,               // time_from
      None,               // time_to
      None,               // sort
      Some(80),           // limit
    )
    .await?;

  let analysis = analyze_sentiment(&news, Some(ticker_strings));
  display_sentiment_analysis(&analysis, &format!("{} Sector", sector_name));

  // Show top performing stocks by sentiment
  println!("\n🏆 Top {} Stocks by Sentiment:", sector_name);
  let mut ticker_sentiments: Vec<_> = analysis.ticker_sentiments.values().collect();
  ticker_sentiments.sort_by(|a, b| b.average_sentiment.partial_cmp(&a.average_sentiment).unwrap());

  for (i, ticker_sentiment) in ticker_sentiments.iter().take(3).enumerate() {
    let emoji = if ticker_sentiment.average_sentiment > 0.3 {
      "🟢"
    } else if ticker_sentiment.average_sentiment < -0.3 {
      "🔴"
    } else {
      "🟡"
    };

    println!(
      "  {}. {} {} {:.3} ({} mentions)",
      i + 1,
      emoji,
      ticker_sentiment.ticker,
      ticker_sentiment.average_sentiment,
      ticker_sentiment.mention_count
    );
  }

  Ok(())
}

/// Analyze earnings-related news
async fn analyze_earnings_news(client: &AlphaVantageClient) -> Result<(), Error> {
  println!("📊 Fetching earnings-related news...");

  let news = client
    .news()
    .news_sentiment(
      None,                                // tickers
      Some("earnings,merger_acquisition"), // topics
      None,                                // time_from
      None,                                // time_to
      None,                                // sort
      Some(50),                            // limit
    )
    .await?;

  let analysis = analyze_sentiment(&news, None);
  display_sentiment_analysis(&analysis, "Earnings News");

  // Show earnings-specific insights
  println!("\n💡 Earnings Insights:");
  let earnings_articles: Vec<_> = news
    .feed
    .iter()
    .filter(|article| {
      article.topics.iter().any(|topic| {
        topic.topic.to_lowercase().contains("earnings")
          || topic.topic.to_lowercase().contains("quarterly")
      })
    })
    .take(5)
    .collect();

  for (i, article) in earnings_articles.iter().enumerate() {
    let sentiment_emoji = match article.overall_sentiment_label.as_str() {
      "Bullish" => "🟢",
      "Bearish" => "🔴",
      _ => "🟡",
    };

    let title = if article.title.len() > 70 {
      format!("{}...", &article.title[..67])
    } else {
      article.title.clone()
    };

    println!("  {}. {} {}", i + 1, sentiment_emoji, title);

    // Show ticker sentiments for this article
    for ticker_sentiment in article.ticker_sentiment.iter().take(2) {
      println!(
        "     {} {}: {:.3}",
        ticker_sentiment.ticker,
        ticker_sentiment.ticker_sentiment_label,
        ticker_sentiment.ticker_sentiment_score.parse::<f64>().unwrap_or(0.0)
      );
    }
  }

  Ok(())
}

/// Analyze cryptocurrency sentiment
async fn analyze_crypto_sentiment(client: &AlphaVantageClient) -> Result<(), Error> {
  println!("₿ Fetching cryptocurrency news...");

  let news = client
    .news()
    .news_sentiment(
      None,
      Some("Blockchain"), // topics
      None,               // time_from
      None,               // time_to
      None,               // sort
      Some(50),           // limit
    )
    .await?;
  let analysis = analyze_sentiment(&news, None);
  display_sentiment_analysis(&analysis, "Cryptocurrency");

  // Show crypto-specific topics
  println!("\n🔗 Top Crypto Topics:");
  for (i, topic) in analysis.topic_analysis.iter().take(5).enumerate() {
    let sentiment_emoji = if topic.associated_sentiment > 0.1 {
      "🟢"
    } else if topic.associated_sentiment < -0.1 {
      "🔴"
    } else {
      "🟡"
    };

    println!(
      "  {}. {} {} (relevance: {:.2}, sentiment: {:.3})",
      i + 1,
      sentiment_emoji,
      topic.topic,
      topic.relevance,
      topic.associated_sentiment
    );
  }

  Ok(())
}

/// Monitor real-time breaking news
async fn monitor_breaking_news(client: &AlphaVantageClient) -> Result<(), Error> {
  println!("⚡ Monitoring latest breaking news...");

  let start_time = Instant::now();
  let mut previous_articles = Vec::new();

  // Monitor for 30 seconds (in a real application, this would run continuously)
  while start_time.elapsed() < Duration::from_secs(30) {
    let news = client
      .news()
      .news_sentiment(
        None,                                // tickers
        Some("earnings,merger_acquisition"), // topics
        None,                                // time_from
        None,                                // time_to
        None,                                // sort
        Some(50),                            // limit
      )
      .await?;

    // Check for new articles
    let mut new_articles = Vec::new();
    for article in &news.feed {
      if !previous_articles.iter().any(|prev: &String| prev == &article.url) {
        new_articles.push(article);
      }
    }

    if !new_articles.is_empty() {
      println!("\n🔔 {} new articles detected:", new_articles.len());
      for article in &new_articles {
        let sentiment_emoji = match article.overall_sentiment_label.as_str() {
          "Bullish" => "🟢",
          "Bearish" => "🔴",
          _ => "🟡",
        };

        let title = if article.title.len() > 60 {
          format!("{}...", &article.title[..57])
        } else {
          article.title.clone()
        };

        println!("  {} {} - {}", sentiment_emoji, title, article.source);

        // Show tickers mentioned
        if !article.ticker_sentiment.is_empty() {
          let tickers: Vec<_> =
            article.ticker_sentiment.iter().take(3).map(|ts| &ts.ticker).collect();

          println!(
            "    Mentions: {}",
            tickers.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
          );
        }
      }
    }

    // Update our tracking list
    previous_articles = news.feed.iter().map(|a| a.url.clone()).collect();

    // Wait before next check
    sleep(Duration::from_secs(10)).await;
    println!("⏱️ Checking for updates...");
  }

  println!("⏹️ Monitoring stopped");
  Ok(())
}

/// Analyze sentiment from news data
fn analyze_sentiment(news: &NewsSentiment, tickers: Option<Vec<String>>) -> SentimentAnalysis {
  // Calculate overall sentiment
  let overall_sentiment = news.average_sentiment().unwrap_or(0.0);

  // Calculate sentiment distribution
  let distribution = news.sentiment_distribution();
  let _total_articles = news.feed.len() as f64;

  let sentiment_distribution = SentimentDistribution {
    bullish_count: distribution.bullish_count,
    neutral_count: distribution.neutral_count,
    bearish_count: distribution.bearish_count,
    bullish_percentage: distribution.bullish_percentage,
    neutral_percentage: distribution.neutral_percentage,
    bearish_percentage: distribution.bearish_percentage,
  };

  // Analyze ticker-specific sentiment
  let mut ticker_sentiments = HashMap::new();
  if let Some(ticker_list) = tickers {
    for ticker in ticker_list {
      let mut sentiments = Vec::new();
      let mut relevances = Vec::new();
      let mut mention_count = 0;

      for article in &news.feed {
        for ticker_sentiment in &article.ticker_sentiment {
          if ticker_sentiment.ticker == ticker {
            if let Ok(sentiment) = ticker_sentiment.ticker_sentiment_score.parse::<f64>() {
              sentiments.push(sentiment);
            }
            if let Ok(relevance) = ticker_sentiment.relevance_score.parse::<f64>() {
              relevances.push(relevance);
            }
            mention_count += 1;
          }
        }
      }

      if !sentiments.is_empty() {
        let avg_sentiment = sentiments.iter().sum::<f64>() / sentiments.len() as f64;
        let avg_relevance = relevances.iter().sum::<f64>() / relevances.len() as f64;
        let sentiment_label = if avg_sentiment > 0.35 {
          "Bullish".to_string()
        } else if avg_sentiment < -0.35 {
          "Bearish".to_string()
        } else {
          "Neutral".to_string()
        };

        ticker_sentiments.insert(
          ticker.clone(),
          TickerSentimentSummary {
            ticker: ticker.clone(),
            average_sentiment: avg_sentiment,
            sentiment_label,
            mention_count,
            relevance_score: avg_relevance,
          },
        );
      }
    }
  }

  // Analyze topics
  let mut topic_counts: HashMap<String, (f64, f64, u32)> = HashMap::new();
  for article in &news.feed {
    let article_sentiment = article.overall_sentiment_as_f64().unwrap_or(0.0);
    for topic in &article.topics {
      let relevance = topic.relevance_as_f64().unwrap_or(0.0);
      let entry = topic_counts.entry(topic.topic.clone()).or_insert((0.0, 0.0, 0));
      entry.0 += relevance;
      entry.1 += article_sentiment;
      entry.2 += 1;
    }
  }

  let mut topic_analysis: Vec<TopicSentiment> = topic_counts
    .into_iter()
    .map(|(topic, (total_relevance, total_sentiment, count))| TopicSentiment {
      topic,
      relevance: total_relevance / count as f64,
      associated_sentiment: total_sentiment / count as f64,
      mention_count: count,
    })
    .collect();

  topic_analysis.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

  // Extract recent headlines
  let recent_headlines: Vec<Headline> = news
    .feed
    .iter()
    .take(10)
    .map(|article| Headline {
      title: article.title.clone(),
      source: article.source.clone(),
      sentiment_label: article.overall_sentiment_label.clone(),
      sentiment_score: article.overall_sentiment_as_f64().unwrap_or(0.0),
      time_published: article.time_published.clone(),
      url: article.url.clone(),
    })
    .collect();

  SentimentAnalysis {
    overall_sentiment,
    sentiment_distribution,
    ticker_sentiments,
    topic_analysis,
    recent_headlines,
  }
}

/// Display sentiment analysis results
fn display_sentiment_analysis(analysis: &SentimentAnalysis, category: &str) {
  println!("\n📊 {} Sentiment Analysis Results:", category);
  println!("─────────────────────────────────────────");

  // Overall sentiment
  let overall_emoji = if analysis.overall_sentiment > 0.1 {
    "🟢"
  } else if analysis.overall_sentiment < -0.1 {
    "🔴"
  } else {
    "🟡"
  };

  println!("Overall Sentiment: {} {:.3}", overall_emoji, analysis.overall_sentiment);

  // Distribution
  println!("Distribution:");
  println!(
    "  🟢 Bullish: {}% ({} articles)",
    analysis.sentiment_distribution.bullish_percentage,
    analysis.sentiment_distribution.bullish_count
  );
  println!(
    "  🟡 Neutral: {}% ({} articles)",
    analysis.sentiment_distribution.neutral_percentage,
    analysis.sentiment_distribution.neutral_count
  );
  println!(
    "  🔴 Bearish: {}% ({} articles)",
    analysis.sentiment_distribution.bearish_percentage,
    analysis.sentiment_distribution.bearish_count
  );

  // Recent headlines
  println!("\n📰 Recent Headlines:");
  for (i, headline) in analysis.recent_headlines.iter().take(5).enumerate() {
    let sentiment_emoji = match headline.sentiment_label.as_str() {
      "Bullish" => "🟢",
      "Bearish" => "🔴",
      _ => "🟡",
    };

    let title = if headline.title.len() > 65 {
      format!("{}...", &headline.title[..62])
    } else {
      headline.title.clone()
    };

    println!("  {}. {} {} ({})", i + 1, sentiment_emoji, title, headline.source);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sentiment_distribution_calculation() {
    // Test sentiment distribution calculations
    let bullish_count = 30;
    let neutral_count = 20;
    let bearish_count = 10;
    let total = (bullish_count + neutral_count + bearish_count) as f64;

    let distribution = SentimentDistribution {
      bullish_count,
      neutral_count,
      bearish_count,
      bullish_percentage: (bullish_count as f64 / total) * 100.0,
      neutral_percentage: (neutral_count as f64 / total) * 100.0,
      bearish_percentage: (bearish_count as f64 / total) * 100.0,
    };

    assert_eq!(distribution.bullish_percentage, 50.0);
    assert_eq!(distribution.neutral_percentage, 33.333333333333336);
    assert_eq!(distribution.bearish_percentage, 16.666666666666668);
  }

  #[test]
  fn test_headline_truncation() {
    let long_title = "This is a very long headline that should be truncated when displayed to users because it exceeds the maximum length";
    let truncated = if long_title.len() > 65 {
      format!("{}...", &long_title[..62])
    } else {
      long_title.to_string()
    };

    assert!(truncated.len() <= 65);
    assert!(truncated.ends_with("..."));
  }
}
