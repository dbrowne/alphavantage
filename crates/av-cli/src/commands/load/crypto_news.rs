use anyhow::{Result, anyhow};
use clap::Parser;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
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
  #[arg(long, conflicts_with = "symbols")]
  all: bool,

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

/// Execute crypto news loading command
pub async fn execute(args: CryptoNewsArgs, config: Config) -> Result<()> {
  info!("🚀 Starting crypto news loading process");

  // Get crypto symbols from database
  let symbols_to_process = if args.all {
    info!("Loading all crypto symbols from database...");
    get_all_crypto_symbols(&config.database_url)?
  } else if let Some(symbols) = args.symbols {
    info!("Loading specified crypto symbols from database...");
    get_specific_crypto_symbols(&config.database_url, &symbols)?
  } else {
    return Err(anyhow!("Either --symbols or --all must be specified"));
  };

  if symbols_to_process.is_empty() {
    warn!("No crypto symbols found to process");
    return Ok(());
  }

  info!("Found {} crypto symbols to process", symbols_to_process.len());

  // Configure loader - use same config as regular news loader
  let mut news_config = NewsLoaderConfig {
    days_back: Some(args.days_back),
    topics: args.topics.clone(),
    sort_order: Some(args.sort.clone()),
    limit: Some(args.limit),
    enable_cache: !args.no_cache,
    cache_ttl_hours: 24,
    force_refresh: args.force_refresh,
    database_url: config.database_url.clone(),
    continue_on_error: args.continue_on_error,
    api_delay_ms: args.api_delay,
    progress_interval: args.progress_interval,
  };

  // Default to blockchain topic for crypto if no topics specified
  if news_config.topics.is_none() {
    news_config.topics = Some(vec!["blockchain".to_string()]);
  }

  info!("Loader configuration:");
  info!("  Days back: {}", args.days_back);
  info!("  Topics: {:?}", news_config.topics.as_ref().unwrap());
  info!("  Sort: {}", args.sort);
  info!("  Limit: {} articles per symbol", args.limit);
  info!("  API delay: {}ms between calls", args.api_delay);

  // Create AlphaVantage client
  let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

  // Create context
  let context = LoaderContext::new(client, LoaderConfig::default());

  // Load crypto news using the wrapper function
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

  // Save to database (reuse existing function)
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
  } else if output.data.is_empty() {
    warn!("⚠️ No data to save to database");
  }

  if !output.errors.is_empty() {
    error!("❌ Errors during crypto news loading:");
    for error in &output.errors {
      error!("  - {}", error);
    }
  }

  info!("🎉 Crypto news loading completed successfully");
  Ok(())
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

  Ok(results.into_iter().map(|(sid, symbol)| SymbolInfo { sid, symbol }).collect())
}

/// Get specific crypto symbols from database
fn get_specific_crypto_symbols(
  database_url: &str,
  symbol_list: &[String],
) -> Result<Vec<SymbolInfo>> {
  use av_database_postgres::schema::symbols;
  use diesel::prelude::*;

  let mut conn = PgConnection::establish(database_url)?;

  let symbols_upper: Vec<String> = symbol_list.iter().map(|s| s.to_uppercase()).collect();

  let mut results = Vec::new();

  for symbol in &symbols_upper {
    // Get the crypto symbol with the highest priority (lowest priority number)
    if let Ok(result) = symbols::table
      .filter(symbols::symbol.eq(symbol))
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .order(symbols::priority.asc()) // Get highest priority (lowest number) first
      .select((symbols::sid, symbols::symbol))
      .first::<(i64, String)>(&mut conn)
    {
      results.push(SymbolInfo { sid: result.0, symbol: result.1 });
      info!("Found {} with SID: {}", symbol, result.0);
    } else {
      warn!("Crypto symbol not found in database: {}", symbol);
    }
  }

  if results.is_empty() && !symbol_list.is_empty() {
    warn!("None of the specified crypto symbols were found in the database");
    warn!("Make sure to load crypto symbols first using 'av load crypto-symbols'");
  }

  Ok(results)
}
