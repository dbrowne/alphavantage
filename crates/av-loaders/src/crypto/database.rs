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

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::{
  DataLoader, LoaderContext, LoaderError, LoaderResult, ProcessState,
  batch_processor::{BatchConfig, BatchProcessor, BatchResult},
};

use super::{CryptoDataSource, CryptoLoaderConfig, CryptoSymbol, CryptoSymbolLoader};

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
  pub priority: i32,
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
      priority: symbol.priority,
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
  async fn load_from_source(
    &self,
    crypto_loader: &CryptoSymbolLoader,
    source: CryptoDataSource,
  ) -> LoaderResult<(Vec<CryptoSymbol>, SourceResultSummary)> {
    let _start = std::time::Instant::now();

    // Use the provided crypto_loader instead of self.crypto_loader
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

    // Convert all symbols to database format
    let mut processed_tokens = self.save_all_crypto_tokens(symbols).await?;

    // Assign priorities based purely on market data
    self.assign_token_priority(&mut processed_tokens);

    let total_count = processed_tokens.len();

    info!("Priority assignment completed for {} tokens", total_count);

    // Return as BatchResult for compatibility with existing code
    Ok(BatchResult {
      success: processed_tokens,
      failures: Vec::new(),
      total_processed: total_count,
    })
  }

  /// Save all crypto tokens without aggressive deduplication
  /// Allow multiple tokens with same trading symbol but different source IDs
  ///

  async fn save_all_crypto_tokens(
    &self,
    symbols: Vec<CryptoSymbol>,
  ) -> LoaderResult<Vec<CryptoSymbolForDb>> {
    let mut processed_symbols = Vec::new();

    for token in symbols {
      // Create a unique composite key using symbol + source + source_id
      // This allows multiple SOL tokens to coexist
      let token_symbol = token.symbol.clone(); // Clone for error message
      match self.process_individual_token(token).await {
        Ok(processed_token) => {
          processed_symbols.push(processed_token);
        }
        Err(e) => {
          warn!("Failed to process token {}: {}", token_symbol, e);
          // Continue processing other tokens
        }
      }
    }

    Ok(processed_symbols)
  }

  /// Find existing token by composite key (symbol + source + source_id)
  async fn find_existing_token(
    &self,
    symbol: &str,
    source: &CryptoDataSource,
    source_id: &str,
  ) -> LoaderResult<Option<CryptoSymbolForDb>> {
    // This function would check if the exact same token already exists
    // by looking for the combination of trading symbol + source + source_id
    //
    // Note: This function returns Option<CryptoSymbolForDb> for now
    // In a real implementation, this would query the database through
    // the crypto_api_map table to find existing mappings

    info!("Checking for existing token: {} from {} with source_id: {}", symbol, source, source_id);

    // For now, return None (no existing token found)
    // The CLI layer will handle the actual database queries
    Ok(None)
  }

  /// Process individual token and prepare for database storage
  async fn process_individual_token(&self, token: CryptoSymbol) -> LoaderResult<CryptoSymbolForDb> {
    // Validate token data
    if token.symbol.is_empty() || token.name.is_empty() {
      return Err(LoaderError::InvalidData(format!(
        "Invalid token data: symbol='{}', name='{}'",
        token.symbol, token.name
      )));
    }

    if token.symbol.len() > 20 {
      return Err(LoaderError::InvalidData(format!(
        "Trading symbol too long: {} (max 20 chars)",
        token.symbol
      )));
    }

    if token.name.len() > 255 {
      warn!("Token name too long for {}, will be truncated", token.symbol);
    }

    // Convert to database format
    let db_token = CryptoSymbolForDb::from(token.clone());

    info!(
      "Processed token: {} '{}' from {} (source_id: {})",
      db_token.symbol, db_token.name, db_token.source, db_token.source_id
    );

    Ok(db_token)
  }

  /// Modified process_symbols method to use new multi-token approach
  async fn process_symbols_multi_token(
    &self,
    symbols: Vec<CryptoSymbol>,
  ) -> LoaderResult<BatchResult<CryptoSymbolForDb>> {
    info!("Processing {} crypto symbols with multi-token support", symbols.len());

    // Process all tokens without aggressive deduplication
    let processed_tokens = self.save_all_crypto_tokens(symbols).await?;

    // Return as BatchResult for compatibility with existing code
    let total_count = processed_tokens.len();
    Ok(BatchResult {
      success: processed_tokens,
      failures: Vec::new(), // We handled errors above by continuing on failure
      total_processed: total_count,
    })
  }

  /// Assign priority to tokens based purely on market cap rank - no hardcoded names
  fn assign_token_priority(&self, tokens: &mut [CryptoSymbolForDb]) {
    // Group tokens by trading symbol
    let mut symbol_groups: HashMap<String, Vec<&mut CryptoSymbolForDb>> = HashMap::new();

    for token in tokens.iter_mut() {
      symbol_groups.entry(token.symbol.clone()).or_insert_with(Vec::new).push(token);
    }

    // Process each symbol group
    for (symbol, mut token_group) in symbol_groups {
      self.assign_priorities_to_token_group(&symbol, &mut token_group);
    }
  }

  /// Assign priorities to a group of tokens sharing the same symbol
  /// Uses purely objective criteria: market cap rank
  fn assign_priorities_to_token_group(&self, symbol: &str, tokens: &mut [&mut CryptoSymbolForDb]) {
    info!("Assigning priorities for {} tokens with symbol '{}'", tokens.len(), symbol);

    if tokens.len() == 1 {
      // Single token - use its market cap rank or default
      let token = &mut tokens[0];
      token.priority = token.market_cap_rank.map(|r| r as i32).unwrap_or(9999999);
      return;
    }

    // Multiple tokens - sort by market cap rank (lower rank = higher priority)
    tokens.sort_by_key(|token| token.market_cap_rank.unwrap_or(9999999));

    for (index, token) in tokens.iter_mut().enumerate() {
      if index == 0 && token.market_cap_rank.is_some() {
        // Best market cap rank gets its actual rank as priority (primary)
        token.priority = token.market_cap_rank.map(|r| r as i32).unwrap_or(9999999);
        info!(
          "Made '{}' primary for '{}' with market cap rank {:?}",
          token.name, symbol, token.market_cap_rank
        );
      } else {
        // Non-primary tokens get default priority
        token.priority = 9999999;
      }
      debug!("Assigned priority {} to '{}' for symbol '{}'", token.priority, token.name, symbol);
    }
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
    // IMPORTANT: We do NOT deduplicate here because we want to create
    // symbol_mappings entries for EACH source, even if the symbol exists
    if let Some(sources) = input.sources {
      // Load from specific sources
      for source in sources {
        let (symbols, result) = self.load_from_source(&crypto_loader, source).await?;

        symbols_fetched_count += symbols.len(); // Track fetched count
        if !result.errors.is_empty() {
          total_errors += result.errors.len();
        }

        // Add ALL symbols from this source (do not deduplicate yet)
        all_symbols.extend(symbols);
        source_results.insert(source, result);
      }
    } else {
      // Load from all configured sources using the corrected method
      match crypto_loader.load_all_symbols().await {
        Ok(result) => {
          info!("Loaded symbols from all sources: {} symbols", result.symbols_loaded);

          symbols_fetched_count = result.symbols_loaded;
          all_symbols = result.symbols;

          // Convert SourceResult to SourceResultSummary
          for (source, src_result) in result.source_results {
            let summary = SourceResultSummary {
              symbols_fetched: src_result.symbols_fetched,
              symbols_processed: src_result.symbols_fetched,
              errors: src_result.errors,
              rate_limited: src_result.rate_limited,
            };
            source_results.insert(source, summary);
          }

          if result.symbols_failed > 0 {
            total_errors += result.symbols_failed;
          }
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
      priority: 0,
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
