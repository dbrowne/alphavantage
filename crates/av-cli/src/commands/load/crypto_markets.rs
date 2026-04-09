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

//! Crypto markets/exchange-pair loader for `av-cli load crypto-markets`.
//!
//! Fetches trading market data from CoinGecko (`/coins/{id}/tickers`) for
//! cryptocurrencies that have CoinGecko mappings, then upserts into the
//! `crypto_markets` table. Each "market" is a `(exchange, base, target)`
//! triple representing one trading venue and pair (e.g., `Binance/BTC/USDT`).
//!
//! ## Data Flow
//!
//! ```text
//! crypto_api_map (CoinGecko mappings)
//!   │
//!   ▼
//! load_crypto_symbols_from_db()    ── filtered by --symbols / --limit
//!   │
//!   ▼
//! [optional] CryptoMappingService::initialize_mappings_for_symbols()
//!   │  (--initialize-mappings discovers missing CoinGecko IDs first)
//!   ▼
//! CryptoMarketsLoader::load_with_cache()
//!   │  ── CoinGecko /coins/{id}/tickers + cache ──▶ Vec<CryptoMarketData>
//!   ▼
//! save_market_data_to_db()
//!   ├── convert_to_new_crypto_market()  (validation against schema constraints)
//!   ├── partition_result()              (split valid/invalid records)
//!   └── CryptoRepository::upsert_market_data()
//! ```
//!
//! ## Validation
//!
//! Records are validated against the `crypto_markets` schema before insertion:
//!
//! - **String length** — `exchange` ≤ 250, `base` ≤ 120, `target` ≤ 100,
//!   `trust_score` and `liquidity_score` ≤ 100 chars
//! - **SID** — Must be non-zero
//! - **Bid-ask spread** — Must be non-negative; rejected if > 1000% (data error)
//! - **Numeric ranges** — `volume_24h` < `1e27` (NUMERIC(30,2) limit),
//!   `volume_percentage` < `999_999_999` (NUMERIC(11,2) limit)
//!
//! Validation failures are logged but the run continues with the valid subset.
//!
//! ## Prerequisites
//!
//! - Crypto symbols must already be loaded (`av-cli load crypto`).
//! - Symbols must have CoinGecko mappings in `crypto_api_map`. Use
//!   `--initialize-mappings` or run `av-cli load crypto-mapping --discover-all`
//!   first if mappings are missing.
//! - `COINGECKO_API_KEY` environment variable is recommended for higher rate limits.
//!
//! ## Usage
//!
//! ```bash
//! # Load markets for all mapped cryptos
//! av-cli load crypto-markets
//!
//! # Load specific symbols, initializing mappings first
//! av-cli load crypto-markets --symbols BTC,ETH,SOL --initialize-mappings
//!
//! # Limit to top 100 markets per symbol with $10k minimum volume
//! av-cli load crypto-markets --max-markets-per-symbol 100 --min-volume 10000
//!
//! # Dry run with verbose output
//! av-cli load crypto-markets --dry-run --verbose --limit 10
//! ```

use anyhow::{Context, Result, anyhow};
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

/// Command-line arguments for `av-cli load crypto-markets`.
///
/// Controls symbol selection, API authentication, rate limiting, market
/// filtering thresholds, caching, and dry-run/update behavior.
#[derive(Args, Debug)]
pub struct CryptoMarketsArgs {
  /// Comma-separated list of cryptocurrency symbols to load markets for.
  ///
  /// When omitted, loads markets for **all** crypto symbols that have an
  /// existing CoinGecko mapping in `crypto_api_map`.
  #[arg(long, value_delimiter = ',')]
  pub symbols: Option<Vec<String>>,

  /// Fetch market data but skip database writes.
  #[arg(short, long)]
  pub dry_run: bool,

  /// Update existing rows in `crypto_markets`.
  ///
  /// Note: persistence uses `UPSERT` regardless of this flag, so this
  /// effectively only matters for legacy code paths.
  #[arg(long)]
  pub update_existing: bool,

  /// CoinGecko API key for higher rate limits (recommended).
  ///
  /// Can also be set via the `COINGECKO_API_KEY` environment variable.
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub coingecko_api_key: Option<String>,

  /// AlphaVantage API key (used for AlphaVantage market data sources).
  ///
  /// Can also be set via the `ALPHA_VANTAGE_API_KEY` environment variable.
  #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
  pub alphavantage_api_key: Option<String>,

  /// Maximum number of concurrent API requests.
  #[arg(long, default_value = "5")]
  pub concurrent: usize,

  /// Fetch tickers from all available exchanges (not just top exchanges).
  ///
  /// Increases API calls per symbol but provides comprehensive market coverage.
  #[arg(long)]
  pub fetch_all_exchanges: bool,

