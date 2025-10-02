use anyhow::Result;
use av_database_postgres::establish_connection;
use av_loaders::crypto::mapping_service::CryptoMappingService;
use clap::Parser;
use diesel::PgConnection;
use std::collections::HashMap;
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
  let mut conn = establish_connection(&config.database_url)?;

  if args.stats {
    show_mapping_stats(&mut conn)?;
    return Ok(());
  }

  if let Some(ref symbol_list) = args.symbols {
    info!("🔍 Initializing mappings for specific symbols: {:?}", symbol_list);
    let initialized =
      mapping_service.initialize_mappings_for_symbols(&mut conn, symbol_list).await?;
    info!("✅ Initialized {} symbol mappings", initialized);
    return Ok(());
  }

  if args.discover_all {
    info!("🔍 Discovering all missing CoinGecko mappings...");
    let discovered = mapping_service.discover_missing_mappings(&mut conn, "CoinGecko").await?;
    info!("✅ Discovered {} new mappings", discovered);
  } else if let Some(source) = args.source {
    info!("🔍 Discovering missing {} mappings...", source);
    let discovered = mapping_service.discover_missing_mappings(&mut conn, &source).await?;
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

fn show_mapping_stats(conn: &mut PgConnection) -> Result<()> {
  use av_database_postgres::models::crypto::CryptoApiMap;

  let summary = CryptoApiMap::get_crypto_summary(conn)?;

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
