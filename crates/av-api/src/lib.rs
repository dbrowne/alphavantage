/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! # av-api
//!
//! Unified facade for the Alpha Vantage financial data workspace.
//!
//! This crate is the **single entry point** for external programs that want
//! to consume Alpha Vantage market data. It re-exports types from the
//! internal workspace crates behind feature gates so consumers only pull in
//! the dependencies they need.
//!
//! # Feature flags
//!
//! | Feature    | What it enables                                  | Default |
//! |------------|--------------------------------------------------|---------|
//! | `models`   | Response model structs (no HTTP, no DB)           | -       |
//! | `client`   | HTTP client + models (the common case)            | **yes** |
//! | `database` | `DatabaseContext`, repository traits, ORM models  | -       |
//! | `loaders`  | Data ingestion pipelines (implies client + database) | -    |
//! | `full`     | All of the above                                 | -       |
//!
//! # Quick start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! av-api = { git = "https://github.com/dbrowne/alphavantage" }
//! ```
//!
//! Then:
//!
//! ```rust,no_run
//! use av_api::prelude::*;
//!
//! # async fn run() -> av_api::error::Result<()> {
//! let av = AlphaVantage::from_env()?;
//! let client = av.client()?;
//! // client.time_series().daily("AAPL", "compact").await?
//! # Ok(())
//! # }
//! ```
//!
//! # Crate layout
//!
//! ```text
//! av-api (this crate)
//!   ├── error      → unified ApiError wrapping all sub-crate errors
//!   ├── builder    → AlphaVantage entry point (config + lazy client + optional DB)
//!   ├── prelude    → convenience glob import
//!   ├── types      → re-exported from av-core (always available)
//!   ├── models     → re-exported from av-models   [feature = "models"]
//!   ├── client     → re-exported from av-client   [feature = "client"]
//!   ├── database   → re-exported from av-database [feature = "database"]
//!   ├── loaders    → re-exported from av-loaders  [feature = "loaders"]
//!   └── queries    → pre-built cross-table queries [feature = "database"]
//! ```

pub mod builder;
pub mod error;
pub mod prelude;

/// Pre-built queries that join across tables for common data retrieval
/// (e.g., security snapshot with latest price).
///
/// Requires the `database` feature.
#[cfg(feature = "database")]
pub mod queries;

// ─── Always available (av-core is lightweight) ──────────────────────────

/// Re-exported from `av-core`: canonical API base URL.
pub use av_core::ALPHA_VANTAGE_BASE_URL;
/// Re-exported from `av-core`: API configuration.
pub use av_core::Config;
/// Re-exported from `av-core`: free-tier rate limit (75 RPM).
pub use av_core::DEFAULT_RATE_LIMIT;
/// Re-exported from `av-core`: core error type.
pub use av_core::Error as CoreError;
/// Re-exported from `av-core`: type-safe API function identifiers.
pub use av_core::FuncType;
/// Re-exported from `av-core`: premium rate limit (600 RPM).
pub use av_core::PREMIUM_RATE_LIMIT;

/// Shared domain types: exchanges, security types, intervals, currencies, etc.
///
/// Always available regardless of feature flags.
pub mod types {
  pub use av_core::types::*;
}

// ─── Feature-gated re-exports ───────────────────────────────────────────

/// Strongly-typed response models for all Alpha Vantage API endpoints.
///
/// Enabled by the `models` feature (implied by `client`).
#[cfg(feature = "models")]
pub mod models {
  pub use av_models::*;
}

/// HTTP client and endpoint accessors.
///
/// Enabled by the `client` feature (default).
#[cfg(feature = "client")]
pub mod client {
  pub use av_client::AlphaVantageClient;
  pub use av_client::{
    CryptoEndpoints, CryptoSocialEndpoints, ForexEndpoints, FundamentalsEndpoints, NewsEndpoints,
    TimeSeriesEndpoints,
  };
}

/// Database access: connection pooling, repository traits, and Diesel ORM models.
///
/// Enabled by the `database` feature.
#[cfg(feature = "database")]
pub mod database {
  pub use av_database_postgres::models;
  pub use av_database_postgres::{
    CacheRepository, CacheRepositoryExt, CryptoRepository, DatabaseContext, NewsRepository,
    OverviewRepository, OverviewSymbolFilter, Repository, RepositoryError, RepositoryResult,
    SymbolInfo, SymbolRepository, Transactional,
  };
}

/// Data ingestion loaders for all asset classes.
///
/// Enabled by the `loaders` feature (implies `client` + `database`).
#[cfg(feature = "loaders")]
pub mod loaders {
  // Core infrastructure
  pub use av_loaders::{
    BatchConfig, BatchProcessor, CacheConfig, CacheHelper, DataLoader, LoaderConfig, LoaderContext,
    LoaderError, LoaderResult, ProcessState, ProcessTracker,
  };

  // Equity loaders
  pub use av_loaders::{
    IntradayInterval, IntradayPriceConfig, IntradayPriceData, IntradayPriceLoader,
    IntradayPriceLoaderInput, IntradayPriceLoaderOutput, NewsLoader, NewsLoaderConfig,
    NewsLoaderInput, NewsLoaderOutput, OverviewData, OverviewLoader, OverviewLoaderInput,
    OverviewLoaderOutput, SecurityData, SecurityLoader, SecurityLoaderConfig, SecurityLoaderInput,
    SecurityLoaderOutput, SummaryPriceConfig, SummaryPriceData, SummaryPriceLoader,
    SummaryPriceLoaderInput, SummaryPriceLoaderOutput, TopMoversConfig, TopMoversLoader,
    TopMoversLoaderInput, TopMoversLoaderOutput,
  };

  // Crypto loaders
  pub use av_loaders::{CryptoDataSource, CryptoLoaderConfig, CryptoSymbolLoader, SourceResult};
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_core_types_always_available() {
    let _ = FuncType::TimeSeriesDaily;
    assert_eq!(DEFAULT_RATE_LIMIT, 75);
    assert!(ALPHA_VANTAGE_BASE_URL.starts_with("https://"));
  }

  #[test]
  fn test_types_module() {
    use types::{Exchange, Interval, SecurityType};

    let nyse = "NYSE".parse::<Exchange>().unwrap();
    assert_eq!(nyse, Exchange::NYSE);

    let interval = Interval::Min5;
    assert_eq!(interval.minutes(), 5);

    assert!(SecurityType::Equity.is_equity());
  }

  #[test]
  fn test_builder_construction() {
    let config = Config::default_with_key("demo".to_string());
    let av = builder::AlphaVantage::new(config);
    assert_eq!(av.config().api_key, "demo");
  }

  #[test]
  fn test_error_from_core() {
    let core_err = av_core::Error::Config("test".to_string());
    let api_err: error::ApiError = core_err.into();
    assert!(api_err.to_string().contains("test"));
  }
}
