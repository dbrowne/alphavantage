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

pub mod coincap;
pub mod coingecko;
pub mod coinmarketcap;
pub mod coinpaprika;
pub mod sosovalue;

use crate::crypto::{CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use reqwest::Client;
use std::sync::Arc;

#[async_trait]
pub trait CryptoDataProvider {
  async fn fetch_symbols(
    &self,
    client: &Client,
    cache_repo: Option<&Arc<dyn CacheRepository>>,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError>;
  fn source_name(&self) -> &'static str;
  fn rate_limit_delay(&self) -> u64;
  fn requires_api_key(&self) -> bool;
}
