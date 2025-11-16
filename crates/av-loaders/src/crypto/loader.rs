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

use super::sources::{
  CryptoDataProvider, coincap::CoinCapProvider, coingecko::CoinGeckoProvider,
  coinmarketcap::CoinMarketCapProvider, coinpaprika::CoinPaprikaProvider,
  sosovalue::SosoValueProvider,
};
use super::{
  CryptoDataSource, CryptoLoaderConfig, CryptoLoaderError, CryptoLoaderResult, CryptoSymbol,
  SourceResult,
};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{error, info, warn};

pub struct CryptoSymbolLoader {
  config: CryptoLoaderConfig,
  client: Client,
  providers: HashMap<CryptoDataSource, Box<dyn CryptoDataProvider + Send + Sync>>,
}

impl CryptoSymbolLoader {
  pub fn new(config: CryptoLoaderConfig) -> Self {
    let client = Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .user_agent("AlphaVantage-Rust-Client/1.0")
      .build()
      .expect("Failed to create HTTP client");

    let mut providers: HashMap<CryptoDataSource, Box<dyn CryptoDataProvider + Send + Sync>> =
      HashMap::new();

    // Initialize providers based on config
    for source in &config.sources {
      match source {
        CryptoDataSource::CoinMarketCap => {
          if let Ok(api_key) = std::env::var("CMC_API_KEY") {
            providers.insert(
              CryptoDataSource::CoinMarketCap,
              Box::new(CoinMarketCapProvider::new(api_key)),
            );
          }
        }
        CryptoDataSource::CoinGecko => {
          if let Ok(api_key) = std::env::var("COINGECKO_API_KEY") {
            providers.insert(
              CryptoDataSource::CoinGecko,
              Box::new(CoinGeckoProvider::new(Some(api_key))), // TODO: Make API consistent
            );
          }
        }
        CryptoDataSource::CoinPaprika => {
          providers.insert(CryptoDataSource::CoinPaprika, Box::new(CoinPaprikaProvider));
        }
        CryptoDataSource::CoinCap => {
          providers.insert(CryptoDataSource::CoinCap, Box::new(CoinCapProvider));
        }
        CryptoDataSource::SosoValue => {
          if let Ok(api_key) = std::env::var("SOSOVALUE_API_KEY") {
            providers.insert(
              CryptoDataSource::SosoValue,
              Box::new(SosoValueProvider::new(Some(api_key))), // TODO: make api consistend
            );
          }
        }
      }
    }

    Self { config, client, providers }
  }

  pub fn with_api_keys(mut self, api_keys: HashMap<CryptoDataSource, String>) -> Self {
    // Recreate providers with API keys
    for (source, api_key) in api_keys {
      match source {
        CryptoDataSource::CoinGecko => {
          self.providers.insert(source, Box::new(CoinGeckoProvider::new(Some(api_key))));
        }
        CryptoDataSource::CoinMarketCap => {
          self.providers.insert(source, Box::new(CoinMarketCapProvider::new(api_key)));
        }
        CryptoDataSource::SosoValue => {
          self.providers.insert(source, Box::new(SosoValueProvider::new(Some(api_key))));
        }
        _ => {
          warn!("API key provided for source that doesn't support it: {}", source);
        }
      }
    }
    self
  }

