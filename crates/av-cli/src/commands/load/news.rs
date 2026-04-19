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

//! Equity news and sentiment loader for `av-cli load news`.
//!
//! Fetches news articles and AlphaVantage sentiment scores for **equity**
//! securities from the AlphaVantage `NEWS_SENTIMENT` endpoint, then persists
//! them via [`save_news_to_database`] into the news-related tables (news
//! overviews, feeds, articles, ticker sentiments, topics).
//!
//! ## Companion Module
//!
//! This is the **equity** counterpart to
//! [`crypto_news`](super::crypto_news). The two modules share the same
//! news persistence infrastructure but query different symbol pools:
//!
//! | Module                            | Symbol selection                  |
//! |-----------------------------------|-----------------------------------|
//! | [`news`](self)                    | Equities with `overview = true`   |
//! | [`crypto_news`](super::crypto_news) | Cryptos via `--top`/`--all`/`--symbols` |
//!
//! ## Symbol Selection
//!
//! Two mutually exclusive modes — exactly one is required:
//!
//! - **`--all-equity`** — Loads news for all equity symbols where
//!   `overview = true`. Uses [`NewsLoader::get_equity_symbols_with_overview`].
//!   The `overview = true` filter ensures we only fetch news for symbols
//!   that have already been fully ingested via `av-cli load overviews`.
//! - **`--symbols TSLA,AAPL,...`** — Explicit list. Symbols are looked up via
//!   [`get_specific_symbols`] which calls [`NewsRepository::get_all_symbols`]
//!   and filters by the input list.
//!
//! ## Data Flow
//!
//! ```text
//! symbols (Equity, overview = true)
//!   │
//!   ▼  via NewsRepository
//! Vec<SymbolInfo>
//!   │
//!   ▼
//! NewsLoader::load()  ── AlphaVantage NEWS_SENTIMENT API + cache
//!   │
//!   ▼
//! save_news_to_database() (from super::news_utils)
//!   ├── news overviews
//!   ├── feeds
//!   ├── articles  (deduplicated by hash)
//!   ├── ticker sentiments
//!   └── topics
//! ```
//!
//! ## API Limits
//!
//! - `--limit` is capped at **1000** (the AlphaVantage API maximum) and must
//!   be at least 1. Validation runs at the start of [`execute`].
//! - `--api-delay-ms` defaults to 800 ms ≈ 75 calls/minute, suitable for the
//!   premium tier.
//!
//! ## Usage
//!
//! ```bash
//! # Load news for all equities with overview data
//! av-cli load news --all-equity --days-back 7
//!
//! # Load specific equity symbols with topic filter
//! av-cli load news --symbols AAPL,MSFT,GOOGL --topics earnings,technology
//!
//! # Test with first 10 symbols, dry run
//! av-cli load news --all-equity --symbol-limit 10 --dry-run
//!
//! # Force refresh, bypass cache
//! av-cli load news --symbols TSLA --force-refresh
//! ```

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

/// Type alias for [`av_loaders::news_loader::SymbolInfo`] — the lightweight
/// `(sid, symbol)` struct used by the news loader.
type SymbolInfo = av_loaders::news_loader::SymbolInfo;

/// Command-line arguments for `av-cli load news`.
///
/// Controls symbol selection (`--all-equity` vs `--symbols`), date range,
/// topic filtering, sort order, caching, rate limiting, and error handling.
#[derive(Args, Clone, Debug)]
pub struct NewsArgs {
  /// Load news for **all** equity symbols where `symbols.overview = true`.
  ///
  /// Mutually exclusive with `--symbols` (one of the two is required).
  /// Uses [`NewsLoader::get_equity_symbols_with_overview`] to query the
  /// pool — the `overview = true` filter ensures we only fetch news for
  /// symbols that have already been ingested via `av-cli load overviews`.
  #[arg(long)]
  all_equity: bool,

  /// Comma-separated list of specific equity tickers to load news for.
  ///
  /// Mutually exclusive with `--all-equity`. Symbols not found in the
  /// database are silently skipped (the filter is `HashMap::get` against the
  /// repository's symbol map).
  #[arg(short = 's', long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Number of days of historical news to fetch. Defaults to 7.
  ///
  /// Translated to a `time_from = now - days_back` parameter on the
  /// AlphaVantage API.
  #[arg(short = 'd', long, default_value = "7")]
  days_back: u32,

  /// Comma-separated list of AlphaVantage news topics to filter by.
  ///
  /// Optional. Valid topics include `earnings`, `ipo`, `mergers_and_acquisitions`,
  /// `financial_markets`, `economy_macro`, `economy_monetary`, `economy_fiscal`,
  /// `energy_transportation`, `finance`, `life_sciences`, `manufacturing`,
  /// `real_estate`, `retail_wholesale`, `technology`.
  #[arg(short = 't', long, value_delimiter = ',')]
  topics: Option<Vec<String>>,

