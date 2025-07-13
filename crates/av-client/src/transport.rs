//! HTTP transport layer for AlphaVantage API requests

use av_core::{Config, Error, FuncType, Result};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use url::Url;

/// HTTP transport layer for making requests to the AlphaVantage API
pub struct Transport {
    client: Client,
    base_url: String,
    api_key: String,
    timeout: Duration,
    max_retries: u32,
}

impl Transport {
    /// Create a new transport instance
    pub async fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("av-client/0.1.0")
            .build()
            .map_err(|e| Error::Http(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            timeout: Duration::from_secs(config.timeout_secs),
            max_retries: config.max_retries,
        })
    }

    /// Create a mock transport for testing
    #[cfg(test)]
    pub fn new_mock() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://mock.alphavantage.co".to_string(),
            api_key: "test_key".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }

    /// Make a GET request to the AlphaVantage API
    ///
    /// # Arguments
    ///
    /// * `function` - The AlphaVantage API function to call
    /// * `params` - Additional query parameters for the request
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the deserialized response or an error
    #[instrument(skip(self), fields(function = %function))]
    pub async fn get<T>(&self, function: FuncType, params: HashMap<String, String>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(function, params)?;
        debug!("Making request to: {}", url);

        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(2_u64.pow(attempt) * 1000); // Exponential backoff
                warn!("Retrying request in {}ms (attempt {})", delay.as_millis(), attempt + 1);
                tokio::time::sleep(delay).await;
            }

            match self.make_request(&url).await {
                Ok(response) => {
                    let text = response.text().await.map_err(|e| {
                        Error::Http(format!("Failed to read response body: {}", e))
                    })?;

                    debug!("Response body length: {} bytes", text.len());

                    // Check for API error messages in the response
                    if let Err(api_error) = self.check_api_error(&text) {
                        return Err(api_error);
                    }

                    // Try to deserialize the response
                    match serde_json::from_str::<T>(&text) {
                        Ok(data) => {
                            info!("Successfully parsed response for function: {}", function);
                            return Ok(data);
                        }
                        Err(e) => {
                            error!("Failed to parse JSON response: {}", e);
                            error!("Response text (first 500 chars): {}", 
                                   &text[..std::cmp::min(500, text.len())]);
                            return Err(Error::Parse(format!(
                                "Failed to parse response: {}. Response: {}",
                                e,
                                &text[..std::cmp::min(200, text.len())]
                            )));
                        }
                    }
                }
                Err(e) => {
                    warn!("Request failed (attempt {}): {}", attempt + 1, e);
                    last_error = Some(e);
                    attempt += 1;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Http("Max retries exceeded".to_string())
        }))
    }

    /// Build the full URL for an API request
    fn build_url(&self, function: FuncType, mut params: HashMap<String, String>) -> Result<String> {
        let mut url = Url::parse(&format!("{}/query", self.base_url))
            .map_err(|e| Error::Http(format!("Invalid base URL: {}", e)))?;

        // Add the function parameter
        params.insert("function".to_string(), function.to_string());
        
        // Add the API key
        params.insert("apikey".to_string(), self.api_key.clone());

        // Add all parameters to the URL
        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params {
                query_pairs.append_pair(&key, &value);
            }
        }

        Ok(url.to_string())
    }

    /// Make the actual HTTP request
    async fn make_request(&self, url: &str) -> Result<Response> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Request failed: {}", e)))?;

        let status = response.status();
        
        if status.is_success() {
            debug!("Request successful with status: {}", status);
            Ok(response)
        } else {
            error!("Request failed with status: {}", status);
            Err(Error::Http(format!("HTTP error: {}", status)))
        }
    }

    /// Check for AlphaVantage API error messages in the response
    fn check_api_error(&self, response_text: &str) -> Result<()> {
        // Check for common API error patterns
        if response_text.contains("Error Message") {
            if let Ok(error_response) = serde_json::from_str::<HashMap<String, String>>(response_text) {
                if let Some(error_msg) = error_response.get("Error Message") {
                    return Err(Error::Api(error_msg.clone()));
                }
            }
        }

        // Check for API call frequency exceeded
        if response_text.contains("API call frequency") || response_text.contains("higher API call frequency") {
            return Err(Error::RateLimit(
                "API call frequency limit exceeded".to_string()
            ));
        }

        // Check for invalid API key
        if response_text.contains("Invalid API call") || response_text.contains("Invalid API key") {
            return Err(Error::ApiKey(
                "Invalid API key or unauthorized request".to_string()
            ));
        }

        // Check for invalid function
        if response_text.contains("Invalid function") {
            return Err(Error::Api(
                "Invalid function parameter".to_string()
            ));
        }

        Ok(())
    }

    /// Get the base URL being used
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get request timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use av_core::FuncType;
    use std::collections::HashMap;

    #[test]
    fn test_build_url() {
        let transport = Transport::new_mock();
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), "AAPL".to_string());
        
        let url = transport.build_url(FuncType::TimeSeriesDaily, params).unwrap();
        
        assert!(url.contains("function=TIME_SERIES_DAILY"));
        assert!(url.contains("symbol=AAPL"));
        assert!(url.contains("apikey=test_key"));
        assert!(url.starts_with("https://mock.alphavantage.co/query"));
    }

    #[test]
    fn test_check_api_error_rate_limit() {
        let transport = Transport::new_mock();
        let response = r#"{"Note": "Thank you for using Alpha Vantage! Our standard API call frequency is 5 calls per minute and 500 calls per day."}"#;
        
        let result = transport.check_api_error(response);
        assert!(result.is_err());
        
        if let Err(Error::RateLimit(_)) = result {
            // Expected
        } else {
            panic!("Expected RateLimit error");
        }
    }

    #[test]
    fn test_check_api_error_invalid_key() {
        let transport = Transport::new_mock();
        let response = r#"{"Error Message": "Invalid API call. Please retry or visit the documentation"}"#;
        
        let result = transport.check_api_error(response);
        assert!(result.is_err());
        
        if let Err(Error::Api(_)) = result {
            // Expected
        } else {
            panic!("Expected Api error");
        }
    }

    #[test]
    fn test_check_api_error_success() {
        let transport = Transport::new_mock();
        let response = r#"{"Time Series (Daily)": {}}"#;
        
        let result = transport.check_api_error(response);
        assert!(result.is_ok());
    }
}
