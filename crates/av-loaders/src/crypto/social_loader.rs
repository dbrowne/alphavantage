use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult};
use av_database_postgres::{
    models::crypto::{NewCryptoSocial},
    schema::crypto_social,
};
pub use av_models::crypto_social::{CoinGeckoSocialResponse, ProcessedSocialData, GitHubRepoInfo};
use diesel::prelude::*;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct CryptoSocialConfig {
    pub batch_size: usize,
    pub max_concurrent_requests: usize,
    pub rate_limit_delay_ms: u64,
    pub coingecko_api_key: Option<String>,
    pub github_token: Option<String>,
    pub enable_progress_bar: bool,
    pub fetch_github_data: bool,
    pub update_existing: bool,
}

impl Default for CryptoSocialConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            max_concurrent_requests: 5,
            rate_limit_delay_ms: 2000, // Conservative for free tier
            coingecko_api_key: None,
            github_token: None,
            enable_progress_bar: true,
            fetch_github_data: true,
            update_existing: false,
        }
    }
}

#[derive(Debug)]
pub struct CryptoSocialInput {
    pub symbols: Option<Vec<CryptoSymbolForSocial>>,
    pub coingecko_ids: Option<Vec<String>>,
    pub update_existing: bool,
    pub batch_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct CryptoSymbolForSocial {
    pub sid: i64,
    pub symbol: String,
    pub name: String,
    pub coingecko_id: Option<String>,
}

#[derive(Debug)]
pub struct CryptoSocialOutput {
    pub social_data_fetched: usize,
    pub social_data_processed: usize,
    pub github_repos_fetched: usize,
    pub errors: Vec<String>,
    pub social_data: Vec<ProcessedSocialData>,
}

pub struct CryptoSocialLoader {
    config: CryptoSocialConfig,
}

impl CryptoSocialLoader {
    pub fn new(config: CryptoSocialConfig) -> Self {
        Self { config }
    }

    /// Create HTTP client for external API calls
    fn create_http_client(&self) -> Result<reqwest::Client, LoaderError> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("AlphaVantage-Rust-Client/1.0")
            .build()
            .map_err(|e| LoaderError::ApiError(format!("Failed to create HTTP client: {}", e)))
    }

