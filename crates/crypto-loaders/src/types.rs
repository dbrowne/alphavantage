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

//! Core types for cryptocurrency data loading.
//!
//! This module contains the fundamental types used across all crypto data providers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a cryptocurrency symbol with metadata from various sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbol {
  pub symbol: String,
  pub priority: i32,
  pub name: String,
  pub base_currency: Option<String>,
  pub quote_currency: Option<String>,
  pub market_cap_rank: Option<u32>,
  pub source: CryptoDataSource,
  pub source_id: String,
  pub is_active: bool,
  pub created_at: DateTime<Utc>,
  pub additional_data: HashMap<String, serde_json::Value>,
}

/// Data source for cryptocurrency information.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CryptoDataSource {
  CoinMarketCap,
  CoinGecko,
  CoinPaprika,
  CoinCap,
  SosoValue,
}

impl std::fmt::Display for CryptoDataSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CryptoDataSource::CoinMarketCap => write!(f, "coinmarketcap"),
      CryptoDataSource::CoinGecko => write!(f, "coingecko"),
      CryptoDataSource::CoinPaprika => write!(f, "coinpaprika"),
      CryptoDataSource::CoinCap => write!(f, "coincap"),
      CryptoDataSource::SosoValue => write!(f, "sosovalue"),
    }
  }
}

/// Configuration for cryptocurrency data loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderConfig {
  pub max_concurrent_requests: usize,
  pub retry_attempts: u32,
  pub retry_delay_ms: u64,
  pub rate_limit_delay_ms: u64,
  pub enable_progress_bar: bool,
  pub sources: Vec<CryptoDataSource>,
  pub batch_size: usize,
  pub cache_ttl_hours: Option<u64>,
}

impl Default for CryptoLoaderConfig {
  fn default() -> Self {
    Self {
      max_concurrent_requests: 10,
      retry_attempts: 3,
      retry_delay_ms: 1000,
      rate_limit_delay_ms: 200,
      enable_progress_bar: true,
      sources: vec![
        CryptoDataSource::CoinGecko,
        CryptoDataSource::CoinPaprika,
        CryptoDataSource::CoinCap,
        CryptoDataSource::SosoValue,
        CryptoDataSource::CoinMarketCap,
      ],
      batch_size: 250,
      cache_ttl_hours: Some(24),
    }
  }
}

/// Result of a crypto loading operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderResult {
  pub symbols_loaded: usize,
  pub symbols_failed: usize,
  pub symbols_skipped: usize,
  pub source_results: HashMap<CryptoDataSource, SourceResult>,
  pub processing_time_ms: u64,
}

/// Result from a single data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
  pub symbols_fetched: usize,
  pub errors: Vec<String>,
  pub rate_limited: bool,
  pub response_time_ms: u64,
}
