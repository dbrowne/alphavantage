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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("Environment variable error: {0}")]
  EnvVar(#[from] std::env::VarError),

  #[error("Configuration error: {0}")]
  Config(String),

  #[error("Failed to retrieve API key")]
  ApiKey(String),

  #[error("Serialization error")]
  Serde(#[from] serde_json::Error),

  #[error("Date parsing error")]
  ParseDate(#[from] chrono::ParseError),

  #[error("Missing required field: {0}")]
  MissingField(String),

  #[error("Rate limit exceeded: {0}")]
  RateLimit(String),

  #[error("Invalid API response: {0}")]
  InvalidResponse(String),

  #[error("Unexpected error: {0}")]
  Unexpected(String),

  #[error("HTTP error: {0}")]
  Http(String),

  #[error("API error: {0}")]
  Api(String),

  #[error("Parse error: {0}")]
  Parse(String),
}

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
