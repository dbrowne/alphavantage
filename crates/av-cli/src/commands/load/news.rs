//! News and sentiment data loading command implementation

use anyhow::Result;
use av_client::AlphaVantageClient;
use av_loaders::{
    NewsLoader, NewsLoaderConfig, NewsLoaderInput,
    LoaderContext, LoaderConfig, DataLoader,
    load_news_for_equity_symbols,
    NewsSymbolInfo,  // Use the aliased name exported from lib.rs
};
use chrono::{Duration, Utc};
use clap::Args;
use diesel::prelude::*;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::config::Config;

#[derive(Args, Clone, Debug)]
pub struct NewsArgs {
    /// Load for all equity symbols with overview=true
    #[arg(long)]
    all_equity: bool,

    /// Comma-separated list of specific tickers
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Option<Vec<String>>,

    /// Number of days back to fetch news
    #[arg(short = 'd', long, default_value = "7")]
    days_back: u32,

    /// Topics to filter by (comma-separated)
    /// Supported: blockchain, earnings, ipo, mergers_and_acquisitions,
    /// financial_markets, economy_fiscal, economy_monetary, economy_macro,
    /// energy_transportation, finance, life_sciences, manufacturing,
    /// real_estate, retail_wholesale, technology
    #[arg(short = 't', long, value_delimiter = ',')]
    topics: Option<Vec<String>>,

    /// Sort order (LATEST, EARLIEST, RELEVANCE)
    #[arg(long, default_value = "LATEST")]
    sort_order: String,

    /// Maximum number of articles to fetch (default: 100, max: 1000)
    #[arg(short, long, default_value = "100")]
    limit: u32,

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
    #[arg(long)]
    continue_on_error: bool,

    /// Dry run - fetch but don't save to database
    #[arg(long)]
    dry_run: bool,
}

/// Main execute function for news loading
pub async fn execute(args: NewsArgs, config: Config) -> Result<()> {
    info!("Starting news sentiment loader");

    // Validate limit
    if args.limit > 1000 {
        return Err(anyhow::anyhow!("Limit cannot exceed 1000 (API maximum)"));
    }
    if args.limit < 1 {
        return Err(anyhow::anyhow!("Limit must be at least 1"));
    }

    // Create API client
    let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Configure news loader
    let news_config = NewsLoaderConfig {
        days_back: Some(args.days_back),
        topics: args.topics.clone(),
        sort_order: Some(args.sort_order.clone()),
        limit: Some(args.limit),
        enable_cache: !args.no_cache,
        cache_ttl_hours: args.cache_ttl_hours,
        force_refresh: args.force_refresh,
        database_url: config.database_url.clone(),
    };

    info!("📰 News Loader Configuration:");
    info!("  Limit: {} articles per symbol", args.limit);
    info!("  Days back: {}", args.days_back);
    info!("  Sort: {}", args.sort_order);
    info!("  Cache: {}", if args.no_cache { "Disabled" } else { "Enabled" });

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
    }

    if args.all_equity {
        // Load news for all equity symbols with overview=true
        info!("Loading news for all equity symbols with overview=true");

        match load_news_for_equity_symbols(
            client,
            &config.database_url,
            news_config
        ).await {
            Ok(output) => {
                info!("✅ News loading complete!");
                info!("  Articles processed: {}", output.articles_processed);
                info!("  News data items loaded: {}", output.loaded_count);

                if output.cache_hits > 0 {
                    info!("  Cache hits: {}", output.cache_hits);
                }
                if output.api_calls > 0 {
                    info!("  API calls made: {}", output.api_calls);
                }

                if !output.errors.is_empty() {
                    warn!("{} errors encountered during processing", output.errors.len());
                    for error in &output.errors {
                        error!("  - {}", error);
                    }
                    if !args.continue_on_error {
                        return Err(anyhow::anyhow!("Errors occurred during processing"));
                    }
                }
            }
            Err(e) => {
                error!("Failed to load news: {}", e);
                if !args.continue_on_error {
                    return Err(e.into());
                }
            }
        }
    } else if let Some(symbols) = &args.symbols {
        // Load news for specific symbols - need to look them up in database first
        info!("Looking up {} symbols in database", symbols.len());

        // Look up symbols in the database to get real sids
        let symbol_infos = tokio::task::spawn_blocking({
            let db_url = config.database_url.clone();
            let syms = symbols.clone();
            move || -> Result<Vec<NewsSymbolInfo>> {
                use av_database_postgres::schema::symbols;

                let mut conn = diesel::PgConnection::establish(&db_url)?;
                let mut infos = Vec::new();

                for sym in syms {
                    let result: Option<(i64, String, String, bool)> = symbols::table
                        .filter(symbols::symbol.eq(&sym))
                        .select((symbols::sid, symbols::symbol, symbols::sec_type, symbols::overview))
                        .first(&mut conn)
                        .optional()?;

                    match result {
                        Some((sid, symbol, sec_type, overview)) => {
                            if sec_type == "Equity" && overview {
                                infos.push(NewsSymbolInfo { sid, symbol });
                            } else {
                                warn!("Symbol {} found but is not an equity with overview=true (type: {}, overview: {})",
                                      sym, sec_type, overview);
                            }
                        }
                        None => {
                            warn!("Symbol {} not found in database", sym);
                        }
                    }
                }

                Ok(infos)
            }
        }).await??;

        if symbol_infos.is_empty() {
            return Err(anyhow::anyhow!(
                "No valid equity symbols with overview=true found. Symbols must be loaded in database first."
            ));
        }

        info!("Found {} valid symbols to process", symbol_infos.len());

        // Create loader
        let loader = NewsLoader::new(5).with_config(news_config);

        // Create input
        let input = NewsLoaderInput {
            symbols: symbol_infos,
            time_from: Some(Utc::now() - Duration::days(args.days_back as i64)),
            time_to: Some(Utc::now()),
        };

        // Create loader context
        let loader_config = LoaderConfig::default();
        let context = LoaderContext::new(client, loader_config);

        // Load news data
        match loader.load(&context, input).await {
            Ok(output) => {
                info!("✅ News loading complete!");
                info!("  Articles processed: {}", output.articles_processed);
                info!("  News data items loaded: {}", output.loaded_count);

                if output.cache_hits > 0 {
                    info!("  Cache hits: {}", output.cache_hits);
                }
                if output.api_calls > 0 {
                    info!("  API calls made: {}", output.api_calls);
                }

                if !output.errors.is_empty() {
                    warn!("{} errors encountered", output.errors.len());
                    for error in &output.errors {
                        error!("  - {}", error);
                    }
                    if !args.continue_on_error {
                        return Err(anyhow::anyhow!("Errors occurred during processing"));
                    }
                }

                if args.dry_run {
                    info!("Dry run mode - data fetched but not saved to database");
                }
            }
            Err(e) => {
                error!("Failed to load news: {}", e);
                if !args.continue_on_error {
                    return Err(e.into());
                }
            }
        }
    } else {
        return Err(anyhow::anyhow!("Please specify either --all-equity or --symbols"));
    }

    Ok(())
}