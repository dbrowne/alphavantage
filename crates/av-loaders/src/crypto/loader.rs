use super::sources::{
  CoinCapProvider, CoinGeckoProvider, CoinPaprikaProvider, CryptoDataProvider, SosoValueProvider,
};
use super::{
  CryptoDataSource, CryptoLoaderConfig, CryptoLoaderError, CryptoLoaderResult, CryptoSymbol,
  SourceResult,
};
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
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
        CryptoDataSource::CoinGecko => {
          providers.insert(
            CryptoDataSource::CoinGecko,
            Box::new(CoinGeckoProvider::new(None)), // TODO: Get from env
          );
        }
        CryptoDataSource::CoinPaprika => {
          providers.insert(CryptoDataSource::CoinPaprika, Box::new(CoinPaprikaProvider));
        }
        CryptoDataSource::CoinCap => {
          providers.insert(CryptoDataSource::CoinCap, Box::new(CoinCapProvider));
        }
        CryptoDataSource::SosoValue => {
          providers.insert(
            CryptoDataSource::SosoValue,
            Box::new(SosoValueProvider::new(None)), // TODO: Get from env
          );
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

  pub async fn load_all_symbols(&self) -> Result<CryptoLoaderResult, CryptoLoaderError> {
    let start_time = Instant::now();
    info!("Starting crypto symbol loading from {} sources", self.providers.len());

    let progress = if self.config.enable_progress_bar {
      let pb = ProgressBar::new(self.providers.len() as u64);
      pb.set_style(
        ProgressStyle::default_bar()
          .template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
          )
          .unwrap()
          .progress_chars("#>-"),
      );
      Some(pb)
    } else {
      None
    };

    let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_requests));
    let mut source_results = HashMap::new();
    let mut all_symbols = Vec::new();

    // Create futures for each provider
    let mut futures = Vec::new();

    for (source, provider) in &self.providers {
      let semaphore = semaphore.clone();
      let client = self.client.clone();
      let source = *source;
      let provider_ref = provider.as_ref();
      let progress = progress.clone();
      let retry_attempts = self.config.retry_attempts;
      let retry_delay = self.config.retry_delay_ms;

      let future = async move {
        let _permit = semaphore.acquire().await.expect("Semaphore acquire failed");
        let provider_start = Instant::now();

        if let Some(ref pb) = progress {
          pb.set_message(format!("Loading from {}", provider_ref.source_name()));
        }

        let mut attempts = 0;
        let result = loop {
          attempts += 1;

          match provider_ref.fetch_symbols(&client).await {
            Ok(symbols) => break Ok(symbols),
            Err(e) => {
              if attempts >= retry_attempts {
                break Err(e);
              }
              warn!(
                "Attempt {} failed for {}: {}. Retrying...",
                attempts,
                provider_ref.source_name(),
                e
              );
              tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay * attempts as u64))
                .await;
            }
          }
        };

        let provider_time = provider_start.elapsed().as_millis() as u64;

        if let Some(ref pb) = progress {
          pb.inc(1);
        }

        // Add rate limiting delay
        tokio::time::sleep(tokio::time::Duration::from_millis(provider_ref.rate_limit_delay()))
          .await;

        (source, result, provider_time)
      };

      futures.push(future);
    }

    // Execute all provider requests concurrently
    let results = join_all(futures).await;

    // Process results
    let mut total_loaded = 0;
    let mut total_failed = 0;

    for (source, result, response_time) in results {
      match result {
        Ok(symbols) => {
          info!("Successfully loaded {} symbols from {}", symbols.len(), source);
          total_loaded += symbols.len();
          all_symbols.extend(symbols);

          source_results.insert(
            source,
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
            source,
            SourceResult {
              symbols_fetched: 0,
              errors: vec![e.to_string()],
              rate_limited,
              response_time_ms: response_time,
            },
          );
        }
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

    Ok(CryptoLoaderResult {
      symbols_loaded: final_count,
      symbols_failed: total_failed,
      symbols_skipped: total_loaded - final_count, // Duplicates removed
      source_results,
      processing_time_ms: processing_time,
    })
  }

  /// Remove duplicate symbols, preferring symbols from sources in priority order
  fn deduplicate_symbols(&self, symbols: Vec<CryptoSymbol>) -> Vec<CryptoSymbol> {
    let mut unique_symbols = HashMap::new();

    // Define source priority (higher number = higher priority)
    let source_priority = |source: &CryptoDataSource| -> u8 {
      match source {
        CryptoDataSource::CoinGecko => 4,
        CryptoDataSource::CoinPaprika => 3,
        CryptoDataSource::CoinCap => 2,
        CryptoDataSource::SosoValue => 1,
      }
    };

    for symbol in symbols {
      let key = symbol.symbol.clone();

      match unique_symbols.get(&key) {
        Some(existing) => {
          // Keep the one from higher priority source
          if source_priority(&symbol.source) > source_priority(&existing.source) {
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
      info!("Loading symbols from single source: {}", source);
      provider.fetch_symbols(&self.client).await
    } else {
      Err(CryptoLoaderError::SourceUnavailable(source.to_string()))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio_test;

  #[tokio_test::tokio::test]
  async fn test_crypto_loader_creation() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoSymbolLoader::new(config);
    assert_eq!(loader.providers.len(), 4);
  }

  #[tokio_test::tokio::test]
  async fn test_deduplication() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoSymbolLoader::new(config);

    let symbols = vec![
      CryptoSymbol {
        symbol: "BTC".to_string(),
        name: "Bitcoin".to_string(),
        base_currency: None,
        quote_currency: None,
        market_cap_rank: Some(1),
        source: CryptoDataSource::CoinGecko,
        source_id: "bitcoin".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        additional_data: HashMap::new(),
      },
      CryptoSymbol {
        symbol: "BTC".to_string(),
        name: "Bitcoin".to_string(),
        base_currency: None,
        quote_currency: None,
        market_cap_rank: Some(1),
        source: CryptoDataSource::CoinPaprika,
        source_id: "btc-bitcoin".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        additional_data: HashMap::new(),
      },
    ];

    let unique = loader.deduplicate_symbols(symbols);
    assert_eq!(unique.len(), 1);
    assert_eq!(unique[0].source, CryptoDataSource::CoinGecko); // Higher priority
  }
}
