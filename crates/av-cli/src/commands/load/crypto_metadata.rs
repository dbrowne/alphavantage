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

//! Crypto metadata loader for `av-cli load crypto-metadata`.
//!
//! Fetches enhanced cryptocurrency metadata from CoinGecko (and optionally
//! AlphaVantage) and persists it into the `crypto_metadata` table. Metadata
//! includes the canonical source identifier, market cap rank, base/quote
//! currencies, active status, and a JSON blob of source-specific extras
//! stored in the `additional_data` column.
//!
//! ## Data Flow
//!
//! ```text
//! symbols ⟕ crypto_api_map  (crypto symbols with mappings, priority < NO_PRIORITY)
//!   │
//!   ▼
//! load_crypto_symbols_from_db()  ── filtered by --symbols / --limit, deduplicated
//!   │
//!   ▼
//! [optional] CryptoMetadataLoader::cleanup_expired_cache()  (--cleanup-cache)
//!   │
//!   ▼
//! CryptoMetadataLoader::load()
//!   │  ── CoinGecko + AlphaVantage APIs + cache ──▶ ProcessedCryptoMetadata
//!   ▼
//! save_metadata_to_db()
//!   ├── per-row existence check on (sid)
//!   ├── INSERT ... ON CONFLICT DO NOTHING       (new rows)
//!   └── UPDATE ... SET ...                       (existing rows, --update-existing)
//! ```
//!
//! ## Sources
//!
//! Currently supported:
//!
//! - **CoinGecko** (default, recommended) — Provides market cap rank and
//!   detailed metadata. Requires `COINGECKO_API_KEY` for higher rate limits.
//! - **AlphaVantage** — Currently mapped to [`CryptoDataSource::SosoValue`]
//!   as a placeholder until the enum has a dedicated `AlphaVantage` variant.
//!   Skipped by default (`--skip-alphavantage = true`) since the value of
//!   AlphaVantage's crypto metadata for this project is still being evaluated.
//!
//! ## Prerequisites
//!
//! - Crypto symbols must be loaded with mappings in `crypto_api_map` (run
//!   `av-cli load crypto` and `av-cli load crypto-mapping --discover-all` first).
//! - At least one of `COINGECKO_API_KEY` or `ALPHA_VANTAGE_API_KEY` should be set.
//!
//! ## Usage
//!
//! ```bash
//! # Load metadata for all mapped cryptos using CoinGecko
//! av-cli load crypto-metadata
//!
//! # Load specific symbols and update existing records
//! av-cli load crypto-metadata --symbols BTC,ETH,SOL --update-existing
//!
//! # Force refresh, bypassing cache
//! av-cli load crypto-metadata --force-refresh --limit 100
//!
//! # Clean cache first, then load with verbose output
//! av-cli load crypto-metadata --cleanup-cache --verbose
//! ```

use anyhow::{Context, Result, anyhow};
use clap::Args;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info, warn};

use av_client::AlphaVantageClient;
use av_core::Config as AvConfig;
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext,
  crypto::{
    CryptoDataSource, CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataLoader,
    CryptoSymbolForMetadata, ProcessedCryptoMetadata,
  },
};

use crate::config::Config;

/// Sentinel value for non-primary tokens (wrapped/bridged variants).
/// Symbols with this priority are excluded from metadata loading.
const NO_PRIORITY: i32 = 9_999_999;

/// Command-line arguments for `av-cli load crypto-metadata`.
///
/// Controls symbol selection, source selection (CoinGecko/AlphaVantage),
/// API authentication, rate limiting, retries, caching, and dry-run/update
/// behavior.
#[derive(Args, Debug)]
pub struct CryptoMetadataArgs {
  /// Comma-separated list of cryptocurrency symbols to load metadata for.
  ///
  /// When omitted, loads metadata for **all** crypto symbols that have
  /// existing API mappings in `crypto_api_map`.
  #[arg(long, value_delimiter = ',')]
  pub symbols: Option<Vec<String>>,

  /// Fetch metadata from APIs but skip database writes.
  #[arg(short, long)]
  pub dry_run: bool,

  /// Update existing rows in `crypto_metadata` instead of skipping them.
  ///
  /// When `false` (default), existing rows are left untouched. When `true`,
  /// all fields except `sid` are overwritten.
  #[arg(long)]
  pub update_existing: bool,

