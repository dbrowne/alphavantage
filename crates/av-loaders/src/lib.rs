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
pub mod loader;
pub mod news_loader;
pub mod overview_loader;
pub mod process_tracker;
pub mod security_loader;

pub use news_loader::{
  NewsLoader, NewsLoaderConfig, NewsLoaderInput, NewsLoaderOutput, SymbolInfo as NewsSymbolInfo,
  load_news_for_equity_symbols,
};

// Re-export commonly used types
pub use batch_processor::{BatchConfig, BatchProcessor};
pub use error::{LoaderError, LoaderResult};
pub use loader::{DataLoader, LoaderConfig, LoaderContext};
pub use process_tracker::{ProcessState, ProcessTracker};

// Re-export loaders with their data types
pub use security_loader::{
  SecurityData, SecurityLoader, SecurityLoaderInput, SecurityLoaderOutput, SymbolMatchMode,
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
  // Export markets loader types
  markets_loader::{
    CryptoMarketData, CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader,
    CryptoSymbolForMarkets,
  },
  metadata_loader::{
    CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataLoader, CryptoMetadataOutput,
    CryptoSymbolForMetadata, ProcessedCryptoMetadata,
  },
  // Export social loader types
  social_loader::{
    CryptoSocialConfig, CryptoSocialInput, CryptoSocialLoader, CryptoSymbolForSocial,
    ProcessedSocialData,
  },
};

pub mod prelude {
  pub use crate::{
    BatchConfig,
    BatchProcessor,
    // Include crypto types in prelude
    CryptoDataSource,
    CryptoLoaderConfig,
    CryptoMarketsLoader,
    CryptoMetadataLoader,
    CryptoSocialLoader,
    CryptoSymbolLoader,
    DataLoader,
    LoaderConfig,
    LoaderContext,
    LoaderError,
    LoaderResult,
    NewsLoader,
    NewsLoaderConfig,
    ProcessState,
    ProcessTracker,
    crypto::crypto_news_loader::load_crypto_news,
    load_news_for_equity_symbols,
  };
}
