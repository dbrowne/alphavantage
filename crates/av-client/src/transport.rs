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

use av_core::{Config, Error, FuncType, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

/// HTTP transport for API requests
///
/// Handles the low-level HTTP communication with the AlphaVantage API,
/// including request construction, response parsing, error handling, and retries.
pub struct Transport {
  client: Client,
  api_key: String,
  base_url: String,
}

impl Transport {
  /// Create a new transport instance
  ///
  /// # Arguments
  ///
  /// * `config` - Configuration containing API key and other settings
  pub fn new(config: Config) -> Self {
    let timeout = Duration::from_secs(config.timeout_secs);

    let client = Client::builder()
      .timeout(timeout)
      .user_agent("av-client/1.0")
      .build()
      .map_err(|e| Error::Http(format!("Failed to create HTTP client: {}", e)))
      .expect("Failed to create HTTP client");

    let base_url = config.base_url;

    Self { client, api_key: config.api_key, base_url }
  }

  /// Get access to the internal reqwest client for direct API calls
  pub fn client(&self) -> &Client {
    &self.client
  }

  /// Execute a GET request to the AlphaVantage API
  ///
  /// # Arguments
  ///
  /// * `function` - The API function to call
  /// * `params` - Additional parameters for the request
  ///
  /// # Returns
  ///
  /// Returns the deserialized response data or an error
  #[instrument(skip(self), fields(function = %function))]
  pub async fn get<T>(&self, function: FuncType, mut params: HashMap<String, String>) -> Result<T>
  where
    T: DeserializeOwned,
  {
    // Add function and API key to parameters
    params.insert("function".to_string(), function.to_string());
    params.insert("apikey".to_string(), self.api_key.clone());

    // Retry logic
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
      match self.execute_request(&params).await {
        Ok(response) => match self.parse_response::<T>(response, function).await {
          Ok(data) => {
            info!("Successfully parsed response for function: {:?}", function);
            return Ok(data);
          }
          Err(e) => {
            error!("Failed to parse response for function {:?}: {}", function, e);
            return Err(Error::Parse(format!(
              "Failed to parse response for function {:?}: {}",
              function, e
            )));
          }
        },
        Err(e) => {
          warn!("Request attempt {} failed for function {:?}: {}", attempt, function, e);
          last_error = Some(e);

          if attempt < MAX_RETRIES {
            // Exponential backoff
            let delay = Duration::from_millis(1000 * (2_u64.pow(attempt - 1)));
            tokio::time::sleep(delay).await;
          }
        }
      }
    }

    Err(last_error.unwrap_or_else(|| Error::Http("Max retries exceeded".to_string())))
  }

  /// Execute the actual HTTP request
  async fn execute_request(&self, params: &HashMap<String, String>) -> Result<reqwest::Response> {
    let mut url = reqwest::Url::parse(&self.base_url)
      .map_err(|e| Error::Http(format!("Invalid base URL: {}", e)))?;

    url.query_pairs_mut().extend_pairs(params);

    debug!("Making request to: {}", url);

    let response = self
      .client
      .get(url)
      .send()
      .await
      .map_err(|e| Error::Http(format!("Request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
      error!("HTTP error: {}", status);
      return Err(Error::Http(format!("HTTP error: {}", status)));
    }

    Ok(response)
  }

  /// Parse the HTTP response and handle API errors
  async fn parse_response<T>(&self, response: reqwest::Response, function: FuncType) -> Result<T>
  where
    T: DeserializeOwned,
  {
    let text = response
      .text()
      .await
      .map_err(|e| Error::Http(format!("Failed to read response body: {}", e)))?;

    debug!("Raw response: {}", text);

    // Check for API errors in the response
    if let Ok(error_response) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&text) {
      if let Some(error_msg) = error_response.get("Error Message") {
        if let Some(error_str) = error_msg.as_str() {
          return Err(Error::Api(error_str.to_string()));
        }
      }

      if let Some(note) = error_response.get("Note") {
        if let Some(note_str) = note.as_str() {
          if note_str.contains("rate limit") || note_str.contains("call frequency") {
            return Err(Error::RateLimit(note_str.to_string()));
          }
        }
      }
    }

    // Parse the successful response
    serde_json::from_str(&text).map_err(|e| {
      Error::Parse(format!(
        "Failed to deserialize response for function {:?}: {}. Raw response: {}",
        function, e, text
      ))
    })
  }

  /// Get the base URL being used
  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  /// Create a mock transport for testing
  #[cfg(test)]
  pub fn new_mock() -> Self {
    let config = Config {
      api_key: "mock_key".to_string(),
      base_url: "https://mock.alphavantage.co".to_string(),
      rate_limit: 75,
      timeout_secs: 10,
      max_retries: 3,
    };
    Self::new(config)
  }
}

impl std::fmt::Debug for Transport {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Transport")
      .field("base_url", &self.base_url)
      .field("api_key", &"[REDACTED]")
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_transport_creation() {
    let config = Config {
      api_key: "test_key".to_string(),
      base_url: "https://www.alphavantage.co/query".to_string(),
      rate_limit: 75,
      timeout_secs: 30,
      max_retries: 3,
    };

    let transport = Transport::new(config);
    assert_eq!(transport.base_url(), "https://www.alphavantage.co/query");
  }

  #[test]
  fn test_transport_custom_base_url() {
    let custom_url = "https://custom.alphavantage.co/query";
    let config = Config {
      api_key: "test_key".to_string(),
      base_url: custom_url.to_string(),
      rate_limit: 75,
      timeout_secs: 10,
      max_retries: 3,
    };

    let transport = Transport::new(config);
    assert_eq!(transport.base_url(), custom_url);
  }

  #[tokio::test]
  async fn test_mock_transport() {
    let transport = Transport::new_mock();
    assert_eq!(transport.base_url(), "https://mock.alphavantage.co");
  }
}
