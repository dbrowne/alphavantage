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
