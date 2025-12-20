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

//! # av-client
//!
//! Async HTTP client for the AlphaVantage financial data API.
//!
//! This crate provides a rate-limited, async client for accessing AlphaVantage endpoints
//! including time series, fundamentals, forex, cryptocurrency, and news sentiment data.
//!
//! ## Features
//!
//! - **Async/Await**: Built on tokio and reqwest
//! - **Rate Limiting**: Automatic rate limiting (75/min free, 600/min premium)
//! - **Type Safety**: Strongly-typed responses via `av-models`
//! - **Organized Endpoints**: Modular access to API domains
//!
//! ## Example
//!
//! ```ignore
//! use av_client::{AlphaVantageClient, Config};
//!
//! let config = Config::default_with_key("your_api_key".to_string());
//! let client = AlphaVantageClient::new(config);
//!
//! // Fetch daily time series
//! let data = client.time_series().daily("AAPL", false).await?;
//! ```

#![warn(clippy::all)]

pub mod client;
pub mod endpoints;
pub mod transport;

// Re-export the main client and common types
pub use av_core::{Config, Error, Result};
pub use av_models::*;
pub use client::AlphaVantageClient;

// Re-export endpoint modules for direct access if needed
pub use endpoints::{
  crypto::CryptoEndpoints, crypto_social::CryptoSocialEndpoints, forex::ForexEndpoints,
  fundamentals::FundamentalsEndpoints, news::NewsEndpoints, time_series::TimeSeriesEndpoints,
};

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_client_creation() {
    let config = Config::default_with_key("test_key".to_string());
    // Test that we can create the client configuration
    assert_eq!(config.api_key, "test_key");
  }
}
