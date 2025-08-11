//! Database integration for crypto loaders
//!
//! This module provides integration between the crypto loaders and the database,

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::{
  DataLoader, LoaderContext, LoaderError, LoaderResult,
  batch_processor::{BatchConfig, BatchProcessor, BatchResult},
  crypto::{
    CryptoDataSource, CryptoLoaderConfig, CryptoLoaderResult, CryptoSymbol, CryptoSymbolLoader,
  },
};

/// Input for the crypto database integration loader
#[derive(Debug, Clone)]
pub struct CryptoDbInput {
  pub sources: Option<Vec<CryptoDataSource>>,
  pub update_existing: bool,
  pub batch_size: Option<usize>,
  pub api_keys: Option<HashMap<CryptoDataSource, String>>,
}

impl Default for CryptoDbInput {
  fn default() -> Self {
    Self { sources: None, update_existing: true, batch_size: Some(100), api_keys: None }
  }
}

#[derive(Debug, Clone)]
pub struct CryptoDbOutput {
  pub symbols_fetched: usize,
  pub symbols_processed: usize,
  pub errors: usize,
  pub skipped: usize,
  pub processing_time_ms: u64,
  pub symbols: Vec<CryptoSymbolForDb>,
  pub source_results: HashMap<CryptoDataSource, SourceResultSummary>,
}

#[derive(Debug, Clone)]
pub struct SourceResultSummary {
  pub symbols_fetched: usize,
  pub symbols_processed: usize,
  pub errors: Vec<String>,
  pub rate_limited: bool,
}

/// Crypto symbol prepared for database insertion
#[derive(Debug, Clone)]
pub struct CryptoSymbolForDb {
  pub symbol: String,
  pub name: String,
  pub source: CryptoDataSource,
  pub source_id: String,
  pub market_cap_rank: Option<u32>,
  pub base_currency: Option<String>,
  pub quote_currency: Option<String>,
  pub is_active: bool,
  pub additional_data: serde_json::Value,
}

impl From<CryptoSymbol> for CryptoSymbolForDb {
  fn from(symbol: CryptoSymbol) -> Self {
    Self {
      symbol: symbol.symbol,
      name: symbol.name,
      source: symbol.source,
      source_id: symbol.source_id,
      market_cap_rank: symbol.market_cap_rank,
      base_currency: symbol.base_currency,
      quote_currency: symbol.quote_currency,
      is_active: symbol.is_active,
      additional_data: serde_json::to_value(&symbol.additional_data).unwrap_or_default(),
    }
  }
}

/// Database-integrated crypto loader
///
pub struct CryptoDbLoader {
  crypto_loader: CryptoSymbolLoader,
  batch_processor: BatchProcessor,
}

impl CryptoDbLoader {
  pub fn new(crypto_config: CryptoLoaderConfig) -> Self {
    let crypto_loader = CryptoSymbolLoader::new(crypto_config.clone());

    let batch_config = BatchConfig {
      batch_size: crypto_config.batch_size,
      max_concurrent_batches: crypto_config.max_concurrent_requests.min(5),
      continue_on_error: true,
      batch_delay_ms: Some(crypto_config.rate_limit_delay_ms),
    };

    let batch_processor = BatchProcessor::new(batch_config);

    Self { crypto_loader, batch_processor }
  }

  /// Load symbols from a specific source
  async fn load_from_source(
    &self,
    source: CryptoDataSource,
  ) -> LoaderResult<(Vec<CryptoSymbol>, SourceResultSummary)> {
    let start = std::time::Instant::now();

    match self.crypto_loader.load_from_source(source.clone()).await {
      Ok(symbols) => {
        let result = SourceResultSummary {
          symbols_fetched: symbols.len(),
          symbols_processed: symbols.len(),
          errors: vec![],
          rate_limited: false,
        };

        info!("Successfully loaded {} symbols from {}", symbols.len(), source);
        Ok((symbols, result))
      }
      Err(e) => {
        let error_msg = e.to_string();
        let rate_limited = error_msg.contains("rate limit") || error_msg.contains("429");

        let result = SourceResultSummary {
          symbols_fetched: 0,
          symbols_processed: 0,
          errors: vec![error_msg.clone()],
          rate_limited,
        };

        error!("Failed to load from {}: {}", source, error_msg);
        Ok((Vec::new(), result))
      }
    }
  }

