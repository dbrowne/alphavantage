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

//! Error types for cryptocurrency data loading.
//!
//! Defines [`CryptoLoaderError`], the unified error enum for all operations
//! in the `crypto-loaders` crate, and the [`CryptoLoaderResult<T>`]
//! convenience alias.
//!
//! # Automatic conversions
//!
//! Two `#[from]` conversions are provided:
//! - [`reqwest::Error`] → [`RequestFailed`](CryptoLoaderError::RequestFailed)
//! - [`serde_json::Error`] → [`JsonParseFailed`](CryptoLoaderError::JsonParseFailed)
//!
//! All other variants are constructed manually with descriptive `String` payloads.
//!
//! # Error categories
//!
//! | Category           | Variants                                                      |
//! |--------------------|---------------------------------------------------------------|
//! | **Network/HTTP**   | `RequestFailed`, `NetworkError`                               |
//! | **Rate limiting**  | `RateLimitExceeded`                                           |
//! | **Authentication** | `ApiKeyMissing`, `MissingAPIKey`, `InvalidAPIKey`, `AccessDenied` |
//! | **API response**   | `InvalidResponse`, `ApiError`, `InternalServerError`, `ServiceUnavailable`, `CoinGeckoEndpoint` |
//! | **Parsing**        | `JsonParseFailed`, `ParseError`                               |
//! | **Configuration**  | `SourceUnavailable`                                           |
//! | **Caching**        | `CacheError`                                                  |

use thiserror::Error;

/// Unified error type for all crypto-loader operations.
///
/// Covers the full spectrum of failures: network transport, authentication,
/// rate limiting, response parsing, API-level errors, configuration issues,
/// and cache operations.
#[derive(Error, Debug)]
pub enum CryptoLoaderError {
  /// An HTTP request failed at the transport level (connection, DNS, TLS, timeout).
  /// Auto-converted from [`reqwest::Error`] via `#[from]`.
  #[error("HTTP request failed: {0}")]
  RequestFailed(#[from] reqwest::Error),

  /// JSON deserialization failed.
  /// Auto-converted from [`serde_json::Error`] via `#[from]`.
  #[error("JSON parsing failed: {0}")]
  JsonParseFailed(#[from] serde_json::Error),

  /// The provider returned HTTP 429 — too many requests.
  /// The `String` payload names the provider (e.g., `"CoinGecko"`).
  #[error("Rate limit exceeded for source: {0}")]
  RateLimitExceeded(String),

  /// A required API key was not provided for a provider.
  #[error("API key missing for source: {0}")]
  ApiKeyMissing(String),

  /// The API returned a parseable response but the content was unexpected.
  /// Includes the provider name and a descriptive message.
  #[error("Invalid response format from {api_source}: {message}")]
  InvalidResponse {
    /// Which provider returned the unexpected response.
    api_source: String,
    /// What was wrong with the response.
    message: String,
  },

  /// No provider is configured for the requested data source.
  #[error("Source not available: {0}")]
  SourceUnavailable(String),

  /// The provider returned HTTP 500.
  #[error("Internal Server error: {0}")]
  InternalServerError(String),

  /// The provider returned HTTP 503.
  #[error("Service Unavailable: {0}")]
  ServiceUnavailable(String),

  /// The provider returned HTTP 403 — forbidden.
  #[error("Access denied: {0}")]
  AccessDenied(String),

  /// A CoinGecko-specific endpoint error.
  #[error("Access Endpoint: {0}")]
  CoinGeckoEndpoint(String),

  /// Alias for `ApiKeyMissing` — retained for backward compatibility.
  #[error("Missing API key: {0}")]
  MissingAPIKey(String),

  /// The provided API key was rejected by the provider (HTTP 401).
  #[error("Invalid API key: {0}")]
  InvalidAPIKey(String),

  /// A network-level error not covered by `RequestFailed`
  /// (e.g., constructed from a non-reqwest networking layer).
  #[error("Network error: {0}")]
  NetworkError(String),

  /// A generic API-level error returned by the provider.
  #[error("API error: {0}")]
  ApiError(String),

  /// A value-parsing error (e.g., date string, numeric conversion).
  #[error("Parse error: {0}")]
  ParseError(String),

  /// A cache read/write operation failed.
  #[error("Cache error: {0}")]
  CacheError(String),
}

/// Convenience alias for `Result<T, CryptoLoaderError>`.
pub type CryptoLoaderResult<T> = Result<T, CryptoLoaderError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_error_display() {
    let err = CryptoLoaderError::ApiKeyMissing("CoinGecko".to_string());
    assert!(err.to_string().contains("CoinGecko"));
  }

  #[test]
  fn test_rate_limit_error() {
    let err = CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string());
    assert!(err.to_string().contains("Rate limit"));
  }
}
