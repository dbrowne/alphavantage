//! Portfolio Tracker Example
//!
//! This example demonstrates how to use av-client to:
//! - Track a portfolio of stocks
//! - Get real-time quotes and fundamental data
//! - Analyze news sentiment
//! - Calculate portfolio metrics
//! - Handle errors and rate limiting

use av_client::AlphaVantageClient;
use av_core::{Config, Error};
use av_models::{
  fundamentals::{CompanyOverview, TopGainersLosers},
  news::NewsSentiment,
  time_series::{DailyTimeSeries, GlobalQuote},
};
use std::collections::HashMap;
use tokio::time::{Duration, sleep};

/// Portfolio holding information
#[derive(Debug, Clone)]
struct Holding {
  symbol: String,
  shares: f64,
  cost_basis: f64, // Average cost per share
}

/// Portfolio analysis results
#[derive(Debug)]
struct PortfolioAnalysis {
  total_value: f64,
  total_cost: f64,
  total_gain_loss: f64,
  total_gain_loss_percent: f64,
  holdings_analysis: Vec<HoldingAnalysis>,
}

/// Individual holding analysis
#[derive(Debug)]
struct HoldingAnalysis {
  symbol: String,
  shares: f64,
  cost_basis: f64,
  current_price: f64,
  market_value: f64,
  unrealized_gain_loss: f64,
  unrealized_gain_loss_percent: f64,
  weight: f64, // Percentage of total portfolio
  sentiment_score: Option<f64>,
  pe_ratio: Option<f64>,
  dividend_yield: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  // Load configuration from environment
  let config = Config::from_env().map_err(|e| {
    eprintln!("Failed to load configuration. Make sure ALPHA_VANTAGE_API_KEY is set.");
    eprintln!("Error: {}", e);
    e
  })?;

  // Create the AlphaVantage client
  let client = AlphaVantageClient::new(config);
  println!("📊 AlphaVantage Portfolio Tracker initialized");
  println!("Rate limit: {} requests/minute\n", client.config().rate_limit);

  // Define our portfolio
  let portfolio = vec![
    Holding { symbol: "AAPL".to_string(), shares: 50.0, cost_basis: 150.00 },
    Holding { symbol: "GOOGL".to_string(), shares: 20.0, cost_basis: 2500.00 },
    Holding { symbol: "MSFT".to_string(), shares: 30.0, cost_basis: 300.00 },
    Holding { symbol: "TSLA".to_string(), shares: 10.0, cost_basis: 800.00 },
    Holding { symbol: "NVDA".to_string(), shares: 25.0, cost_basis: 400.00 },
  ];

  println!("🎯 Portfolio Holdings:");
  for holding in &portfolio {
    println!(
      "  {} - {} shares @ ${:.2} avg cost",
      holding.symbol, holding.shares, holding.cost_basis
    );
  }
  println!();

  // Analyze the portfolio
  let analysis = analyze_portfolio(&client, &portfolio).await?;
  display_portfolio_analysis(&analysis);

  // Get market overview
  println!("\n📈 Market Overview:");
  get_market_overview(&client).await?;

  // Analyze news sentiment for portfolio
  println!("\n📰 Portfolio News Sentiment:");
  analyze_portfolio_sentiment(&client, &portfolio).await?;

  // Get top movers for context
  println!("\n🚀 Market Movers:");
  get_top_movers(&client).await?;

  println!("\n✅ Portfolio analysis complete!");
  Ok(())
}

/// Analyze the entire portfolio
async fn analyze_portfolio(
  client: &AlphaVantageClient,
  portfolio: &[Holding],
) -> Result<PortfolioAnalysis, Error> {
  println!("🔍 Analyzing portfolio...");

  let mut holdings_analysis = Vec::new();
  let mut total_value = 0.0;
  let mut total_cost = 0.0;

  for holding in portfolio {
    println!("  Analyzing {}...", holding.symbol);

    // Get current quote
    let quote = get_quote_with_retry(client, &holding.symbol).await?;
    let current_price = quote
      .global_quote
      .price_as_f64()
      .map_err(|e| Error::Parse(format!("Failed to parse price: {}", e)))?;

    // Get fundamental data
    let (pe_ratio, dividend_yield) = get_fundamentals_with_retry(client, &holding.symbol).await?;

    // Get sentiment (optional, may fail)
    let sentiment_score = get_sentiment_with_retry(client, &holding.symbol).await.ok();

    // Calculate metrics
    let market_value = holding.shares * current_price;
    let cost_value = holding.shares * holding.cost_basis;
    let unrealized_gain_loss = market_value - cost_value;
    let unrealized_gain_loss_percent = (unrealized_gain_loss / cost_value) * 100.0;

    total_value += market_value;
    total_cost += cost_value;

    holdings_analysis.push(HoldingAnalysis {
      symbol: holding.symbol.clone(),
      shares: holding.shares,
      cost_basis: holding.cost_basis,
      current_price,
      market_value,
      unrealized_gain_loss,
      unrealized_gain_loss_percent,
      weight: 0.0, // Will calculate after we have total
      sentiment_score,
      pe_ratio,
      dividend_yield,
    });

    // Respect rate limits
    sleep(Duration::from_millis(1000)).await;
  }

  // Calculate weights
  for analysis in &mut holdings_analysis {
    analysis.weight = (analysis.market_value / total_value) * 100.0;
  }

  let total_gain_loss = total_value - total_cost;
  let total_gain_loss_percent = (total_gain_loss / total_cost) * 100.0;

  Ok(PortfolioAnalysis {
    total_value,
    total_cost,
    total_gain_loss,
    total_gain_loss_percent,
    holdings_analysis,
  })
}

