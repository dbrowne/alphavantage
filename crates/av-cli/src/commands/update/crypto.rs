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

//! Crypto update command definitions and dispatch for `av-cli update crypto`.
//!
//! This module is the entry point for all cryptocurrency data update operations.
//! It defines the [`CryptoUpdateCommands`] subcommand enum and the
//! [`handle_crypto_update`] dispatcher, both of which are imported by `main.rs`.
//!
//! ## Status: Partially Implemented
//!
//! - **`basic`**, **`social`**, **`technical`**, **`all`** — Parse correctly but
//!   panic at runtime via `unimplemented!()` in
//!   [`crypto_update_functions::update_crypto_command`]. The dispatch logic and
//!   flag-setting in this module are complete.
//! - **`metadata`** — **Fully implemented.** Delegates to
//!   [`crypto_metadata_etl::execute_metadata_etl`].
//!
//! ## Architecture
//!
//! This module coordinates three sibling modules:
//!
//! ```text
//! crypto.rs (this file)
//!   ├── uses crypto_update_cli::UpdateCryptoArgs     (argument struct)
//!   ├── uses crypto_update_functions::update_crypto_command  (execution logic)
//!   └── uses crypto_metadata_etl::execute_metadata_etl      (metadata ETL)
//! ```
//!
//! The `Basic`, `Social`, `Technical`, and `All` variants all share the same
//! [`UpdateCryptoArgs`] argument struct and are routed to the same
//! `update_crypto_command()` function. The dispatcher differentiates them by
//! **mutating the `args` struct** before calling the shared function:
//!
//! | Variant      | Mutation                  | Effect                        |
//! |-------------|---------------------------|-------------------------------|
//! | `Basic`      | `args.basic_only = true`  | Only update descriptions/ranks|
//! | `Social`     | `args.social_only = true` | Only update social metrics    |
//! | `Technical`  | `args.technical_only = true` | Only update blockchain/GitHub |
//! | `All`        | (none)                    | Update all categories         |
//! | `Metadata`   | N/A (separate path)       | Run metadata ETL pipeline     |
//!
//! ## Re-exports
//!
//! This module glob re-exports both [`crypto_update_cli`] and
//! [`crypto_update_functions`] via `pub use`, making [`UpdateCryptoArgs`] and
//! `update_crypto_command` accessible as `commands::update::crypto::UpdateCryptoArgs`
//! etc. without consumers needing to know which sibling module defines them.
//!
//! ## Configuration
//!
//! This module receives [`av_core::Config`] (not the full CLI
//! [`Config`](crate::config::Config)) because the crypto update commands interact
//! with external APIs rather than the database directly. The conversion from CLI
//! config to core config happens in [`handle_update`](crate::handle_update) in
//! `main.rs`.
//!
//! The `Metadata` variant is an exception — it needs `DATABASE_URL` and reads it
//! directly from the environment via [`dotenvy`], bypassing the config entirely.
//!
//! ## Usage
//!
//! ```bash
//! # Update only basic crypto data (descriptions, market cap ranks)
//! av-cli update crypto basic --limit 100
//!
//! # Update social metrics for specific symbols with dry run
//! av-cli update crypto social --symbols BTC,ETH,SOL --dry-run
//!
//! # Update all crypto data with custom delay and CoinGecko key
//! av-cli update crypto all --delay-ms 3000 --coingecko-api-key <KEY>
//!
//! # Run the metadata ETL pipeline
//! av-cli update crypto metadata
//! ```

use anyhow::Result;
use av_core::Config;
use clap::Subcommand;

use crate::commands::update::crypto_update_cli;
use crate::commands::update::crypto_update_functions;

use crate::commands::update::crypto_metadata_etl;

/// Re-export all items from [`crypto_update_cli`], primarily
/// [`UpdateCryptoArgs`] — the shared clap argument struct used by the
/// `Basic`, `Social`, `Technical`, and `All` variants.
pub use crypto_update_cli::*;

/// Re-export all items from [`crypto_update_functions`], primarily
/// `update_crypto_command()` — the shared async execution function.
pub use crypto_update_functions::*;

