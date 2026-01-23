/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! CoinGecko details loader wrapper.
//!
//! This module provides a wrapper around the crypto-loaders CoinGeckoDetailsLoader
//! that integrates with the av-loaders DataLoader trait.

use async_trait::async_trait;
use av_database_postgres::repository::CacheRepository;
use std::sync::Arc;

use crate::{DataLoader, LoaderContext, LoaderResult, ProcessState};
use super::sources::CacheRepositoryAdapter;

// Re-export types from crypto-loaders for backward compatibility
pub use crypto_loaders::{
  CoinGeckoDetailsLoader as BaseCoinGeckoDetailsLoader,
  CoinGeckoDetailsOutput, CoinGeckoDetailedCoin, CoinInfo,
  CryptoDetailedData, CryptoSocialData, CryptoTechnicalData, DetailsLoaderConfig,
};

/// Input for the loader
#[derive(Debug)]
pub struct CoinGeckoDetailsInput {
  pub coins: Vec<CoinInfo>,
}

/// CoinGecko details loader with DataLoader trait support.
pub struct CoinGeckoDetailsLoader {
  inner: BaseCoinGeckoDetailsLoader,
  cache_repository: Option<Arc<dyn CacheRepository>>,
}

impl CoinGeckoDetailsLoader {
  pub fn new(api_key: String, max_concurrent: usize) -> Self {
    let config = DetailsLoaderConfig {
      max_concurrent,
      retry_delay_ms: 200,
      show_progress: true,
    };
    Self {
      inner: BaseCoinGeckoDetailsLoader::new(api_key, config),
      cache_repository: None,
    }
  }

  pub fn with_cache_repository(mut self, cache_repo: Arc<dyn CacheRepository>) -> Self {
    let cache_adapter = CacheRepositoryAdapter::as_arc(cache_repo.clone());
    self.inner = self.inner.with_cache(cache_adapter);
    self.cache_repository = Some(cache_repo);
    self
  }
}

#[async_trait]
impl DataLoader for CoinGeckoDetailsLoader {
  type Input = CoinGeckoDetailsInput;
  type Output = CoinGeckoDetailsOutput;

  async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
    // Track process if enabled
    if let Some(tracker) = &context.process_tracker {
      tracker.start("coingecko_details_loader").await?;
    }

    // Use the inner loader
    let result = self.inner.load(input.coins).await.map_err(|e| {
      crate::LoaderError::ApiError(e.to_string())
    })?;

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      tracker
        .complete(if result.errors > 0 {
          ProcessState::CompletedWithErrors
        } else {
          ProcessState::Success
        })
        .await?;
    }

    Ok(result)
  }

  fn name(&self) -> &'static str {
    "CoinGeckoDetailsLoader"
  }
}

impl Clone for CoinGeckoDetailsLoader {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      cache_repository: self.cache_repository.clone(),
    }
  }
}