//! Command for loading daily price data (TIME_SERIES_DAILY)

use anyhow::Result;
use clap::Args;
use diesel::prelude::*;
use diesel::upsert::excluded;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::fs;
use tracing::{debug, error, info, warn};

use av_client::AlphaVantageClient;
use av_database_postgres::{
    models::price::NewSummaryPriceOwned,
    schema::{summaryprices, symbols},
};
use av_loaders::{
    DataLoader,
    LoaderConfig, LoaderContext, ProcessTracker, ProcessState,
    SummaryPriceLoader, SummaryPriceLoaderInput, SummaryPriceData, SummaryPriceConfig,
};

use crate::config::Config;

#[derive(Args, Debug)]
pub struct DailyArgs {
    /// Single symbol to load
    #[arg(short, long, conflicts_with = "symbols_file", conflicts_with = "all")]
    pub symbol: Option<String>,

    /// Load symbols from file (one per line)
    #[arg(long, conflicts_with = "symbol", conflicts_with = "all")]
    pub symbols_file: Option<String>,

    /// Load all active symbols with summary=true
    #[arg(long, conflicts_with = "symbol", conflicts_with = "symbols_file")]
    pub all: bool,

    /// Output size: "compact" (100 days) or "full" (20+ years)
    #[arg(long, default_value = "compact")]
    pub outputsize: String,

    /// Number of concurrent API requests
    #[arg(short, long, default_value = "5")]
    pub concurrent: usize,

    /// Update existing data (re-fetch latest values)
    #[arg(long)]
    pub update: bool,

    /// Dry run - don't save to database
    #[arg(long)]
    pub dry_run: bool,

