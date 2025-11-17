/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
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
use chrono::Utc;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use crate::{DataLoader, LoaderContext, LoaderResult, process_tracker::ProcessState};
use av_database_postgres::repository::CacheRepositoryExt;
use av_models::fundamentals::CompanyOverview;

/// Configuration for overview loader caching behavior
#[derive(Debug, Clone)]
pub struct OverviewLoaderConfig {
  /// Enable caching (requires cache_repository in LoaderContext)
  pub enable_cache: bool,
  /// Cache TTL in hours
  pub cache_ttl_hours: i64,
  /// Force refresh (bypass cache)
  pub force_refresh: bool,
}

impl Default for OverviewLoaderConfig {
  fn default() -> Self {
    Self {
      enable_cache: true,
      cache_ttl_hours: 720, // 30 days - fundamental data changes infrequently
      force_refresh: false,
    }
  }
}

/// Loader for company overview fundamental data
pub struct OverviewLoader {
  semaphore: Arc<Semaphore>,
  config: OverviewLoaderConfig,
}

impl OverviewLoader {
  pub fn new(max_concurrent: usize) -> Self {
    Self {
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      config: OverviewLoaderConfig::default(),
    }
  }

  /// Create with custom configuration
  pub fn with_config(mut self, config: OverviewLoaderConfig) -> Self {
    self.config = config;
    self
  }

  /// Generate cache key for overview requests
  fn generate_cache_key(&self, symbol: &str) -> String {
    format!("overview_{}", symbol.to_uppercase())
  }

  /// Get cached response if available and not expired
  async fn get_cached_response(
    &self,
    cache_key: &str,
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) -> Option<CompanyOverview> {
    if !self.config.enable_cache || self.config.force_refresh {
      return None;
    }

    match cache_repo.get::<CompanyOverview>(cache_key, "alphavantage").await {
      Ok(Some(overview)) => {
        info!("📦 Cache hit for overview: {}", cache_key);
        debug!("Successfully retrieved cached overview");
        Some(overview)
      }
      Ok(None) => {
        debug!("Cache miss for overview: {}", cache_key);
        None
      }
      Err(e) => {
        debug!("Cache read error for overview {}: {}", cache_key, e);
        None
      }
    }
  }

  /// Cache the API response
  async fn cache_response(
    &self,
    cache_key: &str,
    overview: &CompanyOverview,
    symbol: &str,
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) {
    if !self.config.enable_cache {
      return;
    }

    let endpoint_url = format!("OVERVIEW:{}", symbol);

    match cache_repo
      .set(cache_key, "alphavantage", &endpoint_url, overview, self.config.cache_ttl_hours)
      .await
    {
      Ok(()) => {
        let expires_at = Utc::now() + chrono::Duration::hours(self.config.cache_ttl_hours);
        info!("💾 Cached overview for {} (expires: {})", cache_key, expires_at);
      }
      Err(e) => {
        warn!("Failed to cache overview for {}: {}", cache_key, e);
      }
    }
  }

  /// Clean expired cache entries
  pub async fn cleanup_expired_cache(
    cache_repo: &Arc<dyn av_database_postgres::repository::CacheRepository>,
  ) -> Result<usize, crate::error::LoaderError> {
    match cache_repo.cleanup_expired("alphavantage").await {
      Ok(deleted_count) => {
        if deleted_count > 0 {
          info!("🧹 Cleaned up {} expired overview cache entries", deleted_count);
        }
        Ok(deleted_count)
      }
      Err(e) => {
        Err(crate::error::LoaderError::DatabaseError(format!("Cache cleanup failed: {}", e)))
      }
    }
  }

