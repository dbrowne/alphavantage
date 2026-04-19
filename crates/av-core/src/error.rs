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

//! Unified error handling for the `av-core` crate and its downstream consumers.
//!
//! This module defines a single [`Error`] enum that covers every failure mode
//! the crate can encounter — from missing environment variables through HTTP
//! transport errors to malformed API responses. A convenience [`Result<T>`] type
//! alias is provided so callers don't have to spell `Result<T, av_core::Error>`
//! everywhere.
//!
//! # Design
//!
//! The enum is built with [`thiserror`], which auto-derives [`std::error::Error`],
//! [`Display`](std::fmt::Display), and `From` conversions for the variants that
//! wrap standard-library or third-party error types.
//!
//! ## Automatic `From` conversions
//!
//! Three variants carry `#[from]` and can be produced via `?` from the wrapped
//! error type:
//!
//! | Variant      | Converts from                | Triggered by                          |
//! |--------------|------------------------------|---------------------------------------|
//! | `EnvVar`     | [`std::env::VarError`]       | `env::var("KEY")?`                    |
//! | `Serde`      | [`serde_json::Error`]        | `serde_json::from_str(…)?`            |
//! | `ParseDate`  | [`chrono::ParseError`]       | `NaiveDate::parse_from_str(…)?`       |
//!
//! All other variants are constructed manually with a descriptive `String` payload.
//!
//! # Error categories
//!
//! | Category        | Variants                           | Typical cause                              |
//! |-----------------|------------------------------------|--------------------------------------------|
//! | **Setup**       | `EnvVar`, `Config`, `ApiKey`       | Missing or malformed configuration         |
//! | **Parsing**     | `Serde`, `ParseDate`, `Parse`      | Malformed JSON, dates, or numeric values   |
//! | **Validation**  | `MissingField`, `InvalidResponse`  | API response missing expected data         |
//! | **Runtime**     | `RateLimit`, `Http`, `Api`         | Network/transport or API-level failures    |
//! | **Catch-all**   | `Unexpected`                       | Anything that doesn't fit above            |
//!
//! # Examples
//!
//! ```rust
//! use av_core::error::{Error, Result};
//!
//! fn get_symbol() -> Result<String> {
//!     Err(Error::MissingField("symbol".to_string()))
//! }
//!
//! let err = get_symbol().unwrap_err();
//! assert_eq!(err.to_string(), "Missing required field: symbol");
//! ```

use thiserror::Error;

