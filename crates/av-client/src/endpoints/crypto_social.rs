use crate::transport::Transport;
use av_core::{Error, Result};
use av_models::crypto_social::{CoinGeckoSocialResponse, GitHubRepoInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::debug;

pub struct CryptoSocialEndpoints {
    transport: Arc<Transport>,
}

impl CryptoSocialEndpoints {
    pub fn new(transport: Arc<Transport>) -> Self {
        Self {
            transport: transport.clone(),
            client: transport.client().clone(),
        }
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

        let response = self.transport.get(&url).await?;

        if !response.status().is_success() {
            return Err(Error::RequestFailed(format!(
                "CoinGecko API error: HTTP {}", response.status()
            )));
        }

        let social_data: CoinGeckoSocialResponse = response.json().await
            .map_err(|e| Error::ParseError(format!("Failed to parse CoinGecko response: {}", e)))?;

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
            .ok_or_else(|| Error::InvalidInput("Invalid GitHub URL format".to_string()))?;

        let api_url = format!("https://api.github.com/repos/{}", repo_path);

        debug!("Fetching GitHub repo info for: {}", repo_path);

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "AlphaVantage-Rust-Client/1.0".to_string());
        headers.insert("Accept".to_string(), "application/vnd.github.v3+json".to_string());

        if let Some(token) = github_token {
            headers.insert("Authorization".to_string(), format!("token {}", token));
        }

        let response = self.transport.get_with_headers(&api_url, headers).await?;

        if !response.status().is_success() {
            if response.status() == 404 {
                return Err(Error::NotFound("GitHub repository not found".to_string()));
            }
            return Err(Error::RequestFailed(format!(
                "GitHub API error: HTTP {}", response.status()
            )));
        }

        let repo_info: GitHubRepoInfo = response.json().await
            .map_err(|e| Error::ParseError(format!("Failed to parse GitHub response: {}", e)))?;

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
