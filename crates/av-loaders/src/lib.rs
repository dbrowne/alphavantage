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

//! # av-loaders
//!
//! Data loading functionality for AlphaVantage market data.
//!
//! This crate provides loaders for various data types including:
//! - Securities (symbols) from CSV files via API lookup
//! - Company overviews and fundamentals
//! - Intraday and daily price data
//! - News articles with sentiment analysis
//! - Market movers (top gainers/losers)
//! - Crypto markets and social data
//!
//! The loaders fetch data from various APIs (AlphaVantage, CoinGecko, etc.)
//! and return it for further processing. Database operations should be handled
//! by the consuming application.

pub mod batch_processor;
pub mod crypto;
pub mod csv_processor;
pub mod error;
pub mod intraday_price_loader;
pub mod loader;
pub mod news_loader;
pub mod overview_loader;
pub mod process_tracker;
pub mod security_loader;
pub mod summary_price_loader;
pub mod top_movers_loader;

pub use news_loader::{
  NewsLoader, NewsLoaderConfig, NewsLoaderInput, NewsLoaderOutput, SymbolInfo as NewsSymbolInfo,
  load_news_for_equity_symbols,
};

pub use intraday_price_loader::{
  IntradayInterval, IntradayPriceConfig, IntradayPriceData, IntradayPriceLoader,
  IntradayPriceLoaderInput, IntradayPriceLoaderOutput, SymbolInfo as IntradaySymbolInfo,
};

pub use summary_price_loader::{
  SummaryPriceConfig, SummaryPriceData, SummaryPriceLoader, SummaryPriceLoaderInput,
  SummaryPriceLoaderOutput,
};

pub use top_movers_loader::{TopMoversLoader, TopMoversLoaderInput, TopMoversLoaderOutput};

// Re-export commonly used types
pub use batch_processor::{BatchConfig, BatchProcessor};
pub use error::{LoaderError, LoaderResult};
pub use loader::{DataLoader, LoaderConfig, LoaderContext};
pub use process_tracker::{ProcessState, ProcessTracker};

// Re-export loaders with their data types
pub use security_loader::{
  SecurityData, SecurityLoader, SecurityLoaderConfig, SecurityLoaderInput, SecurityLoaderOutput,
  SymbolMatchMode,
};

pub use overview_loader::{
  OverviewData, OverviewLoader, OverviewLoaderInput, OverviewLoaderOutput, SymbolInfo,
};

// Re-export crypto module types including markets loader
pub use crypto::{
  CryptoDataSource,
  CryptoLoaderConfig,
  CryptoLoaderError,
  CryptoLoaderResult,
  CryptoSymbol,
  CryptoSymbolLoader,
  SourceResult,
  crypto_news_loader::load_crypto_news,
  database::{CryptoDbInput, CryptoDbLoader, CryptoDbOutput, SourceResultSummary},

  intraday_loader::{
    CryptoIntradayConfig, CryptoIntradayInput, CryptoIntradayLoader, CryptoIntradayLoaderInput,
    CryptoIntradayLoaderOutput, CryptoIntradayOutput, CryptoIntradayPriceData,
    CryptoSymbolInfo as CryptoIntradaySymbolInfo,
  },
  // Export markets loader types
  markets_loader::{
    CryptoMarketData, CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader,
    CryptoSymbolForMarkets,
  },
  metadata_loader::{
    CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataLoader, CryptoMetadataOutput,
    CryptoSymbolForMetadata, ProcessedCryptoMetadata,
  },

};

pub mod prelude {
  pub use crate::{
    BatchConfig,
    BatchProcessor,
    // Include crypto types in prelude
    CryptoDataSource,
    CryptoIntradayConfig,
    CryptoIntradayInput,
    CryptoIntradayLoader,
    CryptoIntradayOutput,
    CryptoLoaderConfig,
    CryptoMarketsLoader,
    CryptoMetadataLoader,
    CryptoSymbolLoader,
    DataLoader,
    IntradayInterval,
    IntradayPriceConfig,
    IntradayPriceLoader,
    LoaderConfig,
    LoaderContext,
    LoaderError,
    LoaderResult,
    NewsLoader,
    NewsLoaderConfig,
    ProcessState,
    ProcessTracker,
    SummaryPriceConfig,
    SummaryPriceLoader,
    crypto::crypto_news_loader::load_crypto_news,
    load_news_for_equity_symbols,
  };
}