    /// Fetch social data from CoinGecko
    async fn fetch_coingecko_social(
        &self,
        client: &reqwest::Client,
        coingecko_id: &str,
    ) -> Result<CoinGeckoSocialResponse, String> {
        let mut url = format!(
            "https://api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&market_data=false&community_data=true&developer_data=true&sparkline=false",
            coingecko_id
        );

        if let Some(ref key) = self.config.coingecko_api_key {
            url.push_str(&format!("&x_cg_pro_api_key={}", key));
        }

        debug!("Fetching CoinGecko social data for: {}", coingecko_id);

        let response = client.get(&url).send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("CoinGecko API error: HTTP {}", response.status()));
        }

        response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Fetch GitHub repository information
    async fn fetch_github_repo_info(
        &self,
        client: &reqwest::Client,
        repo_url: &str,
    ) -> Result<GitHubRepoInfo, String> {
        let repo_path = repo_url
            .strip_prefix("https://github.com/")
            .or_else(|| repo_url.strip_prefix("http://github.com/"))
            .ok_or_else(|| "Invalid GitHub URL format".to_string())?;

        let api_url = format!("https://api.github.com/repos/{}", repo_path);
        debug!("Fetching GitHub repo info for: {}", repo_path);

        let mut request = client.get(&api_url)
            .header("User-Agent", "AlphaVantage-Rust-Client/1.0")
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(ref token) = self.config.github_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        let response = request.send().await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("GitHub API error: HTTP {}", response.status()));
        }

        response.json().await
            .map_err(|e| format!("Failed to parse GitHub response: {}", e))
    }

    /// Process social data into database-ready format
    fn process_social_data(
        &self,
        sid: i64,
        data: &CoinGeckoSocialResponse,
        _github_info: Option<&GitHubRepoInfo>,
    ) -> ProcessedSocialData {
        ProcessedSocialData {
            sid,
            website_url: data.links.homepage.first().cloned(),
            whitepaper_url: data.links.whitepaper.clone(),
            github_url: data.links.github.as_ref()
                .and_then(|repos| repos.github.first().cloned()),
            twitter_handle: data.links.twitter_screen_name.clone(),
            twitter_followers: data.community_data.as_ref()
                .and_then(|cd| cd.twitter_followers),
            telegram_url: data.links.telegram_channel_identifier.clone(),
            telegram_members: data.community_data.as_ref()
                .and_then(|cd| cd.telegram_channel_user_count),
            discord_url: data.links.discord.clone(),
            discord_members: None, // Not available in CoinGecko API
            reddit_url: data.links.subreddit_url.clone(),
            reddit_subscribers: data.community_data.as_ref()
                .and_then(|cd| cd.reddit_subscribers),
            facebook_url: data.links.facebook_username.clone(),
            facebook_likes: data.community_data.as_ref()
                .and_then(|cd| cd.facebook_likes),
            coingecko_score: data.coingecko_score.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            developer_score: data.developer_score.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            community_score: data.community_score.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            liquidity_score: data.liquidity_score.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            public_interest_score: data.public_interest_score.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            sentiment_votes_up_pct: data.sentiment_votes_up_percentage.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
            sentiment_votes_down_pct: data.sentiment_votes_down_percentage.map(|s| rust_decimal::Decimal::from_f64_retain(s).unwrap_or_default()),
        }
    }

    /// Convert ProcessedSocialData to NewCryptoSocial for database insertion
    pub fn to_new_crypto_social(&self, data: &ProcessedSocialData) -> NewCryptoSocial {
        let now = Utc::now();

        NewCryptoSocial {
            sid: data.sid,
            website_url: data.website_url.clone(),
            whitepaper_url: data.whitepaper_url.clone(),
            github_url: data.github_url.clone(),
            twitter_handle: data.twitter_handle.clone(),
            twitter_followers: data.twitter_followers,
            telegram_url: data.telegram_url.clone(),
            telegram_members: data.telegram_members,
            discord_url: data.discord_url.clone(),
            discord_members: data.discord_members,
            reddit_url: data.reddit_url.clone(),
            reddit_subscribers: data.reddit_subscribers,
            facebook_url: data.facebook_url.clone(),
            facebook_likes: data.facebook_likes,
            coingecko_score: data.coingecko_score.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            developer_score: data.developer_score.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            community_score: data.community_score.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            liquidity_score: data.liquidity_score.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            public_interest_score: data.public_interest_score.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            sentiment_votes_up_pct: data.sentiment_votes_up_pct.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            sentiment_votes_down_pct: data.sentiment_votes_down_pct.as_ref().map(|d| {
                BigDecimal::from_str(&d.to_string()).unwrap_or_default()
            }),
            c_time: now,
            m_time: now,
        }
    }

    /// Save social data to database
    async fn save_social_data(
        &self,
        conn: &mut PgConnection,
        social_data: &[ProcessedSocialData],
        update_existing: bool,
    ) -> LoaderResult<(usize, usize)> {
        let mut inserted = 0;
        let mut updated = 0;

        for data in social_data {
            let new_social = self.to_new_crypto_social(data);

            if update_existing {
                // Try to update first, then insert if not found
                let rows_affected = diesel::update(crypto_social::table.find(data.sid))
                    .set((
                        crypto_social::website_url.eq(&new_social.website_url),
                        crypto_social::whitepaper_url.eq(&new_social.whitepaper_url),
                        crypto_social::github_url.eq(&new_social.github_url),
                        crypto_social::twitter_handle.eq(&new_social.twitter_handle),
                        crypto_social::twitter_followers.eq(&new_social.twitter_followers),
                        crypto_social::telegram_url.eq(&new_social.telegram_url),
                        crypto_social::telegram_members.eq(&new_social.telegram_members),
                        crypto_social::discord_url.eq(&new_social.discord_url),
                        crypto_social::discord_members.eq(&new_social.discord_members),
                        crypto_social::reddit_url.eq(&new_social.reddit_url),
                        crypto_social::reddit_subscribers.eq(&new_social.reddit_subscribers),
                        crypto_social::facebook_url.eq(&new_social.facebook_url),
                        crypto_social::facebook_likes.eq(&new_social.facebook_likes),
                        crypto_social::coingecko_score.eq(&new_social.coingecko_score),
                        crypto_social::developer_score.eq(&new_social.developer_score),
                        crypto_social::community_score.eq(&new_social.community_score),
                        crypto_social::liquidity_score.eq(&new_social.liquidity_score),
                        crypto_social::public_interest_score.eq(&new_social.public_interest_score),
                        crypto_social::sentiment_votes_up_pct.eq(&new_social.sentiment_votes_up_pct),
                        crypto_social::sentiment_votes_down_pct.eq(&new_social.sentiment_votes_down_pct),
                        crypto_social::m_time.eq(&new_social.m_time),
                    ))
                    .execute(conn)
                    .map_err(|e| LoaderError::ApiError(e.to_string()))?;

                if rows_affected > 0 {
                    updated += 1;
                } else {
                    // Insert if update didn't affect any rows
                    match diesel::insert_into(crypto_social::table)
                        .values(&new_social)
                        .execute(conn)
                    {
                        Ok(_) => inserted += 1,
                        Err(e) => {
                            error!("Failed to insert social data for sid {}: {}", data.sid, e);
                            return Err(LoaderError::ApiError(e.to_string()));
                        }
                    }
                }
            } else {
                // Insert only mode
                match diesel::insert_into(crypto_social::table)
                    .values(&new_social)
                    .execute(conn)
                {
                    Ok(_) => inserted += 1,
                    Err(diesel::result::Error::DatabaseError(
                            diesel::result::DatabaseErrorKind::UniqueViolation,
                            _,
                        )) => {
                        debug!("Social data already exists for sid {}, skipping", data.sid);
                    }
                    Err(e) => {
                        error!("Failed to insert social data for sid {}: {}", data.sid, e);
                        return Err(LoaderError::ApiError(e.to_string()));
                    }
                }
            }
        }

        Ok((inserted, updated))
    }
}

