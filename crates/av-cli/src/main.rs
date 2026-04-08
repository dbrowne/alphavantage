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

//! # av-cli — AlphaVantage Command-Line Interface
//!
//! `av-cli` is an async CLI application for loading, querying, syncing, and updating
//! financial market data from the [AlphaVantage API](https://www.alphavantage.co/).
//! It supports both traditional securities (equities, ETFs) and cryptocurrency data,
//! with a PostgreSQL database backend for persistent storage.
//!
//! ## Architecture
//!
//! The CLI is built with [`clap`] for argument parsing and [`tokio`] for async execution.
//! Configuration is loaded from environment variables (via `.env` files) and encapsulated
//! in [`config::Config`], which wraps the core API configuration from [`av_core::Config`]
//! along with database and CSV path settings.
//!
//! ## Command Hierarchy
//!
//! ```text
//! av-cli [-v|--verbose]
//! ├── load              Load data from AlphaVantage into the database
//! │   ├── securities        Load security listings (NASDAQ/NYSE CSV files)
//! │   ├── overviews         Load company overviews
//! │   ├── crypto            Load cryptocurrency exchange rates
//! │   ├── crypto-overview   Load crypto overview data
//! │   ├── update-github     Update GitHub metadata for crypto projects
//! │   ├── crypto-markets    Load crypto market data
//! │   ├── crypto-mapping    Load crypto symbol mappings
//! │   ├── crypto-metadata   Load crypto metadata
//! │   ├── crypto-news       Load crypto news/sentiment
//! │   ├── crypto-intraday   Load crypto intraday prices
//! │   ├── crypto-prices     Load crypto prices
//! │   ├── crypto-details    Load crypto detail records
//! │   ├── missing-symbols   Identify and load missing symbols
//! │   ├── news              Load news/sentiment for equities
//! │   ├── top-movers        Load top market gainers/losers
//! │   ├── daily             Load daily time series data
//! │   └── intraday          Load intraday time series data
//! ├── query             Query stored data (currently unimplemented)
//! │   ├── symbol            Look up a specific symbol
//! │   └── list-symbols      List symbols with optional exchange filter
//! ├── sync              Sync data from AlphaVantage (currently unimplemented)
//! │   ├── market            Sync market data (optional --symbol filter)
//! │   └── crypto            Sync crypto data (optional --limit)
//! └── update            Update existing database records
//!     ├── crypto            Update crypto records from external sources
//!     │   ├── basic             Update descriptions and market cap ranks
//!     │   ├── social            Update social media metrics
//!     │   ├── technical         Update blockchain/GitHub data
//!     │   ├── all               Update all crypto data categories
//!     │   └── metadata          Run metadata ETL operations
//!     └── stats             Generate statistics reports
//!         ├── crypto-mapping    Report on API symbol mapping coverage
//!         ├── crypto-markets    Report on market data by exchange/volume
//!         └── crypto-overview   Report on database overview statistics
//! ```
//!
//! ## Configuration (Environment Variables)
//!
//! | Variable               | Required | Description                                |
//! |------------------------|----------|--------------------------------------------|
//! | `ALPHA_VANTAGE_API_KEY`| Yes      | API key for AlphaVantage                   |
//! | `DATABASE_URL`         | Yes      | PostgreSQL connection string                |
//! | `NASDAQ_LISTED`        | No       | Path to NASDAQ CSV file (has default)      |
//! | `OTHER_LISTED`         | No       | Path to NYSE/other CSV file (has default)  |
//!
//! ## Example Usage
//!
//! ```bash
//! # Load NASDAQ/NYSE security listings into the database
//! av-cli load securities
//!
//! # Load daily time series data with verbose logging
//! av-cli -v load daily
//!
//! # Update all crypto records from CoinGecko (dry run)
//! av-cli update crypto all --dry-run
//!
//! # View crypto mapping statistics with stale symbol detection
//! av-cli update stats crypto-mapping --stale --stale-days 30
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

mod commands;
use crate::commands::update::stats::{StatsCommands, handle_stats};
use commands::{
  load::LoadCommand,
  query::QueryCommand,
  sync::{SyncCommands, handle_sync},
  update::crypto::{CryptoUpdateCommands, handle_crypto_update},
};

mod config;

/// Top-level CLI structure parsed by [`clap`].
///
/// Provides a single global flag (`--verbose`) that switches the tracing log level
/// from `info` to `debug`, and delegates to one of the four primary [`Commands`]
/// subcommands.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "av-cli")]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  /// Verbose output — sets log level to `debug` (default is `info`)
  #[arg(short, long, global = true)]
  verbose: bool,
}

/// Primary command categories for the CLI.
///
/// Each variant maps to a top-level subcommand:
/// - [`Load`](Commands::Load) — Ingest data from AlphaVantage API or CSV files into
///   the PostgreSQL database. Supports 17 distinct subcommands covering equities,
///   crypto, news, and market movers.
/// - [`Query`](Commands::Query) — Look up stored symbol data. Currently unimplemented
///   (`todo!` placeholders).
/// - [`Sync`](Commands::Sync) — Synchronize market and crypto data. Currently
///   unimplemented (`todo!` placeholders).
/// - [`Update`](Commands::Update) — Update existing records and generate statistics
///   reports. Delegates to [`UpdateCommands`].
#[derive(Subcommand, Debug)]
enum Commands {
  Load(LoadCommand),
  Query(QueryCommand),
  Sync {
    #[command(subcommand)]
    cmd: SyncCommands,
  },
  Update {
    #[command(subcommand)]
    cmd: UpdateCommands,
  },
}

