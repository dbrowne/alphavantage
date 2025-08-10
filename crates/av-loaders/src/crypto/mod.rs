pub mod loader;
pub mod sources;
pub mod types;

pub use loader::CryptoSymbolLoader;
pub use types::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
  #[error("Invalid response format from {source}: {message}")]
  InvalidResponse { source: String, message: String },
  #[error("Source not available: {0}")]
  SourceUnavailable(String),
}
