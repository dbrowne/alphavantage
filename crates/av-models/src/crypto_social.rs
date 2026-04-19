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

//! CoinGecko social, community, and developer data models.
//!
//! This module provides deserialization structs for the CoinGecko
//! `/coins/{id}` API response (community data, developer stats, scores),
//! an optional GitHub repository enrichment model, and a
//! [`ProcessedSocialData`] struct that normalizes the raw API data into a
//! flat form suitable for database insertion.
//!
//! # Data flow
//!
//! ```text
//! CoinGecko API JSON
//!   └──► CoinGeckoSocialResponse  (raw API shape)
//!          └──► ProcessedSocialData    (flat, DB-ready, Decimal scores)
//!                └──► crypto_social table  (via av-database-postgres)
//! ```
//!
//! # Type inventory
//!
//! ## CoinGecko API models (raw JSON shapes)
//!
//! | Type                       | JSON path / purpose                         |
//! |----------------------------|---------------------------------------------|
//! | [`CoinGeckoSocialResponse`]| Top-level response: id, scores, nested data |
//! | [`CoinGeckoLinks`]         | `.links` — homepage, social URLs, repos     |
//! | [`CoinGeckoRepos`]         | `.links.repos_url` — GitHub repo list       |
//! | [`CoinGeckoCommunityData`] | `.community_data` — followers, subscribers  |
//! | [`CoinGeckoDeveloperData`] | `.developer_data` — GitHub stats, commits   |
//! | [`CoinGeckoCodeChanges`]   | `.developer_data.code_additions_deletions_4_weeks` |
//! | [`CoinGeckoPublicInterest`]| `.public_interest_stats` — Alexa rank, Bing |
//!
//! ## GitHub enrichment
//!
//! | Type              | Purpose                                          |
//! |-------------------|--------------------------------------------------|
//! | [`GitHubRepoInfo`]| GitHub API `/repos/{owner}/{repo}` response      |
//! | [`GitHubLicense`] | Repository license metadata                      |
//!
//! ## Normalized output
//!
//! | Type                    | Purpose                                        |
//! |-------------------------|------------------------------------------------|
//! | [`ProcessedSocialData`] | Flat struct with `Decimal` scores for DB insert|

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ─── CoinGecko API response ────────────────────────────────────────────────

/// Top-level response from the CoinGecko `/coins/{id}` endpoint.
///
/// Contains the coin's identity (`id`, `symbol`, `name`), nested social/dev
/// data, and composite scores computed by CoinGecko. The nested structs
/// mirror the JSON structure — optional fields are `None` when the API
/// omits them for a particular coin.
///
/// # Scores
///
/// CoinGecko computes five composite scores (all `Option<f64>`):
/// - `coingecko_score` — overall composite.
/// - `developer_score` — GitHub activity-based.
/// - `community_score` — social media engagement.
/// - `liquidity_score` — market depth and volume.
/// - `public_interest_score` — search engine and web interest.
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

/// Social media URLs, project homepage, whitepaper, and repository links.
///
/// The `github` field is renamed from `repos_url` in the JSON. `homepage`
/// and `announcement_url` are arrays because CoinGecko may list multiple URLs.
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

/// GitHub repository URL list nested under `links.repos_url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoRepos {
  /// List of GitHub repository URLs (may be empty).
  pub github: Vec<String>,
}

/// Community / social media metrics from the `community_data` JSON field.
///
/// Follower and subscriber counts from Twitter, Reddit, Telegram, and Facebook.
/// Activity metrics (posts, comments, active accounts) are 48-hour rolling averages.
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

/// GitHub development activity metrics from the `developer_data` JSON field.
///
/// Tracks repository health indicators: forks, stars, issues, PRs, and
/// recent commit activity. All fields are optional because not every coin
/// has a public GitHub presence.
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

/// Code additions and deletions over the last 4 weeks.
///
/// Nested under `developer_data.code_additions_deletions_4_weeks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoCodeChanges {
  pub additions: Option<i32>,
  pub deletions: Option<i32>,
}

/// Public interest / web visibility metrics.
///
/// Nested under `public_interest_stats`. Alexa rank and Bing search matches
/// provide a rough measure of mainstream visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinGeckoPublicInterest {
  pub alexa_rank: Option<i32>,
  pub bing_matches: Option<i32>,
}

// ─── GitHub enrichment ──────────────────────────────────────────────────────

/// Repository metadata from the GitHub API (`/repos/{owner}/{repo}`).
///
/// Used to supplement CoinGecko's developer data with more granular
/// GitHub statistics. Fetched separately from the CoinGecko response.
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

/// Repository license from the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLicense {
  /// Human-readable license name (e.g., `"MIT License"`).
  pub name: String,
  /// SPDX identifier (e.g., `"MIT"`, `"Apache-2.0"`). `None` for
  /// non-standard or unrecognized licenses.
  pub spdx_id: Option<String>,
}

// ─── Normalized output ──────────────────────────────────────────────────────

/// Flat, database-ready representation of social data for a cryptocurrency.
///
/// Produced by the data ingestion pipeline from a [`CoinGeckoSocialResponse`]
/// (and optionally enriched from [`GitHubRepoInfo`]). This struct maps
/// directly to the fields of the `crypto_social` database table.
///
/// # Key differences from [`CoinGeckoSocialResponse`]
///
/// - **Flat structure:** Nested data (links, community, dev) is flattened
///   into top-level fields.
/// - **`Decimal` scores:** Composite scores use [`rust_decimal::Decimal`]
///   instead of `f64` for database-compatible precision.
/// - **`sid` field:** Includes the internal security ID for database foreign-key
///   linkage.
/// - **Derived URLs:** Social platform URLs are extracted from the nested
///   [`CoinGeckoLinks`] struct into individual `Option<String>` fields.
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