#[async_trait::async_trait]
impl DataLoader for CryptoSocialLoader {
    type Input = CryptoSocialInput;
    type Output = CryptoSocialOutput;

    fn name(&self) -> &'static str {
        "CryptoSocialLoader"
    }

    async fn load(
        &self,
        _context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output> {
        info!("Starting crypto social data loading");

        let symbols = input.symbols.ok_or_else(|| {
            LoaderError::InvalidData("No symbols provided for social data loading".to_string())
        })?;

        if symbols.is_empty() {
            warn!("No symbols to process for social data");
            return Ok(CryptoSocialOutput {
                social_data_fetched: 0,
                social_data_processed: 0,
                github_repos_fetched: 0,
                errors: vec![],
                social_data: vec![],
            });
        }

        // Create HTTP client for external APIs
        let http_client = self.create_http_client()?;

        let mut all_social_data = Vec::new();
        let mut errors = Vec::new();
        let mut social_data_fetched = 0;
        let mut github_repos_fetched = 0;

        // Process symbols in batches
        let batch_size = input.batch_size.unwrap_or(self.config.batch_size);
        let chunks: Vec<_> = symbols.chunks(batch_size).collect();

        for (batch_idx, batch) in chunks.iter().enumerate() {
            info!("Processing batch {} of {} ({} symbols)",
                  batch_idx + 1, chunks.len(), batch.len());

            for symbol in batch.iter() {
                // Only fetch social data if we have a CoinGecko ID
                if let Some(ref coingecko_id) = symbol.coingecko_id {
                    match self.fetch_coingecko_social(&http_client, coingecko_id).await {
                        Ok(social_response) => {
                            social_data_fetched += 1;

                            // Optionally fetch GitHub data if enabled and GitHub URL is available
                            let github_info = if self.config.fetch_github_data {
                                if let Some(ref github_repos) = social_response.links.github {
                                    if let Some(github_url) = github_repos.github.first() {
                                        match self.fetch_github_repo_info(&http_client, github_url).await {
                                            Ok(repo_info) => {
                                                github_repos_fetched += 1;
                                                Some(repo_info)
                                            }
                                            Err(e) => {
                                                warn!("Failed to fetch GitHub data for {}: {}", symbol.symbol, e);
                                                None
                                            }
                                        }
                                    } else { None }
                                } else { None }
                            } else { None };

                            // Process and add to results
                            let processed_data = self.process_social_data(
                                symbol.sid,
                                &social_response,
                                github_info.as_ref(),
                            );
                            all_social_data.push(processed_data);
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to fetch social data for {} ({}): {}",
                                                    symbol.symbol, coingecko_id, e);
                            warn!("{}", error_msg);
                            errors.push(error_msg);
                        }
                    }
                } else {
                    debug!("No CoinGecko ID for symbol {}, skipping", symbol.symbol);
                }

                // Rate limiting
                if social_data_fetched > 0 && social_data_fetched % 10 == 0 {
                    sleep(Duration::from_millis(self.config.rate_limit_delay_ms)).await;
                }
            }
        }

        info!("Completed social data fetching: {} fetched, {} GitHub repos, {} errors",
              social_data_fetched, github_repos_fetched, errors.len());

        Ok(CryptoSocialOutput {
            social_data_fetched,
            social_data_processed: all_social_data.len(),
            github_repos_fetched,
            errors,
            social_data: all_social_data,
        })
    }
}