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

use anyhow::{Result, anyhow};
use av_client::AlphaVantageClient;
use av_database_postgres::repository::DatabaseContext;
use av_loaders::{
  DataLoader,
  // Remove the SymbolInfo import - we'll use the type from NewsLoader
  LoaderConfig,
  LoaderContext,
  NewsLoader,
  NewsLoaderConfig,
  NewsLoaderInput,
};
use chrono::{Duration, Utc};
use clap::Args;
use std::sync::Arc;
use tracing::{error, info, warn};

use super::news_utils::save_news_to_database;
use crate::config::Config;

// Use the NewsSymbolInfo type alias from av_loaders
type SymbolInfo = av_loaders::news_loader::SymbolInfo;

#[derive(Args, Clone, Debug)]
pub struct NewsArgs {
  /// Load for all equity symbols with overview=true
  #[arg(long)]
  all_equity: bool,

  /// Comma-separated list of specific tickers
  #[arg(short = 's', long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Number of days back to fetch news
  #[arg(short = 'd', long, default_value = "7")]
  days_back: u32,

  /// Topics to filter by (comma-separated)
  #[arg(short = 't', long, value_delimiter = ',')]
  topics: Option<Vec<String>>,

  /// Sort order (LATEST, EARLIEST, RELEVANCE)
  #[arg(long, default_value = "LATEST")]
  sort_order: String,

  /// Maximum number of articles to fetch per symbol (default: 1000, max: 1000)
  #[arg(short, long, default_value = "1000")]
  limit: u32,

  /// Disable caching
  #[arg(long)]
  no_cache: bool,

  /// Force refresh (bypass cache)
  #[arg(long)]
  force_refresh: bool,

  /// Cache TTL in hours
  #[arg(long, default_value = "24")]
  cache_ttl_hours: i64,

  /// Continue on error instead of stopping
  #[arg(long, default_value = "true")]
  continue_on_error: bool,

  /// Stop on first error (opposite of continue-on-error)
  #[arg(long)]
  stop_on_error: bool,

  /// Dry run - fetch but don't save to database
  #[arg(long)]
  dry_run: bool,

  /// Delay between API calls in milliseconds (default: 800ms for ~75 requests/minute)
  #[arg(long, default_value = "800")]
  api_delay_ms: u64,

  /// Process only first N symbols (for testing)
  #[arg(long)]
  symbol_limit: Option<usize>,
}