/// Get quote with retry logic
async fn get_quote_with_retry(
  client: &AlphaVantageClient,
  symbol: &str,
) -> Result<GlobalQuote, Error> {
  for attempt in 1..=3 {
    match client.time_series().daily(symbol, "compact").await {
      Ok(daily_data) => {
        // Extract latest price from daily data
        if let Some((_, latest)) = daily_data.latest() {
          // Create a mock GlobalQuote from daily data
          // In a real implementation, you'd use the actual GLOBAL_QUOTE endpoint
          return Ok(create_mock_quote(symbol, &latest.close));
        }
      }
      Err(Error::RateLimit(_)) if attempt < 3 => {
        println!("    Rate limited, waiting...");
        client.wait_for_rate_limit().await?;
        continue;
      }
      Err(e) => return Err(e),
    }
  }
  Err(Error::Api("Failed to get quote after retries".to_string()))
}

/// Get fundamental data with retry
async fn get_fundamentals_with_retry(
  client: &AlphaVantageClient,
  symbol: &str,
) -> Result<(Option<f64>, Option<f64>), Error> {
  for attempt in 1..=3 {
    match client.fundamentals().company_overview(symbol).await {
      Ok(overview) => {
        let pe_ratio = overview.pe_ratio.parse().ok();
        let dividend_yield = overview.dividend_yield.parse().ok();
        return Ok((pe_ratio, dividend_yield));
      }
      Err(Error::RateLimit(_)) if attempt < 3 => {
        println!("    Rate limited on fundamentals, waiting...");
        client.wait_for_rate_limit().await?;
        continue;
      }
      Err(_) => {
        // Don't fail the whole analysis for missing fundamentals
        return Ok((None, None));
      }
    }
  }
  Ok((None, None))
}

/// Get sentiment with retry
async fn get_sentiment_with_retry(client: &AlphaVantageClient, symbol: &str) -> Result<f64, Error> {
  for attempt in 1..=3 {
    match client
      .news()
      .news_sentiment(
        Some(symbol), // tickers
        None,         // topics
        None,         // time_from
        None,         // time_to
        None,         // sort
        Some(20),     // limit
      )
      .await
    {
      Ok(news) => {
        if let Ok(avg_sentiment) = news.average_sentiment() {
          return Ok(avg_sentiment);
        }
      }
      Err(Error::RateLimit(_)) if attempt < 3 => {
        client.wait_for_rate_limit().await?;
        continue;
      }
      Err(_) => break,
    }
  }
  Err(Error::Api("No sentiment data available".to_string()))
}

/// Display portfolio analysis results
fn display_portfolio_analysis(analysis: &PortfolioAnalysis) {
  println!("\n💼 Portfolio Analysis Results:");
  println!("═══════════════════════════════════════════════");

  println!("📊 Portfolio Summary:");
  println!("  Total Market Value: ${:.2}", analysis.total_value);
  println!("  Total Cost Basis:   ${:.2}", analysis.total_cost);

  let gain_loss_symbol = if analysis.total_gain_loss >= 0.0 { "📈" } else { "📉" };
  println!(
    "  Total Gain/Loss:    {} ${:.2} ({:+.2}%)",
    gain_loss_symbol, analysis.total_gain_loss, analysis.total_gain_loss_percent
  );

  println!("\n📋 Individual Holdings:");
  println!("┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐");
  println!("│ Symbol  │ Shares  │  Price  │  Value  │ Gain/Loss│ Gain %  │ Weight  │   P/E   │");
  println!("├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤");

  for holding in &analysis.holdings_analysis {
    let gain_loss_symbol = if holding.unrealized_gain_loss >= 0.0 { "+" } else { "" };
    let pe_str =
      holding.pe_ratio.map(|pe| format!("{:.1}", pe)).unwrap_or_else(|| "N/A".to_string());

    println!(
      "│ {:<7} │ {:<7.1} │ ${:<6.2} │ ${:<6.0} │ {}{:<6.0} │ {:+<6.1}% │ {:<6.1}% │ {:<7} │",
      holding.symbol,
      holding.shares,
      holding.current_price,
      holding.market_value,
      gain_loss_symbol,
      holding.unrealized_gain_loss,
      holding.unrealized_gain_loss_percent,
      holding.weight,
      pe_str
    );
  }
  println!("└─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘");

  // Show sentiment analysis
  println!("\n🎭 Sentiment Analysis:");
  for holding in &analysis.holdings_analysis {
    if let Some(sentiment) = holding.sentiment_score {
      let sentiment_label = if sentiment > 0.35 {
        "🟢 Bullish"
      } else if sentiment < -0.35 {
        "🔴 Bearish"
      } else {
        "🟡 Neutral"
      };
      println!("  {}: {} ({:.3})", holding.symbol, sentiment_label, sentiment);
    } else {
      println!("  {}: No sentiment data", holding.symbol);
    }
  }
}

