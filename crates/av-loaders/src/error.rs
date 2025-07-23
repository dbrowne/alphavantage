use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("API error: {0}")]
    ApiError(#[from] av_client::error::ApiError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] av_database::error::DatabaseError),

    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

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

pub type LoaderResult<T> = Result<T, LoaderError>;