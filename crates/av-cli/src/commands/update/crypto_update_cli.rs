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

//! Shared clap argument definitions for the `av-cli update crypto` subcommands.
//!
//! This module defines [`UpdateCryptoArgs`], the argument struct shared by the
//! `basic`, `social`, `technical`, and `all` variants of
//! [`CryptoUpdateCommands`](super::crypto::CryptoUpdateCommands). It is purely
//! declarative — no execution logic lives here.
//!
//! ## Argument Categories
//!
//! The fields fall into four groups:
//!
//! - **Filtering** — `symbols` and `limit` control which and how many
//!   cryptocurrency records are processed.
//! - **Category selection** — `basic_only`, `social_only`, and `technical_only`
//!   restrict the update to a single data category. These flags are also set
//!   **programmatically** by [`handle_crypto_update`](super::crypto::handle_crypto_update)
//!   based on which subcommand variant was selected. When none are set, all
//!   categories are updated.
//! - **API credentials** — `coingecko_api_key` and `github_token` authenticate
//!   with external data sources. Both support environment variable fallback
//!   (`COINGECKO_API_KEY`, `GITHUB_TOKEN`).
//! - **Execution control** — `delay_ms` (rate limiting), `dry_run` (skip
//!   database writes), and `verbose` (detailed output).
//!
//! ## Usage
//!
//! ```bash
//! # Update basic data for specific symbols with a dry run
//! av-cli update crypto basic --symbols BTC,ETH,SOL --dry-run
//!
//! # Update all categories with custom rate limiting
//! av-cli update crypto all --delay-ms 3000 --limit 100
//! ```

use clap::Args;

/// Shared command-line arguments for the `av-cli update crypto` subcommands.
///
/// This struct is wrapped by the `Basic`, `Social`, `Technical`, and `All`
/// variants of [`CryptoUpdateCommands`](super::crypto::CryptoUpdateCommands)
/// and passed to
/// [`update_crypto_command`](super::crypto_update_functions::update_crypto_command)
/// for execution.
///
/// The `*_only` boolean flags determine which data category to update. They are
/// available as CLI flags but are also **set programmatically** by the
/// subcommand dispatcher in
/// [`handle_crypto_update`](super::crypto::handle_crypto_update):
///
/// | Subcommand   | Flag set automatically     |
/// |-------------|---------------------------|
/// | `basic`      | `basic_only = true`       |
/// | `social`     | `social_only = true`      |
/// | `technical`  | `technical_only = true`   |
/// | `all`        | (none — all categories)   |
#[derive(Args, Debug)]
pub struct UpdateCryptoArgs {
  /// Comma-separated list of cryptocurrency symbols to update (e.g., `BTC,ETH,SOL`).
  ///
  /// When omitted, all symbols in the database are processed.
  #[arg(short, long, value_delimiter = ',')]
  pub symbols: Option<String>,

  /// Maximum number of symbols to process.
  ///
  /// When omitted, no limit is applied and all matching symbols are updated.
  #[arg(short, long)]
  pub limit: Option<usize>,

  /// Restrict update to basic crypto data only (descriptions, market cap ranks).
  ///
  /// Set automatically by the `basic` subcommand; also available as a CLI flag.
  #[arg(long)]
  pub basic_only: bool,

  /// Restrict update to social data only (Twitter followers, Reddit subscribers,
  /// community scores).
  ///
  /// Set automatically by the `social` subcommand; also available as a CLI flag.
  #[arg(long)]
  pub social_only: bool,

  /// Restrict update to technical data only (blockchain statistics, GitHub
  /// repository metrics).
  ///
  /// Set automatically by the `technical` subcommand; also available as a CLI flag.
  #[arg(long)]
  pub technical_only: bool,

  /// Delay between API requests in milliseconds.
  ///
  /// Controls rate limiting for CoinGecko and GitHub API calls. Defaults to
  /// 2000 ms (2 seconds).
  #[arg(long, default_value = "2000")]
  pub delay_ms: u64,

  /// CoinGecko API key for authenticated access to market data.
  ///
  /// Can also be provided via the `COINGECKO_API_KEY` environment variable.
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub coingecko_api_key: Option<String>,

  /// GitHub personal access token for GitHub API access.
  ///
  /// Used by the `technical` update path to fetch repository statistics
  /// (commits, stars, forks). Can also be provided via the `GITHUB_TOKEN`
  /// environment variable.
  #[arg(long, env = "GITHUB_TOKEN")]
  pub github_token: Option<String>,

  /// Enable dry-run mode — print actions without writing to the database.
  #[arg(long)]
  pub dry_run: bool,

  /// Enable verbose output, including the full `Debug` representation of
  /// the parsed arguments at startup.
  #[arg(short, long)]
  pub verbose: bool,
}
