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

//! Crypto news and sentiment loader for `av-cli load crypto-news`.
//!
//! Fetches news articles and AlphaVantage sentiment scores for cryptocurrencies
//! from the AlphaVantage `NEWS_SENTIMENT` endpoint, then persists them via
//! [`super::news_utils::save_news_to_database`] into the news-related tables
//! (news overviews, feeds, articles, ticker sentiments, topics).
//!
//! ## Symbol Selection
//!
//! Three mutually exclusive modes for choosing which crypto symbols to process:
//!
//! - **`--symbols BTC,ETH,...`** — Explicit list. Each symbol is looked up in
//!   `symbols` (filtered to `sec_type = "Cryptocurrency"`); missing ones are
//!   logged as warnings but do not abort.
//! - **`--all`** — Every crypto symbol in the database.
//! - **`--top N`** — Top N cryptocurrencies by `crypto_overview_basic.market_cap_rank`
//!   ascending. Requires that `crypto_overview_basic` has been populated (e.g.,
//!   via `av-cli update crypto-metadata` or `av-cli load crypto-overview`).
//!
//! At least one mode is required — the command errors if none is set.
//!
//! ## Data Flow
//!
//! ```text
//! symbols (sec_type = "Cryptocurrency")
//!   │  [optionally joined with crypto_overview_basic for --top]
//!   ▼
//! get_top_crypto_symbols_by_market_cap()
//! get_all_crypto_symbols()                  ── one of three selectors
//! get_specific_crypto_symbols()
//!   │
//!   ▼
//! load_crypto_news()  ── AlphaVantage NEWS_SENTIMENT API + cache
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
//! ## Topic Filtering
//!
//! When `--topics` is omitted, defaults to `["blockchain"]` — appropriate for
//! crypto coverage. Other AlphaVantage topics include `defi`, `nft`,
//! `cryptocurrency`, etc.
//!
//! ## Usage
//!
//! ```bash
//! # Top 50 cryptos by market cap, last 7 days
//! av-cli load crypto-news --top 50
//!
//! # Specific symbols with custom topics
//! av-cli load crypto-news --symbols BTC,ETH,SOL --topics blockchain,defi,nft
//!
//! # All crypto symbols, last 30 days, force refresh
//! av-cli load crypto-news --all --days-back 30 --force-refresh
//!
//! # Dry run with verbose progress
//! av-cli load crypto-news --top 10 --dry-run --progress-interval 1
//! ```

use anyhow::{Result, anyhow};
use av_database_postgres::repository::DatabaseContext;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info, warn};

use av_client::AlphaVantageClient;
use av_loaders::{
  LoaderConfig, LoaderContext, NewsLoaderConfig, crypto::crypto_news_loader::load_crypto_news,
  news_loader::SymbolInfo,
};
use chrono::{Duration, Utc};

use super::news_utils::save_news_to_database;
use crate::config::Config;

/// Command-line arguments for `av-cli load crypto-news`.
///
/// Three symbol-selection flags (`--symbols`, `--all`, `--top`) are mutually
/// exclusive — clap enforces this via `conflicts_with_all`. Other flags
/// control date range, topic filtering, sort order, caching, rate limiting,
/// and error handling.
#[derive(Debug, Parser)]
pub struct CryptoNewsArgs {
  /// Comma-separated list of cryptocurrency symbols to load news for
  /// (e.g., `BTC,ETH,ADA`).
  ///
  /// Mutually exclusive with `--all` and `--top`. Symbols not found in the
  /// database are logged as warnings but do not abort the run.
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Load news for **all** crypto symbols in the database.
  ///
  /// Mutually exclusive with `--symbols` and `--top`. May result in many
  /// API calls — consider `--limit` and `--days-back` to control scope.
  #[arg(long, conflicts_with_all = ["symbols", "top"])]
  all: bool,

  /// Load news for the top N cryptocurrencies by `market_cap_rank`.
  ///
  /// Requires `crypto_overview_basic.market_cap_rank` to be populated.
  /// Mutually exclusive with `--symbols` and `--all`.
  #[arg(long, conflicts_with_all = ["symbols", "all"])]
  top: Option<usize>,

  /// Number of days of historical news to fetch. Defaults to 7.
  ///
  /// Translated to a `time_from` parameter on the AlphaVantage API.
  #[arg(short = 'd', long, default_value = "7")]
  days_back: u32,

  /// Comma-separated list of AlphaVantage news topics to filter by.
  ///
  /// Defaults to `["blockchain"]` if omitted. Other valid topics include
  /// `defi`, `nft`, `cryptocurrency`, `economy_macro`, etc.
  #[arg(short = 't', long, value_delimiter = ',')]
  topics: Option<Vec<String>>,

  /// Sort order for fetched articles.
  ///
  /// Valid values: `LATEST`, `EARLIEST`, `RELEVANCE`. Defaults to `LATEST`.
  #[arg(long, default_value = "LATEST")]
  sort: String,

