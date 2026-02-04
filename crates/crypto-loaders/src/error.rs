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

  #[error("Rate limit exceeded for {provider}")]
  RateLimitExceeded { provider: String, retry_after_secs: Option<u64> },

  #[error("API key missing for {0}")]
  ApiKeyMissing(String),

  #[error("Invalid response from {provider}: {message}")]
  InvalidResponse { provider: String, message: String },

  #[error("Provider not available: {0}")]
  SourceUnavailable(String),

  #[error("Server error from {provider}: {message}")]
  ServerError { provider: String, message: String },

  #[error("Access denied for {provider}: {message}")]
  AccessDenied { provider: String, message: String },

  #[error("Network error: {0}")]
  NetworkError(String),

  #[error("API error from {provider}: {message}")]
  ApiError { provider: String, message: String },

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
    let err = CryptoLoaderError::RateLimitExceeded {
      provider: "CoinGecko".to_string(),
      retry_after_secs: Some(60),
    };
    assert!(err.to_string().contains("Rate limit"));
    assert!(err.to_string().contains("CoinGecko"));
  }

  #[test]
  fn test_api_error() {
    let err = CryptoLoaderError::ApiError {
      provider: "CoinMarketCap".to_string(),
      message: "invalid symbol".to_string(),
    };
    assert!(err.to_string().contains("CoinMarketCap"));
    assert!(err.to_string().contains("invalid symbol"));
  }

  #[test]
  fn test_server_error() {
    let err = CryptoLoaderError::ServerError {
      provider: "CoinGecko".to_string(),
      message: "internal error".to_string(),
    };
    assert!(err.to_string().contains("CoinGecko"));
    assert!(err.to_string().contains("internal error"));
  }
}