/// Subcommands for `av-cli update crypto`.
///
/// Five variants covering four data-category updates and one metadata operation.
/// The first four (`Basic`, `Social`, `Technical`, `All`) share the same
/// [`UpdateCryptoArgs`] struct and are dispatched to the same
/// `update_crypto_command()` function with different flag configurations.
/// `Metadata` takes no arguments and runs the ETL pipeline directly.
///
/// # Variants
///
/// - [`Basic`](CryptoUpdateCommands::Basic) — Update the `crypto_overview_basic`
///   database table with descriptions and market capitalization rankings sourced
///   from CoinGecko. Accepts [`UpdateCryptoArgs`] for symbol filtering, rate
///   limiting, dry-run mode, etc.
///
/// - [`Social`](CryptoUpdateCommands::Social) — Update the `crypto_social`
///   database table with social media metrics and community scoring data from
///   CoinGecko (e.g., Twitter followers, Reddit subscribers, community score).
///   Accepts [`UpdateCryptoArgs`].
///
/// - [`Technical`](CryptoUpdateCommands::Technical) — Update the
///   `crypto_technical` database table with blockchain statistics and GitHub
///   repository data (e.g., commit activity, stars, forks). Sources data from
///   CoinGecko and optionally the GitHub API (when `--github-token` is provided).
///   Accepts [`UpdateCryptoArgs`].
///
/// - [`All`](CryptoUpdateCommands::All) — Update all three crypto tables
///   (`crypto_overview_basic`, `crypto_social`, `crypto_technical`) in a single
///   run. No filter flags are mutated — all categories are enabled by default
///   when none of the `*_only` flags are set. Accepts [`UpdateCryptoArgs`].
///
/// - [`Metadata`](CryptoUpdateCommands::Metadata) — Run the metadata ETL
///   (extract-transform-load) pipeline. Takes no arguments. Unlike the other
///   variants, this reads `DATABASE_URL` directly from the environment rather
///   than using the passed [`Config`], and calls the synchronous
///   [`crypto_metadata_etl::execute_metadata_etl`] function.
#[derive(Subcommand, Debug)]
pub enum CryptoUpdateCommands {
  /// Update crypto_overview_basic table with descriptions and market cap ranks
  Basic(UpdateCryptoArgs),
  /// Update crypto_social table with social media and scoring data
  Social(UpdateCryptoArgs),
  /// Update crypto_technical table with blockchain and GitHub data
  Technical(UpdateCryptoArgs),
  /// Update all crypto tables at once
  All(UpdateCryptoArgs),
  /// Run the metadata ETL pipeline (no arguments required)
  Metadata,
}

/// Dispatches `av-cli update crypto` subcommands to their handlers.
///
/// This function is the main entry point called by [`handle_update`](crate::handle_update)
/// in `main.rs`. It routes each [`CryptoUpdateCommands`] variant to the appropriate
/// execution path.
///
/// ## Dispatch Strategy
///
/// For the `Basic`, `Social`, and `Technical` variants, the dispatcher **mutates
/// the `args` struct** by setting the corresponding `*_only` flag to `true` before
/// forwarding to the shared `update_crypto_command()` function. This approach allows
/// all four data-update variants to share a single argument struct and execution
/// function while still supporting category-specific behavior.
///
/// For `All`, no flags are mutated — when none of the `*_only` flags are set,
/// `update_crypto_command()` treats it as "update everything."
///
/// For `Metadata`, the function bypasses the shared update path entirely and
/// calls [`crypto_metadata_etl::execute_metadata_etl`] directly with a
/// `DATABASE_URL` read from the environment.
///
/// # Arguments
///
/// * `cmd` — The parsed [`CryptoUpdateCommands`] variant with its associated
///   arguments (if any).
/// * `config` — The [`av_core::Config`] containing API credentials and request
///   settings. Used by `update_crypto_command()` for external API calls. **Not
///   used** by the `Metadata` variant, which reads `DATABASE_URL` from the
///   environment independently.
///
/// # Errors
///
/// Returns errors from:
/// - `update_crypto_command()` — API call failures, database write errors
/// - `crypto_metadata_etl::execute_metadata_etl()` — Database connection or
///   ETL processing failures
/// - Missing `DATABASE_URL` environment variable (for the `Metadata` variant only)
pub async fn handle_crypto_update(cmd: CryptoUpdateCommands, config: Config) -> Result<()> {
  match cmd {
    CryptoUpdateCommands::Basic(mut args) => {
      args.basic_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::Social(mut args) => {
      args.social_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::Technical(mut args) => {
      args.technical_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::All(args) => update_crypto_command(args, config).await,
    CryptoUpdateCommands::Metadata => {
      // Metadata ETL needs DATABASE_URL but doesn't use the av_core::Config.
      // It reads the URL directly from the environment because the Config
      // passed to this function is the API-only av_core::Config (no database_url).
      dotenvy::dotenv().ok();
      let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set in .env file"))?;

      crypto_metadata_etl::execute_metadata_etl(&database_url)?;
      Ok(())
    }
  }
}
