//! Simple wrapper for loading crypto news using the existing NewsLoader
//!
//! This module provides a thin wrapper around NewsLoader that formats
//! crypto symbols correctly for the AlphaVantage API (CRYPTO:BTC format).

use crate::{
    NewsLoader, NewsLoaderConfig, NewsLoaderInput, NewsLoaderOutput,
    LoaderContext, LoaderResult, DataLoader,
    news_loader::SymbolInfo,
};
use chrono::{DateTime, Utc};
use tracing::info;

/// Load crypto news by converting symbols to CRYPTO:XXX format
pub async fn load_crypto_news(
    context: &LoaderContext,
    symbols: Vec<SymbolInfo>,
    config: NewsLoaderConfig,
    time_from: Option<DateTime<Utc>>,
    time_to: Option<DateTime<Utc>>,
) -> LoaderResult<NewsLoaderOutput> {
    // Convert symbols to crypto format (CRYPTO:BTC, CRYPTO:ETH, etc.)
    let crypto_symbols: Vec<SymbolInfo> = symbols
        .into_iter()
        .map(|s| SymbolInfo {
            sid: s.sid,
            symbol: format!("CRYPTO:{}", s.symbol),
        })
        .collect();

    info!("Loading news for {} crypto symbols", crypto_symbols.len());

    // Use the existing NewsLoader with crypto-formatted symbols
    let loader = NewsLoader::new(5).with_config(config);

    let input = NewsLoaderInput {
        symbols: crypto_symbols,
        time_from,
        time_to,
    };

    loader.load(context, input).await
}