  /// Sort order for fetched articles.
  ///
  /// Valid values: `LATEST`, `EARLIEST`, `RELEVANCE`. Defaults to `LATEST`.
  #[arg(long, default_value = "LATEST")]
  sort_order: String,

  /// Maximum number of articles to fetch per symbol.
  ///
  /// Defaults to 1000 (also the API maximum). Validated at the start of
  /// [`execute`] — values >1000 or <1 return an error.
  #[arg(short, long, default_value = "1000")]
  limit: u32,

  /// Disable response caching entirely.
  ///
  /// When set, every request hits the AlphaVantage API directly.
  #[arg(long)]
  no_cache: bool,

  /// Bypass the cache and fetch fresh data, but continue to write new
  /// responses into the cache.
  #[arg(long)]
  force_refresh: bool,

  /// Cache TTL in hours. Defaults to 24.
  #[arg(long, default_value = "24")]
  cache_ttl_hours: i64,

  /// Continue processing remaining symbols on error. Defaults to `true`.
  #[arg(long, default_value = "true")]
  continue_on_error: bool,

  /// Stop on the first error (semantic inverse of `--continue-on-error`).
  ///
  /// **Note**: There's a TODO marker in source — currently the binding
  /// `_continue_on_error` is computed but unused; the original
  /// `continue_on_error` is what actually controls behavior.
  #[arg(long)]
  stop_on_error: bool,

  /// Fetch news from the API but skip database writes.
  #[arg(long)]
  dry_run: bool,

  /// Delay between API calls in milliseconds.
  ///
  /// Defaults to 800 ms ≈ 75 calls/minute, suitable for AlphaVantage premium tier.
  #[arg(long, default_value = "800")]
  api_delay_ms: u64,

  /// Process only the first N symbols (useful for testing).
  ///
  /// Applied **after** symbol selection (`--all-equity` or `--symbols`).
  #[arg(long)]
  symbol_limit: Option<usize>,
}

/// Main entry point for `av-cli load news`.
///
/// Orchestrates the equity news loading pipeline:
///
/// 1. **Limit validation** — Returns an error if `--limit > 1000` or `--limit < 1`.
/// 2. **Infrastructure setup** — Creates [`AlphaVantageClient`],
///    [`DatabaseContext`], and the news/cache repositories.
/// 3. **Symbol selection** — Dispatches to one of two paths:
///    - `--all-equity` → [`NewsLoader::get_equity_symbols_with_overview`]
///    - `--symbols` → [`get_specific_symbols`]
///    Returns an error if neither flag is set.
/// 4. **Symbol limit** — Applies `--symbol-limit` (post-selection cap).
/// 5. **Time estimation** — Logs an estimated runtime based on
///    `api_delay_ms × symbol_count`.
/// 6. **Loader configuration** — Builds [`NewsLoaderConfig`] with date range,
///    topics, sort order, limit, cache settings, retry behavior, and rate
///    limiting (progress every 10 symbols).
/// 7. **API loading** — Calls [`NewsLoader::load`] (with concurrency = 5)
///    over a date range from `now - days_back` to `now`.
/// 8. **Persistence** — Unless `--dry-run`, calls [`save_news_to_database`]
///    which writes news overviews, feeds, articles (deduplicated by hash),
///    ticker sentiments, and topics in a transaction.
/// 9. **Error reporting** — Logs any per-symbol errors collected by the
///    loader. Returns an error if `--continue-on-error = false` and any
///    occurred.
///
/// # Errors
///
/// Returns errors from: limit validation, missing symbol-selection flag,
/// API client creation, database context creation, symbol query, news loader
/// execution (unless `--continue-on-error`), database save, or aggregated
/// loader errors.
pub async fn execute(args: NewsArgs, config: Config) -> Result<()> {
  info!("Starting news sentiment loader");

  // Validate limit
  if args.limit > 1000 {
    return Err(anyhow!("Limit cannot exceed 1000 (API maximum)"));
  }
  if args.limit < 1 {
    return Err(anyhow!("Limit must be at least 1"));
  }

  let _continue_on_error = if args.stop_on_error { false } else { args.continue_on_error }; //todo:: fix this

  // Create API client
  let client = Arc::new(
    AlphaVantageClient::new(config.api_config.clone())
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );

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

/// Looks up specific equity symbols by name and returns [`SymbolInfo`] structs.
///
/// Calls [`NewsRepository::get_all_symbols`] to fetch the full
/// `HashMap<symbol, sid>`, then filters by the input list. Symbols not found
/// in the map are silently dropped — no warning is logged (unlike the more
/// strict resolution paths in [`super::crypto_news`] and [`super::missing_symbols`]).
///
/// Used by [`execute`] when `--symbols` is set.
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
