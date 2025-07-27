use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LoaderError {
    #[error("API error: {0}")]
    ApiError(String), // Changed to String to make it Clone

    #[error("CSV parsing error: {0}")]
    CsvError(String), // Changed to String

    #[error("IO error: {0}")]
    IoError(String), // Changed to String

    #[error("Serialization error: {0}")]
    SerializationError(String), // Changed to String

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

pub type LoaderResult<T> = Result<T, LoaderError>;