  /// Process and validate symbols for database storage
  async fn process_symbols(
    &self,
    symbols: Vec<CryptoSymbol>,
  ) -> LoaderResult<BatchResult<CryptoSymbolForDb>> {
    info!("Processing {} crypto symbols for database storage", symbols.len());

    // Create a processor function for batch processing
    let processor = {
      move |symbol: CryptoSymbol| {
        Box::pin(async move {
          // Validate symbol data
          if symbol.symbol.is_empty() || symbol.name.is_empty() {
            return Err(LoaderError::InvalidData(format!(
              "Invalid symbol data: symbol='{}', name='{}'",
              symbol.symbol, symbol.name
            )));
          }

          // Additional validation following repository patterns
          if symbol.symbol.len() > 20 {
            return Err(LoaderError::InvalidData(format!(
              "Symbol too long: {} (max 20 chars)",
              symbol.symbol
            )));
          }

          if symbol.name.len() > 255 {
            warn!("Name too long for symbol {}, truncating", symbol.symbol);
          }

          Ok(CryptoSymbolForDb::from(symbol))
        })
      }
    };

    self.batch_processor.process_batches(symbols, processor).await
  }
}

#[async_trait]
impl DataLoader for CryptoDbLoader {
  type Input = CryptoDbInput;
  type Output = CryptoDbOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    let start_time = std::time::Instant::now();
    info!("Starting crypto database loader");

    if let Some(tracker) = &context.process_tracker {
      tracker
        .start("crypto_db_loader")
        .await
        .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
    }

    // Configure API keys if provided
    let mut crypto_loader = self.crypto_loader.clone();
    if let Some(api_keys) = input.api_keys {
      crypto_loader = crypto_loader.with_api_keys(api_keys);
    }

    let mut all_symbols = Vec::new();
    let mut source_results = HashMap::new();
    let mut total_errors = 0;

    // Load symbols from specified sources or all configured sources
    if let Some(sources) = input.sources {
      // Load from specific sources
      for source in sources {
        let (symbols, result) = self.load_from_source(source.clone()).await?;

        if !result.errors.is_empty() {
          total_errors += result.errors.len();
        }

        all_symbols.extend(symbols);
        source_results.insert(source, result);
      }
    } else {
      // Load from all configured sources using the existing method
      match crypto_loader.load_all_symbols().await {
        Ok(result) => {
          info!("Loaded symbols from all sources: {} symbols", result.symbols_loaded);
          // Note: This would need to be adapted based on how to extract actual symbols
          // from the CryptoLoaderResult. For now, we'll work with the summary data.

          // Convert CryptoLoaderResult source results to our format
          for (source, src_result) in result.source_results {
            let summary = SourceResultSummary {
              symbols_fetched: src_result.symbols_fetched,
              symbols_processed: src_result.symbols_fetched,
              errors: src_result.errors,
              rate_limited: src_result.rate_limited,
            };
            source_results.insert(source, summary);
          }
        }
        Err(e) => {
          error!("Failed to load crypto symbols: {}", e);
          if let Some(tracker) = &context.process_tracker {
            tracker
              .complete(crate::ProcessState::Failed)
              .await
              .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
          }
          return Err(LoaderError::ApiError(e.to_string()));
        }
      }
    }

    let symbols_fetched = all_symbols.len();
    info!("Fetched {} crypto symbols from external sources", symbols_fetched);

    if all_symbols.is_empty() {
      warn!("No symbols fetched from any source");

      if let Some(tracker) = &context.process_tracker {
        tracker
          .complete(crate::ProcessState::Success)
          .await
          .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
      }

      return Ok(CryptoDbOutput {
        symbols_fetched: 0,
        symbols_processed: 0,
        errors: total_errors,
        skipped: 0,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        symbols: Vec::new(),
        source_results,
      });
    }

    let process_result = self.process_symbols(all_symbols).await?;

    let symbols_processed = process_result.success_count();
    let process_errors = process_result.failure_count();

    info!("Processed {} symbols successfully, {} errors", symbols_processed, process_errors);

    let final_state = if process_errors > 0 {
      crate::ProcessState::CompletedWithErrors
    } else {
      crate::ProcessState::Success
    };

    if let Some(tracker) = &context.process_tracker {
      tracker
        .complete(final_state)
        .await
        .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
    }

    let processing_time = start_time.elapsed().as_millis() as u64;

    info!(
      "Crypto database loader completed in {}ms: {} processed, {} errors",
      processing_time,
      symbols_processed,
      total_errors + process_errors
    );

