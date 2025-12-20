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

#[derive(Error, Debug, Clone)]
pub enum LoaderError {
  #[error("API error: {0}")]
  ApiError(String),

  #[error("CSV parsing error: {0}")]
  CsvError(String),

  #[error("IO error: {0}")]
  IoError(String),

  #[error("Serialization error: {0}")]
  SerializationError(String),

  #[error("Database error: {0}")]
  DatabaseError(String),

  #[error("Rate limit exceeded, retry after {retry_after} seconds")]
  RateLimitExceeded { retry_after: u64 },

  #[error("Invalid data: {0}")]
  InvalidData(String),

  #[error("Process tracking error: {0}")]
  ProcessTrackingError(String),

  #[error("Batch processing error: {0}")]
  BatchProcessingError(String),

  #[error("Configuration error: {0}")]
  ConfigurationError(String),
}

// Implement conversions manually
impl From<csv::Error> for LoaderError {
  fn from(err: csv::Error) -> Self {
    LoaderError::CsvError(err.to_string())
  }
}

impl From<std::io::Error> for LoaderError {
  fn from(err: std::io::Error) -> Self {
    LoaderError::IoError(err.to_string())
  }
}

impl From<serde_json::Error> for LoaderError {
  fn from(err: serde_json::Error) -> Self {
    LoaderError::SerializationError(err.to_string())
  }
}

impl From<av_core::Error> for LoaderError {
  fn from(err: av_core::Error) -> Self {
    LoaderError::ApiError(err.to_string())
  }
}

// Add conversion from diesel errors  might be superfluous. but working fast
impl From<diesel::result::Error> for LoaderError {
  fn from(err: diesel::result::Error) -> Self {
    LoaderError::DatabaseError(err.to_string())
  }
}

impl From<diesel::ConnectionError> for LoaderError {
  fn from(err: diesel::ConnectionError) -> Self {
    LoaderError::DatabaseError(err.to_string())
  }
}

pub type LoaderResult<T> = Result<T, LoaderError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_loader_error_display_api_error() {
    let err = LoaderError::ApiError("connection failed".to_string());
    assert_eq!(err.to_string(), "API error: connection failed");
  }

  #[test]
  fn test_loader_error_display_csv_error() {
    let err = LoaderError::CsvError("invalid header".to_string());
    assert_eq!(err.to_string(), "CSV parsing error: invalid header");
  }

  #[test]
  fn test_loader_error_display_io_error() {
    let err = LoaderError::IoError("file not found".to_string());
    assert_eq!(err.to_string(), "IO error: file not found");
  }

  #[test]
  fn test_loader_error_display_serialization_error() {
    let err = LoaderError::SerializationError("invalid json".to_string());
    assert_eq!(err.to_string(), "Serialization error: invalid json");
  }

  #[test]
  fn test_loader_error_display_database_error() {
    let err = LoaderError::DatabaseError("connection refused".to_string());
    assert_eq!(err.to_string(), "Database error: connection refused");
  }

  #[test]
  fn test_loader_error_display_rate_limit_exceeded() {
    let err = LoaderError::RateLimitExceeded { retry_after: 60 };
    assert_eq!(err.to_string(), "Rate limit exceeded, retry after 60 seconds");
  }

  #[test]
  fn test_loader_error_display_invalid_data() {
    let err = LoaderError::InvalidData("missing symbol".to_string());
    assert_eq!(err.to_string(), "Invalid data: missing symbol");
  }

  #[test]
  fn test_loader_error_display_process_tracking_error() {
    let err = LoaderError::ProcessTrackingError("tracker failed".to_string());
    assert_eq!(err.to_string(), "Process tracking error: tracker failed");
  }

  #[test]
  fn test_loader_error_display_batch_processing_error() {
    let err = LoaderError::BatchProcessingError("batch 3 failed".to_string());
    assert_eq!(err.to_string(), "Batch processing error: batch 3 failed");
  }

  #[test]
  fn test_loader_error_display_configuration_error() {
    let err = LoaderError::ConfigurationError("invalid batch size".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid batch size");
  }

  #[test]
  fn test_loader_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err = LoaderError::from(io_err);
    assert!(matches!(err, LoaderError::IoError(_)));
    assert!(err.to_string().contains("file missing"));
  }

  #[test]
  fn test_loader_error_from_serde_json_error() {
    let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
    let err = LoaderError::from(json_err);
    assert!(matches!(err, LoaderError::SerializationError(_)));
  }

  #[test]
  fn test_loader_error_from_av_core_error() {
    let core_err = av_core::Error::Config("bad config".to_string());
    let err = LoaderError::from(core_err);
    assert!(matches!(err, LoaderError::ApiError(_)));
    assert!(err.to_string().contains("Configuration error"));
  }

  #[test]
  fn test_loader_error_clone() {
    let err = LoaderError::ApiError("test".to_string());
    let cloned = err.clone();
    assert_eq!(err.to_string(), cloned.to_string());
  }

  #[test]
  fn test_loader_error_debug() {
    let err = LoaderError::InvalidData("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("InvalidData"));
    assert!(debug_str.contains("test"));
  }

  #[test]
  fn test_loader_result_ok() {
    let result: LoaderResult<i32> = Ok(42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
  }

  #[test]
  fn test_loader_result_err() {
    let result: LoaderResult<i32> = Err(LoaderError::InvalidData("bad".to_string()));
    assert!(result.is_err());
  }
}
