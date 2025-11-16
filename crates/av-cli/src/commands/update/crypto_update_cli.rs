/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

use clap::Args;

#[derive(Args, Debug)]
pub struct UpdateCryptoArgs {
  /// Specific symbols to update (comma-separated)
  #[arg(short, long, value_delimiter = ',')]
  pub symbols: Option<String>,

  /// Limit number of symbols to update
  #[arg(short, long)]
  pub limit: Option<usize>,

  /// Only update basic crypto data (descriptions, market cap ranks)
  #[arg(long)]
  pub basic_only: bool,

  /// Only update social data (social media metrics)
  #[arg(long)]
  pub social_only: bool,

  /// Only update technical data (blockchain/GitHub data)
  #[arg(long)]
  pub technical_only: bool,

  /// Delay between requests in milliseconds
  #[arg(long, default_value = "2000")]
  pub delay_ms: u64,

  /// CoinGecko API key for enhanced data
  #[arg(long, env = "COINGECKO_API_KEY")]
  pub coingecko_api_key: Option<String>,

  /// GitHub token for GitHub API access
  #[arg(long, env = "GITHUB_TOKEN")]
  pub github_token: Option<String>,

  /// Dry run mode - don't update database
  #[arg(long)]
  pub dry_run: bool,

  /// Verbose output
  #[arg(short, long)]
  pub verbose: bool,
}
