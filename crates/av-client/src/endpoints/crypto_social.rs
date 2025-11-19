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

use crate::transport::Transport;
use av_core::{Error, Result};
use av_models::crypto_social::{CoinGeckoSocialResponse, GitHubRepoInfo};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::debug;

pub struct CryptoSocialEndpoints {
  transport: Arc<Transport>,
}

impl CryptoSocialEndpoints {
  pub fn new(transport: Arc<Transport>) -> Self {
    Self { transport }
  }

  /// Fetch social data from CoinGecko for a specific cryptocurrency
  pub async fn fetch_coingecko_social_data(
    &self,
    coingecko_id: &str,
    api_key: Option<&str>,
  ) -> Result<CoinGeckoSocialResponse> {
    let mut url = format!(
      "https://api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&market_data=false&community_data=true&developer_data=true&sparkline=false",
      coingecko_id
    );

    if let Some(key) = api_key {
      url.push_str(&format!("&x_cg_pro_api_key={}", key));
    }

    debug!("Fetching CoinGecko social data for: {}", coingecko_id);

    // Use the HTTP client directly for external API calls
    let client = self.transport.client();
    let response = client
      .get(&url)
      .send()
      .await
      .map_err(|e| Error::Http(format!("CoinGecko API request failed: {}", e)))?;

    if !response.status().is_success() {
      return Err(Error::Http(format!("CoinGecko API error: HTTP {}", response.status())));
    }

    let social_data: CoinGeckoSocialResponse = response
      .json()
      .await
      .map_err(|e| Error::Parse(format!("Failed to parse CoinGecko response: {}", e)))?;

    Ok(social_data)
  }

  /// Fetch GitHub repository information for enhanced social data
  pub async fn fetch_github_repo_info(
    &self,
    repo_url: &str,
    github_token: Option<&str>,
  ) -> Result<GitHubRepoInfo> {
    // Extract owner/repo from GitHub URL
    let repo_path = repo_url
      .strip_prefix("https://github.com/")
      .or_else(|| repo_url.strip_prefix("http://github.com/"))
      .ok_or_else(|| Error::Config("Invalid GitHub URL format".to_string()))?;

    let api_url = format!("https://api.github.com/repos/{}", repo_path);

    debug!("Fetching GitHub repo info for: {}", repo_path);

    // Use the HTTP client directly for external API calls
    let client = self.transport.client();
    let mut request = client.get(&api_url);

    // Add headers
    request = request.header("User-Agent", "AlphaVantage-Rust-Client/1.0");
    request = request.header("Accept", "application/vnd.github.v3+json");

    if let Some(token) = github_token {
      request = request.header("Authorization", format!("token {}", token));
    }

    let response =
      request.send().await.map_err(|e| Error::Http(format!("GitHub API request failed: {}", e)))?;

    if !response.status().is_success() {
      if response.status() == 404 {
        return Err(Error::Api("GitHub repository not found".to_string()));
      }
      return Err(Error::Http(format!("GitHub API error: HTTP {}", response.status())));
    }

    let repo_info: GitHubRepoInfo = response
      .json()
      .await
      .map_err(|e| Error::Parse(format!("Failed to parse GitHub response: {}", e)))?;

    Ok(repo_info)
  }

  /// Get multiple coin social data with rate limiting
  pub async fn batch_fetch_social_data(
    &self,
    coingecko_ids: Vec<&str>,
    api_key: Option<&str>,
    delay_ms: u64,
  ) -> Vec<Result<CoinGeckoSocialResponse>> {
    let mut results = Vec::new();

    for (i, id) in coingecko_ids.iter().enumerate() {
      if i > 0 {
        sleep(Duration::from_millis(delay_ms)).await;
      }

      let result = self.fetch_coingecko_social_data(id, api_key).await;
      results.push(result);
    }

    results
  }
}
