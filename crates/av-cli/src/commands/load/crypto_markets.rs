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

use anyhow::{Context, Result};
use bigdecimal::ToPrimitive;
use clap::Args;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use av_client::AlphaVantageClient;
use av_database_postgres::models::crypto_markets::NewCryptoMarket;
use av_loaders::{
  LoaderConfig, LoaderContext,
  crypto::{
    CryptoDataSource,
    mapping_service::CryptoMappingService,
    markets_loader::{
      CryptoMarketData, CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader,
      CryptoSymbolForMarkets,
    },
  },
};
use std::collections::HashMap;

#[derive(Args, Debug)]
pub struct CryptoMarketsArgs {
  /// Specific symbols to load (comma-separated). If not provided, loads for all crypto symbols
  #[arg(long, value_delimiter = ',')]
  pub symbols: Option<Vec<String>>,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  pub dry_run: bool,

  /// Update existing market data entries
  #[arg(long)]
  pub update_existing: bool,

  /// CoinGecko API key for higher rate limits
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub coingecko_api_key: Option<String>,

  /// AlphaVantage API key
  #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
  pub alphavantage_api_key: Option<String>,

  /// Number of concurrent requests
  #[arg(long, default_value = "5")]
  pub concurrent: usize,

  /// Fetch data from all available exchanges
  #[arg(long)]
  pub fetch_all_exchanges: bool,

  /// Minimum volume threshold (USD)
  #[arg(long, default_value = "1000.0")]
  pub min_volume: f64,

  /// Maximum markets per symbol
  #[arg(long, default_value = "20")]
  pub max_markets_per_symbol: usize,

  /// Limit number of symbols to process (for testing)
  #[arg(short, long)]
  pub limit: Option<usize>,

  /// Batch size for processing
  #[arg(long, default_value = "50")]
  pub batch_size: usize,

  /// Show detailed progress information
  #[arg(long)]
  pub verbose: bool,

  /// Enable response caching to reduce API costs
  #[arg(long, default_value = "true")]
  pub enable_cache: bool,

  /// Cache TTL in hours
  #[arg(long, default_value = "6")]
  pub cache_hours: u32,

  /// Force refresh - ignore cache and fetch fresh data
  #[arg(long)]
  pub force_refresh: bool,

  /// Clean expired cache entries before running
  #[arg(long, default_value = "false")]
  pub cleanup_cache: bool,

  /// Pre-initialize mappings for requested symbols before loading markets
  #[arg(long)]
  pub initialize_mappings: bool,
}

