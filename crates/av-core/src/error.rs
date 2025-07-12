//! Error types for the AlphaVantage Rust client

use thiserror::Error;

/// The main error type for av-* crates
#[derive(Error, Debug)]
pub enum Error {
  /// Environment variable error
  #[error("Environment variable error: {0}")]
  EnvVar(#[from] std::env::VarError),

  /// Configuration error
  #[error("Configuration error: {0}")]
  Config(String),

  /// API key error
  #[error("Failed to retrieve API key")]
  ApiKey(String),

  /// Serialization/Deserialization error
  #[error("Serialization error")]
  Serde(#[from] serde_json::Error),

  /// Date/Time parsing error
  #[error("Date parsing error")]
  ParseDate(#[from] chrono::ParseError),

  /// Missing required field in response
  #[error("Missing required field: {0}")]
  MissingField(String),

  /// API rate limit exceeded
  #[error("Rate limit exceeded: {0}")]
  RateLimit(String),

  /// Invalid response from API
  #[error("Invalid API response: {0}")]
  InvalidResponse(String),

  /// General unexpected error
  #[error("Unexpected error: {0}")]
  Unexpected(String),
}

/// Result type alias for av-* crates
pub type Result<T> = std::result::Result<T, Error>;
