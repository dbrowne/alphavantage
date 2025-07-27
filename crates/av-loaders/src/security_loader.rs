//! Security loader that reads symbols from CSV files and searches for them via AlphaVantage API

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, debug, error};

use av_models::time_series::SymbolSearch;
use av_models::common::SymbolMatch;
use crate::{
  DataLoader, LoaderContext, LoaderResult, LoaderError,
  csv_processor::CsvProcessor,
  process_tracker::ProcessState,
};

/// Configuration for symbol matching behavior
#[derive(Debug, Clone)]
pub enum SymbolMatchMode {
  /// Only accept exact symbol matches (case-insensitive)
  ExactMatch,
  /// Accept all symbols returned from search
  AllMatches,
  /// Accept top N matches based on match score
  TopMatches(usize),
}

impl Default for SymbolMatchMode {
  fn default() -> Self {
    SymbolMatchMode::AllMatches
  }
}

pub struct SecurityLoader {
  semaphore: Arc<Semaphore>,
  match_mode: SymbolMatchMode,
}

impl SecurityLoader {
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      match_mode: SymbolMatchMode::default(),
    }
  }

  /// Set the symbol match mode
  pub fn with_match_mode(mut self, mode: SymbolMatchMode) -> Self {
    self.match_mode = mode;
    self
  }

  /// Get matching symbols based on the configured match mode
  fn get_matching_symbols(
    &self,
    search_query: &str,
    search_results: SymbolSearch,
  ) -> Vec<SymbolMatch> {
    match &self.match_mode {
      SymbolMatchMode::ExactMatch => {
        search_results.best_matches
            .into_iter()
            .filter(|m| m.symbol.eq_ignore_ascii_case(search_query))
            .collect()
      }
      SymbolMatchMode::AllMatches => search_results.best_matches,
      SymbolMatchMode::TopMatches(n) => {
        let mut matches = search_results.best_matches;
        // Sort by match score (descending)
        matches.sort_by(|a, b| {
          let score_a: f64 = a.match_score.parse().unwrap_or(0.0);
          let score_b: f64 = b.match_score.parse().unwrap_or(0.0);
          score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.into_iter().take(*n).collect()
      }
    }
  }
}

#[async_trait]
impl DataLoader for SecurityLoader {
  type Input = SecurityLoaderInput;
  type Output = SecurityLoaderOutput;

  async fn load(
    &self,
    context: &LoaderContext,
    input: Self::Input,
  ) -> LoaderResult<Self::Output> {
    info!("Loading securities from {:?} with match mode {:?}",
          input.file_path, self.match_mode);

    // Parse CSV file to get symbols
    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(&input.file_path)?;

    info!("Found {} symbols in CSV", symbols.len());

    // Track process if enabled
    if let Some(tracker) = &context.process_tracker {
      tracker.start("security_loader").await?;
    }

    // Use Arc for progress bar to share it across async tasks
    let progress = if context.config.show_progress {
      Some(Arc::new(ProgressBar::new(symbols.len() as u64)))
    } else {
      None
    };

    // Clone for use after the stream processing
    let progress_for_finish = progress.clone();

    // Create owned copies for the async closures
    let exchange = input.exchange.clone();
    let client_ref = context.client.clone();
    let retry_delay = context.config.retry_delay_ms;
    let max_concurrent = context.config.max_concurrent_requests;

    // Query AlphaVantage API for each symbol
    let results = stream::iter(symbols.into_iter())
        .map(move |symbol| {
          let client = client_ref.clone();
          let semaphore = self.semaphore.clone();
          let progress = progress.clone();
          let exchange = exchange.clone();
          let original_symbol = symbol.clone();

          async move {
            let _permit = semaphore.acquire().await.unwrap();

            // Search for the symbol
            let search_results = match client
                .time_series()
                .symbol_search(&symbol)
                .await
            {
              Ok(results) => results,
              Err(e) => {
                warn!("Symbol search failed for {}: {}", symbol, e);
                if let Some(pb) = &progress {
                  pb.inc(1);
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
                return Err(e);
              }
            };

            // Get matching symbols based on mode
            let matches = self.get_matching_symbols(&symbol, search_results);

            if matches.is_empty() {
              warn!("No matches found for symbol {}", symbol);
              if let Some(pb) = &progress {
                pb.inc(1);
              }
              tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
              return Ok(vec![]);
            }

            // Convert matches to SecurityData
            let mut security_data = Vec::new();

            for symbol_match in matches {
              // Validate the symbol match data from API
              if symbol_match.symbol.len() > 19 {
                error!("API PARSING ERROR: Received symbol '{}' with length {} from symbol search",
          symbol_match.symbol, symbol_match.symbol.len());
                error!("  API Response: {:?}", symbol_match);
                error!("  Original query was: '{}'", original_symbol);
                continue; // Skip this malformed result
              }
              debug!("Found match for {}: {} (score: {}, type: {}, region: {})",
                    original_symbol,
                    symbol_match.symbol,
                    symbol_match.match_score,
                    symbol_match.stock_type,
                    symbol_match.region);

              // Additional validation
              if symbol_match.symbol.is_empty() {
                warn!("Received empty symbol from API for query '{}'", original_symbol);
                continue;
              }
              security_data.push(SecurityData {
                symbol: symbol_match.symbol,
                name: symbol_match.name,
                stock_type: symbol_match.stock_type,
                region: symbol_match.region,
                market_open: symbol_match.market_open,
                market_close: symbol_match.market_close,
                timezone: symbol_match.timezone,
                currency: symbol_match.currency,
                exchange: exchange.clone(),
                match_score: symbol_match.match_score.parse::<f64>().ok(),
                original_query: Some(original_symbol.clone()),
              });
            }

            if let Some(pb) = &progress {
              pb.inc(1);
            }

            // Add delay to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;

            Ok(security_data)
          }
        })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await;

    if let Some(pb) = progress_for_finish {
      pb.finish_with_message("Security loading complete");
    }

    // Process results - flatten nested vectors
    let mut loaded = Vec::new();
    let mut errors = 0;
    let mut skipped = 0;

    for result in results {
      match result {
        Ok(data_vec) => {
          if data_vec.is_empty() {
            skipped += 1;
          } else {
            for data in data_vec {
              loaded.push(data);
            }
          }
        }
        Err(e) => {
          warn!("Error in security loading: {}", e);
          errors += 1;
        }
      }
    }

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      tracker.complete(if errors > 0 {
        ProcessState::CompletedWithErrors
      } else {
        ProcessState::Success
      }).await?;
    }

    let total_symbols = loaded.len() + errors + skipped;

    info!("Security loading complete: {} loaded, {} errors, {} skipped",
          loaded.len(), errors, skipped);

    Ok(SecurityLoaderOutput {
      total_symbols,
      loaded_count: loaded.len(),
      errors,
      skipped_count: skipped,
      duplicates_prevented: 0, // TODO: Implement duplicate tracking
      data: loaded,
    })
  }

  fn name(&self) -> &'static str {
    "SecurityLoader"
  }
}

#[derive(Debug)]
pub struct SecurityLoaderInput {
  pub file_path: String,
  pub exchange: String,
}

#[derive(Debug, Clone)]
pub struct SecurityData {
  /// Stock symbol
  pub symbol: String,
  /// Company name
  pub name: String,
  /// Stock type (e.g., "Equity", "ETF")
  pub stock_type: String,
  /// Region (e.g., "United States")
  pub region: String,
  /// Market open time
  pub market_open: String,
  /// Market close time
  pub market_close: String,
  /// Timezone
  pub timezone: String,
  /// Currency
  pub currency: String,
  /// Exchange (from input, not from API)
  pub exchange: String,
  /// Match score from symbol search (if available)
  pub match_score: Option<f64>,
  /// Original symbol queried (useful when match mode returns different symbols)
  pub original_query: Option<String>,
}

#[derive(Debug)]
pub struct SecurityLoaderOutput {
  pub total_symbols: usize,
  pub loaded_count: usize,
  pub errors: usize,
  pub skipped_count: usize,
  pub duplicates_prevented: usize,
  pub data: Vec<SecurityData>,
}