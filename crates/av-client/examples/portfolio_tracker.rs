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
use colored::*;
use std::io::{self, Write};

use av_models::{fundamentals::TopGainersLosers, time_series::GlobalQuote};
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
#[derive(Debug, Clone)]
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
  if !atty::is(atty::Stream::Stdout) {
    colored::control::set_override(false);
  } else {
    colored::control::set_override(true);
  }
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
  let (available, _reset_time) = client.rate_limit_status();
  println!("Rate limit: {} requests/minute\n", available);

  // Define our portfolio
  let portfolio = vec![
    Holding { symbol: "AAPL".to_string(), shares: 50.0, cost_basis: 150.00 },
    Holding { symbol: "GOOGL".to_string(), shares: 20.0, cost_basis: 2500.00 },
    Holding { symbol: "MSFT".to_string(), shares: 30.0, cost_basis: 300.00 },
    Holding { symbol: "TSLA".to_string(), shares: 10.0, cost_basis: 800.00 },
    Holding { symbol: "NVDA".to_string(), shares: 25.0, cost_basis: 400.00 },
    Holding { symbol: "INTC".to_string(), shares: 2555.0, cost_basis: 15.00 },
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

/// Display portfolio analysis with formatted columns and colored values
fn display_portfolio_analysis(analysis: &PortfolioAnalysis) {
  // Check if terminal supports colors
  colored::control::set_override(true);

  println!("\n📊 Portfolio Analysis");
  println!("{}", "═".repeat(100));

  // Portfolio Summary Section
  println!("\n💼 Portfolio Summary:");
  println!("{}", "─".repeat(100));

  // Define column widths for summary
  let label_width = 25;

  // Total Value
  println!(
    "{:<width$} ${:>12.2}",
    "Total Market Value:",
    analysis.total_value,
    width = label_width
  );

  // Total Cost
  println!("{:<width$} ${:>12.2}", "Total Cost Basis:", analysis.total_cost, width = label_width);

  // Total Gain/Loss with color
  let gain_loss_str = format!("${:>12.2}", analysis.total_gain_loss);
  let colored_gain_loss =
    if analysis.total_gain_loss >= 0.0 { gain_loss_str.green() } else { gain_loss_str.red() };
  println!("{:<width$} {}", "Total Gain/Loss:", colored_gain_loss, width = label_width);

  // Percentage with color
  let percent_str = format!("{:>11.2}%", analysis.total_gain_loss_percent);
  let colored_percent =
    if analysis.total_gain_loss_percent >= 0.0 { percent_str.green() } else { percent_str.red() };
  println!("{:<width$} {}", "Total Return:", colored_percent, width = label_width);

  // Holdings Detail Section
  println!("\n📈 Holdings Detail:");
  println!("{}", "─".repeat(100));

  // Header row
  println!(
    "{:<8} {:>10} {:>12} {:>12} {:>14} {:>14} {:>10} {:>8} {:>10}",
    "Symbol",
    "Shares",
    "Cost/Share",
    "Current",
    "Market Value",
    "Gain/Loss",
    "Return %",
    "Weight",
    "Sentiment"
  );
  println!("{}", "─".repeat(100));

  // Sort holdings by weight (largest positions first)
  let mut sorted_holdings = analysis.holdings_analysis.clone();
  sorted_holdings
    .sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

  // Display each holding
  for holding in &sorted_holdings {
    // Format basic columns
    print!(
      "{:<8} {:>10.2} {:>12.2} {:>12.2} ",
      holding.symbol, holding.shares, holding.cost_basis, holding.current_price
    );

    // Market value
    print!("{:>14.2} ", holding.market_value);

    // Gain/Loss with color
    let gain_loss = format!("{:>14.2}", holding.unrealized_gain_loss);
    if holding.unrealized_gain_loss >= 0.0 {
      print!("{} ", gain_loss.green());
    } else {
      print!("{} ", gain_loss.red());
    }

    // Return % with color
    let return_pct = format!("{:>9.2}%", holding.unrealized_gain_loss_percent);
    if holding.unrealized_gain_loss_percent >= 0.0 {
      print!("{} ", return_pct.green());
    } else {
      print!("{} ", return_pct.red());
    }

    // Weight
    print!("{:>7.1}% ", holding.weight);

    // Sentiment with color
    if let Some(sentiment) = holding.sentiment_score {
      let sentiment_str = format!("{:>9.3}", sentiment);
      if sentiment > 0.35 {
        print!("{}", sentiment_str.green());
      } else if sentiment < -0.35 {
        print!("{}", sentiment_str.red());
      } else {
        print!("{}", sentiment_str.yellow());
      }
    } else {
      print!("{:>9}", "N/A");
    }

    println!(); // End the line
  }

  // Footer
  println!("{}", "─".repeat(100));

  // Additional metrics
  println!("\n📊 Additional Metrics:");
  println!("{}", "─".repeat(50));

  // Best and worst performers
  if let Some(best) = sorted_holdings.iter().max_by(|a, b| {
    a.unrealized_gain_loss_percent.partial_cmp(&b.unrealized_gain_loss_percent).unwrap()
  }) {
    let best_str = format!("{} ({:+.2}%)", best.symbol, best.unrealized_gain_loss_percent);
    println!(
      "{:<25} {}",
      "Best Performer:",
      if best.unrealized_gain_loss_percent > 0.0 { best_str.green() } else { best_str.red() }
    );
  }

  if let Some(worst) = sorted_holdings.iter().min_by(|a, b| {
    a.unrealized_gain_loss_percent.partial_cmp(&b.unrealized_gain_loss_percent).unwrap()
  }) {
    let worst_str = format!("{} ({:+.2}%)", worst.symbol, worst.unrealized_gain_loss_percent);
    println!(
      "{:<25} {}",
      "Worst Performer:",
      if worst.unrealized_gain_loss_percent < 0.0 { worst_str.red() } else { worst_str.green() }
    );
  }

  // Diversification check
  let max_weight = sorted_holdings
    .iter()
    .map(|h| h.weight)
    .max_by(|a, b| a.partial_cmp(b).unwrap())
    .unwrap_or(0.0);

  let diversification_msg = if max_weight > 30.0 {
    format!("⚠️  Concentrated position: {} at {:.1}%", sorted_holdings[0].symbol, max_weight)
      .yellow()
  } else {
    "✅ Well diversified".green()
  };

  println!("{:<25} {}", "Diversification:", diversification_msg);
}

/// Display top gainers and losers with formatted columns
fn display_top_movers(movers: &TopGainersLosers) {
  println!("\n🚀 Market Movers");
  println!("{}", "═".repeat(80));

  // Top Gainers
  println!("\n📈 Top Gainers:");
  println!("{:<10} {:>10} {:>12} {:>10} {:>15}", "Symbol", "Price", "Change", "Change %", "Volume");
  println!("{}", "─".repeat(60));

  for gainer in movers.top_gainers.iter().take(5) {
    println!(
      "{:<10} {:>10} {:>12} {:>9}% {:>15}",
      gainer.ticker,
      gainer.price,
      gainer.change_amount.green(),
      gainer.change_percentage.green(),
      format_volume(&gainer.volume)
    );
  }

  // Top Losers
  println!("\n📉 Top Losers:");
  println!("{:<10} {:>10} {:>12} {:>10} {:>15}", "Symbol", "Price", "Change", "Change %", "Volume");
  println!("{}", "─".repeat(60));

  for loser in movers.top_losers.iter().take(5) {
    println!(
      "{:<10} {:>10} {:>12} {:>9}% {:>15}",
      loser.ticker,
      loser.price,
      loser.change_amount.red(),
      loser.change_percentage.red(),
      format_volume(&loser.volume)
    );
  }

  // Most Active
  if !movers.most_actively_traded.is_empty() {
    println!("\n📊 Most Active:");
    println!(
      "{:<10} {:>10} {:>12} {:>10} {:>15}",
      "Symbol", "Price", "Change", "Change %", "Volume"
    );
    println!("{}", "─".repeat(60));

    for active in movers.most_actively_traded.iter().take(5) {
      let change_color = if active.change_amount.starts_with('-') {
        active.change_amount.red()
      } else {
        active.change_amount.green()
      };

      let pct_color = if active.change_percentage.starts_with('-') {
        active.change_percentage.red()
      } else {
        active.change_percentage.green()
      };

      println!(
        "{:<10} {:>10} {:>12} {:>9}% {:>15}",
        active.ticker,
        active.price,
        change_color,
        pct_color,
        format_volume(&active.volume)
      );
    }
  }
}

/// Format large numbers with K/M/B suffixes
fn format_volume(volume: &str) -> String {
  if let Ok(vol) = volume.parse::<f64>() {
    if vol >= 1_000_000_000.0 {
      format!("{:.1}B", vol / 1_000_000_000.0)
    } else if vol >= 1_000_000.0 {
      format!("{:.1}M", vol / 1_000_000.0)
    } else if vol >= 1_000.0 {
      format!("{:.1}K", vol / 1_000.0)
    } else {
      format!("{:.0}", vol)
    }
  } else {
    volume.to_string()
  }
}

/// Display portfolio news sentiment with colors
fn display_portfolio_sentiment(portfolio: &[Holding], sentiment_data: &HashMap<String, f64>) {
  println!("\n📰 Portfolio News Sentiment");
  println!("{}", "═".repeat(80));

  println!(
    "{:<10} {:>20} {:>20} {:>15}",
    "Symbol", "Sentiment Score", "Sentiment Label", "Market Impact"
  );
  println!("{}", "─".repeat(70));

  let mut total_weighted_sentiment = 0.0;
  let mut total_weight = 0.0;

  for holding in portfolio {
    if let Some(&sentiment) = sentiment_data.get(&holding.symbol) {
      // Determine sentiment label and color
      let (label, color) = if sentiment > 0.35 {
        ("Bullish", "green")
      } else if sentiment < -0.35 {
        ("Bearish", "red")
      } else {
        ("Neutral", "yellow")
      };

      // Format sentiment score with color
      let score_str = format!("{:>20.3}", sentiment);
      let colored_score = match color {
        "green" => score_str.green(),
        "red" => score_str.red(),
        _ => score_str.yellow(),
      };

      // Calculate position weight for weighted average
      let position_value = holding.shares * holding.cost_basis;
      total_weighted_sentiment += sentiment * position_value;
      total_weight += position_value;

      // Market impact indicator
      let impact = if sentiment.abs() > 0.5 && holding.shares > 50.0 {
        "High".bright_yellow()
      } else if sentiment.abs() > 0.3 {
        "Medium".yellow()
      } else {
        "Low".dimmed()
      };

      println!(
        "{:<10} {} {:>20} {:>15}",
        holding.symbol,
        colored_score,
        match color {
          "green" => label.green(),
          "red" => label.red(),
          _ => label.yellow(),
        },
        impact
      );
    } else {
      println!(
        "{:<10} {:>20} {:>20} {:>15}",
        holding.symbol,
        "N/A".dimmed(),
        "No Data".dimmed(),
        "-".dimmed()
      );
    }
  }

  // Portfolio weighted sentiment
  if total_weight > 0.0 {
    let weighted_avg = total_weighted_sentiment / total_weight;
    println!("{}", "─".repeat(70));

    let avg_str = format!("{:>20.3}", weighted_avg);
    let (avg_label, avg_colored) = if weighted_avg > 0.2 {
      ("Bullish", avg_str.green())
    } else if weighted_avg < -0.2 {
      ("Bearish", avg_str.red())
    } else {
      ("Neutral", avg_str.yellow())
    };

    println!(
      "{:<10} {} {:>20} {:>15}",
      "WEIGHTED",
      avg_colored,
      if weighted_avg > 0.2 {
        avg_label.green()
      } else if weighted_avg < -0.2 {
        avg_label.red()
      } else {
        avg_label.yellow()
      },
      "Portfolio Avg".bright_white()
    );
  }
}

/// Helper function to create a progress bar for loading
fn show_loading_progress(current: usize, total: usize, symbol: &str) {
  let percentage = (current as f32 / total as f32 * 100.0) as u32;
  let filled = (percentage / 2) as usize;
  let bar = "█".repeat(filled) + &"░".repeat(50 - filled);

  print!("\r Loading {} [{}] {}% ", symbol, bar, percentage);
  io::stdout().flush().unwrap();

  if current == total {
    println!(" ✓");
  }
}

/// Display a single holding with colors (for real-time updates)
fn display_holding_update(holding: &HoldingAnalysis) {
  let gain_loss_str = format!("{:+.2}", holding.unrealized_gain_loss);
  let percent_str = format!("{:+.2}%", holding.unrealized_gain_loss_percent);

  let (gl_color, pct_color) = if holding.unrealized_gain_loss >= 0.0 {
    (gain_loss_str.green(), percent_str.green())
  } else {
    (gain_loss_str.red(), percent_str.red())
  };

  println!(
    "{}: ${:.2} {} ({}) | Weight: {:.1}%",
    holding.symbol.bright_white(),
    holding.current_price,
    gl_color,
    pct_color,
    holding.weight
  );
}

/// Clear screen and move cursor to top (for live updates)
fn clear_screen() {
  print!("\x1B[2J\x1B[1;1H");
  io::stdout().flush().unwrap();
}

/// Enable raw mode for better terminal control (optional)
#[cfg(unix)]
fn setup_terminal_colors() {
  // Enable 256 colors if available
  if let Ok(term) = std::env::var("TERM") {
    if term.contains("256color") {
      colored::control::set_override(true);
    }
  }
}

#[cfg(windows)]
fn setup_terminal_colors() {
  // Enable ANSI escape sequences on Windows 10+
  colored::control::set_virtual_terminal(true).ok();
}

/// Helper function to create a progress bar for loading
fn show_progress(current: usize, total: usize, message: &str) {
  let percentage = (current as f32 / total as f32 * 100.0) as u32;
  let filled = (percentage / 2) as usize;
  let bar = "█".repeat(filled) + &"░".repeat(50 - filled);

  print!("\r{} [{}] {}% ", message, bar, percentage);
  io::stdout().flush().unwrap();

  if current == total {
    println!(" ✓");
  }
}

/// Alternative display using a table crate for even better formatting
#[cfg(feature = "pretty-table")]
fn display_portfolio_table(analysis: &PortfolioAnalysis) {
  use prettytable::{Cell, Row, Table, format};

  let mut table = Table::new();
  table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

  // Add header
  table.set_titles(row![
    "Symbol",
    "Shares",
    "Cost",
    "Current",
    "Value",
    "Gain/Loss",
    "Return",
    "Weight",
    "Sentiment"
  ]);

  // Add rows
  for holding in &analysis.holdings_analysis {
    let gain_loss_cell = if holding.unrealized_gain_loss >= 0.0 {
      Cell::new(&format!("{:.2}", holding.unrealized_gain_loss)).style_spec("Fg")
    } else {
      Cell::new(&format!("{:.2}", holding.unrealized_gain_loss)).style_spec("Fr")
    };

    table.add_row(Row::new(vec![
      Cell::new(&holding.symbol),
      Cell::new(&format!("{:.2}", holding.shares)),
      Cell::new(&format!("{:.2}", holding.cost_basis)),
      Cell::new(&format!("{:.2}", holding.current_price)),
      Cell::new(&format!("{:.2}", holding.market_value)),
      gain_loss_cell,
      Cell::new(&format!("{:.2}%", holding.unrealized_gain_loss_percent)),
      Cell::new(&format!("{:.1}%", holding.weight)),
      Cell::new(
        &holding.sentiment_score.map(|s| format!("{:.3}", s)).unwrap_or_else(|| "N/A".to_string()),
      ),
    ]));
  }

  table.printstd();
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
