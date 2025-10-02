use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// CoinGecko API response for social data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoSocialResponse {
  pub id: String,
  pub symbol: String,
  pub name: String,
  pub links: CoinGeckoLinks,
  pub community_data: Option<CoinGeckoCommunityData>,
  pub developer_data: Option<CoinGeckoDeveloperData>,
  pub public_interest_stats: Option<CoinGeckoPublicInterest>,
  pub sentiment_votes_up_percentage: Option<f64>,
  pub sentiment_votes_down_percentage: Option<f64>,
  pub coingecko_score: Option<f64>,
  pub developer_score: Option<f64>,
  pub community_score: Option<f64>,
  pub liquidity_score: Option<f64>,
  pub public_interest_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoLinks {
  pub homepage: Vec<String>,
  pub whitepaper: Option<String>,
  #[serde(rename = "repos_url")]
  pub github: Option<CoinGeckoRepos>,
  pub telegram_channel_identifier: Option<String>,
  pub twitter_screen_name: Option<String>,
  pub facebook_username: Option<String>,
  pub subreddit_url: Option<String>,
  pub discord: Option<String>,
  pub announcement_url: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoRepos {
  pub github: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoCommunityData {
  pub facebook_likes: Option<i32>,
  pub twitter_followers: Option<i32>,
  pub reddit_average_posts_48h: Option<f64>,
  pub reddit_average_comments_48h: Option<f64>,
  pub reddit_subscribers: Option<i32>,
  pub reddit_accounts_active_48h: Option<i32>,
  pub telegram_channel_user_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoDeveloperData {
  pub forks: Option<i32>,
  pub stars: Option<i32>,
  pub subscribers: Option<i32>,
  pub total_issues: Option<i32>,
  pub closed_issues: Option<i32>,
  pub pull_requests_merged: Option<i32>,
  pub pull_request_contributors: Option<i32>,
  pub code_additions_deletions_4_weeks: Option<CoinGeckoCodeChanges>,
  pub commit_count_4_weeks: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoCodeChanges {
  pub additions: Option<i32>,
  pub deletions: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoPublicInterest {
  pub alexa_rank: Option<i32>,
  pub bing_matches: Option<i32>,
}

/// GitHub repository information for enhanced social data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepoInfo {
  pub full_name: String,
  pub description: Option<String>,
  pub stargazers_count: i32,
  pub forks_count: i32,
  pub subscribers_count: i32,
  pub open_issues_count: i32,
  pub updated_at: DateTime<Utc>,
  pub language: Option<String>,
  pub license: Option<GitHubLicense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLicense {
  pub name: String,
  pub spdx_id: Option<String>,
}

/// Processed social data ready for database insertion
#[derive(Debug, Clone)]
pub struct ProcessedSocialData {
  pub sid: i64,
  pub website_url: Option<String>,
  pub whitepaper_url: Option<String>,
  pub github_url: Option<String>,
  pub twitter_handle: Option<String>,
  pub twitter_followers: Option<i32>,
  pub telegram_url: Option<String>,
  pub telegram_members: Option<i32>,
  pub discord_url: Option<String>,
  pub discord_members: Option<i32>,
  pub reddit_url: Option<String>,
  pub reddit_subscribers: Option<i32>,
  pub facebook_url: Option<String>,
  pub facebook_likes: Option<i32>,
  pub coingecko_score: Option<Decimal>,
  pub developer_score: Option<Decimal>,
  pub community_score: Option<Decimal>,
  pub liquidity_score: Option<Decimal>,
  pub public_interest_score: Option<Decimal>,
  pub sentiment_votes_up_pct: Option<Decimal>,
  pub sentiment_votes_down_pct: Option<Decimal>,
}