  /// Minimum 24-hour volume threshold (USD) — markets below this are filtered out.
  ///
  /// Defaults to $1,000 to exclude very low-volume markets.
  #[arg(long, default_value = "1000.0")]
  pub min_volume: f64,

  /// Cap the number of markets retained per symbol.
  ///
  /// Markets are typically sorted by volume; this prevents a single popular
  /// coin from contributing thousands of low-volume rows. Defaults to 20.
  #[arg(long, default_value = "20")]
  pub max_markets_per_symbol: usize,

  /// Cap the number of symbols to process (useful for testing).
  #[arg(short, long)]
  pub limit: Option<usize>,

  /// Batch size for upsert operations.
  #[arg(long, default_value = "50")]
  pub batch_size: usize,

  /// Show detailed progress output and enable the loader progress bar.
  #[arg(long)]
  pub verbose: bool,

  /// Enable HTTP response caching to reduce API costs.
  #[arg(long, default_value = "true")]
  pub enable_cache: bool,

  /// Cache TTL in hours. Defaults to 6.
  #[arg(long, default_value = "6")]
  pub cache_hours: u32,

  /// Bypass the response cache and fetch fresh data.
  #[arg(long)]
  pub force_refresh: bool,

  /// Clean expired cache entries before running. Defaults to `false`.
  #[arg(long, default_value = "false")]
  pub cleanup_cache: bool,

  /// Pre-initialize CoinGecko mappings for requested symbols before loading markets.
  ///
  /// Useful when loading markets for symbols that don't yet have a CoinGecko
  /// mapping in `crypto_api_map`. Requires `--symbols` and `COINGECKO_API_KEY`.
  /// Internally calls
  /// [`CryptoMappingService::initialize_mappings_for_symbols`].
  #[arg(long)]
  pub initialize_mappings: bool,
}

/// Main entry point for `av-cli load crypto-markets`.
///
/// Orchestrates the full markets loading pipeline:
///
/// 1. **Environment check** — Logs whether `COINGECKO_API_KEY` and
///    `ALPHA_VANTAGE_API_KEY` are present (warning, not error, if missing).
/// 2. **Mapping service setup** — Creates a [`CryptoMappingService`] from
///    available API keys (only constructed if at least CoinGecko is set).
/// 3. **Mapping pre-initialization** (`--initialize-mappings`) — When set,
///    requires both a mapping service and `--symbols`. Calls
///    [`CryptoMappingService::initialize_mappings_for_symbols`] to discover
///    CoinGecko IDs for the requested symbols. Returns an error if
///    `COINGECKO_API_KEY` is missing.
/// 4. **Symbol query** — [`load_crypto_symbols_from_db`] returns crypto
///    symbols that have CoinGecko mappings, optionally filtered by `--symbols`
///    and `--limit`. If no mapped symbols are found and a symbol filter was
///    provided, returns an error suggesting `--initialize-mappings`.
/// 5. **Loader configuration** — Builds [`CryptoMarketsConfig`] with rate
///    limiting (1 s delay, 2 s rate-limit delay), retries (3), timeout (30 s),
///    cache settings, and volume/exchange filters.
/// 6. **API loading** — Calls [`CryptoMarketsLoader::load_with_cache`] which
///    fetches CoinGecko `/coins/{id}/tickers` for each symbol with caching.
/// 7. **Persistence** — Unless `--dry-run`, calls [`save_market_data_to_db`]
///    which validates and upserts records.
///
/// # Errors
///
/// Returns errors from: database context creation, mapping initialization,
/// missing API keys (when required), API client creation, loader execution,
/// or database upserts.
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
  let client = Arc::new(
    AlphaVantageClient::new(av_config)
      .map_err(|e| anyhow!("Failed to create API client: {}", e))?,
  );
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

