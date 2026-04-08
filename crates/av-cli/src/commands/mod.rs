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

//! Command module registry for `av-cli`.
//!
//! This module serves as the top-level organizational hub for all CLI commands.
//! It re-exports four public submodules, each corresponding to a primary command
//! category in the [`Commands`](crate::Commands) enum defined in `main.rs`.
//!
//! ## Module Structure
//!
//! ```text
//! commands/
//! ├── mod.rs               ← this file (module registry)
//! ├── load/                ← Data ingestion commands (directory module)
//! │   ├── mod.rs               LoadCommand, LoadSubcommands, execute()
//! │   ├── securities.rs        Load NASDAQ/NYSE listings from CSV
//! │   ├── overviews.rs         Load company overview data
//! │   ├── crypto.rs            Load crypto exchange rates
//! │   ├── crypto_overview.rs   Load crypto overviews / update GitHub metadata
//! │   ├── crypto_markets.rs    Load crypto market data
//! │   ├── crypto_mapping.rs    Load crypto symbol mappings
//! │   ├── crypto_metadata.rs   Load crypto metadata records
//! │   ├── crypto_news.rs       Load crypto news/sentiment
//! │   ├── crypto_intraday.rs   Load crypto intraday price data
//! │   ├── crypto_prices.rs     Load crypto price records
//! │   ├── crypto_details.rs    Load crypto detail records
//! │   ├── missing_symbols.rs   Identify and load missing symbols
//! │   ├── news.rs              Load equity news/sentiment
//! │   ├── top_movers.rs        Load top market gainers/losers
//! │   ├── daily.rs             Load daily time series data
//! │   ├── intraday.rs          Load intraday time series data
//! │   ├── news_utils.rs        Shared news parsing utilities
//! │   ├── numeric_helpers.rs   Numeric conversion helpers
//! │   ├── sid_generator.rs     Security ID generation
//! │   └── missing_symbol_logger.rs  Logging for missing symbol detection
//! ├── query.rs             ← Data query commands (single file)
//! │                            QueryCommand, QuerySubcommands, execute()
//! │                            Subcommands: symbol, list-symbols
//! │                            Status: UNIMPLEMENTED (todo! placeholders)
//! ├── sync.rs              ← Data synchronization commands (single file)
//! │                            SyncCommand, SyncCommands, handle_sync()
//! │                            Subcommands: market, crypto
//! │                            Status: UNIMPLEMENTED (todo! placeholders)
//! └── update/              ← Data update & reporting commands (directory module)
//!     ├── mod.rs               Submodule registry
//!     ├── crypto.rs            CryptoUpdateCommands, handle_crypto_update()
//!     ├── crypto_update_cli.rs UpdateCryptoArgs (clap argument definitions)
//!     ├── crypto_update_functions.rs  update_crypto_command() implementation
//!     ├── crypto_metadata_etl.rs      Metadata ETL pipeline
//!     └── stats.rs             StatsCommands, handle_stats() + reporting
//! ```
//!
//! ## Design Pattern
//!
//! Each submodule follows a consistent pattern:
//! - A **clap-derived struct** (e.g., `LoadCommand`, `QueryCommand`) that implements
//!   `clap::Args` and contains a `#[command(subcommand)]` enum field
//! - A **subcommand enum** (e.g., `LoadSubcommands`, `SyncCommands`) with variants
//!   for each operation
//! - An **async handler function** (e.g., `execute()`, `handle_sync()`) that matches
//!   on the subcommand enum and dispatches to the appropriate implementation
//!
//! All handlers accept a [`Config`](crate::config::Config) parameter and return
//! `anyhow::Result<()>`. The `load` and `update` modules use the full CLI config
//! (database URL + API config), while `query` and `sync` currently ignore it
//! (prefixed with `_config`).
//!
//! ## Imports from `main.rs`
//!
//! The following types and functions are imported by `main.rs` for CLI construction
//! and command dispatch:
//! - [`load::LoadCommand`] — Clap args struct for `av-cli load`
//! - [`query::QueryCommand`] — Clap args struct for `av-cli query`
//! - [`sync::SyncCommands`], [`sync::handle_sync`] — Enum + handler for `av-cli sync`
//! - [`update::crypto::CryptoUpdateCommands`], [`update::crypto::handle_crypto_update`]
//!   — Enum + handler for `av-cli update crypto`
//! - [`update::stats::StatsCommands`], [`update::stats::handle_stats`]
//!   — Enum + handler for `av-cli update stats`

/// Data ingestion commands — load data from AlphaVantage API and CSV files into
/// the PostgreSQL database. This is the largest command group with 17 subcommands
/// spanning equities, cryptocurrency, news, and market movers. See
/// [`load::LoadCommand`] and [`load::execute`].
pub mod load;

/// Data query commands — look up stored symbol data from the database.
/// Currently **unimplemented** (both subcommands contain `todo!` placeholders).
/// Exports [`query::QueryCommand`] and [`query::execute`].
pub mod query;

/// Data synchronization commands — sync market and crypto data from AlphaVantage.
/// Currently **unimplemented** (both subcommands contain `todo!` placeholders).
/// Exports [`sync::SyncCommands`] and [`sync::handle_sync`].
pub mod sync;

/// Data update and reporting commands — update existing crypto records from
/// external sources (CoinGecko, GitHub) and generate statistics reports on
/// database contents. Contains submodules for crypto updates, metadata ETL,
/// CLI argument definitions, and stats reporting. See [`update::crypto`] and
/// [`update::stats`].
pub mod update;