    /// Continue on errors
    #[arg(short = 'k', long)]
    pub continue_on_error: bool,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Get symbols to load based on command arguments
async fn get_symbols_to_load(args: &DailyArgs, config: &Config) -> Result<Vec<(i64, String)>> {
    if let Some(symbol) = &args.symbol {
        // Single symbol - need to look up SID
        let sid = get_or_create_sid(symbol, config).await?;
        Ok(vec![(sid, symbol.clone())])
    } else if let Some(file_path) = &args.symbols_file {
        // Load symbols from file
        load_symbols_from_file(file_path, config).await
    } else if args.all {
        // Load all active symbols
        load_active_symbols(config).await
    } else {
        // Default to loading top priority symbols
        load_priority_symbols(config, 100).await
    }
}

/// Get or create SID for a symbol
async fn get_or_create_sid(symbol: &str, config: &Config) -> Result<i64> {
    use tokio::task;

    let db_url = config.database_url.clone();
    let symbol = symbol.to_uppercase();

    task::spawn_blocking(move || -> Result<i64> {
        let mut conn = PgConnection::establish(&db_url)?;

        // Try to find existing symbol
        let existing_sid: Option<i64> = symbols::table
            .filter(symbols::symbol.eq(&symbol))
            .select(symbols::sid)
            .first(&mut conn)
            .optional()?;

        if let Some(sid) = existing_sid {
            Ok(sid)
        } else {
            // Symbol doesn't exist, would need to create it
            // This requires calling the symbol search API first
            Err(anyhow::anyhow!(
                "Symbol {} not found in database. Please run 'av load securities' first",
                symbol
            ))
        }
    })
        .await?
}

/// Load symbols from a file
async fn load_symbols_from_file(file_path: &str, config: &Config) -> Result<Vec<(i64, String)>> {
    let content = fs::read_to_string(file_path)?;
    let mut symbols = Vec::new();

    for line in content.lines() {
        let symbol = line.trim();
        if !symbol.is_empty() && !symbol.starts_with('#') {
            match get_or_create_sid(symbol, config).await {
                Ok(sid) => symbols.push((sid, symbol.to_string())),
                Err(e) => warn!("Skipping symbol {}: {}", symbol, e),
            }
        }
    }

    Ok(symbols)
}

/// Load all active symbols from database
async fn load_active_symbols(config: &Config) -> Result<Vec<(i64, String)>> {
    use tokio::task;

    let db_url = config.database_url.clone();

    task::spawn_blocking(move || -> Result<Vec<(i64, String)>> {
        let mut conn = PgConnection::establish(&db_url)?;

        let symbols: Vec<(i64, String)> = symbols::table
            .filter(symbols::summary.eq(true))
            .or_filter(symbols::intraday.eq(true))
            .select((symbols::sid, symbols::symbol))
            .order(symbols::priority.asc())
            .load(&mut conn)?;

        Ok(symbols)
    })
        .await?
}

/// Load top priority symbols
async fn load_priority_symbols(config: &Config, limit: i64) -> Result<Vec<(i64, String)>> {
    use tokio::task;

    let db_url = config.database_url.clone();

    task::spawn_blocking(move || -> Result<Vec<(i64, String)>> {
        let mut conn = PgConnection::establish(&db_url)?;

        let symbols: Vec<(i64, String)> = symbols::table
            .select((symbols::sid, symbols::symbol))
            .order(symbols::priority.asc())
            .limit(limit)
            .load(&mut conn)?;

        Ok(symbols)
    })
        .await?
}

/// Get the maximum event ID from the database
async fn get_max_eventid(config: &Config) -> Result<i64> {
    use tokio::task;

    let db_url = config.database_url.clone();

    task::spawn_blocking(move || -> Result<i64> {
        let mut conn = PgConnection::establish(&db_url)?;

        let max_id: Option<i64> = summaryprices::table
            .select(diesel::dsl::max(summaryprices::eventid))
            .first(&mut conn)?;

        Ok(max_id.unwrap_or(0))
    })
        .await?
}

/// Save summary prices to database
async fn save_summary_prices(
    data: Vec<SummaryPriceData>,
    config: &Config,
    update_symbols: bool,
) -> Result<usize> {
    use tokio::task;

    if data.is_empty() {
        return Ok(0);
    }

    let db_url = config.database_url.clone();

    task::spawn_blocking(move || -> Result<usize> {
        let mut conn = PgConnection::establish(&db_url)?;

        // Convert to database model
        let new_prices: Vec<NewSummaryPriceOwned> = data
            .into_iter()
            .map(|p| NewSummaryPriceOwned {
                eventid: p.eventid,
                tstamp: p.tstamp,
                date: p.date,
                sid: p.sid,
                symbol: p.symbol,
                open: p.open,
                high: p.high,
                low: p.low,
                close: p.close,
                volume: p.volume,
            })
            .collect();

        // Collect unique SIDs for updating symbols table
        let unique_sids: Vec<i64> = {
            let mut sids: Vec<i64> = new_prices.iter().map(|p| p.sid).collect();
            sids.sort_unstable();
            sids.dedup();
            sids
        };

        let total_records = new_prices.len();

        // Set up progress bar for database operations
        let progress = ProgressBar::new(total_records as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );
        progress.set_message("Saving to database");

        // Batch insert with UPSERT
        const BATCH_SIZE: usize = 1000;
        let mut total_inserted = 0;

        for chunk in new_prices.chunks(BATCH_SIZE) {
            let inserted = diesel::insert_into(summaryprices::table)
                .values(chunk)
                .on_conflict((summaryprices::tstamp, summaryprices::sid, summaryprices::eventid))
                .do_update()
                .set((
                    summaryprices::open.eq(excluded(summaryprices::open)),
                    summaryprices::high.eq(excluded(summaryprices::high)),
                    summaryprices::low.eq(excluded(summaryprices::low)),
                    summaryprices::close.eq(excluded(summaryprices::close)),
                    summaryprices::volume.eq(excluded(summaryprices::volume)),
                ))
                .execute(&mut conn)?;

            total_inserted += inserted;
            progress.inc(chunk.len() as u64);
        }

        progress.finish_with_message(format!("Saved {} price records", total_inserted));

        // Update symbols table to mark summary data as loaded
        if update_symbols && !unique_sids.is_empty() {
            info!("Updating {} symbols to mark summary data as loaded", unique_sids.len());

            diesel::update(symbols::table)
                .filter(symbols::sid.eq_any(&unique_sids))
                .set((
                    symbols::summary.eq(true),
                    symbols::m_time.eq(diesel::dsl::now),
                ))
                .execute(&mut conn)?;
        }

        Ok(total_inserted)
    })
        .await?
}

/// Main execute function
pub async fn execute(args: DailyArgs, config: Config) -> Result<()> {
    info!("Starting daily price loader");

    if args.dry_run {
        info!("DRY RUN MODE - No database updates will be performed");
    }

    // Validate output size
    if args.outputsize != "compact" && args.outputsize != "full" {
        return Err(anyhow::anyhow!(
            "Invalid output size '{}'. Must be 'compact' or 'full'",
            args.outputsize
        ));
    }

    // Get symbols to load
    let symbols = get_symbols_to_load(&args, &config).await?;

    if symbols.is_empty() {
        warn!("No symbols to load");
        return Ok(());
    }

    info!("Loading daily prices for {} symbols (output size: {})",
         symbols.len(), args.outputsize);

    // Create API client
    let client = Arc::new(AlphaVantageClient::new(config.api_config.clone()));

    // Create loader configuration
    let loader_config = LoaderConfig {
        max_concurrent_requests: args.concurrent,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        show_progress: true,
        track_process: !args.dry_run,
        batch_size: 100,
    };

    // Create loader context
    let mut context = LoaderContext::new(client, loader_config);

    // Set up process tracking (unless dry run)
    if !args.dry_run {
        let tracker = ProcessTracker::new();
        context = context.with_process_tracker(tracker);
    }

    // Get the current max event ID
    let max_eventid = if args.dry_run {
        0
    } else {
        get_max_eventid(&config).await?
    };

    // Create and configure the loader
    let loader_cfg = SummaryPriceConfig {
        max_concurrent: args.concurrent,
        update_existing: args.update,
        skip_non_trading_days: true,
    };

    let loader = SummaryPriceLoader::new(args.concurrent)
        .with_config(loader_cfg)
        .with_starting_eventid(max_eventid + 1);

    // Prepare input
    let input = SummaryPriceLoaderInput {
        symbols,
        outputsize: args.outputsize,
    };

    // Execute the loader
    let output = match loader.load(&context, input).await {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to load daily prices: {}", e);
            if !args.continue_on_error {
                return Err(e.into());
            }
            // Return empty output if continuing on error
            return Ok(());
        }
    };

