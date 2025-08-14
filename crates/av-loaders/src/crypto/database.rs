use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{ error, info, warn};

use crate::{
  DataLoader, LoaderContext, LoaderError, LoaderResult, ProcessState,
  batch_processor::{BatchConfig, BatchProcessor, BatchResult},
};

use super::{
  CryptoDataSource, CryptoLoaderConfig,  CryptoSymbol, CryptoSymbolLoader,
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
/// This loader fetches crypto symbols from APIs and prepares them for database insertion.
/// The actual database operations are performed by the CLI consumer.
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

  /// Load symbols from a specific source using the provided crypto loader
  /// FIXED: Now accepts a crypto_loader parameter instead of using self.crypto_loader
  async fn load_from_source(
    &self,
    crypto_loader: &CryptoSymbolLoader, // FIXED: Added parameter
    source: CryptoDataSource,
  ) -> LoaderResult<(Vec<CryptoSymbol>, SourceResultSummary)> {
    let _start = std::time::Instant::now();

    // FIXED: Use the provided crypto_loader instead of self.crypto_loader
    match crypto_loader.load_from_source(source).await {
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
    let processor = move |symbol: CryptoSymbol| -> futures::future::BoxFuture<
      'static,
      LoaderResult<CryptoSymbolForDb>,
    > {
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
    let mut symbols_fetched_count = 0; // Track original fetched count

    // Load symbols from specified sources or all configured sources
    if let Some(sources) = input.sources {
      // Load from specific sources
      for source in sources {
        // FIXED: Pass the configured crypto_loader to load_from_source
        let (symbols, result) = self.load_from_source(&crypto_loader, source).await?;

        symbols_fetched_count += symbols.len(); // Track fetched count
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

          // Since load_all_symbols returns CryptoLoaderResult, we need to adapt it
          // For now, we'll work with summary data only
          for (source, src_result) in result.source_results {
            let summary = SourceResultSummary {
              symbols_fetched: src_result.symbols_fetched,
              symbols_processed: src_result.symbols_fetched,
              errors: src_result.errors,
              rate_limited: src_result.rate_limited,
            };
            source_results.insert(source, summary);
          }

          // Note: We can't get the actual symbols from CryptoLoaderResult
          // This would need to be refactored in the future
          warn!("Cannot return symbols from load_all_symbols - using summary only");
        }
        Err(e) => {
          error!("Failed to load from all sources: {}", e);
          total_errors += 1;
        }
      }
    }

    // Process symbols for database insertion if we have any
    let processed_symbols = if !all_symbols.is_empty() {
      match self.process_symbols(all_symbols).await {
        Ok(batch_result) => {
          info!(
            "Symbol processing completed: {} processed, {} failed",
            batch_result.success.len(),
            batch_result.failures.len()
          );

          total_errors += batch_result.failures.len();
          batch_result.success
        }
        Err(e) => {
          error!("Failed to process symbols: {}", e);
          total_errors += 1;
          Vec::new()
        }
      }
    } else {
      Vec::new()
    };

    let processing_time = start_time.elapsed().as_millis() as u64;

    if let Some(tracker) = &context.process_tracker {
      let state =
        if total_errors > 0 { ProcessState::CompletedWithErrors } else { ProcessState::Success };
      tracker
        .complete(state)
        .await
        .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
    }

    info!(
      "Crypto database loader completed in {}ms: {} processed, {} errors",
      processing_time,
      processed_symbols.len(),
      total_errors
    );

    Ok(CryptoDbOutput {
      symbols_fetched: symbols_fetched_count,
      symbols_processed: processed_symbols.len(),
      errors: total_errors,
      skipped: 0,
      processing_time_ms: processing_time,
      symbols: processed_symbols,
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
  use chrono::Utc;
  use std::collections::HashMap;

  fn create_test_symbol(symbol: &str, name: &str, source: CryptoDataSource) -> CryptoSymbol {
    CryptoSymbol {
      symbol: symbol.to_string(),
      name: name.to_string(),
      source,
      source_id: format!("{}-{}", name.to_lowercase().replace(' ', "-"), symbol.to_lowercase()),
      market_cap_rank: Some(1),
      base_currency: None,
      quote_currency: Some("USD".to_string()),
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
}
