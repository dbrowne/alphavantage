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

pub mod error;
pub mod loader;
pub mod security_loader;
pub mod csv_processor;
pub mod batch_processor;
pub mod process_tracker;

// Re-export commonly used types
pub use error::{LoaderError, LoaderResult};
pub use loader::{DataLoader, LoaderConfig, LoaderContext};
pub use batch_processor::{BatchConfig, BatchProcessor};
pub use process_tracker::{ProcessTracker, ProcessState};

// Common types used across loaders
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub sid: i64,
    pub symbol: String,
}

// Re-export loaders
pub use security_loader::{SecurityLoader, SecurityLoaderInput, SecurityLoaderOutput};
pub use overview_loader::{OverviewLoader, OverviewLoaderInput, OverviewLoaderOutput};
pub use price_loader::{
    IntradayPriceLoader, SummaryPriceLoader,
    IntradayLoaderInput, SummaryLoaderInput, PriceLoaderOutput,
    OutputSize, TimeSeriesInterval
};
pub use news_loader::{NewsLoader, NewsLoaderInput, NewsLoaderOutput};
pub use topstats_loader::{TopStatsLoader, TopStatsLoaderInput, TopStatsLoaderOutput, MoverType};

// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        DataLoader, LoaderConfig, LoaderContext, LoaderError, LoaderResult,
        BatchConfig, BatchProcessor, ProcessTracker, ProcessState,
    };
}