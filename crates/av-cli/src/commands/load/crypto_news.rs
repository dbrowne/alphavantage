use anyhow::{anyhow, Result};
use av_client::AlphaVantageClient;
use av_loaders::{
    crypto::{
        CryptoNewsLoader, CryptoNewsConfig, CryptoNewsInput, CryptoSymbolForNews,
    },
    LoaderContext, LoaderConfig, DataLoader,
};
use chrono::{Duration, Utc};
use clap::Args;
use diesel::prelude::*;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::config::Config;
use super::news_utils::save_news_to_database;

#[derive(Args, Clone, Debug)]
pub struct CryptoNewsArgs {
    /// Load for all active cryptocurrencies
    #[arg(long)]
    all_crypto: bool,

    /// Load for top N cryptocurrencies by market cap
    #[arg(long)]
    top: Option<usize>,

    /// Comma-separated list of specific crypto symbols (without CRYPTO: prefix)
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Number of days back to fetch news
    #[arg(short = 'd', long, default_value = "7")]
    days_back: u32,

    /// Topics to filter by (comma-separated)
    #[arg(short = 't', long, value_delimiter = ',')]
    topics: Option<Vec<String>>,

    /// Include blockchain as a default topic
    #[arg(long, default_value = "true")]
    include_blockchain_topic: bool,

    /// Sort order (LATEST, EARLIEST, RELEVANCE)
    #[arg(long, default_value = "LATEST")]
    sort_order: String,

    /// Maximum number of articles to fetch per symbol (default: 1000, max: 1000)
    #[arg(short, long, default_value = "1000")]
    limit: u32,

    /// Include FOREX:USD in queries
    #[arg(long)]
    include_forex: bool,

    /// Include related market pairs (e.g., COIN stock for Bitcoin)
    #[arg(long)]
    include_market_pairs: bool,

    /// Disable caching
    #[arg(long)]
    no_cache: bool,

    /// Force refresh (bypass cache)
    #[arg(long)]
    force_refresh: bool,

    /// Cache TTL in hours
    #[arg(long, default_value = "24")]
    cache_ttl_hours: u32,

    /// Continue on error instead of stopping
    #[arg(long, default_value = "true")]
    continue_on_error: bool,

    /// Stop on first error (opposite of continue-on-error)
    #[arg(long)]
    stop_on_error: bool,

    /// Dry run - fetch but don't save to database
    #[arg(long)]
    dry_run: bool,

    /// Delay between API calls in milliseconds (default: 800ms for ~75 requests/minute)
    #[arg(long, default_value = "800")]
    api_delay_ms: u64,

    /// Process only first N symbols (for testing)
    #[arg(long)]
    symbol_limit: Option<usize>,
}

/// Get specific crypto symbols from database
fn get_specific_crypto_symbols(database_url: &str, symbols: &[String]) -> Result<Vec<CryptoSymbolForNews>> {
    use av_database_postgres::schema::{symbols as sym_table, crypto_api_map};

    let mut conn = PgConnection::establish(database_url)?;

    let results = sym_table::table
        .left_join(crypto_api_map::table.on(
            crypto_api_map::sid.eq(sym_table::sid)
                .and(crypto_api_map::api_source.eq("AlphaVantage"))
        ))
        .filter(sym_table::symbol.eq_any(symbols))
        .filter(sym_table::sec_type.eq("Cryptocurrency"))
        .select((
            sym_table::sid,
            sym_table::symbol,
            crypto_api_map::api_symbol.nullable(),
        ))
        .load::<(i64, String, Option<String>)>(&mut conn)?;

    Ok(results.into_iter().map(|(sid, symbol, api_symbol)| {
        // Use mapped API symbol if available, otherwise construct it
        let api_sym = api_symbol.unwrap_or_else(|| format!("CRYPTO:{}", symbol));
        CryptoSymbolForNews {
            sid,
            symbol: symbol.clone(),
            api_symbol: api_sym,
        }
    }).collect())
}

/// Get top crypto symbols by market cap rank
fn get_top_crypto_symbols(database_url: &str, limit: usize) -> Result<Vec<CryptoSymbolForNews>> {
    use av_database_postgres::schema::{symbols as sym_table, crypto_api_map, crypto_metadata};

    let mut conn = PgConnection::establish(database_url)?;

    // Get top cryptos by market_cap_rank from crypto_metadata
    let results = sym_table::table
        .inner_join(crypto_metadata::table.on(
            crypto_metadata::sid.eq(sym_table::sid)
        ))
        .left_join(crypto_api_map::table.on(
            crypto_api_map::sid.eq(sym_table::sid)
                .and(crypto_api_map::api_source.eq("AlphaVantage"))
        ))
        .filter(sym_table::sec_type.eq("Cryptocurrency"))
        .filter(crypto_metadata::market_cap_rank.is_not_null())
        .select((
            sym_table::sid,
            sym_table::symbol,
            crypto_api_map::api_symbol.nullable(),
            crypto_metadata::market_cap_rank.nullable(),
        ))
        .order(crypto_metadata::market_cap_rank.asc())
        .limit(limit as i64)
        .load::<(i64, String, Option<String>, Option<i32>)>(&mut conn)?;

    Ok(results.into_iter().map(|(sid, symbol, api_symbol, _rank)| {
        let api_sym = api_symbol.unwrap_or_else(|| format!("CRYPTO:{}", symbol));
        CryptoSymbolForNews {
            sid,
            symbol: symbol.clone(),
            api_symbol: api_sym,
        }
    }).collect())
}