  /// Maximum number of articles to fetch per symbol. Defaults to 1000.
  #[arg(short = 'l', long, default_value = "1000")]
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

  /// Continue processing remaining symbols if one fails. Defaults to `true`.
  #[arg(short = 'c', long, default_value = "true")]
  continue_on_error: bool,

  /// Fetch news from the API but skip database writes.
  #[arg(long)]
  dry_run: bool,

  /// Delay between API calls in milliseconds. Defaults to 800 ms.
  #[arg(long, default_value = "800")]
  api_delay: u64,

  /// Log progress every N symbols. Defaults to 10.
  #[arg(long, default_value = "10")]
  progress_interval: usize,
}

/// Returns the top N cryptocurrency symbols ordered by `market_cap_rank` ascending.
///
/// Joins `symbols` (filtered to `sec_type = "Cryptocurrency"`) with
/// `crypto_overview_basic` on `sid`, filters out rows with NULL ranks, and
/// orders by `market_cap_rank ASC` (lower rank = higher market cap).
///
/// Logs each selected symbol with its rank. If the result is empty, logs a
/// warning suggesting that the user run `av update crypto-metadata` first to
/// populate market cap rankings.
fn get_top_crypto_symbols_by_market_cap(
  database_url: &str,
  limit: usize,
) -> Result<Vec<SymbolInfo>> {
  use av_database_postgres::schema::{crypto_overview_basic, symbols};
  use diesel::prelude::*;

  let mut conn = PgConnection::establish(database_url)?;

  // Query joining symbols with crypto_overview_basic, ordered by market cap rank
  let results = symbols::table
    .inner_join(crypto_overview_basic::table.on(symbols::sid.eq(crypto_overview_basic::sid)))
    .filter(symbols::sec_type.eq("Cryptocurrency"))
    .filter(crypto_overview_basic::market_cap_rank.is_not_null())
    .select((symbols::sid, symbols::symbol, crypto_overview_basic::market_cap_rank))
    .order(crypto_overview_basic::market_cap_rank.asc())
    .limit(limit as i64)
    .load::<(i64, String, Option<i32>)>(&mut conn)?;

  info!("Top {} cryptocurrencies by market cap:", limit);
  let mut symbol_infos = Vec::new();

  for (sid, symbol, rank) in results {
    if let Some(rank) = rank {
      info!("  #{}: {} (SID: {})", rank, symbol, sid);
      symbol_infos.push(SymbolInfo { sid, symbol });
    }
  }

  if symbol_infos.is_empty() {
    warn!(
      "No crypto symbols found with market cap ranking. You may need to run 'av update crypto-metadata' first."
    );
  }

  Ok(symbol_infos)
}

/// Returns every cryptocurrency symbol in the database.
///
/// Queries `symbols` filtered to `sec_type = "Cryptocurrency"` with no other
/// filters or limits. Each found symbol is logged. Used by the `--all` flag.
fn get_all_crypto_symbols(database_url: &str) -> Result<Vec<SymbolInfo>> {
  use av_database_postgres::schema::symbols;
  use diesel::prelude::*;

  let mut conn = PgConnection::establish(database_url)?;

  let results = symbols::table
    .filter(symbols::sec_type.eq("Cryptocurrency"))
    .select((symbols::sid, symbols::symbol))
    .load::<(i64, String)>(&mut conn)?;

  Ok(
    results
      .into_iter()
      .map(|(sid, symbol)| {
        info!("Found {} with SID: {}", symbol, sid);
        SymbolInfo { sid, symbol }
      })
      .collect(),
  )
}

/// Looks up a specific list of crypto symbols in the database.
///
/// Queries `symbols` for rows where `symbol IN (...)` AND `sec_type =
/// "Cryptocurrency"`. Symbols in the input list that are not found in the
/// database (or are not cryptocurrencies) are logged as warnings but do not
/// cause an error. Used by the `--symbols` flag.
fn get_specific_crypto_symbols(database_url: &str, symbols: &[String]) -> Result<Vec<SymbolInfo>> {
  use av_database_postgres::schema::symbols as sym_table;
  use diesel::prelude::*;

  let mut conn = PgConnection::establish(database_url)?;

  let results = sym_table::table
    .filter(sym_table::symbol.eq_any(symbols))
    .filter(sym_table::sec_type.eq("Cryptocurrency"))
    .select((sym_table::sid, sym_table::symbol))
    .load::<(i64, String)>(&mut conn)?;

  for (sid, symbol) in &results {
    info!("Found {} with SID: {}", symbol, sid);
  }

  // Warn about symbols not found
  let found_symbols: Vec<String> = results.iter().map(|(_, s)| s.clone()).collect();
  for requested in symbols {
    if !found_symbols.contains(requested) {
      warn!("Symbol {} not found in database or not a crypto", requested);
    }
  }

  Ok(results.into_iter().map(|(sid, symbol)| SymbolInfo { sid, symbol }).collect())
}

