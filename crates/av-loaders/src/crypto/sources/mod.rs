pub mod coincap;
pub mod coingecko;
pub mod coinmarketcap;
pub mod coinpaprika;
pub mod sosovalue;

use crate::crypto::{CryptoLoaderError, CryptoSymbol};
use async_trait::async_trait;
use reqwest::Client;

#[async_trait]
pub trait CryptoDataProvider {
  async fn fetch_symbols(&self, client: &Client) -> Result<Vec<CryptoSymbol>, CryptoLoaderError>;
  fn source_name(&self) -> &'static str;
  fn rate_limit_delay(&self) -> u64;
  fn requires_api_key(&self) -> bool;
}