/// Unified error type for the `av-core` crate.
///
/// Every function in the crate that can fail returns this enum (via the
/// [`Result<T>`] alias). Variants are grouped by failure category — see the
/// [module-level docs](self) for a category table.
///
/// # Display messages
///
/// Each variant produces a human-readable message via `#[error("…")]`.
/// Variants that carry a `String` payload interpolate it into the message;
/// variants with `#[from]` delegate to the wrapped error's `Display`.
#[derive(Error, Debug)]
pub enum Error {
  /// An environment variable was missing or not valid Unicode.
  ///
  /// Automatically converted from [`std::env::VarError`] via `#[from]`.
  /// Typically triggered when [`Config::from_env`](crate::Config::from_env)
  /// calls `env::var()`.
  #[error("Environment variable error: {0}")]
  EnvVar(#[from] std::env::VarError),

  /// A configuration value was present but invalid (e.g., non-numeric rate limit).
  ///
  /// Constructed manually in [`Config::from_env`](crate::Config::from_env)
  /// when a `parse()` call fails on an optional environment variable.
  #[error("Configuration error: {0}")]
  Config(String),

  /// The `ALPHA_VANTAGE_API_KEY` environment variable is not set.
  ///
  /// The `String` payload contains a diagnostic message. Note that the
  /// `#[error]` template does **not** interpolate the payload — the display
  /// message is always `"Failed to retrieve API key"`.
  #[error("Failed to retrieve API key")]
  ApiKey(String),

  /// JSON serialization or deserialization failed.
  ///
  /// Automatically converted from [`serde_json::Error`] via `#[from]`.
  /// Raised when parsing Alpha Vantage API JSON responses.
  #[error("Serialization error")]
  Serde(#[from] serde_json::Error),

  /// A date/time string could not be parsed.
  ///
  /// Automatically converted from [`chrono::ParseError`] via `#[from]`.
  /// Common when parsing timestamps from API response fields.
  #[error("Date parsing error")]
  ParseDate(#[from] chrono::ParseError),

  /// A required field was absent from an API response or request.
  ///
  /// The `String` payload names the missing field (e.g., `"symbol"`,
  /// `"close"`).
  #[error("Missing required field: {0}")]
  MissingField(String),

  /// The API rate limit has been exceeded.
  ///
  /// Alpha Vantage returns a specific JSON message when the per-minute or
  /// per-day request cap is hit. The `String` payload contains details
  /// about the limit that was exceeded.
  #[error("Rate limit exceeded: {0}")]
  RateLimit(String),

  /// The API returned a response that could not be interpreted.
  ///
  /// This covers cases where the HTTP status was 200 but the body did not
  /// match the expected schema — e.g., an empty body, an error message
  /// embedded in the JSON, or an unexpected top-level key.
  #[error("Invalid API response: {0}")]
  InvalidResponse(String),

  /// A catch-all for errors that don't fit any other variant.
  ///
  /// Use sparingly — prefer a more specific variant when possible.
  #[error("Unexpected error: {0}")]
  Unexpected(String),

  /// An HTTP transport-level error occurred.
  ///
  /// Covers connection failures, DNS resolution errors, TLS handshake
  /// failures, and timeouts. The `String` payload contains the underlying
  /// error message.
  #[error("HTTP error: {0}")]
  Http(String),

  /// The Alpha Vantage API returned a logical error.
  ///
  /// Distinct from [`InvalidResponse`](Error::InvalidResponse): this variant
  /// means the response was well-formed but indicated a domain error
  /// (e.g., `"Invalid API call"`, `"No data found for symbol"`).
  #[error("API error: {0}")]
  Api(String),

  /// A generic value-parsing error.
  ///
  /// Used when converting string fields to numeric types (e.g., parsing a
  /// price string like `"182.63"` into `f64`) fails. For date-specific
  /// parse failures, prefer [`ParseDate`](Error::ParseDate).
  #[error("Parse error: {0}")]
  Parse(String),
}

/// Convenience alias for `std::result::Result<T, av_core::error::Error>`.
///
/// Used throughout the crate and re-exported at the crate root as
/// [`av_core::Result`](crate::Result).
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_error_display_config() {
    let err = Error::Config("invalid timeout".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid timeout");
  }

  #[test]
  fn test_error_display_api_key() {
    let err = Error::ApiKey("key not found".to_string());
    assert_eq!(err.to_string(), "Failed to retrieve API key");
  }

  #[test]
  fn test_error_display_missing_field() {
    let err = Error::MissingField("symbol".to_string());
    assert_eq!(err.to_string(), "Missing required field: symbol");
  }

  #[test]
  fn test_error_display_rate_limit() {
    let err = Error::RateLimit("75 requests per minute exceeded".to_string());
    assert_eq!(err.to_string(), "Rate limit exceeded: 75 requests per minute exceeded");
  }

  #[test]
  fn test_error_display_invalid_response() {
    let err = Error::InvalidResponse("empty body".to_string());
    assert_eq!(err.to_string(), "Invalid API response: empty body");
  }

  #[test]
  fn test_error_display_unexpected() {
    let err = Error::Unexpected("unknown state".to_string());
    assert_eq!(err.to_string(), "Unexpected error: unknown state");
  }

  #[test]
  fn test_error_display_http() {
    let err = Error::Http("connection refused".to_string());
    assert_eq!(err.to_string(), "HTTP error: connection refused");
  }

  #[test]
  fn test_error_display_api() {
    let err = Error::Api("invalid symbol".to_string());
    assert_eq!(err.to_string(), "API error: invalid symbol");
  }

  #[test]
  fn test_error_display_parse() {
    let err = Error::Parse("invalid number".to_string());
    assert_eq!(err.to_string(), "Parse error: invalid number");
  }

  #[test]
  fn test_error_from_env_var() {
    let env_err = std::env::VarError::NotPresent;
    let err = Error::from(env_err);
    assert!(matches!(err, Error::EnvVar(_)));
    assert!(err.to_string().contains("Environment variable error"));
  }

  #[test]
  fn test_error_from_serde_json() {
    let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
    let err = Error::from(json_err);
    assert!(matches!(err, Error::Serde(_)));
    assert_eq!(err.to_string(), "Serialization error");
  }

  #[test]
  fn test_error_from_chrono_parse() {
    let parse_err = chrono::NaiveDate::parse_from_str("invalid", "%Y-%m-%d").unwrap_err();
    let err = Error::from(parse_err);
    assert!(matches!(err, Error::ParseDate(_)));
    assert_eq!(err.to_string(), "Date parsing error");
  }

  #[test]
  fn test_error_debug_impl() {
    let err = Error::Config("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Config"));
    assert!(debug_str.contains("test"));
  }

  #[test]
  fn test_result_type_alias() {
    fn returns_ok() -> Result<i32> {
      Ok(42)
    }
    fn returns_err() -> Result<i32> {
      Err(Error::Config("test".to_string()))
    }
    assert_eq!(returns_ok().unwrap(), 42);
    assert!(returns_err().is_err());
  }
}
