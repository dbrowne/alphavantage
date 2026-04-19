/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency mapping service.
//!
//! Orchestrates the discovery and persistence of cross-provider cryptocurrency
//! ID mappings. The service sits between the stateless
//! [`discovery`](super::discovery) functions and a database-agnostic
//! [`MappingRepository`] trait, adding:
//!
//! - **Cache-first lookup:** checks the repository before calling external APIs.
//! - **Auto-persist:** newly discovered mappings are immediately stored.
//! - **Rate limiting:** configurable delay between API calls.
//! - **Batch discovery:** [`discover_missing_mappings`](CryptoMappingService::discover_missing_mappings)
//!   iterates over all unmapped symbols for a given source.
//!
//! # Architecture
//!
//! ```text
//! caller
//!   └──► CryptoMappingService
//!          ├── MappingRepository::get_api_id()   ← check DB first
//!          ├── discover_coingecko_id() / discover_coinpaprika_id()  ← API fallback
//!          └── MappingRepository::upsert_api_mapping()  ← persist result
//! ```
//!
//! # Supported sources
//!
//! - `"CoinGecko"` — requires an API key in `config.api_keys["coingecko"]`.
//! - `"CoinPaprika"` — no API key required (free public API).

use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::CryptoLoaderError;

use super::discovery::{discover_coingecko_id, discover_coinpaprika_id};

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for [`CryptoMappingService`].
///
/// # Fields
///
/// - `api_keys` — map of provider name → API key (e.g., `"coingecko"` → `"CG-..."`).
/// - `rate_limit_delay_ms` — delay in milliseconds between consecutive API calls
///   during batch discovery (default: 1000ms).
#[derive(Debug, Clone)]
pub struct MappingConfig {
  /// Provider API keys keyed by lowercase provider name.
  pub api_keys: HashMap<String, String>,
  /// Delay between API calls in milliseconds.
  pub rate_limit_delay_ms: u64,
}

impl Default for MappingConfig {
  fn default() -> Self {
    Self { api_keys: HashMap::new(), rate_limit_delay_ms: 1000 }
  }
}

// ─── Repository trait ───────────────────────────────────────────────────────

/// Database-agnostic trait for storing and retrieving cryptocurrency ID mappings.
///
/// Implement this trait on your database layer (e.g., wrapping
/// [`CryptoRepository`](av_database_postgres::CryptoRepository)) to provide
/// persistence to the [`CryptoMappingService`].
///
/// # Methods
///
/// - [`get_api_id`](MappingRepository::get_api_id) — look up an existing mapping.
/// - [`upsert_api_mapping`](MappingRepository::upsert_api_mapping) — store or update a mapping.
/// - [`get_symbols_needing_mapping`](MappingRepository::get_symbols_needing_mapping) —
///   return `(sid, symbol, name)` tuples for coins missing a mapping.
#[async_trait]
pub trait MappingRepository: Send + Sync {
  /// Returns the external API ID for a `(sid, source)` pair, or `None` if
  /// no mapping exists.
  async fn get_api_id(&self, sid: i64, source: &str) -> Result<Option<String>, CryptoLoaderError>;

  /// Inserts or updates an API mapping for a `(sid, source)` pair.
  async fn upsert_api_mapping(
    &self,
    sid: i64,
    source: &str,
    api_id: &str,
    api_slug: Option<&str>,
    api_symbol: Option<&str>,
    is_active: Option<bool>,
  ) -> Result<(), CryptoLoaderError>;

  /// Returns `(sid, symbol, name)` tuples for symbols that do not yet have
  /// a mapping for the given source.
  async fn get_symbols_needing_mapping(
    &self,
    source: &str,
  ) -> Result<Vec<(i64, String, String)>, CryptoLoaderError>;
}

// ─── Service ────────────────────────────────────────────────────────────────

/// Service for discovering and managing cryptocurrency ID mappings.
///
/// Constructed via [`new`](Self::new), [`with_config`](Self::with_config),
/// or [`with_client`](Self::with_client). Implements `Clone` (shares the
/// HTTP client).
pub struct CryptoMappingService {
  client: Client,
  config: MappingConfig,
}

impl CryptoMappingService {
  /// Creates a mapping service with API keys and default rate-limit delay (1000ms).
  pub fn new(api_keys: HashMap<String, String>) -> Self {
    Self { client: Client::new(), config: MappingConfig { api_keys, rate_limit_delay_ms: 1000 } }
  }

  /// Creates a mapping service with full configuration.
  pub fn with_config(config: MappingConfig) -> Self {
    Self { client: Client::new(), config }
  }

  /// Creates a mapping service with a custom HTTP client and configuration.
  pub fn with_client(config: MappingConfig, client: Client) -> Self {
    Self { client, config }
  }

  /// Gets or discovers a CoinGecko ID for a symbol.
  ///
  /// **Two-phase lookup:**
  /// 1. Checks `mapping_repo` for an existing `(sid, "CoinGecko")` mapping.
  /// 2. If not found, calls [`discover_coingecko_id`] and persists the result.
  ///
  /// # Returns
  ///
  /// `(Option<coingecko_id>, api_called)` — the boolean indicates whether
  /// an external API call was made (`false` = cache hit, `true` = API called).
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

  /// Discovers mappings for all symbols that lack one for the given source.
  ///
  /// Queries the repository for unmapped symbols, then iterates through
  /// them one at a time, calling the appropriate discovery function and
  /// persisting results. Applies `config.rate_limit_delay_ms` after each
  /// API call.
  ///
  /// Supports `"CoinGecko"` and `"CoinPaprika"` as source values.
  ///
  /// Returns the number of newly discovered mappings.
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