pub async fn execute(args: CryptoMarketsArgs, config: &Config) -> Result<()> {
  info!("Starting crypto markets data loader with dynamic mapping");

  // Debug: Check if environment variables are loaded
  info!("Checking environment variables...");
  if let Ok(coingecko_key) = std::env::var("COINGECKO_API_KEY") {
    info!("✅ COINGECKO_API_KEY found (length: {})", coingecko_key.len());
  } else {
    warn!("❌ COINGECKO_API_KEY not found in environment");
  }

  if let Ok(av_key) = std::env::var("ALPHA_VANTAGE_API_KEY") {
    info!("✅ ALPHA_VANTAGE_API_KEY found (length: {})", av_key.len());
  } else {
    warn!("❌ ALPHA_VANTAGE_API_KEY not found in environment");
  }

  // Setup mapping service using environment variables
  let mapping_service = {
    let mut api_keys = HashMap::new();

    // Read CoinGecko API key from environment
    if let Ok(coingecko_key) = std::env::var("COINGECKO_API_KEY") {
      api_keys.insert("coingecko".to_string(), coingecko_key);
    }

    if !api_keys.is_empty() { Some(CryptoMappingService::new(api_keys)) } else { None }
  };

  // Pre-initialize mappings if requested
  if args.initialize_mappings {
    if let (Some(ref service), Some(ref symbol_list)) = (&mapping_service, &args.symbols) {
      // Create database context and repository
      let db_context = av_database_postgres::repository::DatabaseContext::new(&config.database_url)
        .context("Failed to create database context")?;
      let crypto_repo: Arc<dyn av_database_postgres::repository::CryptoRepository> =
        Arc::new(db_context.crypto_repository());

      info!("🔍 Pre-initializing mappings for {} symbols", symbol_list.len());
      let initialized = service
        .initialize_mappings_for_symbols(&crypto_repo, &db_context, symbol_list)
        .await
        .context("Failed to initialize mappings")?;

      info!("✅ Initialized {} symbol mappings", initialized);
    } else if mapping_service.is_none() {
      error!("Cannot initialize mappings: COINGECKO_API_KEY not found in environment");
      return Err(anyhow::anyhow!(
        "COINGECKO_API_KEY environment variable is required for mapping initialization"
      ));
    } else {
      warn!("Cannot initialize mappings: no symbol list provided");
    }
  }

  // Create database context and crypto repository for loading symbols
  let db_context = av_database_postgres::repository::DatabaseContext::new(&config.database_url)
    .context("Failed to create database context")?;
  let crypto_repo: Arc<dyn av_database_postgres::repository::CryptoRepository> =
    Arc::new(db_context.crypto_repository());

  // Load symbols from database (this will only find symbols with existing mappings)
  let symbols = if let Some(ref symbol_list) = args.symbols {
    info!("Loading specific symbols: {:?}", symbol_list);
    load_crypto_symbols_from_db(&crypto_repo, &Some(symbol_list.clone()), args.limit).await?
  } else {
    info!("Loading all crypto symbols with existing mappings");
    load_crypto_symbols_from_db(&crypto_repo, &None, args.limit).await?
  };

  if symbols.is_empty() {
    if args.symbols.is_some() && mapping_service.is_some() {
      error!(
        "No symbols found with CoinGecko mappings. Try running with --initialize-mappings first"
      );
      return Err(anyhow::anyhow!(
        "No mapped symbols found. Use --initialize-mappings to discover mappings via API"
      ));
    } else {
      warn!("No cryptocurrency symbols with mappings found in database");
      return Ok(());
    }
  }

  info!("Loaded {} crypto symbols with existing mappings", symbols.len());

  // Configure and run loader using existing structure
  let loader_config = CryptoMarketsConfig {
    coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
    delay_ms: 1000,
    batch_size: args.batch_size,
    max_retries: 3,
    timeout_seconds: 30,
    max_concurrent_requests: args.concurrent,
    rate_limit_delay_ms: 2000,
    enable_progress_bar: args.verbose,
    alphavantage_api_key: std::env::var("ALPHAVANTAGE_API_KEY").ok(),
    fetch_all_exchanges: args.fetch_all_exchanges,
    min_volume_threshold: Some(args.min_volume),
    max_markets_per_symbol: Some(args.max_markets_per_symbol),
    enable_response_cache: args.enable_cache,
    cache_ttl_hours: args.cache_hours,
    force_refresh: args.force_refresh,
  };

  let input = CryptoMarketsInput {
    symbols: Some(symbols),
    exchange_filter: None,
    min_volume_threshold: Some(args.min_volume),
    max_markets_per_symbol: Some(args.max_markets_per_symbol),
    update_existing: args.update_existing,
    sources: vec![CryptoDataSource::CoinGecko],
    batch_size: Some(args.batch_size),
  };

  // Create loader context with proper parameters - FIX: Use av_core::Config
  let av_config = av_core::Config {
    api_key: config.api_config.api_key.clone(),
    base_url: config.api_config.base_url.clone(),
    rate_limit: config.api_config.rate_limit,
    timeout_secs: config.api_config.timeout_secs,
    max_retries: config.api_config.max_retries,
  };
  let client = Arc::new(AlphaVantageClient::new(av_config));
  let loader_config_for_context = LoaderConfig::default();
  let loader_context = LoaderContext::new(client, loader_config_for_context);
  let markets_loader = CryptoMarketsLoader::new(loader_config);

  info!("Starting market data fetching...");

  // Use the existing cached loader
  match markets_loader.load_with_cache(&loader_context, input, &config.database_url).await {
    Ok(market_data) => {
      info!("Fetched market data for {} symbols", market_data.len());

      if !args.dry_run && !market_data.is_empty() {
        info!("Saving market data to database...");

        let (inserted, updated) =
          save_market_data_to_db(&crypto_repo, &market_data, args.update_existing)
            .await
            .context("Failed to save market data to database")?;

        info!("Successfully saved market data: {} inserted, {} updated", inserted, updated);
      } else if args.dry_run {
        info!("Dry run completed. Found {} market data entries", market_data.len());
      }
    }
    Err(e) => {
      error!("Failed to load market data: {}", e);
      return Err(anyhow::anyhow!("Market data loading failed: {}", e));
    }
  }

  Ok(())
}

