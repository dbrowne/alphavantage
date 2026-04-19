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

//! Cryptocurrency API mapping management for `av-cli load crypto-mapping`.
//!
//! This module provides three operations for managing the link between
//! cryptocurrency symbols in the local database and their identifiers in
//! external APIs (primarily CoinGecko):
//!
//! 1. **Statistics** (`--stats`) — Report on mapping coverage across all
//!    cryptocurrencies.
//! 2. **Targeted initialization** (`--symbols`) — Look up and create mappings
//!    for an explicit list of symbols.
//! 3. **Bulk discovery** (`--discover-all` / `--source`) — Find and create
//!    mappings for all currently unmapped symbols.
//!
//! ## Relationship to Other Commands
//!
//! Mappings populate the `crypto_api_map` and `symbol_mappings` tables, which
//! are then read by downstream commands:
//!
//! - [`crypto_details`](super::crypto_details) — Reads CoinGecko mappings to
//!   fetch social/technical data via `/coins/{id}`.
//! - [`crypto_overview`](super::crypto_overview) — Reads mappings for price
//!   and market cap enrichment.
//! - [`crypto_metadata`](super::crypto_metadata) — Reads mappings for metadata
//!   enrichment.
//!
//! Without mappings, those commands cannot connect a local symbol (e.g., `BTC`)
//! to its external API ID (e.g., `bitcoin` for CoinGecko).
//!
//! ## Delegation
//!
//! All mapping work is delegated to
//! [`CryptoMappingService`](av_loaders::crypto::mapping_service::CryptoMappingService).
//! This module is a thin CLI wrapper that handles argument parsing, repository
//! setup, and result reporting.
//!
//! ## Usage
//!
//! ```bash
//! # Show coverage statistics
//! av-cli load crypto-mapping --stats
//!
//! # Discover all missing CoinGecko mappings
//! av-cli load crypto-mapping --discover-all
//!
//! # Discover missing mappings for a specific source
//! av-cli load crypto-mapping --source CoinGecko
//!
//! # Initialize mappings for specific symbols
//! av-cli load crypto-mapping --symbols SOL,BTC,ETH
//! ```

use anyhow::Result;
use av_loaders::crypto::mapping_service::CryptoMappingService;
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Command-line arguments for `av-cli load crypto-mapping`.
///
/// The four flags are mutually orthogonal but processed in priority order by
/// [`execute`]: `--stats` short-circuits and returns; `--symbols` runs the
/// targeted initialization path; `--discover-all` runs bulk discovery against
/// CoinGecko; `--source` runs bulk discovery against the named source. If
/// none of these are set, [`execute`] prints usage examples and returns
/// without error.
#[derive(Parser, Debug)]
pub struct MappingArgs {
  /// Discover missing mappings for a specific API source (e.g., `CoinGecko`).
  ///
  /// Mutually exclusive in practice with `--discover-all`. When both are set,
  /// `--discover-all` takes precedence (see [`execute`] dispatch order).
  #[arg(long)]
  pub source: Option<String>,

  /// Discover missing mappings for all sources (currently CoinGecko only).
  ///
  /// Equivalent to `--source CoinGecko` but uses a hardcoded source name.
  #[arg(long)]
  pub discover_all: bool,

  /// Print mapping coverage statistics and exit.
  ///
  /// Short-circuits the rest of the command — no discovery or initialization
  /// is performed.
  #[arg(long)]
  pub stats: bool,

  /// Comma-separated list of specific symbols to initialize mappings for.
  ///
  /// Used for targeted mapping creation when `--discover-all` would be too
  /// broad. Each symbol is looked up via the configured API source and a
  /// mapping is created if found.
  #[arg(long, value_delimiter = ',')]
  pub symbols: Option<Vec<String>>,
}