/// Loads cryptocurrency symbols that already have CoinGecko mappings.
///
/// Queries [`CryptoRepository::get_crypto_symbols_with_mappings`] for the
/// `"coingecko"` source. Optionally filters the result set by an explicit
/// symbol list (case-insensitive comparison). Returns
/// [`CryptoSymbolForMarkets`] structs ready for the loader.
///
/// **No hardcoded fallbacks** — if a symbol has no CoinGecko mapping, it is
/// silently excluded. The caller is expected to use `--initialize-mappings`
/// or run `crypto-mapping --discover-all` first if mappings are missing.
async fn load_crypto_symbols_from_db(
  crypto_repo: &Arc<dyn av_database_postgres::repository::CryptoRepository>,
  symbol_filter: &Option<Vec<String>>,
  limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForMarkets>> {
  // Get all crypto symbols with CoinGecko mappings from repository
  let results = crypto_repo
    .get_crypto_symbols_with_mappings("coingecko", limit)
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

/// Validates and persists market data via the crypto repository.
///
/// Steps:
///
/// 1. **Validation** — Maps each [`CryptoMarketData`] through
///    [`convert_to_new_crypto_market`] and partitions the results into valid
///    [`NewCryptoMarket`] records and validation error strings.
/// 2. **Error logging** — Validation errors are logged as warnings (with the
///    record index) but do not abort the save.
/// 3. **Upsert** — Calls [`CryptoRepository::upsert_market_data`] with the
///    valid records. The repository handles `INSERT ... ON CONFLICT ... DO UPDATE`
///    so the `_update_existing` parameter is unused.
///
/// Returns `(inserted_count, updated_count)` from the upsert. Note that the
/// underlying repository may not differentiate between insert and update for
/// some operations.
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

/// Converts a [`CryptoMarketData`] into a [`NewCryptoMarket`] with full
/// schema validation.
///
/// ## Validations
///
/// - **String lengths** match the Postgres column constraints:
///   - `exchange` ≤ 250 chars
///   - `base` ≤ 120 chars
///   - `target` ≤ 100 chars
///   - `trust_score` ≤ 100 chars
///   - `liquidity_score` ≤ 100 chars
/// - **SID** must be non-zero
/// - **Bid-ask spread** must be ≥ 0 (negatives indicate bad data) and ≤ 1000%
///   (above this is treated as a data error). Wide spreads in the 100–1000%
///   range are accepted as valid for illiquid markets.
/// - **Numeric ranges**:
///   - `volume_24h` ≤ `1e27` (NUMERIC(30,2) limit)
///   - `volume_percentage` ≤ `999_999_999` (NUMERIC(11,2) limit)
/// - **Datetime parsing** — `last_traded_at` and `last_fetch_at` are parsed
///   from RFC 3339 strings; `last_fetch_at` defaults to `Utc::now()` if
///   unparseable or absent.
///
/// Returns the validated [`NewCryptoMarket`] or a descriptive error string.
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

  // Validate bid-ask spread (only reject negative spreads - wide spreads are valid for illiquid markets)
  if let Some(ref spread) = market.bid_ask_spread_pct {
    if let Some(spread_f64) = spread.to_f64() {
      if spread_f64 < 0.0 {
        return Err(format!(
          "Invalid bid-ask spread: {:.4}% (negative spreads indicate bad data)",
          spread_f64
        ));
      }
      // Note: Spreads > 100% are valid for illiquid crypto markets
      if spread_f64 > 1000.0 {
        return Err(format!("Unrealistic bid-ask spread: {:.4}% (likely data error)", spread_f64));
      }
    }
  }

  // Validate numeric field ranges against database schema
  // volume_24h: NUMERIC(30,2) - max ~9.99e27
  let volume_24h = if let Some(ref vol) = market.volume_24h {
    if let Some(vol_f64) = vol.to_f64() {
      if vol_f64 > 1e27 {
        return Err(format!("volume_24h too large: {:.2e} (exceeds NUMERIC(30,2) limit)", vol_f64));
      }
    }
    market.volume_24h.clone()
  } else {
    None
  };

  // volume_percentage: NUMERIC(11,2) - max 999999999.99 (100% max in practice)
  let volume_percentage = if let Some(ref pct) = market.volume_percentage {
    if let Some(pct_f64) = pct.to_f64() {
      if pct_f64.abs() > 999999999.0 {
        return Err(format!(
          "volume_percentage too large: {:.2}% (exceeds NUMERIC(11,2) limit)",
          pct_f64
        ));
      }
    }
    market.volume_percentage.clone()
  } else {
    None
  };

  // bid_ask_spread_pct: NUMERIC(10,4) - max 999999.9999 (already validated < 1000% above)
  let bid_ask_spread_pct = market.bid_ask_spread_pct.clone();

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
    volume_24h,
    volume_percentage,
    bid_ask_spread_pct,
    liquidity_score: market.liquidity_score.clone(),
    is_active: Some(market.is_active),
    is_anomaly: Some(market.is_anomaly),
    is_stale: Some(market.is_stale),
    trust_score: market.trust_score.clone(),
    last_traded_at,
    last_fetch_at: Some(last_fetch_at),
  })
}

/// Helper trait that partitions an iterator of [`Result<T, E>`] into two
/// vectors: successes and errors.
///
/// Equivalent to the unstable `Iterator::partition_map` pattern. Used by
/// [`save_market_data_to_db`] to separate validated records from validation
/// errors in a single pass.
trait PartitionResult<T, E> {
  /// Consumes the iterator and returns `(successes, errors)`.
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
