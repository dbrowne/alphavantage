use anyhow::{Context, Result};
use clap::Args;
use std::sync::Arc;
use tracing::{info, warn, error};

use av_core::{AlphaVantageClient, Config as AvConfig};
use av_loaders::{
    LoaderContext, LoaderConfig,
    crypto::{
        CryptoDataSource,
        metadata_loader::{
            CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataLoader,
            CryptoSymbolForMetadata, ProcessedCryptoMetadata,
        },
    },
};

use crate::config::Config;

/// Arguments for the crypto metadata command
#[derive(Args, Debug)]
pub struct CryptoMetadataArgs {
    /// Specific symbols to load (comma-separated). If not provided, loads for all crypto symbols
    #[arg(long, value_delimiter = ',')]
    pub symbols: Option<Vec<String>>,

    /// Skip database updates (dry run)
    #[arg(short, long)]
    pub dry_run: bool,

    /// Update existing metadata entries
    #[arg(long)]
    pub update_existing: bool,

    /// AlphaVantage API key
    #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
    pub alphavantage_api_key: Option<String>,

    /// CoinGecko API key for enhanced metadata
    #[arg(long, env = "COINGECKO_API_KEY")]
    pub coingecko_api_key: Option<String>,

    /// Number of concurrent requests
    #[arg(long, default_value = "5")]
    pub concurrent: usize,

    /// Delay between requests in milliseconds
    #[arg(long, default_value = "1000")]
    pub delay_ms: u64,

    /// Batch size for processing
    #[arg(long, default_value = "50")]
    pub batch_size: usize,

    /// Maximum retry attempts per symbol
    #[arg(long, default_value = "3")]
    pub max_retries: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout_seconds: u64,

    /// Limit number of symbols to process (for testing)
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Show detailed progress information
    #[arg(long)]
    pub verbose: bool,

    /// Fetch enhanced metadata from CoinGecko
    #[arg(long, default_value = "true")]
    pub fetch_enhanced: bool,

    /// Data sources to use (alphavantage, coingecko)
    #[arg(long, value_delimiter = ',', default_values = ["coingecko", "alphavantage"])]
    pub sources: Vec<String>,

    /// Skip AlphaVantage metadata (use only CoinGecko)
    #[arg(long)]
    pub skip_alphavantage: bool,

    /// Enable response caching to reduce API costs
    #[arg(long, default_value = "true")]
    pub enable_cache: bool,

    /// Cache TTL in hours
    #[arg(long, default_value = "24")]
    pub cache_hours: u32,

    /// Force refresh - ignore cache and fetch fresh data
    #[arg(long)]
    pub force_refresh: bool,

    /// Skip CoinGecko metadata (use only AlphaVantage)
    #[arg(long)]
    pub skip_coingecko: bool,
}

