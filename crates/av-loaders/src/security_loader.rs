//! Security loader that reads symbols from CSV files and fetches company data from AlphaVantage API

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use av_models::fundamentals::CompanyOverview;
use crate::{
  DataLoader, LoaderContext, LoaderResult, LoaderError,
  csv_processor::CsvProcessor,
  process_tracker::ProcessState,
};

pub struct SecurityLoader {
  semaphore: Arc<Semaphore>,
}

impl SecurityLoader {
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
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
    info!("Loading securities from {:?}", input.file_path);

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

          async move {
            let _permit = semaphore.acquire().await.unwrap();

            let result = client
                .fundamentals()
                .company_overview(&symbol)
                .await
                .map(|overview| SecurityData {
                  symbol: symbol.clone(),
                  exchange: exchange.clone(),
                  overview,
                });

            if let Some(pb) = &progress {
              pb.inc(1);
            }

            // Add delay to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;

            result
          }
        })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await;

    if let Some(pb) = progress_for_finish {
      pb.finish_with_message("Security loading complete");
    }

    // Process results - no database operations
    let mut loaded = Vec::new();
    let mut errors = 0;

    for result in results {
      match result {
        Ok(data) => {
          if data.overview.symbol.is_empty() || data.overview.symbol == "None" {
            warn!("Skipping symbol {} - no data returned", data.symbol);
          } else {
            loaded.push(data);
          }
        }
        Err(e) => {
          warn!("Error fetching company overview: {}", e);
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

    let total_symbols = loaded.len() + errors;

    Ok(SecurityLoaderOutput {
      total_symbols,
      loaded_count: loaded.len(),
      errors,
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
  pub symbol: String,
  pub exchange: String,
  pub overview: CompanyOverview,
}

#[derive(Debug)]
pub struct SecurityLoaderOutput {
  pub total_symbols: usize,
  pub loaded_count: usize,
  pub errors: usize,
  pub data: Vec<SecurityData>,
}