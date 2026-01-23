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

use thiserror::Error;

/// Errors that can occur during crypto data loading operations.
#[derive(Error, Debug)]
pub enum CryptoLoaderError {
  #[error("HTTP request failed: {0}")]
  RequestFailed(#[from] reqwest::Error),

  #[error("JSON parsing failed: {0}")]
  JsonParseFailed(#[from] serde_json::Error),

  #[error("Rate limit exceeded for source: {0}")]
  RateLimitExceeded(String),

  #[error("API key missing for source: {0}")]
  ApiKeyMissing(String),

  #[error("Invalid response format from {api_source}: {message}")]
  InvalidResponse { api_source: String, message: String },

  #[error("Source not available: {0}")]
  SourceUnavailable(String),

  #[error("Internal Server error: {0}")]
  InternalServerError(String),

  #[error("Service Unavailable: {0}")]
  ServiceUnavailable(String),

  #[error("Access denied: {0}")]
  AccessDenied(String),

  #[error("Access Endpoint: {0}")]
  CoinGeckoEndpoint(String),

  #[error("Missing API key: {0}")]
  MissingAPIKey(String),

  #[error("Invalid API key: {0}")]
  InvalidAPIKey(String),

  #[error("Network error: {0}")]
  NetworkError(String),

  #[error("API error: {0}")]
  ApiError(String),

  #[error("Parse error: {0}")]
  ParseError(String),

  #[error("Cache error: {0}")]
  CacheError(String),
}

/// Result type for crypto loader operations.
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