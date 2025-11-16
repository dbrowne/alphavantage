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

//! Simple wrapper for loading crypto news using the existing NewsLoader
//!
//! This module provides a thin wrapper around NewsLoader that formats
//! crypto symbols correctly for the AlphaVantage API (CRYPTO:BTC format).

use crate::{
  DataLoader, LoaderContext, LoaderResult, NewsLoader, NewsLoaderConfig, NewsLoaderInput,
  NewsLoaderOutput, news_loader::SymbolInfo,
};
use chrono::{DateTime, Utc};
use tracing::info;

/// Load crypto news by converting symbols to CRYPTO:XXX format
pub async fn load_crypto_news(
  context: &LoaderContext,
  symbols: Vec<SymbolInfo>,
  config: NewsLoaderConfig,
  time_from: Option<DateTime<Utc>>,
  time_to: Option<DateTime<Utc>>,
) -> LoaderResult<NewsLoaderOutput> {
  // Convert symbols to crypto format (CRYPTO:BTC, CRYPTO:ETH, etc.)
  let crypto_symbols: Vec<SymbolInfo> = symbols
    .into_iter()
    .map(|s| SymbolInfo { sid: s.sid, symbol: format!("CRYPTO:{}", s.symbol) })
    .collect();

  info!("Loading news for {} crypto symbols", crypto_symbols.len());

  // Use the existing NewsLoader with crypto-formatted symbols
  let loader = NewsLoader::new(5).with_config(config);

  let input = NewsLoaderInput { symbols: crypto_symbols, time_from, time_to };

  loader.load(context, input).await
}
