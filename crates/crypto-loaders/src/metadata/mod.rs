/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency metadata loading.
//!
//! This module provides the configuration, data types, and provider
//! implementations for loading cryptocurrency metadata (market cap rank,
//! trading pairs, activity status, etc.) from external APIs.
//!
//! # Sub-modules
//!
//! ## [`types`] — Configuration and data structures
//!
//! | Type                        | Purpose                                              |
//! |-----------------------------|------------------------------------------------------|
//! | [`CryptoMetadataConfig`]    | API keys, rate limits, batch size, caching settings  |
//! | [`CryptoSymbolForMetadata`] | Input: which symbols to load metadata for            |
//! | [`ProcessedCryptoMetadata`] | Output: normalized metadata for one coin             |
//! | [`CryptoMetadataOutput`]    | Batch result: loaded metadata + per-source stats     |
//! | [`MetadataSourceResult`]    | Per-source success/error/timing counters             |
//!
//! ## [`coingecko_provider`] — CoinGecko metadata provider
//!
//! | Type                          | Purpose                                          |
//! |-------------------------------|--------------------------------------------------|
//! | [`CoinGeckoMetadataProvider`] | Fetches metadata from CoinGecko with caching     |
//!
//! The provider supports two loading modes:
//! - [`load_cached`](CoinGeckoMetadataProvider::load_cached) — checks a
//!   [`CryptoCache`](crate::traits::CryptoCache) first, falling back to API.
//! - [`load`](CoinGeckoMetadataProvider::load) — direct API fetch (no cache).
//!
//! # Data flow
//!
//! ```text
//! CryptoSymbolForMetadata[]
//!   └──► CoinGeckoMetadataProvider::load_cached()
//!          ├── cache check → hit? deserialize + return
//!          └── miss → CoinGecko /coins/markets API
//!                ├── parse → ProcessedCryptoMetadata[]
//!                ├── cache response (configurable TTL)
//!                └── return CryptoMetadataOutput
//! ```

/// CoinGecko-based metadata provider with response caching.
pub mod coingecko_provider;

/// Configuration, input/output DTOs, and result types for metadata loading.
pub mod types;

// ─── Convenience re-exports ─────────────────────────────────────────────────

/// Re-exported from [`coingecko_provider`].
pub use coingecko_provider::CoinGeckoMetadataProvider;

/// Re-exported from [`types`]: config, input/output, and per-source result structs.
pub use types::{
  CryptoMetadataConfig, CryptoMetadataOutput, CryptoSymbolForMetadata, MetadataSourceResult,
  ProcessedCryptoMetadata,
};
