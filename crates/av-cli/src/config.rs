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
      env::var("NASDAQ_LISTED").unwrap_or_else(|_| "./data/nasdaq-listed.csv".to_string());

    let nyse_csv_path =
      env::var("OTHER_LISTED").unwrap_or_else(|_| "./data/nyse-listed.csv".to_string());

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