  /// AlphaVantage API key.
  ///
  /// Can also be set via the `ALPHA_VANTAGE_API_KEY` environment variable.
  /// Note that AlphaVantage metadata is skipped by default (see
  /// `--skip-alphavantage`).
  #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
  pub alphavantage_api_key: Option<String>,

  /// CoinGecko API key for enhanced metadata and higher rate limits.
  ///
  /// Can also be set via the `COINGECKO_API_KEY` environment variable.
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub coingecko_api_key: Option<String>,

  /// Maximum number of concurrent API requests.
  #[arg(long, default_value = "5")]
  pub concurrent: usize,

  /// Delay between API requests in milliseconds. Defaults to 200 ms.
  #[arg(long, default_value = "200")]
  pub delay_ms: u64,

  /// Batch size for processing. Defaults to 50.
  #[arg(long, default_value = "50")]
  pub batch_size: usize,

  /// Maximum retry attempts per symbol when an API call fails. Defaults to 4.
  #[arg(long, default_value = "4")]
  pub max_retries: usize,

  /// Request timeout in seconds. Defaults to 10.
  #[arg(long, default_value = "10")]
  pub timeout_seconds: u64,

  /// Cap the number of symbols to process (useful for testing).
  #[arg(short, long)]
  pub limit: Option<usize>,

  /// Enable verbose output, including per-source error logging.
  #[arg(long)]
  pub verbose: bool,

  /// Fetch enhanced metadata fields from CoinGecko (additional data beyond basics).
  ///
  /// Defaults to `true`. Disable to fetch only minimal metadata.
  #[arg(long, default_value = "true")]
  pub fetch_enhanced: bool,

  /// Data sources to use (parsed but currently informational; actual source
  /// selection is controlled by `--skip-coingecko` and `--skip-alphavantage`).
  #[arg(long, value_delimiter = ',', default_values = ["coingecko", "alphavantage"])]
  pub sources: Vec<String>,

  /// Skip AlphaVantage as a metadata source (use only CoinGecko).
  ///
  /// Defaults to `true` because the value of AlphaVantage's crypto metadata
  /// for this project is still being evaluated. See the TODO marker in source.
  #[arg(long, default_value = "true")]
  //todo: Determine if alphavantage is worth pulling for crypto data skip for now
  pub skip_alphavantage: bool,

  /// Enable response caching to reduce API costs.
  #[arg(long, default_value = "true")]
  pub enable_cache: bool,

  /// Cache TTL in hours. Defaults to 24.
  #[arg(long, default_value = "24")]
  pub cache_hours: u32,

  /// Bypass the response cache and fetch fresh data from the source APIs.
  #[arg(long)]
  pub force_refresh: bool,

  /// Skip CoinGecko as a metadata source (use only AlphaVantage).
  #[arg(long)]
  pub skip_coingecko: bool,

  /// Delete expired entries from `api_response_cache` before processing.
  #[arg(long)]
  pub cleanup_cache: bool,
}

