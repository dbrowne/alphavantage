/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency mapping service wrapper for av-loaders integration.
//!
//! This module provides a wrapper around the crypto-loaders CryptoMappingService
//! that integrates with av-database-postgres CryptoRepository.

use async_trait::async_trait;
use av_database_postgres::repository::CryptoRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// Re-export types from crypto-loaders for backward compatibility
pub use crypto_loaders::{
  discover_coingecko_id, discover_coinpaprika_id, CryptoMappingService as BaseCryptoMappingService,
  MappingConfig, MappingRepository,
};

use crypto_loaders::CryptoLoaderError;

/// Adapter to implement MappingRepository using CryptoRepository.
pub struct CryptoRepositoryMappingAdapter {
  repo: Arc<dyn CryptoRepository>,
}

impl CryptoRepositoryMappingAdapter {
  pub fn new(repo: Arc<dyn CryptoRepository>) -> Self {
    Self { repo }
  }

  pub fn as_arc(repo: Arc<dyn CryptoRepository>) -> Arc<dyn MappingRepository> {
    Arc::new(Self::new(repo))
  }
}

#[async_trait]
impl MappingRepository for CryptoRepositoryMappingAdapter {
  async fn get_api_id(&self, sid: i64, source: &str) -> Result<Option<String>, CryptoLoaderError> {
    self.repo.get_api_id(sid, source).await.map_err(|e| CryptoLoaderError::ApiError(e.to_string()))
  }

  async fn upsert_api_mapping(
    &self,
    sid: i64,
    source: &str,
    api_id: &str,
    api_slug: Option<&str>,
    api_symbol: Option<&str>,
    is_active: Option<bool>,
  ) -> Result<(), CryptoLoaderError> {
    self
      .repo
      .upsert_api_mapping(sid, source, api_id, api_slug, api_symbol, is_active)
      .await
      .map_err(|e| CryptoLoaderError::ApiError(e.to_string()))
  }

  async fn get_symbols_needing_mapping(
    &self,
    source: &str,
  ) -> Result<Vec<(i64, String, String)>, CryptoLoaderError> {
    self
      .repo
      .get_symbols_needing_mapping(source)
      .await
      .map_err(|e| CryptoLoaderError::ApiError(e.to_string()))
  }
}

/// Extended CryptoMappingService that uses CryptoRepository.
pub struct CryptoMappingService {
  inner: BaseCryptoMappingService,
}

impl CryptoMappingService {
  pub fn new(api_keys: HashMap<String, String>) -> Self {
    Self { inner: BaseCryptoMappingService::new(api_keys) }
  }

  /// Get or discover CoinGecko ID for a symbol.
  ///
  /// Returns `(Option<String>, bool)` where the bool indicates if an API call was made.
  pub async fn get_coingecko_id(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    sid: i64,
    symbol: &str,
  ) -> Result<(Option<String>, bool), super::CryptoLoaderError> {
    let mapping_repo = CryptoRepositoryMappingAdapter::as_arc(crypto_repo.clone());
    self.inner.get_coingecko_id(&mapping_repo, sid, symbol).await.map_err(Into::into)
  }

  /// Bulk discovery for missing mappings.
  pub async fn discover_missing_mappings(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    source: &str,
  ) -> Result<usize, super::CryptoLoaderError> {
    let mapping_repo = CryptoRepositoryMappingAdapter::as_arc(crypto_repo.clone());
    self.inner.discover_missing_mappings(&mapping_repo, source).await.map_err(Into::into)
  }

  /// Initialize mappings for a specific set of symbols (discovery-based).
  ///
  /// Note: This method uses direct Diesel queries for symbol lookup
  /// as we don't have a SymbolRepository yet.
  pub async fn initialize_mappings_for_symbols(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    db_context: &av_database_postgres::repository::DatabaseContext,
    symbol_names: &[String],
  ) -> Result<usize, super::CryptoLoaderError> {
    let mut initialized_count = 0;

    for symbol_name in symbol_names {
      let symbol_upper = symbol_name.to_uppercase();
      let symbol_upper_clone = symbol_upper.clone();

      // Look up symbol using DatabaseContext
      let symbol_result = db_context
        .run(move |conn| {
          use av_database_postgres::schema::symbols;
          use diesel::prelude::*;

          let record: Result<(i64, String), diesel::result::Error> = symbols::table
            .filter(symbols::symbol.eq(&symbol_upper_clone))
            .filter(symbols::sec_type.eq("Cryptocurrency"))
            .select((symbols::sid, symbols::symbol))
            .first(conn);

          Ok(record)
        })
        .await;

      let api_called = match symbol_result {
        Ok(Ok((symbol_sid, symbol_code))) => {
          info!("Found symbol {} with SID {}", symbol_code, symbol_sid);

          match self.get_coingecko_id(crypto_repo, symbol_sid, &symbol_code).await {
            Ok((Some(_), api_called)) => {
              initialized_count += 1;
              api_called
            }
            Ok((None, api_called)) => api_called,
            Err(_) => true, // Error means API was called
          }
        }
        _ => {
          tracing::warn!("Symbol {} not found in database", symbol_name);
          false
        }
      };

      // Rate limiting (only if an API call was made)
      if api_called {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
      }
    }

    Ok(initialized_count)
  }
}

impl Clone for CryptoMappingService {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}