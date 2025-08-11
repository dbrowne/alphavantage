// crates/av-loaders/src/lib.rs

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
//!
//! The loaders fetch data from the AlphaVantage API and return it
//! for further processing. Database operations should be handled
//! by the consuming application.

pub mod batch_processor;
pub mod crypto;
pub mod csv_processor;
pub mod error;
pub mod loader;
pub mod overview_loader;
pub mod process_tracker;
pub mod security_loader;

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

pub use crypto::{
  CryptoDataSource, CryptoLoaderConfig, CryptoLoaderError, CryptoLoaderResult, CryptoSymbol,
  CryptoSymbolLoader, SourceResult,
  database::{CryptoDbInput, CryptoDbLoader, CryptoDbOutput, SourceResultSummary},
};
pub mod prelude {
  pub use crate::{
    BatchConfig, BatchProcessor, DataLoader, LoaderConfig, LoaderContext, LoaderError,
    LoaderResult, ProcessState, ProcessTracker,
  };
}

