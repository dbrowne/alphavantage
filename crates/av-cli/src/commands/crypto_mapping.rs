use anyhow::{ Result};
use clap::Parser;
use av_loaders::crypto::mapping_service::CryptoMappingService;
use diesel::PgConnection;
use std::collections::HashMap;

#[derive(Parser, Debug)]
pub struct MappingArgs {
    /// Discover missing mappings for specific source
    #[arg(long, value_enum)]
    pub source: Option<MappingSource>,

    /// Discover all missing mappings
    #[arg(long)]
    pub discover_all: bool,

    /// CoinGecko API key
    #[arg(long, env = "COINGECKO_API_KEY")]
    pub coingecko_api_key: Option<String>,

    /// Show mapping statistics
    #[arg(long)]
    pub stats: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum MappingSource {
    CoinGecko,
    CoinPaprika,
    All,
}

pub async fn execute(args: MappingArgs, config: &crate::config::Config) -> Result<()> {
    let mut api_keys = HashMap::new();

    if let Some(key) = args.coingecko_api_key {
        api_keys.insert("coingecko".to_string(), key);
    }

    let mapping_service = CryptoMappingService::new(api_keys);
    let mut conn = av_database_postgres::establish_connection(&config.database_url)?;

    if args.stats {
        show_mapping_stats(&mut conn)?;
        return Ok(());
    }

    match args.source {
        Some(MappingSource::CoinGecko) | None => {
            mapping_service.discover_missing_mappings(&mut conn, "CoinGecko").await?;
        }
        Some(MappingSource::CoinPaprika) => {
            mapping_service.discover_missing_mappings(&mut conn, "CoinPaprika").await?;
        }
        Some(MappingSource::All) => {
            mapping_service.discover_missing_mappings(&mut conn, "CoinGecko").await?;
            mapping_service.discover_missing_mappings(&mut conn, "CoinPaprika").await?;
        }
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
    } else { 0.0 };

    println!("  CoinGecko Coverage: {:.1}%", coverage_coingecko);

    Ok(())
}
