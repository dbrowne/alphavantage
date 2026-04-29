/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Convenience re-exports for glob import.
//!
//! ```rust
//! use av_api::prelude::*;
//! ```

// ── Always available ────────────────────────────────────────────────────
pub use crate::builder::AlphaVantage;
pub use crate::error::{ApiError, Result};
pub use av_core::types::{Exchange, Interval, OutputSize, SecurityType};
pub use av_core::{Config, FuncType};

// ── Client feature ──────────────────────────────────────────────────────
#[cfg(feature = "client")]
pub use av_client::AlphaVantageClient;

// ── Models feature (implied by client) ──────────────────────────────────
#[cfg(feature = "models")]
pub use av_models::{
  CompanyOverview, CryptoDaily, DailyAdjustedTimeSeries, DailyTimeSeries, ExchangeRate,
  IntradayTimeSeries, NewsSentiment, SymbolSearch, TopGainersLosers,
};

// ── Database feature ────────────────────────────────────────────────────
#[cfg(feature = "database")]
pub use av_database_postgres::DatabaseContext;

#[cfg(feature = "database")]
pub use crate::queries::{
  get_best_sid, get_sids, get_sids_by_type, security_snapshot, security_snapshot_by_sid,
  security_snapshots, security_snapshots_by_sector, SecuritySnapshot, SidEntry,
};

// ── Loaders feature ─────────────────────────────────────────────────────
#[cfg(feature = "loaders")]
pub use av_loaders::{
  DataLoader, IntradayPriceLoader, LoaderConfig, LoaderContext, NewsLoader, OverviewLoader,
  SecurityLoader, SummaryPriceLoader, TopMoversLoader,
};
