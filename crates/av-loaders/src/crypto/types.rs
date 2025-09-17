use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderConfig {
  pub max_concurrent_requests: usize,
  pub retry_attempts: u32,
  pub retry_delay_ms: u64,
  pub rate_limit_delay_ms: u64,
  pub enable_progress_bar: bool,
  pub sources: Vec<CryptoDataSource>,
  pub batch_size: usize,
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
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLoaderResult {
  pub symbols_loaded: usize,
  pub symbols_failed: usize,
  pub symbols_skipped: usize,
  pub source_results: HashMap<CryptoDataSource, SourceResult>,
  pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
  pub symbols_fetched: usize,
  pub errors: Vec<String>,
  pub rate_limited: bool,
  pub response_time_ms: u64,
}
