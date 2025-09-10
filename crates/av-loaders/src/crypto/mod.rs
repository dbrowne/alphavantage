pub mod database;
pub mod loader;
pub mod sources;
pub mod types;


pub mod markets_loader;
pub use markets_loader::{
  CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader, CryptoMarketsOutput,
  CryptoMarketData, CryptoSymbolForMarkets, MarketsSourceResult,
};

pub use loader::CryptoSymbolLoader;
pub use types::*;

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
  #[error("Access denied: {0}]")]
  AccessDenied(String),
  #[error("Access Endpoint: {0}")]
  CoinGeckoEndpoint(String),
  #[error("Missing API key: {0}")]
  MissingAPIKey(String),
  #[error("Invalid API key: {0}")]
  InvalidAPIKey(String)
}

#[cfg(test)]
mod tests;