  pub async fn load_all_symbols(&self) -> CryptoLoaderResult<LoadAllSymbolsResult> {
    let start_time = Instant::now();
    info!("Loading symbols from {} sources", self.config.sources.len());

    let mut all_symbols = Vec::new();
    let mut source_results = HashMap::new();
    let mut total_failed = 0;
    let mut total_loaded = 0;

    // Show progress bar if enabled
    let progress = if self.config.enable_progress_bar {
      let pb = ProgressBar::new(self.config.sources.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
          .unwrap()
          .progress_chars("#>-"),
      );
      pb.set_message("Loading crypto symbols");
      Some(pb)
    } else {
      None
    };

    // Load from each source sequentially to respect rate limits
    for source in &self.config.sources {
      let start = Instant::now();

      if let Some(pb) = &progress {
        pb.set_message(format!("Loading from {}", source));
      }

      let response_time = start.elapsed().as_millis() as u64;

      match self.load_from_source(*source).await {
        Ok(symbols) => {
          info!("Successfully loaded {} symbols from {}", symbols.len(), source);
          total_loaded += symbols.len();
          all_symbols.extend(symbols.clone());

          source_results.insert(
            *source,
            SourceResult {
              symbols_fetched: symbols.len(),
              errors: vec![],
              rate_limited: false,
              response_time_ms: response_time,
            },
          );
        }
        Err(e) => {
          error!("Failed to load symbols from {}: {}", source, e);
          total_failed += 1;

          let rate_limited = matches!(e, CryptoLoaderError::RateLimitExceeded(_));
          source_results.insert(
            *source,
            SourceResult {
              symbols_fetched: 0,
              errors: vec![e.to_string()],
              rate_limited,
              response_time_ms: response_time,
            },
          );
        }
      }

      if let Some(pb) = &progress {
        pb.inc(1);
      }
    }

    if let Some(pb) = progress {
      pb.finish_with_message("Symbol loading complete");
    }

    // Deduplicate symbols by symbol+source combination
    let unique_symbols = self.deduplicate_symbols(all_symbols);
    let final_count = unique_symbols.len();
    let processing_time = start_time.elapsed().as_millis() as u64;

    info!(
      "Crypto symbol loading complete: {} unique symbols loaded, {} sources failed in {}ms",
      final_count, total_failed, processing_time
    );

    Ok(LoadAllSymbolsResult {
      symbols_loaded: final_count,
      symbols_failed: total_failed,
      symbols_skipped: total_loaded - final_count, // Duplicates removed
      symbols: unique_symbols,
      source_results,
      processing_time_ms: processing_time,
    })
  }

  /// Remove duplicate symbols, preferring symbols from sources in priority order
  fn deduplicate_symbols(&self, symbols: Vec<CryptoSymbol>) -> Vec<CryptoSymbol> {
    let mut unique_symbols: HashMap<String, CryptoSymbol> = HashMap::new();

    // Define source priority (higher number = higher priority)
    let source_priority = |source: &CryptoDataSource| -> u8 {
      match source {
        CryptoDataSource::CoinMarketCap => 2,
        CryptoDataSource::CoinGecko => 1,
        CryptoDataSource::CoinPaprika => 4,
        CryptoDataSource::CoinCap => 5,
        CryptoDataSource::SosoValue => 3,
      }
    };

    for symbol in symbols {
      let key = symbol.symbol.clone();

      match unique_symbols.get(&key) {
        Some(existing) => {
          // NEW LOGIC: Prefer symbols with valid market cap ranks
          let should_replace = match (&symbol.market_cap_rank, &existing.market_cap_rank) {
            // If new symbol has rank but existing doesn't, use new symbol
            (Some(_), None) => true,
            // If existing has rank but new doesn't, keep existing
            (None, Some(_)) => false,
            // If both have ranks, prefer the better (lower) rank
            (Some(new_rank), Some(existing_rank)) => new_rank < existing_rank,
            // If neither has rank, fall back to source priority
            (None, None) => source_priority(&symbol.source) > source_priority(&existing.source),
          };

          if should_replace {
            unique_symbols.insert(key, symbol);
          }
        }
        None => {
          unique_symbols.insert(key, symbol);
        }
      }
    }

    unique_symbols.into_values().collect()
  }

  pub async fn load_from_source(
    &self,
    source: CryptoDataSource,
  ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError> {
    if let Some(provider) = self.providers.get(&source) {
      info!("Loading symbols from {}", source);

      let symbols = provider.fetch_symbols(&self.client).await?;

      info!("Fetched {} symbols from {}", symbols.len(), source);
      Ok(symbols)
    } else {
      Err(CryptoLoaderError::SourceUnavailable(source.to_string()))
    }
  }
}

impl Clone for CryptoSymbolLoader {
  fn clone(&self) -> Self {
    // Create a new loader with the same configuration
    Self::new(self.config.clone())
  }
}

// New struct to return from load_all_symbols
#[derive(Debug)]
pub struct LoadAllSymbolsResult {
  pub symbols_loaded: usize,
  pub symbols_failed: usize,
  pub symbols_skipped: usize,
  pub symbols: Vec<CryptoSymbol>,
  pub source_results: HashMap<CryptoDataSource, SourceResult>,
  pub processing_time_ms: u64,
}
