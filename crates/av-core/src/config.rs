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

//! Alpha Vantage API client configuration.
//!
//! This module provides [`Config`], the central configuration struct used by
//! all HTTP-level crates in the workspace to authenticate, rate-limit, and
//! connect to the Alpha Vantage API.
//!
//! # Construction
//!
//! There are two ways to build a `Config`:
//!
//! 1. **From environment variables** via [`Config::from_env`] — loads settings from
//!    the process environment (with `.env` file support via `dotenvy`). This is the
//!    recommended approach for production and CLI usage.
//!
//! 2. **Programmatically** via [`Config::default_with_key`] — builds a config with
//!    sensible defaults and only requires an API key. Convenient for tests and
//!    one-off scripts.
//!
//! # Environment variables
//!
//! | Variable                | Required | Default                                  | Description                 |
//! |-------------------------|----------|------------------------------------------|-----------------------------|
//! | `ALPHA_VANTAGE_API_KEY` | **yes**  | —                                        | Your Alpha Vantage API key  |
//! | `AV_RATE_LIMIT`         | no       | `75`                                     | Max requests per minute     |
//! | `AV_TIMEOUT_SECS`       | no       | `30`                                     | HTTP request timeout (secs) |
//! | `AV_MAX_RETRIES`        | no       | `3`                                      | Retries on transient failure|
//! | `AV_BASE_URL`           | no       | `https://www.alphavantage.co/query`      | API base URL override       |
//!
//! # Examples
//!
//! ```rust,no_run
//! use av_core::Config;
//!
//! // From environment (reads .env if present)
//! let config = Config::from_env().expect("API key must be set");
//!
//! // Programmatic construction for tests
//! let test_config = Config::default_with_key("demo".to_string());
//! assert_eq!(test_config.rate_limit, 75);
//! assert_eq!(test_config.timeout_secs, 30);
//! ```

use crate::error::{Error, Result};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;

/// Central configuration for the Alpha Vantage API client.
///
/// Holds all parameters needed to make authenticated, rate-limited requests to
/// the Alpha Vantage REST API. Implements `Serialize` and `Deserialize` so it
/// can be persisted to / loaded from JSON or TOML configuration files.
///
/// # Fields
///
/// | Field          | Type     | Description                                                         |
/// |----------------|----------|---------------------------------------------------------------------|
/// | `api_key`      | `String` | Alpha Vantage API key (required for all requests)                   |
/// | `rate_limit`   | `u32`    | Max requests per minute; free tier = 75, premium = 600              |
/// | `timeout_secs` | `u64`    | HTTP request timeout in seconds                                     |
/// | `max_retries`  | `u32`    | Number of automatic retries on transient failures (5xx, timeouts)   |
/// | `base_url`     | `String` | API endpoint URL; override for proxies or testing                   |
///
/// # Derives
///
/// - `Debug`, `Clone` — standard value-type ergonomics.
/// - `Serialize`, `Deserialize` — enables JSON/TOML config file round-tripping.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
  /// Alpha Vantage API key.
  ///
  /// Obtain a free key at <https://www.alphavantage.co/support/#api-key>.
  /// This value is sent as the `apikey` query parameter on every request.
  pub api_key: String,

  /// Maximum number of API requests allowed per minute.
  ///
  /// Defaults to [`DEFAULT_RATE_LIMIT`](crate::DEFAULT_RATE_LIMIT) (75) for
  /// free-tier keys. Premium plans can use up to
  /// [`PREMIUM_RATE_LIMIT`](crate::PREMIUM_RATE_LIMIT) (600).
  pub rate_limit: u32,

  /// HTTP request timeout in seconds.
  ///
  /// If the Alpha Vantage server does not respond within this window, the
  /// request is aborted and may be retried (see [`max_retries`](Config::max_retries)).
  /// Defaults to `30`.
  pub timeout_secs: u64,

  /// Number of automatic retries on transient failures.
  ///
  /// Applies to server errors (HTTP 5xx) and network timeouts. Client errors
  /// (4xx) are **not** retried. Defaults to `3`.
  pub max_retries: u32,

  /// Base URL for the Alpha Vantage REST API.
  ///
  /// Defaults to [`ALPHA_VANTAGE_BASE_URL`](crate::ALPHA_VANTAGE_BASE_URL).
  /// Override this to point at a local mock server or corporate proxy.
  pub base_url: String,
}

