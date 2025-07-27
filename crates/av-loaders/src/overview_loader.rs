use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, error};

use av_models::fundamentals::CompanyOverview;
use crate::{
    DataLoader, LoaderContext, LoaderResult, LoaderError,
    process_tracker::ProcessState,
};

/// Loader for company overview fundamental data
pub struct OverviewLoader {
    semaphore: Arc<Semaphore>,
}

impl OverviewLoader {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
}

#[async_trait]
impl DataLoader for OverviewLoader {
    type Input = OverviewLoaderInput;
    type Output = OverviewLoaderOutput;

    async fn load(
        &self,
        context: &LoaderContext,
        input: Self::Input,
    ) -> LoaderResult<Self::Output> {
        info!("Loading overviews for {} symbols", input.symbols.len());

        // Track process if enabled
        if let Some(tracker) = &context.process_tracker {
            tracker.start("overview_loader").await?;
        }

        // Use Arc for progress bar to share it across async tasks
        let progress = if context.config.show_progress {
            Some(Arc::new(ProgressBar::new(input.symbols.len() as u64)))
        } else {
            None
        };

        let progress_for_finish = progress.clone();
        let client_ref = context.client.clone();
        let retry_delay = context.config.retry_delay_ms;
        let max_concurrent = context.config.max_concurrent_requests;

        // Process symbols concurrently
        let results = stream::iter(input.symbols.into_iter())
            .map(move |symbol_info| {
                let client = client_ref.clone();
                let semaphore = self.semaphore.clone();
                let progress = progress.clone();

                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    if let Some(pb) = &progress {
                        pb.set_message(format!("Processing {}", symbol_info.symbol));
                    }

                    // Query the API
                    let result = match client
                        .fundamentals()
                        .company_overview(&symbol_info.symbol)
                        .await
                    {
                        Ok(overview) => {
                            // Validate response
                            if overview.symbol.is_empty() || overview.symbol == "None" {
                                info!("No overview data available for {}", symbol_info.symbol);
                                Ok(None)
                            } else {
                                Ok(Some(OverviewData {
                                    sid: symbol_info.sid,
                                    symbol: symbol_info.symbol.clone(),
                                    overview,
                                }))
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("rate limit") {
                                warn!("Rate limit hit, waiting before retry...");
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                                // Retry once
                                match client
                                    .fundamentals()
                                    .company_overview(&symbol_info.symbol)
                                    .await
                                {
                                    Ok(overview) => {
                                        if overview.symbol.is_empty() || overview.symbol == "None" {
                                            Ok(None)
                                        } else {
                                            Ok(Some(OverviewData {
                                                sid: symbol_info.sid,
                                                symbol: symbol_info.symbol.clone(),
                                                overview,
                                            }))
                                        }
                                    }
                                    Err(retry_err) => {
                                        error!("Retry failed for {}: {}", symbol_info.symbol, retry_err);
                                        Err(retry_err)
                                    }
                                }
                            } else if e.to_string().contains("Invalid API call") {
                                // No data available
                                Ok(None)
                            } else {
                                Err(e)
                            }
                        }
                    };

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
            pb.finish_with_message("Overview loading complete");
        }

        // Process results
        let mut loaded = Vec::new();
        let mut errors = 0;
        let mut no_data = 0;

        for result in results {
            match result {
                Ok(Some(data)) => loaded.push(data),
                Ok(None) => no_data += 1,
                Err(e) => {
                    error!("Failed to load overview: {}", e);
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

        info!(
            "Overview loading complete: {} loaded, {} no data, {} errors",
            loaded.len(), no_data, errors
        );

        Ok(OverviewLoaderOutput {
            total_symbols: loaded.len() + no_data + errors,
            loaded_count: loaded.len(),
            no_data_count: no_data,
            errors,
            data: loaded,
        })
    }

    fn name(&self) -> &'static str {
        "OverviewLoader"
    }
}

#[derive(Debug)]
pub struct SymbolInfo {
    pub sid: i64,
    pub symbol: String,
}

#[derive(Debug)]
pub struct OverviewLoaderInput {
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug)]
pub struct OverviewData {
    pub sid: i64,
    pub symbol: String,
    pub overview: CompanyOverview,
}

#[derive(Debug)]
pub struct OverviewLoaderOutput {
    pub total_symbols: usize,
    pub loaded_count: usize,
    pub no_data_count: usize,
    pub errors: usize,
    pub data: Vec<OverviewData>,
}