/// Main execute function
pub async fn execute(args: CryptoNewsArgs, config: Config) -> Result<()> {
    info!("🚀 Starting crypto news sentiment loader");

    // Validate limit
    if args.limit > 1000 {
        return Err(anyhow!("Limit cannot exceed 1000 (API maximum)"));
    }
    if args.limit < 1 {
        return Err(anyhow!("Limit must be at least 1"));
    }

    let continue_on_error = if args.stop_on_error {
        false
    } else {
        args.continue_on_error
    };

    // Create API client
    let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Get symbols to process
    let mut symbols_to_process = if args.all_crypto {
        info!("Loading all active cryptocurrency symbols");
        CryptoNewsLoader::get_crypto_symbols_from_database(&config.database_url)?
    } else if let Some(top_n) = args.top {
        info!("Loading top {} cryptocurrencies by market cap", top_n);
        get_top_crypto_symbols(&config.database_url, top_n)?
    } else if let Some(ref symbol_list) = args.symbols {
        info!("Loading specific crypto symbols: {:?}", symbol_list);
        get_specific_crypto_symbols(&config.database_url, symbol_list)?
    } else {
        return Err(anyhow!("Must specify either --all-crypto, --top N, or --symbols"));
    };

    // Apply symbol limit if specified (for testing)
    if let Some(limit) = args.symbol_limit {
        symbols_to_process = symbols_to_process.into_iter().take(limit).collect();
    }

    if symbols_to_process.is_empty() {
        warn!("No crypto symbols found to process");
        return Ok(());
    }

    // Calculate estimated time
    let api_delay_ms = args.api_delay_ms;
    let estimated_minutes = (symbols_to_process.len() as f64 * api_delay_ms as f64 / 1000.0) / 60.0;
    info!("Processing {} crypto symbols", symbols_to_process.len());
    info!("Estimated processing time: {:.1} minutes with {}ms delay between calls",
          estimated_minutes, api_delay_ms);

    // Build topics list
    let mut topics = args.topics.clone().unwrap_or_default();
    if args.include_blockchain_topic && !topics.contains(&"blockchain".to_string()) {
        topics.push("blockchain".to_string());
    }

    // Configure crypto news loader
    let news_config = CryptoNewsConfig {
        days_back: Some(args.days_back),
        topics: if topics.is_empty() { None } else { Some(topics) },
        sort_order: Some(args.sort_order.clone()),
        limit: Some(args.limit),
        enable_cache: !args.no_cache,
        cache_ttl_hours: args.cache_ttl_hours,
        force_refresh: args.force_refresh,
        database_url: config.database_url.clone(),
        continue_on_error: args.continue_on_error,
        api_delay_ms,
        progress_interval: 10,
        include_forex: args.include_forex,
    };

    info!("📰 Crypto News Loader Configuration:");
    info!("  Days back: {}", args.days_back);
    info!("  Limit: {} articles per symbol", args.limit);
    info!("  Sort order: {}", args.sort_order);
    info!("  Topics: {:?}", news_config.topics);
    info!("  Include FOREX: {}", args.include_forex);
    info!("  Include market pairs: {}", args.include_market_pairs);
    info!("  Cache: {}", if args.no_cache { "disabled" } else { "enabled" });
    info!("  API delay: {}ms between calls", api_delay_ms);

    // Create loader
    let loader = CryptoNewsLoader::new(5).with_config(news_config);

    // Create input
    let input = CryptoNewsInput {
        symbols: symbols_to_process,
        time_from: Some(Utc::now() - Duration::days(args.days_back as i64)),
        time_to: Some(Utc::now()),
        include_market_pairs: args.include_market_pairs,
    };

    // Create context
    let context = LoaderContext::new(
        client,
        LoaderConfig::default(),
    );

    // Load data from API
    info!("📡 Fetching crypto news from AlphaVantage API...");
    let output = match loader.load(&context, input).await {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to load crypto news: {}", e);
            if !continue_on_error {
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
        output.articles_processed,
        output.loaded_count,
        output.no_data_count,
        output.api_calls
    );

    // Save to database
    if !args.dry_run && !output.data.is_empty() {
        info!("💾 Saving crypto news to database...");

        let stats = save_news_to_database(&config.database_url, output.data, continue_on_error).await?;

        info!(
            "✅ Database persistence complete:\n  \
            - {} news overviews\n  \
            - {} feeds\n  \
            - {} articles\n  \
            - {} ticker sentiments\n  \
            - {} topics",
            stats.news_overviews,
            stats.feeds,
            stats.articles,
            stats.sentiments,
            stats.topics
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
        if !continue_on_error {
            return Err(anyhow!("Crypto news loading completed with errors"));
        }
    }

    info!("🎉 Crypto news loading completed successfully");
    Ok(())
}