/// Main entry point for `av-cli load crypto-news`.
///
/// Orchestrates the full crypto news loading pipeline:
///
/// 1. **Symbol selection** — Dispatches to one of three selectors based on
///    which flag is set: [`get_top_crypto_symbols_by_market_cap`] for
///    `--top`, [`get_all_crypto_symbols`] for `--all`, or
///    [`get_specific_crypto_symbols`] for `--symbols`. Errors if none is set.
/// 2. **Topic configuration** — Defaults to `["blockchain"]` if `--topics` is
///    omitted.
/// 3. **Loader configuration** — Builds [`NewsLoaderConfig`] with date range,
///    topics, sort order, limit, cache settings, retry behavior, and rate
///    limiting.
/// 4. **Time estimation** — Logs an estimated runtime based on
///    `api_delay × symbol_count`.
/// 5. **Infrastructure setup** — Creates [`AlphaVantageClient`],
///    [`DatabaseContext`], and a [`LoaderContext`] with both news and cache
///    repositories attached.
/// 6. **API loading** — Calls [`load_crypto_news`] with a date range from
///    `now - days_back` to `now`. Returns a structured output with
///    `articles_processed`, `loaded_count`, `no_data_count`, `api_calls`,
///    `data` (batches to save), and `errors`.
/// 7. **Persistence** — Unless `--dry-run`, calls
///    [`save_news_to_database`](super::news_utils::save_news_to_database)
///    which writes news overviews, feeds, articles (deduplicated), ticker
///    sentiments, and topics in a transaction.
/// 8. **Error reporting** — Logs any per-symbol errors collected by the
///    loader. Returns an error if `--continue-on-error` is `false` and any
///    occurred.
///
/// # Errors
///
/// Returns errors from: missing symbol-selection flag, database queries, API
/// client creation, news loader execution (unless `--continue-on-error`),
/// database saves, or aggregated loader errors.
pub async fn execute(args: CryptoNewsArgs, config: Config) -> Result<()> {
  info!("🚀 Starting crypto news loading process");

  // Get crypto symbols based on the specified option
  let symbols_to_process = if let Some(top_n) = args.top {
    info!("Loading top {} crypto symbols by market cap...", top_n);
    get_top_crypto_symbols_by_market_cap(&config.database_url, top_n)?
  } else if args.all {
    info!("Loading all crypto symbols from database...");
    get_all_crypto_symbols(&config.database_url)?
  } else if let Some(symbols) = args.symbols {
    info!("Loading specified crypto symbols from database...");
    get_specific_crypto_symbols(&config.database_url, &symbols)?
  } else {
    return Err(anyhow!("Either --symbols, --all, or --top must be specified"));
  };

  if symbols_to_process.is_empty() {
    warn!("No crypto symbols found to process");
    return Ok(());
  }

  info!("Found {} crypto symbols to process", symbols_to_process.len());

  // Configure topics - default to blockchain for crypto
  let topics = args.topics.unwrap_or_else(|| vec!["blockchain".to_string()]);

  // Configure news loader
  let api_delay_ms = args.api_delay;
  let news_config = NewsLoaderConfig {
    days_back: Some(args.days_back),
    topics: Some(topics.clone()),
    sort_order: Some(args.sort.clone()),
    limit: Some(args.limit),
    enable_cache: !args.no_cache,
    cache_ttl_hours: 24,
    force_refresh: args.force_refresh,
    continue_on_error: args.continue_on_error,
    api_delay_ms,
    progress_interval: args.progress_interval,
  };

  info!("Loader configuration:");
  info!("  Days back: {}", args.days_back);
  info!("  Topics: {:?}", topics);
  info!("  Sort: {}", args.sort);
  info!("  Limit: {} articles per symbol", args.limit);
  info!("  API delay: {}ms between calls", api_delay_ms);

  // Estimate time
  let estimated_minutes = (symbols_to_process.len() as f64 * api_delay_ms as f64 / 1000.0) / 60.0;
  info!("Estimated processing time: {:.1} minutes", estimated_minutes);

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

  // Create loader context with repositories
  let context = LoaderContext::new(client, LoaderConfig::default())
    .with_cache_repository(cache_repo)
    .with_news_repository(news_repo);

  // Load data from API
  info!("📡 Fetching crypto news from AlphaVantage API...");
  let output = match load_crypto_news(
    &context,
    symbols_to_process,
    news_config,
    Some(Utc::now() - Duration::days(args.days_back as i64)),
    Some(Utc::now()),
  )
  .await
  {
    Ok(output) => output,
    Err(e) => {
      error!("Failed to load crypto news: {}", e);
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
    info!("💾 Saving crypto news to database...");

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
    error!("❌ Errors during crypto news loading:");
    for error in &output.errors {
      error!("  - {}", error);
    }
    if !args.continue_on_error {
      return Err(anyhow!("Crypto news loading completed with errors"));
    }
  }

  info!("🎉 Crypto news loading completed successfully");
  Ok(())
}
