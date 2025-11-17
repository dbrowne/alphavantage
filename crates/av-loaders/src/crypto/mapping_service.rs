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

use crate::crypto::CryptoLoaderError;
use av_database_postgres::repository::CryptoRepository;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct CryptoMappingService {
  client: Client,
  api_keys: HashMap<String, String>,
}

impl CryptoMappingService {
  pub fn new(api_keys: HashMap<String, String>) -> Self {
    Self { client: Client::new(), api_keys }
  }

  /// Get or discover CoinGecko ID for a symbol using ONLY dynamic discovery
  pub async fn get_coingecko_id(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    sid: i64,
    symbol: &str,
  ) -> Result<Option<String>, CryptoLoaderError> {
    // 1. Check database first
    if let Ok(Some(api_id)) = crypto_repo.get_api_id(sid, "CoinGecko").await {
      info!("✅ Found existing CoinGecko mapping: {} -> {}", symbol, api_id);
      return Ok(Some(api_id));
    }

    // 2. Dynamic discovery using CoinGecko API
    info!("🔍 Dynamically discovering CoinGecko ID for: {}", symbol);

    let api_key = self.api_keys.get("coingecko");
    match av_database_postgres::models::crypto::discover_coingecko_id(
      &self.client,
      symbol,
      api_key.map(|s| s.as_str()),
    )
    .await
    {
      Ok(Some(coingecko_id)) => {
        info!("✅ Discovered CoinGecko ID: {} -> {}", symbol, coingecko_id);

        // Store the discovered mapping
        if let Err(e) = crypto_repo
          .upsert_api_mapping(sid, "CoinGecko", &coingecko_id, None, Some(symbol), None)
          .await
        {
          error!("Failed to store discovered mapping: {}", e);
        } else {
          info!("💾 Stored dynamic mapping: {} -> {}", symbol, coingecko_id);
        }

        Ok(Some(coingecko_id))
      }
      Ok(None) => {
        warn!("❌ No CoinGecko ID found via API for: {}", symbol);
        Ok(None)
      }
      Err(e) => {
        error!("❌ Discovery failed for {}: {}", symbol, e);
        Err(CryptoLoaderError::ApiError(format!("Discovery failed: {}", e)))
      }
    }
  }

  /// Bulk discovery for missing mappings - purely dynamic
  pub async fn discover_missing_mappings(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    source: &str,
  ) -> Result<usize, CryptoLoaderError> {
    let missing_symbols = crypto_repo
      .get_symbols_needing_mapping(source)
      .await
      .map_err(|e| CryptoLoaderError::ApiError(format!("Query failed: {}", e)))?;

    info!("🔍 Discovering {} missing {} mappings via API", missing_symbols.len(), source);

    let mut discovered_count = 0;
    for (sid, symbol, _name) in missing_symbols {
      match source {
        "CoinGecko" => {
          if let Ok(Some(_)) = self.get_coingecko_id(crypto_repo, sid, &symbol).await {
            discovered_count += 1;
          }
        }
        "CoinPaprika" => {
          if let Ok(Some(coinpaprika_id)) =
            av_database_postgres::models::crypto::discover_coinpaprika_id(&self.client, &symbol)
              .await
          {
            let _ = crypto_repo
              .upsert_api_mapping(sid, "CoinPaprika", &coinpaprika_id, None, Some(&symbol), None)
              .await;
            discovered_count += 1;
            info!("✅ Discovered CoinPaprika mapping: {} -> {}", symbol, coinpaprika_id);
          }
        }
        _ => {
          warn!("Unknown source for discovery: {}", source);
        }
      }

      // Rate limiting between API calls
      tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    info!("✅ Dynamically discovered {} new {} mappings", discovered_count, source);
    Ok(discovered_count)
  }

  /// Initialize mappings for a specific set of symbols (discovery-based)
  ///
  /// Note: This method still uses direct Diesel queries for symbol lookup
  /// as we don't have a SymbolRepository yet. This could be refactored
  /// when SymbolRepository is implemented.
  pub async fn initialize_mappings_for_symbols(
    &self,
    crypto_repo: &Arc<dyn CryptoRepository>,
    db_context: &av_database_postgres::repository::DatabaseContext,
    symbol_names: &[String],
  ) -> Result<usize, CryptoLoaderError> {
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

      match symbol_result {
        Ok(Ok((symbol_sid, symbol_code))) => {
          info!("Found symbol {} with SID {}", symbol_code, symbol_sid);

          if let Ok(Some(_)) = self.get_coingecko_id(crypto_repo, symbol_sid, &symbol_code).await {
            initialized_count += 1;
          }
        }
        _ => {
          warn!("Symbol {} not found in database", symbol_name);
        }
      }

      // Rate limiting
      tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(initialized_count)
  }
}
