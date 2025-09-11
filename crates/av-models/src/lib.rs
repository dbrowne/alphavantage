//! # av-models
//!
//! Data models for AlphaVantage API responses.
//!
//! This crate provides strongly-typed Rust structures for all AlphaVantage API
//! response formats, including time series data, fundamental analysis, news
//! sentiment, foreign exchange, and cryptocurrency data.
//!
//! ## Features
//!
//! - **Type Safety**: All API responses are strongly typed
//! - **Serde Integration**: Built-in serialization/deserialization
//! - **Decimal Precision**: Uses `rust_decimal` for financial data precision
//! - **Date Handling**: Proper timezone-aware date/time parsing
//! - **Comprehensive Coverage**: Models for all major AlphaVantage endpoints
//!
//! ## Usage
//!
//! ```ignore
//! use av_models::time_series::DailyTimeSeries;
//! use av_models::fundamentals::CompanyOverview;
//!
//! // Deserialize API responses
//! let daily_data: DailyTimeSeries = serde_json::from_str(&response_json)?;
//! let overview: CompanyOverview = serde_json::from_str(&overview_json)?;
//! ```

#![warn(clippy::all)]

pub mod common;
pub mod crypto;
pub mod forex;
pub mod fundamentals;
pub mod news;
pub mod time_series;
pub mod crypto_social;
pub use crypto_social::*;

// Re-export common types for convenience
pub use common::*;

// Re-export all model types
pub use crypto::*;
pub use forex::*;
pub use fundamentals::*;
pub use news::*;
pub use time_series::*;