/// Subcommands under `av-cli update`.
///
/// - [`Crypto`](UpdateCommands::Crypto) — Update cryptocurrency records from external
///   data sources (CoinGecko, GitHub). Accepts [`CryptoUpdateCommands`] with variants
///   for basic, social, technical, all, and metadata updates. Uses [`av_core::Config`]
///   rather than the full CLI config (see [`handle_update`] for the config conversion).
/// - [`Stats`](UpdateCommands::Stats) — Generate statistics reports on crypto mapping
///   coverage, market data, and database overview. Uses the full CLI [`config::Config`]
///   for direct database access.
#[derive(Subcommand, Debug)]
pub enum UpdateCommands {
  Crypto {
    #[command(subcommand)]
    cmd: CryptoUpdateCommands,
  },
  Stats {
    #[command(subcommand)]
    cmd: StatsCommands,
  },
}

/// Application entry point.
///
/// Performs four setup steps before dispatching the user's command:
///
/// 1. **Environment loading** — Calls [`dotenv()`] to load variables from a `.env` file
///    in the working directory (or parent directories). Failures are silently ignored
///    (`.ok()`), so the app works without a `.env` file as long as the required
///    environment variables are set.
///
/// 2. **Argument parsing** — Uses [`clap`] to parse CLI arguments into the [`Cli`] struct.
///    Invalid arguments produce an automatic error/help message and exit.
///
/// 3. **Logging initialization** — Configures [`tracing_subscriber`] with a format
///    subscriber. The log level is `debug` if `--verbose` is passed, otherwise `info`.
///    The `RUST_LOG` environment variable can further override this via the env filter.
///
/// 4. **Configuration** — Loads [`config::Config`] from environment variables. This
///    includes the AlphaVantage API key, base URL, rate limits, timeouts, retry counts,
///    the PostgreSQL `DATABASE_URL`, and paths to NASDAQ/NYSE CSV listing files.
///
/// After setup, the parsed command is matched and dispatched to the appropriate handler:
/// - `load` → [`commands::load::execute`]
/// - `query` → [`commands::query::execute`]
/// - `sync` → [`handle_sync`]
/// - `update` → [`handle_update`] (local routing function)
#[tokio::main]
async fn main() -> Result<()> {
  // Load environment variables from .env file (if present)
  dotenv().ok();

  // Parse CLI arguments via clap
  let cli = Cli::parse();

  // Initialize tracing subscriber with log level based on --verbose flag
  let log_level = if cli.verbose { "debug" } else { "info" };
  tracing_subscriber::fmt().with_env_filter(log_level).init();

  // Load configuration from environment variables
  let config = config::Config::from_env()?;

  // Dispatch to the appropriate command handler
  match cli.command {
    Commands::Load(cmd) => commands::load::execute(cmd, config).await?,
    Commands::Query(cmd) => commands::query::execute(cmd, config).await?,
    Commands::Sync { cmd } => handle_sync(cmd, config).await?,
    Commands::Update { cmd } => handle_update(cmd, config).await?,
  }

  Ok(())
}

/// Routes `update` subcommands to their respective handlers.
///
/// This function exists because the `Crypto` and `Stats` update subcommands require
/// different configuration types:
///
/// - **Crypto updates** use [`av_core::Config`] (API-only configuration) because the
///   crypto update handlers live in `av_core` and operate through the AlphaVantage API
///   client. The full CLI [`config::Config`] is destructured and its `api_config` fields
///   are mapped into a new `av_core::Config` instance.
///
/// - **Stats commands** use the full CLI [`config::Config`] because they need direct
///   database access (via `DATABASE_URL`) to generate reports on stored data.
///
/// # Arguments
///
/// * `cmd` — The parsed [`UpdateCommands`] variant (either `Crypto` or `Stats`).
/// * `config` — The full CLI configuration loaded from environment variables.
///
/// # Errors
///
/// Returns any error propagated from the underlying command handlers (e.g., API
/// failures, database connection errors, or query errors).
async fn handle_update(cmd: UpdateCommands, config: config::Config) -> Result<()> {
  match cmd {
    UpdateCommands::Crypto { cmd } => {
      // Convert the CLI's config::Config into av_core::Config by extracting
      // only the API-related fields. This is necessary because the crypto update
      // handlers are defined in the av_core crate and expect its own Config type.
      let core_config = av_core::Config {
        api_key: config.api_config.api_key,
        base_url: config.api_config.base_url,
        rate_limit: config.api_config.rate_limit,
        timeout_secs: config.api_config.timeout_secs,
        max_retries: config.api_config.max_retries,
      };
      handle_crypto_update(cmd, core_config).await
    }
    UpdateCommands::Stats { cmd } => handle_stats(cmd, config).await,
  }
}