  /// Fetch overview from API with retry logic
  async fn fetch_overview_from_api(
    &self,
    client: &Arc<av_client::AlphaVantageClient>,
    symbol: &str,
    retry_delay: u64,
  ) -> Result<Option<CompanyOverview>, av_client::Error> {
    match client.fundamentals().company_overview(symbol).await {
      Ok(overview) => {
        if overview.symbol.is_empty() || overview.symbol == "None" {
          info!("No overview data available for {}", symbol);
          Ok(None)
        } else {
          Ok(Some(overview))
        }
      }
      Err(e) => {
        if e.to_string().contains("rate limit") {
          warn!("Rate limit hit for {}, waiting before retry...", symbol);
          tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

          // Retry once
          match client.fundamentals().company_overview(symbol).await {
            Ok(overview) => {
              if overview.symbol.is_empty() || overview.symbol == "None" {
                Ok(None)
              } else {
                Ok(Some(overview))
              }
            }
            Err(retry_err) => {
              error!("Retry failed for {}: {}", symbol, retry_err);
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
    }
  }
}

// Implement Clone for use in async closures
impl Clone for OverviewLoader {
  fn clone(&self) -> Self {
    Self { semaphore: Arc::clone(&self.semaphore), config: self.config.clone() }
  }
}

#[async_trait]
impl DataLoader for OverviewLoader {
  type Input = OverviewLoaderInput;
  type Output = OverviewLoaderOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
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

    // Get cache repository if available
    let cache_repo_opt = context.cache_repository.clone();

    // Process symbols concurrently
    let results = stream::iter(input.symbols.into_iter())
      .map(move |symbol_info| {
        let client = client_ref.clone();
        let semaphore = self.semaphore.clone();
        let progress = progress.clone();
        let cache_repo_opt = cache_repo_opt.clone();
        let loader = self.clone();

        async move {
          let _permit =
            semaphore.acquire().await.expect("Semaphore should not be closed during operation");

          if let Some(pb) = &progress {
            pb.set_message(format!("Processing {}", symbol_info.symbol));
          }

          // Generate cache key
          let cache_key = loader.generate_cache_key(&symbol_info.symbol);

          // Check cache first (if cache repository is available)
          let (overview_result, from_cache) = if let Some(cache_repo) = &cache_repo_opt {
            if let Some(cached_overview) = loader.get_cached_response(&cache_key, cache_repo).await
            {
              info!("📦 Using cached overview for {} (no API call needed)", symbol_info.symbol);
              (Ok(cached_overview), true)
            } else {
              // Cache miss - call API
              info!("🌐 Cache miss - calling API for overview {}", symbol_info.symbol);
              match loader.fetch_overview_from_api(&client, &symbol_info.symbol, retry_delay).await
              {
                Ok(Some(overview)) => {
                  // Cache successful response
                  loader
                    .cache_response(&cache_key, &overview, &symbol_info.symbol, cache_repo)
                    .await;
                  (Ok(overview), false)
                }
                Ok(None) => return Ok((None, false)),
                Err(e) => return Err((e, false)),
              }
            }
          } else {
            // No cache - directly call API
            debug!("No cache repository available - calling API directly");
            match loader.fetch_overview_from_api(&client, &symbol_info.symbol, retry_delay).await {
              Ok(Some(overview)) => (Ok(overview), false),
              Ok(None) => return Ok((None, false)),
              Err(e) => return Err((e, false)),
            }
          };

          if let Some(pb) = &progress {
            pb.inc(1);
          }

          // Add delay to respect rate limits (only if not from cache)
          if !from_cache {
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay)).await;
          }

          // Process result
          match overview_result {
            Ok(overview) => {
              if overview.symbol.is_empty() || overview.symbol == "None" {
                info!("No overview data available for {}", symbol_info.symbol);
                Ok((None, from_cache))
              } else {
                Ok((
                  Some(OverviewData {
                    sid: symbol_info.sid,
                    symbol: symbol_info.symbol.clone(),
                    overview,
                  }),
                  from_cache,
                ))
              }
            }
            Err(e) => Err((e, from_cache)),
          }
        }
      })
      .buffer_unordered(max_concurrent)
      .collect::<Vec<_>>()
      .await;

    if let Some(pb) = progress_for_finish {
      pb.finish_with_message("Overview loading complete");
    }

    // Process results and track cache statistics
    let mut loaded = Vec::new();
    let mut errors = 0;
    let mut no_data = 0;
    let mut cache_hits = 0usize;
    let mut api_calls = 0usize;

    for result in results {
      match result {
        Ok((Some(data), from_cache)) => {
          if from_cache {
            cache_hits += 1;
          } else {
            api_calls += 1;
          }
          loaded.push(data);
        }
        Ok((None, from_cache)) => {
          if from_cache {
            cache_hits += 1;
          } else {
            api_calls += 1;
          }
          no_data += 1;
        }
        Err((e, from_cache)) => {
          if !from_cache {
            api_calls += 1;
          }
          error!("Failed to load overview: {}", e);
          errors += 1;
        }
      }
    }

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      tracker
        .complete(if errors > 0 {
          ProcessState::CompletedWithErrors
        } else {
          ProcessState::Success
        })
        .await?;
    }

    info!(
      "Overview loading complete: {} loaded, {} no data, {} errors, {} cache hits, {} API calls",
      loaded.len(),
      no_data,
      errors,
      cache_hits,
      api_calls
    );

    Ok(OverviewLoaderOutput {
      total_symbols: loaded.len() + no_data + errors,
      loaded_count: loaded.len(),
      no_data_count: no_data,
      errors,
      cache_hits,
      api_calls,
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
  pub cache_hits: usize,
  pub api_calls: usize,
  pub data: Vec<OverviewData>,
}
