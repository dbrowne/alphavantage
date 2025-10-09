use anyhow::{Result, anyhow};
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

/// Load crypto news and sentiment data from AlphaVantage
#[derive(Debug, Parser)]
pub struct CryptoNewsArgs {
  /// Symbols to load (e.g., BTC,ETH,ADA)
  #[arg(short, long, value_delimiter = ',')]
  symbols: Option<Vec<String>>,

  /// Load news for all crypto symbols in the database
  #[arg(long, conflicts_with_all = ["symbols", "top"])]
  all: bool,

  /// Load news for top N cryptocurrencies by market cap
  #[arg(long, conflicts_with_all = ["symbols", "all"])]
  top: Option<usize>,

  /// Number of days back to fetch news
  #[arg(short = 'd', long, default_value = "7")]
  days_back: u32,

  /// Topics to filter by (blockchain, defi, nft, etc.)
  #[arg(short = 't', long, value_delimiter = ',')]
  topics: Option<Vec<String>>,

  /// Sort order (LATEST, EARLIEST, RELEVANCE)
  #[arg(long, default_value = "LATEST")]
  sort: String,

  /// Maximum articles per symbol
  #[arg(short = 'l', long, default_value = "1000")]
  limit: u32,

  /// Disable caching
  #[arg(long)]
  no_cache: bool,

  /// Force refresh (bypass cache)
  #[arg(long)]
  force_refresh: bool,

  /// Continue on error
  #[arg(short = 'c', long, default_value = "true")]
  continue_on_error: bool,

  /// Dry run (don't save to database)
  #[arg(long)]
  dry_run: bool,

  /// API delay in milliseconds
  #[arg(long, default_value = "800")]
  api_delay: u64,

  /// Show progress every N symbols
  #[arg(long, default_value = "10")]
  progress_interval: usize,
}

/// Get top N crypto symbols by market cap rank
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

/// Get all crypto symbols from database
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

/// Get specific crypto symbols from database
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

/// Execute crypto news loading command
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
    database_url: config.database_url.clone(),
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
  let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

  // Create loader context
  let context = LoaderContext::new(client, LoaderConfig::default());

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