/// Load symbols that already have CoinGecko mappings (no hardcoded fallbacks)
async fn load_crypto_symbols_from_db(
  crypto_repo: &Arc<dyn av_database_postgres::repository::CryptoRepository>,
  symbol_filter: &Option<Vec<String>>,
  limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForMarkets>> {
  // Get all crypto symbols with CoinGecko mappings from repository
  let results = crypto_repo
    .get_crypto_symbols_with_mappings("CoinGecko", limit)
    .await
    .context("Failed to query crypto symbols with mappings")?;

  // Filter by specific symbols if requested
  let filtered_results: Vec<_> = if let Some(ref filter_list) = symbol_filter {
    let uppercase_filters: Vec<String> = filter_list.iter().map(|s| s.to_uppercase()).collect();
    results
      .into_iter()
      .filter(|(_, symbol_val, _, _)| uppercase_filters.contains(&symbol_val.to_uppercase()))
      .collect()
  } else {
    results
  };

  // Convert to CryptoSymbolForMarkets
  let crypto_symbols = filtered_results
    .into_iter()
    .filter_map(|(sid_val, symbol_val, name_val, api_id_opt)| {
      api_id_opt.map(|coingecko_id_val| CryptoSymbolForMarkets {
        sid: sid_val,
        symbol: symbol_val.clone(),
        name: name_val,
        coingecko_id: Some(coingecko_id_val),
        alphavantage_symbol: Some(symbol_val),
      })
    })
    .collect();

  Ok(crypto_symbols)
}

/// Save market data to database using repository
async fn save_market_data_to_db(
  crypto_repo: &Arc<dyn av_database_postgres::repository::CryptoRepository>,
  market_data: &[CryptoMarketData],
  _update_existing: bool, // Not needed with UPSERT
) -> Result<(usize, usize)> {
  info!("Processing {} market data entries with UPSERT", market_data.len());

  // Convert and validate data
  let (valid_markets, validation_errors): (Vec<_>, Vec<_>) = market_data
    .iter()
    .enumerate()
    .map(|(index, market)| match convert_to_new_crypto_market(market) {
      Ok(new_market) => Ok(new_market),
      Err(e) => Err(format!("Record {}: {}", index + 1, e)),
    })
    .partition_result();

  // Log validation errors but continue processing
  if !validation_errors.is_empty() {
    warn!("⚠️  {} validation errors:", validation_errors.len());
    for error in &validation_errors {
      warn!("   {}", error);
    }
  }

  if valid_markets.is_empty() {
    warn!("No valid market records to process");
    return Ok((0, 0));
  }

  // Use repository to upsert market data
  let (inserted, updated) =
    crypto_repo.upsert_market_data(&valid_markets).await.context("Failed to upsert market data")?;

  info!(
    "✅ Database save complete: {} inserted/updated, {} validation errors",
    inserted,
    validation_errors.len()
  );

  Ok((inserted, updated))
}

/// Convert CryptoMarketData to NewCryptoMarket with validation
fn convert_to_new_crypto_market(market: &CryptoMarketData) -> Result<NewCryptoMarket, String> {
  // Validate field lengths against database schema
  if market.exchange.len() > 250 {
    return Err(format!("Exchange name too long: {} chars (max 250)", market.exchange.len()));
  }
  if market.base.len() > 120 {
    return Err(format!("Base token too long: {} chars (max 120)", market.base.len()));
  }
  if market.target.len() > 100 {
    return Err(format!("Target token too long: {} chars (max 100)", market.target.len()));
  }
  if let Some(ref trust_score) = market.trust_score {
    if trust_score.len() > 100 {
      return Err(format!("Trust score too long: {} chars (max 100)", trust_score.len()));
    }
  }
  if let Some(ref liquidity_score) = market.liquidity_score {
    if liquidity_score.len() > 100 {
      return Err(format!("Liquidity score too long: {} chars (max 100)", liquidity_score.len()));
    }
  }

  // Validate SID
  if market.sid == 0 {
    return Err("SID cannot be zero".to_string());
  }

  // Validate bid-ask spread range
  if let Some(ref spread) = market.bid_ask_spread_pct {
    if let Some(spread_f64) = spread.to_f64() {
      if spread_f64 < 0.0 || spread_f64 > 100.0 {
        return Err(format!("Invalid bid-ask spread: {:.4}% (must be 0-100%)", spread_f64));
      }
    }
  }

  // Parse datetime strings
  let last_traded_at = market
    .last_traded_at
    .as_ref()
    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&chrono::Utc));

  let last_fetch_at = market
    .last_fetch_at
    .as_ref()
    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| dt.with_timezone(&chrono::Utc))
    .unwrap_or_else(chrono::Utc::now);

  Ok(NewCryptoMarket {
    sid: market.sid,
    exchange: market.exchange.clone(),
    base: market.base.clone(),
    target: market.target.clone(),
    market_type: market.market_type.clone(),
    volume_24h: market.volume_24h.clone(),
    volume_percentage: market.volume_percentage.clone(),
    bid_ask_spread_pct: market.bid_ask_spread_pct.clone(),
    liquidity_score: market.liquidity_score.clone(),
    is_active: Some(market.is_active),
    is_anomaly: Some(market.is_anomaly),
    is_stale: Some(market.is_stale),
    trust_score: market.trust_score.clone(),
    last_traded_at,
    last_fetch_at: Some(last_fetch_at),
  })
}

/// Helper trait for partitioning results
trait PartitionResult<T, E> {
  fn partition_result(self) -> (Vec<T>, Vec<E>);
}

impl<I, T, E> PartitionResult<T, E> for I
where
  I: Iterator<Item = Result<T, E>>,
{
  fn partition_result(self) -> (Vec<T>, Vec<E>) {
    let mut oks = Vec::new();
    let mut errs = Vec::new();

    for item in self {
      match item {
        Ok(ok) => oks.push(ok),
        Err(err) => errs.push(err),
      }
    }

    (oks, errs)
  }
}
