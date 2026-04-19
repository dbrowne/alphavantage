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

//! CLI-level configuration for `av-cli`.
//!
//! This module defines [`Config`], the top-level configuration struct used throughout
//! the CLI application. It composes the API-specific configuration from
//! [`av_core::Config`] with CLI-specific settings (database URL, CSV file paths)
//! that are not needed by the core library.
//!
//! ## Relationship to `av_core::Config`
//!
//! [`av_core::Config`] (aliased here as `CoreConfig`) contains only API-related
//! settings (API key, base URL, rate limit, timeout, retries) and is used by the
//! core AlphaVantage client library. This CLI [`Config`] wraps it and adds:
//! - A PostgreSQL connection string for data persistence
//! - File paths for NASDAQ and NYSE CSV listing files used during security loading
//!
//! Some command handlers (e.g., `update crypto`) require only the API config,
//! so [`main::handle_update`](crate::handle_update) extracts the inner `api_config`
//! fields and constructs an `av_core::Config` for those handlers.
//!
//! ## Environment Variables
//!
//! | Variable               | Required | Default                    | Description                          |
//! |------------------------|----------|----------------------------|--------------------------------------|
//! | `ALPHA_VANTAGE_API_KEY`| **Yes**  | —                          | AlphaVantage API key                 |
//! | `DATABASE_URL`         | **Yes**  | —                          | PostgreSQL connection string         |
//! | `NASDAQ_LISTED`        | No       | `./data/nasdaq-listed.csv` | Path to NASDAQ securities CSV file   |
//! | `OTHER_LISTED`         | No       | `./data/nyse-listed.csv`   | Path to NYSE/other securities CSV    |
//!
//! Note: The API-level environment variables (`AV_RATE_LIMIT`, `AV_TIMEOUT_SECS`,
//! `AV_MAX_RETRIES`, `AV_BASE_URL`) supported by [`av_core::Config::from_env`] are
//! **not** used here. Instead, this module hardcodes the API defaults directly when
//! constructing the inner `CoreConfig` (rate limit: 75 req/min, timeout: 30s,
//! retries: 3, base URL: `https://www.alphavantage.co/query`).

use anyhow::{Context, Result};
use av_core::Config as CoreConfig;
use std::env;

/// Unified configuration for the `av-cli` application.
///
/// Combines API configuration (via the embedded [`av_core::Config`]) with
/// CLI-specific settings needed for database access and CSV file loading.
///
/// # Fields
///
/// - `api_config` — Core API configuration ([`av_core::Config`]) containing the
///   AlphaVantage API key, base URL (`https://www.alphavantage.co/query`), rate
///   limit (75 requests/minute for the free tier), request timeout (30 seconds),
///   and maximum retry count (3). This is extracted and converted back to
///   `av_core::Config` when passed to core library handlers.
///
/// - `database_url` — PostgreSQL connection string (e.g.,
///   `postgres://user:pass@localhost/alphavantage`). Used by command handlers to
///   establish database connections via Diesel for loading, querying, and
///   generating statistics.
///
/// - `nasdaq_csv_path` — File path to the NASDAQ-listed securities CSV file.
///   Used by the `load securities` command to ingest NASDAQ ticker symbols.
///   Defaults to `./data/nasdaq-listed.csv` if `NASDAQ_LISTED` is not set.
///
/// - `nyse_csv_path` — File path to the NYSE/other-listed securities CSV file.
///   Used by the `load securities` command to ingest NYSE and other exchange
///   ticker symbols. Defaults to `./data/nyse-listed.csv` if `OTHER_LISTED`
///   is not set.
///
/// # Example
///
/// ```no_run
/// use av_cli::config::Config;
///
/// let config = Config::from_env().expect("Failed to load config");
/// println!("Database: {}", config.database_url);
/// println!("API key: {}...", &config.api_config.api_key[..4]);
/// ```
#[derive(Debug, Clone)]
pub struct Config {
  pub api_config: CoreConfig,
  pub database_url: String,
  pub nasdaq_csv_path: String,
  pub nyse_csv_path: String,
}

impl Config {
  /// Constructs a [`Config`] from environment variables.
  ///
  /// Reads the following environment variables:
  ///
  /// - **`ALPHA_VANTAGE_API_KEY`** (required) — The AlphaVantage API key. If not
  ///   set, returns an error with a descriptive context message.
  /// - **`DATABASE_URL`** (required) — The PostgreSQL connection string. If not
  ///   set, returns an error with a descriptive context message.
  /// - **`NASDAQ_LISTED`** (optional) — Path to the NASDAQ CSV file. Falls back
  ///   to `./data/nasdaq-listed.csv`.
  /// - **`OTHER_LISTED`** (optional) — Path to the NYSE/other CSV file. Falls
  ///   back to `./data/nyse-listed.csv`.
  ///
  /// The inner [`av_core::Config`] is constructed with hardcoded defaults rather
  /// than reading `AV_RATE_LIMIT` / `AV_TIMEOUT_SECS` / `AV_MAX_RETRIES` /
  /// `AV_BASE_URL` from the environment. The defaults are:
  ///
  /// | Field          | Value                                    |
  /// |----------------|------------------------------------------|
  /// | `base_url`     | `https://www.alphavantage.co/query`      |
  /// | `rate_limit`   | `75` (free-tier: 75 requests per minute) |
  /// | `timeout_secs` | `30`                                     |
  /// | `max_retries`  | `3`                                      |
  ///
  /// # Errors
  ///
  /// Returns [`anyhow::Error`] if either `ALPHA_VANTAGE_API_KEY` or
  /// `DATABASE_URL` is not set in the environment.
  pub fn from_env() -> Result<Self> {
    let api_key = env::var("ALPHA_VANTAGE_API_KEY")
      .context("ALPHA_VANTAGE_API_KEY environment variable not set")?;

    let database_url =
      env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

    let nasdaq_csv_path =
      env::var("NASDAQ_LISTED").unwrap_or_else(|_| "./data/nasdaq-listed.csv".to_string());

    let nyse_csv_path =
      env::var("OTHER_LISTED").unwrap_or_else(|_| "./data/nyse-listed.csv".to_string());

    let api_config = CoreConfig {
      api_key,
      base_url: av_core::ALPHA_VANTAGE_BASE_URL.to_string(),
      rate_limit: 75, // Free tier default (75 requests/minute)
      timeout_secs: 30,
      max_retries: 3,
    };

    Ok(Self { api_config, database_url, nasdaq_csv_path, nyse_csv_path })
  }
}
