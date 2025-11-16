/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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
pub mod crypto_social;
pub mod forex;
pub mod fundamentals;
pub mod news;
pub mod time_series;
pub use crypto_social::*;

// Re-export common types for convenience
pub use common::*;

// Re-export all model types
pub use crypto::*;
pub use forex::*;
pub use fundamentals::*;
pub use news::*;
pub use time_series::*;
