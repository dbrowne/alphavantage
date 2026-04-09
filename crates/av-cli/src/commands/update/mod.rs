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

//! Update and reporting commands for `av-cli`.
//!
//! This module provides the `av-cli update` command group, which enriches existing
//! database records with data from external sources and generates statistics reports
//! on stored data. Unlike [`load`](super::load) (initial ingestion) or
//! [`sync`](super::sync) (refresh from AlphaVantage), the update commands pull data
//! from **third-party sources** (CoinGecko, GitHub) and perform analytics on what is
//! already in the database.
//!
//! ## Subcommand Routing
//!
//! The `update` command has two top-level subcommands, defined by
//! [`UpdateCommands`](crate::UpdateCommands) in `main.rs`:
//!
//! ```text
//! av-cli update
//! ├── crypto            Enrich crypto records from external sources
//! │   ├── basic             Descriptions and market cap rankings (CoinGecko)
//! │   ├── social            Social media metrics (CoinGecko)
//! │   ├── technical         Blockchain and GitHub data (CoinGecko + GitHub API)
//! │   ├── all               Run all of the above
//! │   └── metadata          Run the metadata ETL pipeline
//! └── stats             Generate statistics reports on stored data
//!     ├── crypto-mapping    API symbol mapping coverage and staleness
//!     ├── crypto-markets    Market data by exchange, volume, activity
//!     └── crypto-overview   High-level database overview
//! ```
//!
//! ## Module Organization
//!
//! This directory module contains five submodules, organized into two functional
//! groups:
//!
//! ### Crypto Update Group
//!
//! These modules work together to implement the `av-cli update crypto` subcommands:
//!
//! - [`crypto`] — **Command definition and dispatch.** Defines
//!   [`CryptoUpdateCommands`](crypto::CryptoUpdateCommands) (the clap subcommand
//!   enum with variants `Basic`, `Social`, `Technical`, `All`, `Metadata`) and
//!   [`handle_crypto_update`](crypto::handle_crypto_update) (the async dispatcher).
//!   This is the entry point imported by `main.rs`.
//!
//! - [`crypto_update_cli`] — **Argument definitions.** Defines
//!   [`UpdateCryptoArgs`](crypto_update_cli::UpdateCryptoArgs), the shared clap
//!   `Args` struct used by the `Basic`, `Social`, `Technical`, and `All` variants.
//!   Fields include `symbols` (comma-separated filter), `limit`, `delay_ms`,
//!   `coingecko_api_key`, `github_token`, `dry_run`, and `verbose`.
//!
//! - [`crypto_update_functions`] — **Execution logic.** Contains
//!   `update_crypto_command()`, the async function that performs the actual
//!   CoinGecko/GitHub API calls and database updates. Called by
//!   [`handle_crypto_update`](crypto::handle_crypto_update) for the `Basic`,
//!   `Social`, `Technical`, and `All` variants.
//!
//! - [`crypto_metadata_etl`] — **Metadata ETL pipeline.** Contains the
//!   implementation for the `av-cli update crypto metadata` subcommand. Performs
//!   extract-transform-load operations on cryptocurrency metadata records.
//!
//! ### Statistics Group
//!
//! - [`stats`] — **Statistics reporting.** Defines
//!   [`StatsCommands`](stats::StatsCommands) (subcommand enum with variants
//!   `CryptoMapping`, `CryptoMarkets`, `CryptoOverview`) and
//!   [`handle_stats`](stats::handle_stats) (async dispatcher). Each variant has
//!   dedicated flags for filtering and display options. Queries the PostgreSQL
//!   database directly via Diesel ORM and prints formatted reports to stdout.
//!
//! ## Configuration Differences
//!
//! The two subcommand groups use different configuration types, as handled by
//! [`handle_update`](crate::handle_update) in `main.rs`:
//!
//! - **Crypto commands** receive an [`av_core::Config`] (API key, base URL, rate
//!   limit, timeout, retries) — extracted from the CLI config's `api_config` field.
//!   They interact with external APIs, not the database directly.
//!
//! - **Stats commands** receive the full CLI [`Config`](crate::config::Config)
//!   (including `database_url`) because they query the PostgreSQL database to
//!   generate reports.

/// Crypto update command definitions and dispatch.
///
/// Exports [`CryptoUpdateCommands`](crypto::CryptoUpdateCommands) and
/// [`handle_crypto_update`](crypto::handle_crypto_update), which are imported
/// by `main.rs` for CLI parsing and command routing. Delegates execution to
/// [`crypto_update_functions`] for data updates and [`crypto_metadata_etl`]
/// for metadata ETL.
pub mod crypto;

/// Cryptocurrency metadata ETL pipeline.
///
/// Contains the implementation for `av-cli update crypto metadata`. Performs
/// extract-transform-load operations that process and normalize cryptocurrency
/// metadata records in the database.
pub mod crypto_metadata_etl;

/// Shared clap argument definitions for crypto update commands.
///
/// Defines [`UpdateCryptoArgs`](crypto_update_cli::UpdateCryptoArgs), the
/// argument struct shared by the `basic`, `social`, `technical`, and `all`
/// crypto update subcommands. Provides fine-grained control over which symbols
/// to update, API keys for external services, rate limiting, dry-run mode,
/// and verbosity.
pub mod crypto_update_cli;

/// Crypto update execution logic.
///
/// Contains `update_crypto_command()`, the async function that performs the
/// actual data enrichment — fetching from CoinGecko and GitHub APIs and
/// writing results to the database. Called by
/// [`handle_crypto_update`](crypto::handle_crypto_update) with appropriate
/// filter flags based on which subcommand variant was selected.
pub mod crypto_update_functions;

/// Statistics reporting commands.
///
/// Exports [`StatsCommands`](stats::StatsCommands) and
/// [`handle_stats`](stats::handle_stats), which are imported by `main.rs`
/// for CLI parsing and command routing. Provides three report types:
/// - `crypto-mapping` — API symbol mapping coverage, unmapped symbols, staleness
/// - `crypto-markets` — Market data breakdown by exchange, volume, activity
/// - `crypto-overview` — High-level database overview with record counts
pub mod stats;
