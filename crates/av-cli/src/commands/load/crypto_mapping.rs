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
use av_loaders::crypto::mapping_service::CryptoMappingService;
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
pub struct MappingArgs {
  /// Discover missing mappings for specific source
  #[arg(long)]
  pub source: Option<String>,

  /// Discover all missing mappings
  #[arg(long)]
  pub discover_all: bool,

  /// Show mapping statistics
  #[arg(long)]
  pub stats: bool,

  /// Initialize mappings for specific symbols
  #[arg(long, value_delimiter = ',')]
  pub symbols: Option<Vec<String>>,
}

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
