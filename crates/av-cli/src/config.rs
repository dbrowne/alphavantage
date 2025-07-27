use anyhow::{Context, Result};
use av_core::Config as CoreConfig;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
  pub api_config: CoreConfig,
  pub database_url: String,
  pub nasdaq_csv_path: String,
  pub nyse_csv_path: String,
}

impl Config {
  pub fn from_env() -> Result<Self> {
    let api_key = env::var("ALPHA_VANTAGE_API_KEY")
      .context("ALPHA_VANTAGE_API_KEY environment variable not set")?;

    let database_url =
      env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

    let nasdaq_csv_path =
      env::var("NASDAQ_LISTED").unwrap_or_else(|_| "./data/nasdaq-listed_csv.csv".to_string());

    let nyse_csv_path =
      env::var("OTHER_LISTED").unwrap_or_else(|_| "./data/other-listed.csv".to_string());

    let api_config = CoreConfig {
      api_key,
      base_url: av_core::ALPHA_VANTAGE_BASE_URL.to_string(),
      rate_limit: 75, // Free tier default
      timeout_secs: 30,
      max_retries: 3,
    };

    Ok(Self { api_config, database_url, nasdaq_csv_path, nyse_csv_path })
  }
}
