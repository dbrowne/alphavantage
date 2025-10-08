// crates/av-cli/src/commands/load/top_movers.rs
use anyhow::Result;
use clap::Args;
use std::sync::Arc;
use chrono::NaiveDate;

use av_client::AlphaVantageClient;
use av_loaders::{
    DataLoader,
    LoaderConfig, LoaderContext, ProcessTracker,
    top_movers_loader::{TopMoversLoader, TopMoversLoaderInput},
};

use crate::config::Config;

#[derive(Args, Debug) ]
pub struct TopMoversArgs {
    /// Date to load (YYYY-MM-DD format)
    #[arg(short, long)]
    date: Option<String>,

    /// Dry run - don't save to database
    #[arg(long)]
    dry_run: bool,

    /// Show verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

pub async fn execute(args: TopMoversArgs, config: Config) -> Result<()> {
    // Create API client with the correct Config type
    let client = Arc::new(AlphaVantageClient::new(config.api_config));

    // Create loader configuration
    let loader_config = LoaderConfig {
        max_concurrent_requests: 1,  // Top movers is a single API call
        retry_attempts: 3,
        retry_delay_ms: 1000,
        show_progress: false,  // Single call, no need for progress
        track_process: !args.dry_run,  // Track process unless dry run
        batch_size: 1000,
    };

    // Create loader context
    let mut context = LoaderContext::new(client, loader_config);

    // Setup process tracker unless dry run
    if !args.dry_run {
        let tracker = ProcessTracker::new();
        context = context.with_process_tracker(tracker);
    }

    // Setup loader with database URL (None only for dry run)
    let database_url = if args.dry_run {
        None
    } else {
        Some(config.database_url.clone())
    };

    let loader = TopMoversLoader::new(database_url);

    // Parse date if provided
    let date = args.date.and_then(|d| {
        NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()
    });

    let input = TopMoversLoaderInput {
        date,
    };

    let output = loader.load(&context, input).await?;

    // Display results
    println!("\n╔════════════════════════════════════════╗");
    println!("║       TOP MARKET MOVERS                ║");
    println!("╠════════════════════════════════════════╣");
    println!("║ Date: {:<33} ║", output.date);
    println!("║ Last Updated: {:<24} ║", output.last_updated);
    println!("╚════════════════════════════════════════╝\n");

    println!("📈 Top {} Gainers", output.gainers_count);
    if args.verbose {
        for gainer in output.raw_data.top_gainers.iter().take(5) {
            println!("   {} | ${} | {}% ↑",
                     gainer.ticker, gainer.price, gainer.change_percentage);
        }
        println!();
    }

    println!("📉 Top {} Losers", output.losers_count);
    if args.verbose {
        for loser in output.raw_data.top_losers.iter().take(5) {
            println!("   {} | ${} | {}% ↓",
                     loser.ticker, loser.price, loser.change_percentage);
        }
        println!();
    }

    println!("📊 Top {} Most Active", output.most_active_count);
    if args.verbose {
        for active in output.raw_data.most_actively_traded.iter().take(5) {
            println!("   {} | ${} | Vol: {}",
                     active.ticker, active.price, active.volume);
        }
        println!();
    }

    if args.dry_run {
        println!("\n⚠️  Dry run mode - no data saved to database");
    } else {
        println!("\n✅ Database Update:");
        println!("   Records saved: {}", output.records_saved);
        if !output.missing_symbols.is_empty() {
            println!("   ⚠️  Missing symbols: {}", output.missing_symbols.len());
            if args.verbose {
                println!("   Missing symbols:");
                for symbol in &output.missing_symbols {
                    println!("      - {}", symbol);
                }
            }
        }
    }

    Ok(())
}