/// Get market overview using top gainers/losers
async fn get_market_overview(client: &AlphaVantageClient) -> Result<(), Error> {
  match client.fundamentals().top_gainers_losers().await {
    Ok(top_movers) => {
      println!("🚀 Top 3 Gainers:");
      for (i, gainer) in top_movers.top_gainers.iter().take(3).enumerate() {
        println!(
          "  {}. {} - ${} ({})",
          i + 1,
          gainer.ticker,
          gainer.price,
          gainer.change_percentage
        );
      }

      println!("\n📉 Top 3 Losers:");
      for (i, loser) in top_movers.top_losers.iter().take(3).enumerate() {
        println!("  {}. {} - ${} ({})", i + 1, loser.ticker, loser.price, loser.change_percentage);
      }
    }
    Err(e) => {
      println!("⚠️ Could not get market overview: {}", e);
    }
  }
  Ok(())
}

/// Analyze news sentiment for the portfolio
async fn analyze_portfolio_sentiment(
  client: &AlphaVantageClient,
  portfolio: &[Holding],
) -> Result<(), Error> {
  let symbols: Vec<String> = portfolio.iter().map(|h| h.symbol.clone()).collect();

  let symbols_str = symbols.join(",");
  match client
    .news()
    .news_sentiment(
      Some(&symbols_str), // tickers
      None,               // topics
      None,               // time_from
      None,               // time_to
      None,               // sort
      Some(50),           // limit
    )
    .await
  {
    Ok(news) => {
      let distribution = news.sentiment_distribution();
      println!("📊 Overall Sentiment Distribution:");
      println!("  🟢 Bullish: {:.1}%", distribution.bullish_percentage);
      println!("  🟡 Neutral: {:.1}%", distribution.neutral_percentage);
      println!("  🔴 Bearish: {:.1}%", distribution.bearish_percentage);

      println!("\n📰 Recent Headlines:");
      for (i, article) in news.feed.iter().take(5).enumerate() {
        let sentiment_emoji = if article.is_bullish() {
          "🟢"
        } else if article.is_bearish() {
          "🔴"
        } else {
          "🟡"
        };

        let title = if article.title.len() > 60 {
          format!("{}...", &article.title[..57])
        } else {
          article.title.clone()
        };

        println!("  {}. {} {}", i + 1, sentiment_emoji, title);
      }
    }
    Err(e) => {
      println!("⚠️ Could not get portfolio sentiment: {}", e);
    }
  }
  Ok(())
}

/// Get top market movers
async fn get_top_movers(client: &AlphaVantageClient) -> Result<(), Error> {
  match client.fundamentals().top_gainers_losers().await {
    Ok(top_movers) => {
      println!("📊 Most Active Stocks:");
      for (i, active) in top_movers.most_actively_traded.iter().take(5).enumerate() {
        println!("  {}. {} - ${} (Vol: {})", i + 1, active.ticker, active.price, active.volume);
      }
    }
    Err(e) => {
      println!("⚠️ Could not get top movers: {}", e);
    }
  }
  Ok(())
}

/// Create a mock GlobalQuote for demo purposes
/// In a real implementation, you'd use the actual GLOBAL_QUOTE endpoint
fn create_mock_quote(symbol: &str, price: &str) -> GlobalQuote {
  use av_models::time_series::QuoteData;

  GlobalQuote {
    global_quote: QuoteData {
      symbol: symbol.to_string(),
      open: price.to_string(),
      high: price.to_string(),
      low: price.to_string(),
      price: price.to_string(),
      volume: "1000000".to_string(),
      latest_trading_day: "2024-01-15".to_string(),
      previous_close: price.to_string(),
      change: "0.00".to_string(),
      change_percent: "0.00%".to_string(),
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_holding_creation() {
    let holding = Holding { symbol: "AAPL".to_string(), shares: 100.0, cost_basis: 150.0 };

    assert_eq!(holding.symbol, "AAPL");
    assert_eq!(holding.shares, 100.0);
    assert_eq!(holding.cost_basis, 150.0);
  }

  #[test]
  fn test_portfolio_calculations() {
    // Test portfolio calculation logic
    let current_price = 160.0;
    let shares = 100.0;
    let cost_basis = 150.0;

    let market_value = shares * current_price;
    let cost_value = shares * cost_basis;
    let gain_loss = market_value - cost_value;
    let gain_loss_percent = (gain_loss / cost_value) * 100.0;

    assert_eq!(market_value, 16000.0);
    assert_eq!(cost_value, 15000.0);
    assert_eq!(gain_loss, 1000.0);
    assert_eq!(gain_loss_percent, 6.666666666666667);
  }
}