/// Main execute function with inline persistence
pub async fn execute(args: NewsArgs, config: Config) -> Result<()> {
  info!("Starting news sentiment loader");

  // Validate limit
  if args.limit > 1000 {
    return Err(anyhow!("Limit cannot exceed 1000 (API maximum)"));
  }
  if args.limit < 1 {
    return Err(anyhow!("Limit must be at least 1"));
  }

  let continue_on_error = if args.stop_on_error { false } else { args.continue_on_error };

  // Create API client
  let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

  // Create database context and repositories
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow!("Failed to create database context: {}", e))?;
  let news_repo: Arc<dyn av_database_postgres::repository::NewsRepository> =
    Arc::new(db_context.news_repository());
  let cache_repo: Arc<dyn av_database_postgres::repository::CacheRepository> =
    Arc::new(db_context.cache_repository());

  // Get symbols to process
  let mut symbols_to_process = if args.all_equity {
    info!("Loading all equity symbols with overview=true");
    NewsLoader::get_equity_symbols_with_overview(&news_repo).await?
  } else if let Some(ref symbol_list) = args.symbols {
    info!("Loading specific symbols: {:?}", symbol_list);
    get_specific_symbols(&news_repo, symbol_list).await?
  } else {
    return Err(anyhow!("Must specify either --all-equity or --symbols"));
  };

  // Apply symbol limit if specified (for testing)
  if let Some(limit) = args.symbol_limit {
    symbols_to_process = symbols_to_process.into_iter().take(limit).collect();
  }

  if symbols_to_process.is_empty() {
    warn!("No symbols found to process");
    return Ok(());
  }

  // Calculate estimated time
  let api_delay_ms = args.api_delay_ms;
  let estimated_minutes = (symbols_to_process.len() as f64 * api_delay_ms as f64 / 1000.0) / 60.0;
  info!("Processing {} symbols", symbols_to_process.len());
  info!(
    "Estimated processing time: {:.1} minutes with {}ms delay between calls",
    estimated_minutes, api_delay_ms
  );

  // Configure news loader
  let news_config = NewsLoaderConfig {
    days_back: Some(args.days_back),
    topics: args.topics.clone(),
    sort_order: Some(args.sort_order.clone()),
    limit: Some(args.limit),
    enable_cache: !args.no_cache,
    cache_ttl_hours: args.cache_ttl_hours,
    force_refresh: args.force_refresh,
    continue_on_error: args.continue_on_error,
    api_delay_ms,
    progress_interval: 10,
  };

  info!("📰 News Loader Configuration:");
  info!("  Days back: {}", args.days_back);
  info!("  Limit: {} articles per symbol", args.limit);
  info!("  Sort order: {}", args.sort_order);
  info!("  Cache: {}", if args.no_cache { "disabled" } else { "enabled" });
  info!("  API delay: {}ms between calls", api_delay_ms);

  // Create loader
  let loader = NewsLoader::new(5).with_config(news_config);

  // Create input
  let input = NewsLoaderInput {
    symbols: symbols_to_process,
    time_from: Some(Utc::now() - Duration::days(args.days_back as i64)),
    time_to: Some(Utc::now()),
  };

  // Create context with repositories
  let context = LoaderContext::new(client, LoaderConfig::default())
    .with_cache_repository(cache_repo.clone())
    .with_news_repository(news_repo.clone());

  // Load data from API
  info!("📡 Fetching news from AlphaVantage API...");
  let output = match loader.load(&context, input).await {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load news: {}", e);
      if !args.continue_on_error {
        return Err(e.into());
      }
      return Ok(());
    }
  };

  info!(
    "✅ API fetch complete:\n  \
        - {} articles processed\n  \
        - {} data batches created\n  \
        - {} symbols with no news\n  \
        - {} API calls made",
    output.articles_processed, output.loaded_count, output.no_data_count, output.api_calls
  );

  // Save to database
  if !args.dry_run && !output.data.is_empty() {
    info!("💾 Saving news to database...");

    let stats =
      save_news_to_database(&config.database_url, output.data, args.continue_on_error).await?;

    info!(
      "✅ Database persistence complete:\n  \
            - {} news overviews\n  \
            - {} feeds\n  \
            - {} articles\n  \
            - {} ticker sentiments\n  \
            - {} topics",
      stats.news_overviews, stats.feeds, stats.articles, stats.sentiments, stats.topics
    );
  } else if args.dry_run {
    info!("🔍 Dry run mode - no database updates performed");
    info!("Would have saved {} news data batches", output.loaded_count);
  } else if output.data.is_empty() {
    warn!("⚠️ No data to save to database");
  }

  // Report loader errors
  if !output.errors.is_empty() {
    error!("❌ Errors during news loading:");
    for error in &output.errors {
      error!("  - {}", error);
    }
    if !args.continue_on_error {
      return Err(anyhow!("News loading completed with errors"));
    }
  }

  info!("🎉 News loading completed successfully");
  Ok(())
}

/// Helper function to get specific symbols from database
async fn get_specific_symbols(
  news_repo: &Arc<dyn av_database_postgres::repository::NewsRepository>,
  symbols: &[String],
) -> Result<Vec<SymbolInfo>> {
  // Get all symbols from repository and filter
  let all_symbols =
    news_repo.get_all_symbols().await.map_err(|e| anyhow!("Failed to query symbols: {}", e))?;

  // Filter to only the requested symbols
  Ok(
    symbols
      .iter()
      .filter_map(|s| all_symbols.get(s).map(|&sid| SymbolInfo { sid, symbol: s.clone() }))
      .collect(),
  )
}