/// Main execution function for crypto metadata loading
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

    // Clean up expired cache entries if requested
    if args.cleanup_cache {
        info!("Cleaning up expired cache entries...");
        match CryptoMetadataLoader::cleanup_expired_cache(&config.database_url).await {
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
    let crypto_symbols = load_crypto_symbols_from_db(
        &config.database_url,
        &args.symbols,
        args.limit,
    )?;

    if crypto_symbols.is_empty() {
        warn!("No cryptocurrency symbols found in database");
        return Ok(());
    }

    info!("Loaded {} crypto symbols for metadata processing", crypto_symbols.len());

    // Determine data sources to use
    let mut sources = Vec::new();

    if !args.skip_coingecko && (args.coingecko_api_key.is_some() || std::env::var("COINGECKO_API_KEY").is_ok()) {
        sources.push(CryptoDataSource::CoinGecko);
    }

    // For AlphaVantage, we'll use any available source as a placeholder since the enum doesn't have AlphaVantage
    // The actual AlphaVantage integration happens when the API key is detected in the loader
    if !args.skip_alphavantage && (args.alphavantage_api_key.is_some() || std::env::var("ALPHA_VANTAGE_API_KEY").is_ok()) {
        // Use SosoValue as a placeholder - the loader will detect AlphaVantage API key and use that instead
        sources.push(CryptoDataSource::SosoValue);
        info!("AlphaVantage API key detected - will use AlphaVantage for metadata (via SosoValue placeholder)");
    }

    if sources.is_empty() {
        error!("No valid data sources configured. Please provide API keys or enable sources.");
        return Ok(());
    }

    info!("Using data sources: {:?}", sources);

    // Create metadata loader configuration
    let loader_config = CryptoMetadataConfig {
        alphavantage_api_key: args.alphavantage_api_key.or_else(|| std::env::var("ALPHA_VANTAGE_API_KEY").ok()),
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

    let client = Arc::new(AlphaVantageClient::new(av_config));

    let loader_context = LoaderContext {
        client,
        config: LoaderConfig {
            max_concurrent_requests: args.concurrent,
            retry_attempts: args.max_retries,
            retry_delay_ms: args.delay_ms,
            show_progress: args.verbose,
            track_process: false,
            batch_size: args.batch_size,
        },
        process_tracker: None,
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
    let metadata_result = metadata_loader
        .load(&loader_context, input)
        .await
        .context("Failed to load crypto metadata")?;

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
        ).await
            .context("Failed to save metadata to database")?;

        info!("Successfully saved metadata: {} inserted, {} updated", inserted, updated);
    } else if args.dry_run {
        info!("Dry run completed. Found {} metadata entries", metadata_result.metadata_processed.len());
    }

    Ok(())
}

/// Load crypto symbols from database for metadata processing
fn load_crypto_symbols_from_db(
    database_url: &str,
    symbols_filter: &Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<CryptoSymbolForMetadata>> {
    use diesel::prelude::*;
    use av_database::postgres::{establish_connection, schema::{symbols, crypto_api_map}};

    let mut conn = establish_connection(database_url)
        .context("Failed to connect to database")?;

    let mut query = symbols::table
        .inner_join(crypto_api_map::table.on(symbols::sid.eq(crypto_api_map::sid)))
        .filter(symbols::security_type.eq("Cryptocurrency"))
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
        (i64, String, String, bool), // symbols: (sid, symbol, name, is_active)
        (String, String, String), // crypto_api_map: (api_source, api_id, api_slug)
    )> = query
        .select((
            (symbols::sid, symbols::symbol, symbols::name, symbols::is_active),
            (crypto_api_map::api_source, crypto_api_map::api_id, crypto_api_map::api_slug.assume_not_null()),
        ))
        .load(&mut conn)
        .context("Failed to load crypto symbols from database")?;

    let crypto_symbols = results
        .into_iter()
        .map(|((sid, symbol, name, is_active), (api_source, api_id, api_slug))| {
            let source = match api_source.as_str() {
                "coingecko" => CryptoDataSource::CoinGecko,
                "alphavantage" => CryptoDataSource::SosoValue, // Use SosoValue as placeholder for AlphaVantage
                _ => CryptoDataSource::CoinGecko, // default fallback
            };

            CryptoSymbolForMetadata {
                sid,
                symbol,
                name,
                source,
                source_id: api_slug, // Use slug as source_id for CoinGecko
                is_active,
            }
        })
        .collect();

    Ok(crypto_symbols)
}

/// Save metadata to database
async fn save_metadata_to_db(
    database_url: &str,
    metadata: &[ProcessedCryptoMetadata],
    update_existing: bool,
) -> Result<(usize, usize)> {
    use diesel::prelude::*;
    use av_database::postgres::{establish_connection, schema::crypto_metadata};

    let mut conn = establish_connection(database_url)
        .context("Failed to connect to database")?;

    let mut inserted = 0;
    let mut updated = 0;

    for metadata_entry in metadata {
        let new_metadata = (
            crypto_metadata::sid.eq(metadata_entry.sid),
            crypto_metadata::source.eq(&metadata_entry.source),
            crypto_metadata::source_id.eq(&metadata_entry.source_id),
            crypto_metadata::market_cap_rank.eq(metadata_entry.market_cap_rank),
            crypto_metadata::base_currency.eq(&metadata_entry.base_currency),
            crypto_metadata::quote_currency.eq(&metadata_entry.quote_currency),
            crypto_metadata::is_active.eq(metadata_entry.is_active),
            crypto_metadata::additional_data.eq(&metadata_entry.additional_data),
            crypto_metadata::last_updated.eq(metadata_entry.last_updated),
        );

        if update_existing {
            // Try to update first using the unique constraint (source, source_id)
            let update_result = diesel::update(crypto_metadata::table)
                .filter(
                    crypto_metadata::source.eq(&metadata_entry.source)
                        .and(crypto_metadata::source_id.eq(&metadata_entry.source_id))
                )
                .set((
                    crypto_metadata::sid.eq(metadata_entry.sid),
                    crypto_metadata::market_cap_rank.eq(metadata_entry.market_cap_rank),
                    crypto_metadata::base_currency.eq(&metadata_entry.base_currency),
                    crypto_metadata::quote_currency.eq(&metadata_entry.quote_currency),
                    crypto_metadata::is_active.eq(metadata_entry.is_active),
                    crypto_metadata::additional_data.eq(&metadata_entry.additional_data),
                    crypto_metadata::last_updated.eq(metadata_entry.last_updated),
                ))
                .execute(&mut conn);

            match update_result {
                Ok(rows_affected) if rows_affected > 0 => {
                    updated += 1;
                }
                _ => {
                    // Insert if update didn't affect any rows
                    match diesel::insert_into(crypto_metadata::table)
                        .values(new_metadata)
                        .execute(&mut conn)
                    {
                        Ok(_) => inserted += 1,
                        Err(e) => {
                            // Check if it's a unique constraint violation on (source, source_id)
                            let error_msg = e.to_string();
                            if error_msg.contains("crypto_metadata_source_source_id_key")
                                || error_msg.contains("UNIQUE constraint") {
                                warn!("Duplicate metadata entry for source '{}', source_id '{}' - skipping",
                                      metadata_entry.source, metadata_entry.source_id);
                            } else {
                                warn!("Failed to insert metadata for sid {} ({}:{}): {}",
                                      metadata_entry.sid, metadata_entry.source, metadata_entry.source_id, e);
                            }
                        }
                    }
                }
            }
        } else {
            // Insert only with proper conflict resolution for UNIQUE(source, source_id)
            match diesel::insert_into(crypto_metadata::table)
                .values(new_metadata)
                .on_conflict((crypto_metadata::source, crypto_metadata::source_id))
                .do_nothing()
                .execute(&mut conn)
            {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        inserted += 1;
                    }
                }
                Err(e) => warn!("Failed to insert metadata for sid {} ({}:{}): {}",
                               metadata_entry.sid, metadata_entry.source, metadata_entry.source_id, e),
            }
        }
    }

    Ok((inserted, updated))
}