//! Security loader that reads symbols from CSV files and fetches company data from AlphaVantage API
//!
//! This loader:
//! 1. Reads a list of symbols from a CSV file (e.g., nasdaq-listed.csv)
//! 2. For each symbol, queries the AlphaVantage Company Overview API
//! 3. Stores the retrieved company data in the database
//!
//! The CSV data itself is not persisted - it's only used as a source of symbols to query.

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use av_client::endpoints::CompanyOverview;
use av_database::models::{NewSymbol, Symbol};
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

        let progress = if context.config.show_progress {
            Some(ProgressBar::new(symbols.len() as u64))
        } else {
            None
        };

        // Query AlphaVantage API for each symbol
        let results = stream::iter(symbols.iter())
            .map(|symbol| {
                let client = context.client.clone();
                let semaphore = self.semaphore.clone();
                let progress = progress.clone();
                let exchange = input.exchange.clone();

                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    let result = client
                        .company_overview(symbol)
                        .await
                        .map(|overview| (symbol.clone(), exchange, overview));

                    if let Some(pb) = &progress {
                        pb.inc(1);
                    }

                    // Add delay to respect rate limits
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        context.config.retry_delay_ms
                    )).await;

                    result
                }
            })
            .buffer_unordered(context.config.max_concurrent_requests)
            .collect::<Vec<_>>()
            .await;

        if let Some(pb) = progress {
            pb.finish_with_message("Security loading complete");
        }

        // Process results and save to database
        let mut conn = context.db_pool.get().await?;
        let mut inserted = 0;
        let mut skipped = 0;
        let mut errors = 0;

        for result in results {
            match result {
                Ok((symbol, exchange, overview)) => {
                    match self.save_security(&mut conn, symbol, exchange, overview).await {
                        Ok(true) => inserted += 1,
                        Ok(false) => skipped += 1,
                        Err(e) => {
                            warn!("Error saving security {}: {}", symbol, e);
                            errors += 1;
                        }
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

        Ok(SecurityLoaderOutput {
            total_symbols: symbols.len(),
            inserted,
            skipped,
            errors,
        })
    }

    fn name(&self) -> &'static str {
        "SecurityLoader"
    }
}

impl SecurityLoader {
    async fn save_security(
        &self,
        conn: &mut av_database::DbConnection,
        symbol: String,
        exchange: String,
        overview: CompanyOverview,
    ) -> LoaderResult<bool> {
        // Skip if no valid data returned
        if overview.symbol.is_empty() || overview.symbol == "None" {
            return Ok(false);
        }

        // Create new symbol from API data
        let new_symbol = NewSymbol {
            symbol: overview.symbol.clone(),
            name: overview.name.clone(),
            sec_type: overview.asset_type.clone(),
            region: overview.country.clone(),
            market_open: "09:30:00".parse().unwrap(), // Default NYSE hours
            market_close: "16:00:00".parse().unwrap(),
            timezone: overview.exchange_timezone.clone().unwrap_or_else(|| "US/Eastern".to_string()),
            currency: overview.currency.clone(),
            exchange: overview.exchange.clone().unwrap_or(exchange),
            overview: false, // Will be set to true when overview is loaded
            intraday: false,
            summary: false,
        };

        // Insert or update symbol
        match new_symbol.upsert(conn).await {
            Ok(_) => Ok(true),
            Err(e) => Err(LoaderError::DatabaseError(e)),
        }
    }
}

#[derive(Debug)]
pub struct SecurityLoaderInput {
    pub file_path: String,
    pub exchange: String,
}

#[derive(Debug)]
pub struct SecurityLoaderOutput {
    pub total_symbols: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub errors: usize,
}