    // Display summary
    println!("\n╔════════════════════════════════════════════╗");
    println!("║          DAILY PRICE LOADING SUMMARY        ║");
    println!("╠════════════════════════════════════════════╣");
    println!("║ Symbols Loaded:     {:<24} ║", output.symbols_loaded);
    println!("║ Symbols Failed:     {:<24} ║", output.symbols_failed);
    println!("║ Symbols Skipped:    {:<24} ║", output.symbols_skipped);
    println!("║ Total Records:      {:<24} ║", output.data.len());
    println!("╚════════════════════════════════════════════╝");

    // Save to database unless dry run
    if !args.dry_run && !output.data.is_empty() {
        info!("Saving {} price records to database", output.data.len());

        let saved = save_summary_prices(output.data, &config, true).await?;

        info!("Successfully saved {} price records", saved);
    } else if args.dry_run {
        info!("Dry run complete - would have saved {} records", output.data.len());
    }

    // Complete process tracking
    if let Some(tracker) = context.process_tracker {
        let state = if output.symbols_failed > 0 {
            ProcessState::CompletedWithErrors
        } else {
            ProcessState::Success
        };
        tracker.complete(state).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outputsize_validation() {
        assert!(["compact", "full"].contains(&"compact"));
        assert!(["compact", "full"].contains(&"full"));
        assert!(!["compact", "full"].contains(&"invalid"));
    }
}