impl Config {
  /// Loads configuration from environment variables (with `.env` file fallback).
  ///
  /// Calls [`dotenvy::dotenv()`] first, so a `.env` file in the working directory
  /// (or any parent) is automatically picked up. Environment variables set in the
  /// shell take precedence over `.env` values.
  ///
  /// # Required variables
  ///
  /// - `ALPHA_VANTAGE_API_KEY` — returns [`Error::ApiKey`] if missing.
  ///
  /// # Optional variables (with defaults)
  ///
  /// - `AV_RATE_LIMIT` → `75` (parsed as `u32`)
  /// - `AV_TIMEOUT_SECS` → `30` (parsed as `u64`)
  /// - `AV_MAX_RETRIES` → `3` (parsed as `u32`)
  /// - `AV_BASE_URL` → [`ALPHA_VANTAGE_BASE_URL`](crate::ALPHA_VANTAGE_BASE_URL)
  ///
  /// Returns [`Error::Config`] if an optional variable is present but cannot be
  /// parsed to the expected numeric type.
  ///
  /// # Errors
  ///
  /// - [`Error::ApiKey`] — `ALPHA_VANTAGE_API_KEY` is not set.
  /// - [`Error::Config`] — a numeric variable contains a non-numeric value.
  pub fn from_env() -> Result<Self> {
    dotenv().ok();

    let api_key = env::var("ALPHA_VANTAGE_API_KEY")
      .map_err(|_| Error::ApiKey("ALPHA_VANTAGE_API_KEY not set".to_string()))?;

    let rate_limit = env::var("AV_RATE_LIMIT")
      .unwrap_or_else(|_| "75".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_RATE_LIMIT".to_string()))?;

    let timeout_secs = env::var("AV_TIMEOUT_SECS")
      .unwrap_or_else(|_| "30".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_TIMEOUT_SECS".to_string()))?;

    let max_retries = env::var("AV_MAX_RETRIES")
      .unwrap_or_else(|_| "3".to_string())
      .parse()
      .map_err(|_| Error::Config("Invalid AV_MAX_RETRIES".to_string()))?;

    let base_url =
      env::var("AV_BASE_URL").unwrap_or_else(|_| crate::ALPHA_VANTAGE_BASE_URL.to_string());

    Ok(Config { api_key, rate_limit, timeout_secs, max_retries, base_url })
  }

  /// Creates a `Config` with sensible defaults and only the API key specified.
  ///
  /// This is a convenience constructor primarily intended for **tests** and
  /// **quick scripts** where you don't want to set up environment variables.
  ///
  /// # Defaults
  ///
  /// | Field          | Value                                                |
  /// |----------------|------------------------------------------------------|
  /// | `rate_limit`   | [`DEFAULT_RATE_LIMIT`](crate::DEFAULT_RATE_LIMIT) (75) |
  /// | `timeout_secs` | `30`                                                 |
  /// | `max_retries`  | `3`                                                  |
  /// | `base_url`     | [`ALPHA_VANTAGE_BASE_URL`](crate::ALPHA_VANTAGE_BASE_URL) |
  ///
  /// # Examples
  ///
  /// ```rust
  /// use av_core::Config;
  ///
  /// let cfg = Config::default_with_key("demo".to_string());
  /// assert_eq!(cfg.rate_limit, 75);
  /// assert_eq!(cfg.timeout_secs, 30);
  /// assert_eq!(cfg.max_retries, 3);
  /// ```
  pub fn default_with_key(api_key: String) -> Self {
    Config {
      api_key,
      rate_limit: crate::DEFAULT_RATE_LIMIT,
      timeout_secs: 30,
      max_retries: 3,
      base_url: crate::ALPHA_VANTAGE_BASE_URL.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_from_env() {
    env::set_var("ALPHA_VANTAGE_API_KEY", "test_key");
    let config = Config::from_env().unwrap();
    assert_eq!(config.api_key, "test_key");
    assert_eq!(config.rate_limit, 75);
  }
}
