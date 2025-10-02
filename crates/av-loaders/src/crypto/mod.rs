pub mod crypto_news_loader;
pub mod database;
pub mod loader;
pub mod mapping_service;
pub mod markets_loader;
pub mod metadata_loader;
pub mod social_loader;
pub mod sources;
pub mod types;

// Re-export the main loaders and types
pub use loader::CryptoSymbolLoader;
pub use markets_loader::{
  CryptoMarketData, CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader,
  CryptoSymbolForMarkets,
};
pub use social_loader::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSocialLoader, CryptoSymbolForSocial,
  ProcessedSocialData,
};
pub use types::*;

pub use metadata_loader::{
  CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataLoader, CryptoMetadataOutput,
  CryptoSymbolForMetadata, MetadataSourceResult, ProcessedCryptoMetadata,
};

use thiserror::Error;

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
}

// Implement conversion to LoaderError
impl From<CryptoLoaderError> for crate::LoaderError {
  fn from(err: CryptoLoaderError) -> Self {
    match err {
      CryptoLoaderError::RequestFailed(e) => crate::LoaderError::IoError(e.to_string()),
      CryptoLoaderError::JsonParseFailed(e) => {
        crate::LoaderError::SerializationError(e.to_string())
      }
      CryptoLoaderError::RateLimitExceeded(_msg) => {
        crate::LoaderError::RateLimitExceeded { retry_after: 60 }
      }
      CryptoLoaderError::ApiKeyMissing(msg) => crate::LoaderError::ConfigurationError(msg),
      CryptoLoaderError::InvalidResponse { api_source, message } => {
        crate::LoaderError::ApiError(format!("{}: {}", api_source, message))
      }
      CryptoLoaderError::NetworkError(msg) => crate::LoaderError::IoError(msg),
      CryptoLoaderError::ApiError(msg) => crate::LoaderError::ApiError(msg),
      CryptoLoaderError::ParseError(msg) => crate::LoaderError::SerializationError(msg),
      _ => crate::LoaderError::ApiError(err.to_string()),
    }
  }
}

pub type CryptoLoaderResult<T> = Result<T, CryptoLoaderError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crypto_loader_error_conversion() {
    let crypto_err = CryptoLoaderError::ApiKeyMissing("CoinGecko".to_string());
    let loader_err: crate::LoaderError = crypto_err.into();

    match loader_err {
      crate::LoaderError::ConfigurationError(msg) => {
        assert!(msg.contains("CoinGecko"));
      }
      _ => panic!("Unexpected error type conversion"),
    }
  }

  #[test]
  fn test_rate_limit_error() {
    let crypto_err = CryptoLoaderError::RateLimitExceeded("CoinGecko".to_string());
    let loader_err: crate::LoaderError = crypto_err.into();

    match loader_err {
      crate::LoaderError::RateLimitExceeded { retry_after } => {
        assert_eq!(retry_after, 60);
      }
      _ => panic!("Unexpected error type conversion"),
    }
  }
}
