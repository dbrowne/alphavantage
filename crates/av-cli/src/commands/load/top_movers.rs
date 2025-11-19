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

use anyhow::Result;
use chrono::NaiveDate;
use clap::Args;
use std::sync::Arc;

use av_client::AlphaVantageClient;
use av_database_postgres::repository::DatabaseContext;
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, ProcessTracker,
  top_movers_loader::{TopMoversConfig, TopMoversLoader, TopMoversLoaderInput},
};

use crate::config::Config;

#[derive(Args, Debug)]
pub struct TopMoversArgs {
  /// Date to load (YYYY-MM-DD format)
  #[arg(short, long)]
  date: Option<String>,

  /// Dry run - don't save to database
  #[arg(long)]
  dry_run: bool,

  /// Show verbose output
  #[arg(short = 'v', long)]
  verbose: bool,

  /// Disable caching
  #[arg(long)]
  no_cache: bool,

  /// Force refresh (bypass cache)
  #[arg(long)]
  force_refresh: bool,

  /// Cache TTL in hours (default: 24)
  #[arg(long, default_value = "24")]
  cache_ttl: i64,
}

pub async fn execute(args: TopMoversArgs, config: Config) -> Result<()> {
  // Create API client with the correct Config type
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: 1, // Top movers is a single API call
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: false,         // Single call, no need for progress
    track_process: !args.dry_run, // Track process unless dry run
    batch_size: 1000,
  };

  // Create loader context
  let mut context = LoaderContext::new(client, loader_config);

  // Setup database context and repositories
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;

  if !args.dry_run {
    let news_repo: Arc<dyn av_database_postgres::repository::NewsRepository> =
      Arc::new(db_context.news_repository());
    context = context.with_news_repository(news_repo);

    // Setup process tracker
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Setup cache repository (even for dry run, to enable cache reads)
  if !args.no_cache {
    let cache_repo: Arc<dyn av_database_postgres::repository::CacheRepository> =
      Arc::new(db_context.cache_repository());
    context = context.with_cache_repository(cache_repo);
  }

  // Create loader configuration
  let loader_config = TopMoversConfig {
    track_missing_symbols: !args.dry_run,
    enable_cache: !args.no_cache,
    cache_ttl_hours: args.cache_ttl,
    force_refresh: args.force_refresh,
  };

  // Setup loader with database URL (None only for dry run)
  let database_url = if args.dry_run { None } else { Some(config.database_url.clone()) };
  let loader = TopMoversLoader::new(loader_config, database_url);

  // Parse date if provided
  let date = args.date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

  let input = TopMoversLoaderInput { date };

  let output = loader.load(&context, input).await?;

  // Display results
  println!("\n╔════════════════════════════════════════╗");
  println!("║       TOP MARKET MOVERS                ║");
  println!("╠════════════════════════════════════════╣");
  println!("║ Date: {:<33} ║", output.date);
  println!("║ Last Updated: {:<24} ║", output.last_updated);
  println!("╚════════════════════════════════════════╝\n");

  println!("📈 Top {} Gainers", output.gainers_count);
  if args.verbose {
    for gainer in output.raw_data.top_gainers.iter().take(5) {
      println!("   {} | ${} | {}% ↑", gainer.ticker, gainer.price, gainer.change_percentage);
    }
    println!();
  }

  println!("📉 Top {} Losers", output.losers_count);
  if args.verbose {
    for loser in output.raw_data.top_losers.iter().take(5) {
      println!("   {} | ${} | {}% ↓", loser.ticker, loser.price, loser.change_percentage);
    }
    println!();
  }

  println!("📊 Top {} Most Active", output.most_active_count);
  if args.verbose {
    for active in output.raw_data.most_actively_traded.iter().take(5) {
      println!("   {} | ${} | Vol: {}", active.ticker, active.price, active.volume);
    }
    println!();
  }

  // Show cache status
  if output.from_cache {
    println!("\n📦 Data Source: Cache (TTL: {} hours)", args.cache_ttl);
  } else {
    println!("\n🌐 Data Source: API (fresh data)");
  }

  if args.dry_run {
    println!("\n⚠️  Dry run mode - no data saved to database");
  } else {
    println!("\n✅ Database Update:");
    println!("   Records saved: {}", output.records_saved);
    if !output.missing_symbols.is_empty() {
      println!("   ⚠️  Missing symbols: {}", output.missing_symbols.len());
      println!("   📝 Missing symbols recorded: {}", output.missing_recorded);
      if args.verbose {
        println!("   Missing symbols:");
        for symbol in &output.missing_symbols {
          println!("      - {}", symbol);
        }
      }
    }
  }

  Ok(())
}