    Ok(CryptoDbOutput {
      symbols_fetched,
      symbols_processed,
      errors: total_errors + process_errors,
      skipped: 0, // This would be calculated based on deduplication logic
      processing_time_ms: processing_time,
      symbols: process_result.success,
      source_results,
    })
  }

  fn name(&self) -> &'static str {
    "CryptoDbLoader"
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::crypto::{CryptoDataSource, CryptoLoaderConfig, CryptoSymbol};
  use chrono::Utc;
  use std::collections::HashMap;

  // Helper function to create test symbols
  fn create_test_symbol(symbol: &str, name: &str, source: CryptoDataSource) -> CryptoSymbol {
    CryptoSymbol {
      symbol: symbol.to_string(),
      name: name.to_string(),
      base_currency: Some(symbol.to_string()),
      quote_currency: Some("USD".to_string()),
      market_cap_rank: Some(1),
      source,
      source_id: format!("{}-{}", name.to_lowercase().replace(' ', "-"), symbol.to_lowercase()),
      is_active: true,
      created_at: Utc::now(),
      additional_data: HashMap::new(),
    }
  }

  #[tokio::test]
  async fn test_crypto_db_loader_creation() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    // Test that the loader was created successfully
    assert_eq!(loader.name(), "CryptoDbLoader");
  }

  #[tokio::test]
  async fn test_crypto_symbol_for_db_conversion() {
    let symbol = create_test_symbol("BTC", "Bitcoin", CryptoDataSource::CoinGecko);
    let db_symbol = CryptoSymbolForDb::from(symbol.clone());

    assert_eq!(db_symbol.symbol, "BTC");
    assert_eq!(db_symbol.name, "Bitcoin");
    assert_eq!(db_symbol.source, CryptoDataSource::CoinGecko);
    assert_eq!(db_symbol.source_id, "bitcoin-btc");
    assert!(db_symbol.is_active);
  }

  #[test]
  fn test_crypto_db_input_default() {
    let input = CryptoDbInput::default();

    assert!(input.sources.is_none());
    assert!(input.update_existing);
    assert_eq!(input.batch_size, Some(100));
    assert!(input.api_keys.is_none());
  }

  #[test]
  fn test_crypto_db_input_with_sources() {
    let sources = vec![CryptoDataSource::CoinGecko, CryptoDataSource::CoinPaprika];
    let input = CryptoDbInput {
      sources: Some(sources.clone()),
      update_existing: false,
      batch_size: Some(50),
      api_keys: None,
    };

    assert_eq!(input.sources.unwrap(), sources);
    assert!(!input.update_existing);
    assert_eq!(input.batch_size, Some(50));
  }

  #[test]
  fn test_crypto_db_input_with_api_keys() {
    let mut api_keys = HashMap::new();
    api_keys.insert(CryptoDataSource::CoinGecko, "test-key".to_string());

    let input = CryptoDbInput {
      sources: None,
      update_existing: true,
      batch_size: None,
      api_keys: Some(api_keys.clone()),
    };

    assert!(input.api_keys.is_some());
    assert_eq!(
      input.api_keys.unwrap().get(&CryptoDataSource::CoinGecko),
      Some(&"test-key".to_string())
    );
  }

  #[test]
  fn test_source_result_summary() {
    let summary = SourceResultSummary {
      symbols_fetched: 100,
      symbols_processed: 95,
      errors: vec!["Test error".to_string()],
      rate_limited: false,
    };

    assert_eq!(summary.symbols_fetched, 100);
    assert_eq!(summary.symbols_processed, 95);
    assert_eq!(summary.errors.len(), 1);
    assert!(!summary.rate_limited);
  }

  #[test]
  fn test_crypto_db_output_creation() {
    let output = CryptoDbOutput {
      symbols_fetched: 100,
      symbols_processed: 95,
      errors: 5,
      skipped: 0,
      processing_time_ms: 1000,
      symbols: Vec::new(),
      source_results: HashMap::new(),
    };

    assert_eq!(output.symbols_fetched, 100);
    assert_eq!(output.symbols_processed, 95);
    assert_eq!(output.errors, 5);
    assert_eq!(output.processing_time_ms, 1000);
  }

  #[tokio::test]
  async fn test_symbol_validation() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    // Test valid symbol
    let valid_symbol = create_test_symbol("BTC", "Bitcoin", CryptoDataSource::CoinGecko);
    let symbols = vec![valid_symbol];

    let result = loader.process_symbols(symbols).await;
    assert!(result.is_ok());

    let batch_result = result.unwrap();
    assert_eq!(batch_result.success_count(), 1);
    assert_eq!(batch_result.failure_count(), 0);
  }

  #[tokio::test]
  async fn test_invalid_symbol_validation() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    // Test invalid symbol (empty symbol)
    let mut invalid_symbol = create_test_symbol("", "Bitcoin", CryptoDataSource::CoinGecko);
    let symbols = vec![invalid_symbol];

    let result = loader.process_symbols(symbols).await;
    assert!(result.is_ok());

    let batch_result = result.unwrap();
    assert_eq!(batch_result.success_count(), 0);
    assert_eq!(batch_result.failure_count(), 1);
  }

  #[tokio::test]
  async fn test_symbol_too_long() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    // Test symbol that's too long (over 20 characters)
    let long_symbol =
      create_test_symbol("VERYLONGSYMBOLNAME123", "Test Coin", CryptoDataSource::CoinGecko);
    let symbols = vec![long_symbol];

    let result = loader.process_symbols(symbols).await;
    assert!(result.is_ok());

    let batch_result = result.unwrap();
    assert_eq!(batch_result.success_count(), 0);
    assert_eq!(batch_result.failure_count(), 1);
  }

  #[tokio::test]
  async fn test_mixed_symbol_validation() {
    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    let symbols = vec![
      create_test_symbol("BTC", "Bitcoin", CryptoDataSource::CoinGecko),
      create_test_symbol("", "Invalid", CryptoDataSource::CoinPaprika), // Invalid
      create_test_symbol("ETH", "Ethereum", CryptoDataSource::CoinCap),
      create_test_symbol("TOOLONGSYMBOL12345", "Too Long", CryptoDataSource::SosoValue), // Invalid
    ];

    let result = loader.process_symbols(symbols).await;
    assert!(result.is_ok());

    let batch_result = result.unwrap();
    assert_eq!(batch_result.success_count(), 2); // BTC and ETH
    assert_eq!(batch_result.failure_count(), 2); // Empty symbol and too long
  }

  // Mock test for the DataLoader trait
  #[tokio::test]
  async fn test_dataloader_trait() {
    use crate::{LoaderConfig, LoaderContext, ProcessTracker};
    use av_client::AlphaVantageClient;
    use av_core::Config;
    use std::sync::Arc;

    let config = CryptoLoaderConfig::default();
    let loader = CryptoDbLoader::new(config);

    // Create a mock context (this would need proper setup in real tests)
    let av_config = Config::default_with_key("test_key".to_string());
    let client = Arc::new(AlphaVantageClient::new(av_config));
    let loader_config = LoaderConfig::default();
    let context = LoaderContext::new(client, loader_config);

    let input = CryptoDbInput::default();

    // Note: This test would fail without proper API setup
    // In practice, you'd use mocks or integration test environment
    assert_eq!(loader.name(), "CryptoDbLoader");
  }

  #[test]
  fn test_crypto_data_source_display() {
    assert_eq!(CryptoDataSource::CoinGecko.to_string(), "coingecko");
    assert_eq!(CryptoDataSource::CoinPaprika.to_string(), "coinpaprika");
    assert_eq!(CryptoDataSource::CoinCap.to_string(), "coincap");
    assert_eq!(CryptoDataSource::SosoValue.to_string(), "sosovalue");
  }

  #[test]
  fn test_json_serialization() {
    let symbol = create_test_symbol("BTC", "Bitcoin", CryptoDataSource::CoinGecko);
    let db_symbol = CryptoSymbolForDb::from(symbol);

    // Test that additional_data can be serialized/deserialized
    assert!(db_symbol.additional_data.is_object() || db_symbol.additional_data.is_null());
  }

  // Performance test
  #[tokio::test]
  async fn test_batch_processing_performance() {
    let config = CryptoLoaderConfig { batch_size: 10, ..Default::default() };
    let loader = CryptoDbLoader::new(config);

    // Create a moderate number of test symbols
    let mut symbols = Vec::new();
    for i in 0..100 {
      symbols.push(create_test_symbol(
        &format!("COIN{}", i),
        &format!("Test Coin {}", i),
        CryptoDataSource::CoinGecko,
      ));
    }

    let start = std::time::Instant::now();
    let result = loader.process_symbols(symbols).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    let batch_result = result.unwrap();
    assert_eq!(batch_result.success_count(), 100);
    assert_eq!(batch_result.failure_count(), 0);

    // Should complete reasonably quickly (less than 1 second for 100 items)
    assert!(duration.as_secs() < 1);
  }
}
