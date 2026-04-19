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

//! Query commands for looking up stored financial data.
//!
//! This module provides the `av-cli query` command group, which is intended to
//! allow users to look up symbol information and list symbols already stored in the
//! PostgreSQL database.
//!
//! ## Status: Unimplemented
//!
//! **Both subcommands are currently stubbed with `todo!()` macros** and will panic
//! at runtime if invoked. This module is registered in the CLI and will parse
//! arguments correctly, but execution will abort with a "not yet implemented" panic.
//!
//! ## Planned Subcommands
//!
//! ```text
//! av-cli query
//! ├── symbol <SYMBOL>                  Look up a specific ticker symbol
//! └── list-symbols [-e EXCHANGE] [-l LIMIT]  List stored symbols with filters
//! ```
//!
//! ## Usage (once implemented)
//!
//! ```bash
//! # Look up information for a specific symbol
//! av-cli query symbol AAPL
//!
//! # List up to 50 symbols from the NYSE
//! av-cli query list-symbols --exchange NYSE --limit 50
//!
//! # List symbols with default limit (100)
//! av-cli query list-symbols
//! ```
//!
//! ## Implementation Notes
//!
//! When implemented, these commands will need to:
//! 1. Establish a database connection using `config.database_url`
//!    (currently the `_config` parameter is unused)
//! 2. Query the securities tables via Diesel ORM
//! 3. Format and display results to stdout
//!
//! The `Config` parameter is already wired through from `main.rs` and provides
//! the `database_url` field needed for database access.

use crate::config::Config;
use anyhow::Result;
use clap::{Args, Subcommand};

/// Top-level clap arguments struct for `av-cli query`.
///
/// Wraps the [`QuerySubcommands`] enum and is embedded as the
/// [`Commands::Query`](crate::Commands::Query) variant in the main CLI parser.
/// Dispatched to [`execute`] from `main.rs`.
///
/// # Example CLI Invocation
///
/// ```bash
/// av-cli query symbol AAPL
/// av-cli query list-symbols --exchange NASDAQ --limit 25
/// ```
#[derive(Args, Debug)]
pub struct QueryCommand {
  #[command(subcommand)]
  command: QuerySubcommands,
}

/// Available subcommands for `av-cli query`.
///
/// This enum is **module-private** (not `pub`) — external code interacts with
/// it only through [`QueryCommand`] and [`execute`].
///
/// # Variants
///
/// - [`Symbol`](QuerySubcommands::Symbol) — Look up a specific ticker symbol by
///   name. Accepts a single positional argument (`symbol: String`), e.g., `AAPL`,
///   `BTC`, `MSFT`. Intended to display stored information such as exchange,
///   sector, market cap, and other overview data for that symbol.
///
/// - [`ListSymbols`](QuerySubcommands::ListSymbols) — List stored symbols with
///   optional filtering and pagination:
///   - `--exchange` / `-e` (`Option<String>`) — Filter results to a specific
///     exchange (e.g., `NASDAQ`, `NYSE`). When `None`, returns symbols from
///     all exchanges.
///   - `--limit` / `-l` (`usize`, default: `100`) — Maximum number of symbols
///     to return. Capped at the clap default of 100 if not specified.
#[derive(Subcommand, Debug)]
enum QuerySubcommands {
  /// Query symbol information
  Symbol {
    /// Symbol to query (e.g., "AAPL", "BTC", "MSFT")
    symbol: String,
  },

  /// List all symbols
  ListSymbols {
    /// Filter by exchange (e.g., "NASDAQ", "NYSE")
    #[arg(short, long)]
    exchange: Option<String>,

    /// Limit results (default: 100)
    #[arg(short, long, default_value = "100")]
    limit: usize,
  },
}

/// Executes a `query` subcommand.
///
/// This is the main dispatch function for `av-cli query`, called from
/// [`main`](crate::main) when the user invokes a query subcommand. It matches
/// on the parsed [`QuerySubcommands`] variant and delegates to the appropriate
/// handler.
///
/// # Status: Unimplemented
///
/// **Both branches currently call `todo!()`**, which will panic at runtime with
/// a "not yet implemented" message. The function signature, argument parsing,
/// and dispatch structure are in place — only the database query logic is missing.
///
/// # Arguments
///
/// * `cmd` — The parsed [`QueryCommand`] containing the user's chosen subcommand
///   and its arguments.
/// * `_config` — The CLI [`Config`]. Currently unused (prefixed with `_`), but
///   will be needed for `config.database_url` once the query implementations
///   are added.
///
/// # Errors
///
/// Currently cannot return an error (panics via `todo!()` before reaching `Ok`).
/// Once implemented, expected error conditions include:
/// - Database connection failures
/// - Symbol not found
/// - Invalid exchange filter values
///
/// # Panics
///
/// Panics unconditionally via `todo!()` for both subcommands.
pub async fn execute(cmd: QueryCommand, _config: Config) -> Result<()> {
  match cmd.command {
    QuerySubcommands::Symbol { symbol: _ } => {
      todo!("Implement symbol query")
    }
    QuerySubcommands::ListSymbols { exchange: _, limit: _ } => {
      todo!("Implement list symbols")
    }
  }
}