/// Main entry point for `av-cli load crypto-mapping`.
///
/// Sets up shared infrastructure (API keys, database context, mapping service)
/// then dispatches to one of four code paths based on which flag is set:
///
/// 1. **`--stats`** — Calls [`show_mapping_stats`] and returns. Does not run
///    any discovery.
/// 2. **`--symbols <list>`** — Calls
///    [`CryptoMappingService::initialize_mappings_for_symbols`] with the
///    explicit symbol list.
/// 3. **`--discover-all`** — Calls
///    [`CryptoMappingService::discover_missing_mappings`] with `"CoinGecko"`.
/// 4. **`--source <name>`** — Calls `discover_missing_mappings` with the
///    user-supplied source name.
///
/// If none of the above are set, prints usage examples and returns `Ok(())`.
///
/// ## API Key Handling
///
/// Reads `COINGECKO_API_KEY` from the environment and adds it to the
/// [`HashMap`] passed to [`CryptoMappingService::new`]. The key is optional —
/// the service will fall back to free-tier rate limits if absent (a warning
/// is logged).
///
/// # Errors
///
/// Returns errors from database context creation, mapping discovery, or
/// statistics queries.
pub async fn execute(args: MappingArgs, config: &crate::config::Config) -> Result<()> {
  let mut api_keys = HashMap::new();

  // Read CoinGecko API key from environment
  if let Ok(coingecko_key) = std::env::var("COINGECKO_API_KEY") {
    api_keys.insert("coingecko".to_string(), coingecko_key);
    info!("✅ Using CoinGecko API key from environment");
  } else {
    info!("⚠️ No CoinGecko API key found in environment");
  }

  let mapping_service = CryptoMappingService::new(api_keys);

  // Create database context and repository
  let db_context = av_database_postgres::repository::DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;
  let crypto_repo: Arc<dyn av_database_postgres::repository::CryptoRepository> =
    Arc::new(db_context.crypto_repository());

  if args.stats {
    show_mapping_stats(&crypto_repo).await?;
    return Ok(());
  }

  if let Some(ref symbol_list) = args.symbols {
    info!("🔍 Initializing mappings for specific symbols: {:?}", symbol_list);
    let initialized = mapping_service
      .initialize_mappings_for_symbols(&crypto_repo, &db_context, symbol_list)
      .await?;
    info!("✅ Initialized {} symbol mappings", initialized);
    return Ok(());
  }

  if args.discover_all {
    info!("🔍 Discovering all missing CoinGecko mappings...");
    let discovered = mapping_service.discover_missing_mappings(&crypto_repo, "CoinGecko").await?;
    info!("✅ Discovered {} new mappings", discovered);
  } else if let Some(source) = args.source {
    info!("🔍 Discovering missing {} mappings...", source);
    let discovered = mapping_service.discover_missing_mappings(&crypto_repo, &source).await?;
    info!("✅ Discovered {} new {} mappings", discovered, source);
  } else {
    info!("No action specified. Use --stats, --discover-all, --source, or --symbols");
    info!("Examples:");
    info!("  cargo run load crypto-mapping --stats");
    info!("  cargo run load crypto-mapping --symbols SOL,BTC,ETH");
    info!("  cargo run load crypto-mapping --discover-all");
  }

  Ok(())
}

/// Prints crypto mapping coverage statistics to stdout.
///
/// Queries [`CryptoRepository::get_crypto_summary`] and reports:
///
/// - Total number of cryptocurrencies in the database
/// - Number of active cryptocurrencies
/// - CoinGecko mapping count and coverage percentage (mapped / active × 100)
/// - CoinPaprika mapping count
/// - Unmapped symbol count (active − mapped CoinGecko)
///
/// If any symbols are unmapped, prints a hint suggesting `--discover-all`.
async fn show_mapping_stats(
  crypto_repo: &Arc<dyn av_database_postgres::repository::CryptoRepository>,
) -> Result<()> {
  let summary = crypto_repo
    .get_crypto_summary()
    .await
    .map_err(|e| anyhow::anyhow!("Failed to get crypto summary: {}", e))?;

  println!("📊 Crypto Mapping Statistics:");
  println!("  Total Cryptos: {}", summary.total_cryptos);
  println!("  Active Cryptos: {}", summary.active_cryptos);
  println!("  CoinGecko Mapped: {}", summary.mapped_coingecko);
  println!("  CoinPaprika Mapped: {}", summary.mapped_coinpaprika);

  let coverage_coingecko = if summary.active_cryptos > 0 {
    (summary.mapped_coingecko as f64 / summary.active_cryptos as f64) * 100.0
  } else {
    0.0
  };

  println!("  CoinGecko Coverage: {:.1}%", coverage_coingecko);

  // Show unmapped symbols
  let unmapped_count = summary.active_cryptos - summary.mapped_coingecko;
  if unmapped_count > 0 {
    println!("  Unmapped Symbols: {}", unmapped_count);
    println!("  💡 Run with --discover-all to auto-discover mappings");
  }

  Ok(())
}