/// Main entry point for `av-cli load crypto-metadata`.
///
/// Orchestrates the full metadata loading pipeline:
///
/// 1. **API key validation** — Logs warnings if `ALPHA_VANTAGE_API_KEY` or
///    `COINGECKO_API_KEY` is missing. Does not fail; the loader will use
///    whatever sources are available.
/// 2. **Cache cleanup** — When `--cleanup-cache` is set, calls
///    [`CryptoMetadataLoader::cleanup_expired_cache`] to remove stale
///    `api_response_cache` rows.
/// 3. **Symbol query** — [`load_crypto_symbols_from_db`] joins `symbols` with
///    `crypto_api_map` to find crypto symbols with mappings, optionally
///    filtered by `--symbols` and `--limit`.
/// 4. **Source selection** — Builds the [`CryptoDataSource`] list based on
///    `--skip-coingecko` and `--skip-alphavantage` flags and which API keys
///    are present. AlphaVantage is currently mapped to `SosoValue` as a
///    placeholder.
/// 5. **Loader configuration** — Builds [`CryptoMetadataConfig`] with API
///    keys, rate limiting, retries, timeouts, and cache settings.
/// 6. **API loading** — Calls [`CryptoMetadataLoader::load`] to fetch metadata
///    for all symbols across all sources concurrently.
/// 7. **Per-source reporting** — Logs success/failure counts and (when
///    `--verbose`) detailed errors for each source.
/// 8. **Persistence** — Unless `--dry-run`, calls [`save_metadata_to_db`] to
///    insert/update records.
///
/// # Errors
///
/// Returns errors from: database context creation, symbol query, API client
/// creation, loader execution, or database saves.
pub async fn execute(args: CryptoMetadataArgs, config: &Config) -> Result<()> {
  info!("Starting crypto metadata loader");

  // Validate API keys
  if args.alphavantage_api_key.is_none() && std::env::var("ALPHA_VANTAGE_API_KEY").is_err() {
    warn!("No AlphaVantage API key provided - AlphaVantage metadata will be skipped");
  }

  if args.coingecko_api_key.is_none() && std::env::var("COINGECKO_API_KEY").is_err() {
    warn!("No CoinGecko API key provided - some enhanced metadata may be limited");
  }

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
  }

  // Create database context and cache repository
  let db_context = av_database_postgres::repository::DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;
  let cache_repo: Arc<dyn av_database_postgres::repository::CacheRepository> =
    Arc::new(db_context.cache_repository());

  // Clean up expired cache entries if requested
  if args.cleanup_cache {
    info!("Cleaning up expired cache entries...");
    match CryptoMetadataLoader::cleanup_expired_cache(&cache_repo).await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired cache entries", deleted_count);
        } else {
          info!("No expired cache entries found");
        }
      }
      Err(e) => warn!("Failed to cleanup cache: {}", e),
    }
  }

  // Load crypto symbols from database
  let crypto_symbols =
    load_crypto_symbols_from_db(&config.database_url, &args.symbols, args.limit)?;

  if crypto_symbols.is_empty() {
    warn!("No cryptocurrency symbols found in database");
    return Ok(());
  }

  info!("Loaded {} crypto symbols for metadata processing", crypto_symbols.len());

  // Determine data sources to use
  let mut sources = Vec::new();

  if !args.skip_coingecko
    && (args.coingecko_api_key.is_some() || std::env::var("COINGECKO_API_KEY").is_ok())
  {
    sources.push(CryptoDataSource::CoinGecko);
  }

  // For AlphaVantage, I'm using an available source as a placeholder since the enum doesn't have AlphaVantage
  // The actual AlphaVantage integration happens when the API key is detected in the loader
  if !args.skip_alphavantage
    && (args.alphavantage_api_key.is_some() || std::env::var("ALPHA_VANTAGE_API_KEY").is_ok())
  {
    // Use SosoValue as a placeholder - the loader will detect AlphaVantage API key and use that instead
    sources.push(CryptoDataSource::SosoValue);
    info!(
      "AlphaVantage API key detected - will use AlphaVantage for metadata (via SosoValue placeholder)"
    );
  }

  if sources.is_empty() {
    error!("No valid data sources configured. Please provide API keys or enable sources.");
    return Ok(());
  }

  info!("Using data sources: {:?}", sources);

  // Create metadata loader configuration
  let loader_config = CryptoMetadataConfig {
    alphavantage_api_key: args
      .alphavantage_api_key
      .or_else(|| std::env::var("ALPHA_VANTAGE_API_KEY").ok()),
    coingecko_api_key: args.coingecko_api_key.or_else(|| std::env::var("COINGECKO_API_KEY").ok()),
    delay_ms: args.delay_ms,
    batch_size: args.batch_size,
    max_retries: args.max_retries,
    timeout_seconds: args.timeout_seconds,
    update_existing: args.update_existing,
    fetch_enhanced_metadata: args.fetch_enhanced,
    enable_response_cache: args.enable_cache,
    cache_ttl_hours: args.cache_hours,
    force_refresh: args.force_refresh,
  };

  // Create loader context
  let av_config = AvConfig {
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

  let loader_context = LoaderContext {
    client,
    config: LoaderConfig {
      max_concurrent_requests: args.concurrent,
      retry_attempts: args.max_retries as u32,
      retry_delay_ms: args.delay_ms,
      show_progress: args.verbose,
      track_process: false,
      batch_size: args.batch_size,
    },
    process_tracker: None,
    cache_repository: Some(cache_repo),
    news_repository: None,
  };

  // Create metadata input
  let input = CryptoMetadataInput {
    symbols: Some(crypto_symbols),
    sources,
    update_existing: args.update_existing,
    limit: args.limit,
  };

  // Initialize and run metadata loader
  let metadata_loader = CryptoMetadataLoader::new(loader_config);

  info!("Starting metadata fetching...");
  let metadata_result =
    metadata_loader.load(&loader_context, input).await.context("Failed to load crypto metadata")?;

  info!(
    "Metadata loading completed: {} processed, {} failed",
    metadata_result.metadata_processed.len(),
    metadata_result.symbols_failed
  );

  // Display source-specific results
  for (source, result) in metadata_result.source_results {
    info!(
      "{:?}: {} processed, {} failed, {} errors",
      source,
      result.symbols_processed,
      result.symbols_failed,
      result.errors.len()
    );

    if args.verbose && !result.errors.is_empty() {
      for error in result.errors {
        warn!("{:?} error: {}", source, error);
      }
    }
  }

  if !args.dry_run && !metadata_result.metadata_processed.is_empty() {
    info!("Saving metadata to database...");

    let (inserted, updated) = save_metadata_to_db(
      &config.database_url,
      &metadata_result.metadata_processed,
      args.update_existing,
    )
    .await
    .context("Failed to save metadata to database")?;

    info!("Successfully saved metadata: {} inserted, {} updated", inserted, updated);
  } else if args.dry_run {
    info!("Dry run completed. Found {} metadata entries", metadata_result.metadata_processed.len());
  }

  Ok(())
}

