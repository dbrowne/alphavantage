/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency mapping service.
//!
//! This module provides a service for managing cryptocurrency ID mappings
//! across different data sources.

use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::CryptoLoaderError;

use super::discovery::{discover_coingecko_id, discover_coinpaprika_id};

/// Configuration for the mapping service.
#[derive(Debug, Clone)]
pub struct MappingConfig {
  pub api_keys: HashMap<String, String>,
  pub rate_limit_delay_ms: u64,
}

impl Default for MappingConfig {
  fn default() -> Self {
    Self { api_keys: HashMap::new(), rate_limit_delay_ms: 1000 }
  }
}

/// Trait for mapping repository operations.
///
/// This trait abstracts database operations for storing and retrieving
/// cryptocurrency ID mappings, allowing the mapping service to be
/// database-agnostic.
#[async_trait]
pub trait MappingRepository: Send + Sync {
  /// Get an API ID for a symbol from the specified source.
  async fn get_api_id(&self, sid: i64, source: &str) -> Result<Option<String>, CryptoLoaderError>;

  /// Store or update an API mapping.
  async fn upsert_api_mapping(
    &self,
    sid: i64,
    source: &str,
    api_id: &str,
    api_slug: Option<&str>,
    api_symbol: Option<&str>,
    is_active: Option<bool>,
  ) -> Result<(), CryptoLoaderError>;

  /// Get symbols that need mapping for a specific source.
  async fn get_symbols_needing_mapping(
    &self,
    source: &str,
  ) -> Result<Vec<(i64, String, String)>, CryptoLoaderError>;
}

/// Service for discovering and managing cryptocurrency ID mappings.
pub struct CryptoMappingService {
  client: Client,
  config: MappingConfig,
}

impl CryptoMappingService {
  /// Create a new mapping service with the given API keys.
  pub fn new(api_keys: HashMap<String, String>) -> Self {
    Self { client: Client::new(), config: MappingConfig { api_keys, rate_limit_delay_ms: 1000 } }
  }

  /// Create a new mapping service with full configuration.
  pub fn with_config(config: MappingConfig) -> Self {
    Self { client: Client::new(), config }
  }

  /// Create a new mapping service with a custom HTTP client.
  pub fn with_client(config: MappingConfig, client: Client) -> Self {
    Self { client, config }
  }

  /// Get or discover CoinGecko ID for a symbol.
  ///
  /// First checks the repository for an existing mapping, then attempts
  /// dynamic discovery via the CoinGecko API if not found.
  ///
  /// Returns `(Option<String>, bool)` where the bool indicates if an API call was made.
  pub async fn get_coingecko_id(
    &self,
    mapping_repo: &Arc<dyn MappingRepository>,
    sid: i64,
    symbol: &str,
  ) -> Result<(Option<String>, bool), CryptoLoaderError> {
    // 1. Check repository first
    if let Ok(Some(api_id)) = mapping_repo.get_api_id(sid, "CoinGecko").await {
      info!("Found existing CoinGecko mapping: {} -> {}", symbol, api_id);
      return Ok((Some(api_id), false)); // No API call made
    }

    // 2. Dynamic discovery using CoinGecko API
    info!("Dynamically discovering CoinGecko ID for: {}", symbol);

    let api_key = self.config.api_keys.get("coingecko");
    match discover_coingecko_id(&self.client, symbol, api_key.map(|s| s.as_str())).await {
      Ok(Some(coingecko_id)) => {
        info!("Discovered CoinGecko ID: {} -> {}", symbol, coingecko_id);

        // Store the discovered mapping
        if let Err(e) = mapping_repo
          .upsert_api_mapping(sid, "CoinGecko", &coingecko_id, None, Some(symbol), None)
          .await
        {
          error!("Failed to store discovered mapping: {}", e);
        } else {
          info!("Stored dynamic mapping: {} -> {}", symbol, coingecko_id);
        }

        Ok((Some(coingecko_id), true)) // API call was made
      }
      Ok(None) => {
        warn!("No CoinGecko ID found via API for: {}", symbol);
        Ok((None, true)) // API call was made
      }
      Err(e) => {
        error!("Discovery failed for {}: {}", symbol, e);
        Err(e)
      }
    }
  }

  /// Bulk discovery for missing mappings.
  pub async fn discover_missing_mappings(
    &self,
    mapping_repo: &Arc<dyn MappingRepository>,
    source: &str,
  ) -> Result<usize, CryptoLoaderError> {
    let missing_symbols = mapping_repo.get_symbols_needing_mapping(source).await?;

    info!("Discovering {} missing {} mappings via API", missing_symbols.len(), source);

    let mut discovered_count = 0;
    for (sid, symbol, _name) in missing_symbols {
      let api_called = match source {
        "CoinGecko" => {
          match self.get_coingecko_id(mapping_repo, sid, &symbol).await {
            Ok((Some(_), api_called)) => {
              discovered_count += 1;
              api_called
            }
            Ok((None, api_called)) => api_called,
            Err(_) => true, // Error means API was called
          }
        }
        "CoinPaprika" => {
          if let Ok(Some(coinpaprika_id)) = discover_coinpaprika_id(&self.client, &symbol).await {
            let _ = mapping_repo
              .upsert_api_mapping(sid, "CoinPaprika", &coinpaprika_id, None, Some(&symbol), None)
              .await;
            discovered_count += 1;
            info!("Discovered CoinPaprika mapping: {} -> {}", symbol, coinpaprika_id);
          }
          true // CoinPaprika always makes an API call
        }
        _ => {
          warn!("Unknown source for discovery: {}", source);
          false
        }
      };

      // Rate limiting between API calls (only if an API call was made)
      if api_called {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.config.rate_limit_delay_ms))
          .await;
      }
    }

    info!("Dynamically discovered {} new {} mappings", discovered_count, source);
    Ok(discovered_count)
  }
}

impl Clone for CryptoMappingService {
  fn clone(&self) -> Self {
    Self { client: self.client.clone(), config: self.config.clone() }
  }
}