/// Loads crypto symbols with API mappings from the database.
///
/// Joins `symbols` (filtered to `sec_type = "Cryptocurrency"` and
/// `priority < NO_PRIORITY`) with `crypto_api_map` to get tuples of
/// `(sid, symbol, name, api_source, api_id, api_slug, is_active)`. Optionally
/// filters by an explicit symbol list and applies a row limit.
///
/// ## Deduplication
///
/// Because `crypto_api_map` may contain multiple rows per symbol (one per
/// API source), the result set is deduplicated on `(sid, symbol)` using a
/// [`HashSet`] to keep only the first mapping per symbol.
///
/// ## Source Mapping
///
/// The `api_source` string is mapped to a [`CryptoDataSource`] enum:
///
/// - `"coingecko"` → [`CryptoDataSource::CoinGecko`]
/// - `"alphavantage"` → [`CryptoDataSource::SosoValue`] (placeholder, see TODO)
/// - other / unknown → [`CryptoDataSource::CoinGecko`] (fallback)
///
/// The `source_id` field uses `api_slug` if available, otherwise falls back to `api_id`.
fn load_crypto_symbols_from_db(
  database_url: &str,
  symbols_filter: &Option<Vec<String>>,
  limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForMetadata>> {
  use av_database_postgres::{
    establish_connection,
    schema::{crypto_api_map, symbols},
  };
  use diesel::prelude::*;

  let mut conn = establish_connection(database_url).context("Failed to connect to database")?;

  let mut query = symbols::table
    .inner_join(crypto_api_map::table.on(symbols::sid.eq(crypto_api_map::sid)))
    .filter(symbols::sec_type.eq("Cryptocurrency")) // Fixed: using sec_type instead of security_type
    .filter(symbols::priority.lt(NO_PRIORITY)) // Only load symbols with priority < 9999999
    .into_boxed();

  // Filter by specific symbols if provided
  if let Some(symbol_list) = symbols_filter {
    query = query.filter(symbols::symbol.eq_any(symbol_list));
  }

  // Apply limit if specified
  if let Some(limit_count) = limit {
    query = query.limit(limit_count as i64);
  }

  let results: Vec<(
    (i64, String, String),                          // symbols: (sid, symbol, name)
    (String, String, Option<String>, Option<bool>), // crypto_api_map: (api_source, api_id, api_slug, is_active)
  )> = query
    .select((
      (symbols::sid, symbols::symbol, symbols::name),
      (
        crypto_api_map::api_source,
        crypto_api_map::api_id,
        crypto_api_map::api_slug,
        crypto_api_map::is_active,
      ),
    ))
    .load(&mut conn)
    .context("Failed to load crypto symbols from database")?;

  let mut seen = HashSet::new();

  let crypto_symbols = results
    .into_iter()
    .filter(|((sid, symbol, _), _)| seen.insert((*sid, symbol.clone())))
    .map(|((sid, symbol, name), (api_source, api_id, api_slug, is_active))| {
      let source = match api_source.as_str() {
        "coingecko" => CryptoDataSource::CoinGecko,
        "alphavantage" => CryptoDataSource::SosoValue, // Use SosoValue as placeholder for AlphaVantage TODO: fix this!!
        _ => CryptoDataSource::CoinGecko,              // default fallback
      };

      CryptoSymbolForMetadata {
        sid,
        symbol,
        name,
        source,
        source_id: api_slug.unwrap_or(api_id), // Use api_slug if available, otherwise api_id
        is_active: is_active.unwrap_or(true), // Default to true if is_active is NULL  TODO: address this in future release
      }
    })
    .collect();

  Ok(crypto_symbols)
}

/// Persists processed metadata records to the `crypto_metadata` table.
///
/// Iterates over each [`ProcessedCryptoMetadata`] and decides between insert
/// and update based on whether a row with that `sid` already exists:
///
/// - **New row** — `INSERT ... ON CONFLICT DO NOTHING` (the conflict guard
///   handles the unique constraint on `(source, source_id)`).
/// - **Existing row + `update_existing`** — `UPDATE` overwrites all fields
///   except `sid`.
/// - **Existing row + no update** — Skipped with an info log.
///
/// Returns `(inserted_count, updated_count)`. Note: each record requires two
/// database round-trips (existence check + insert/update), which is acceptable
/// for the relatively low volume of metadata records.
///
/// # Note
///
/// This function is `async` but uses synchronous Diesel operations on a
/// blocking connection — there's no `spawn_blocking` wrapper. This works for
/// CLI use but should not be called from a heavily concurrent async context.
async fn save_metadata_to_db(
  database_url: &str,
  metadata: &[ProcessedCryptoMetadata],
  update_existing: bool,
) -> Result<(usize, usize)> {
  use av_database_postgres::{establish_connection, schema::crypto_metadata};
  use diesel::prelude::*;

  let mut conn = establish_connection(database_url).context("Failed to connect to database")?;

  let mut inserted = 0;
  let mut updated = 0;

  for meta in metadata {
    // Check if record exists for this sid
    let exists = crypto_metadata::table
      .filter(crypto_metadata::sid.eq(meta.sid))
      .select(crypto_metadata::sid)
      .first::<i64>(&mut conn)
      .optional()?;

    if exists.is_some() {
      if update_existing {
        // Update existing record
        diesel::update(crypto_metadata::table.find(meta.sid))
          .set((
            crypto_metadata::source.eq(&meta.source),
            crypto_metadata::source_id.eq(&meta.source_id),
            crypto_metadata::market_cap_rank.eq(meta.market_cap_rank),
            crypto_metadata::base_currency.eq(&meta.base_currency),
            crypto_metadata::quote_currency.eq(&meta.quote_currency),
            crypto_metadata::is_active.eq(meta.is_active),
            crypto_metadata::additional_data.eq(&meta.additional_data),
            crypto_metadata::last_updated.eq(meta.last_updated),
          ))
          .execute(&mut conn)
          .context(format!("Failed to update metadata for sid {}", meta.sid))?;
        updated += 1;
        info!("Updated metadata for sid {} from source {}", meta.sid, meta.source);
      } else {
        info!("Skipping existing metadata for sid {} (update_existing=false)", meta.sid);
      }
    } else {
      // Insert new record
      diesel::insert_into(crypto_metadata::table)
        .values((
          crypto_metadata::sid.eq(meta.sid),
          crypto_metadata::source.eq(&meta.source),
          crypto_metadata::source_id.eq(&meta.source_id),
          crypto_metadata::market_cap_rank.eq(meta.market_cap_rank),
          crypto_metadata::base_currency.eq(&meta.base_currency),
          crypto_metadata::quote_currency.eq(&meta.quote_currency),
          crypto_metadata::is_active.eq(meta.is_active),
          crypto_metadata::additional_data.eq(&meta.additional_data),
          crypto_metadata::last_updated.eq(meta.last_updated),
        ))
        .on_conflict_do_nothing() // Handle unique constraint on (source, source_id)
        .execute(&mut conn)
        .context(format!("Failed to insert metadata for sid {}", meta.sid))?;
      inserted += 1;
      info!("Inserted new metadata for sid {} from source {}", meta.sid, meta.source);
    }
  }

  info!("Metadata database operation complete: {} inserted, {} updated", inserted, updated);
  Ok((inserted, updated))
}
