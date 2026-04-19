/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency ID mapping and discovery.
//!
//! Different cryptocurrency data providers use different identifiers for
//! the same coin (e.g., CoinGecko uses `"bitcoin"`, CoinPaprika uses
//! `"btc-bitcoin"`). This module provides:
//!
//! 1. **Discovery functions** that resolve a ticker symbol to a provider-
//!    specific ID by querying their coin-list APIs.
//! 2. **A mapping service** that orchestrates discovery, caching, and
//!    persistence of these cross-provider ID mappings.
//!
//! # Sub-modules
//!
//! ## [`discovery`] — Stateless API lookup functions
//!
//! | Function                    | Provider    | Match strategy             |
//! |-----------------------------|-------------|----------------------------|
//! | [`discover_coingecko_id`]   | CoinGecko   | Exact symbol match (lowercase) |
//! | [`discover_coinpaprika_id`] | CoinPaprika | Exact symbol match (uppercase) |
//!
//! Both functions fetch the full coin list from the provider's API and
//! scan for an exact match. They return `Ok(None)` if no match is found.
//!
//! ## [`service`] — Orchestration layer
//!
//! | Type                     | Purpose                                           |
//! |--------------------------|---------------------------------------------------|
//! | [`MappingConfig`]        | API keys and rate-limit delay configuration        |
//! | [`MappingRepository`]    | Database-agnostic trait for storing/retrieving mappings |
//! | [`CryptoMappingService`] | Service that discovers mappings and persists them via `MappingRepository` |
//!
//! The [`CryptoMappingService`] provides two main operations:
//! - [`get_coingecko_id`](CryptoMappingService::get_coingecko_id) — look up
//!   or discover a single symbol's CoinGecko ID.
//! - [`discover_missing_mappings`](CryptoMappingService::discover_missing_mappings) —
//!   batch-discover mappings for all symbols that lack them.

/// Stateless async functions for resolving ticker symbols to provider-specific
/// coin IDs via external API calls.
pub mod discovery;

/// The [`CryptoMappingService`] orchestration layer with configurable
/// rate limiting, a database-agnostic [`MappingRepository`] trait, and
/// batch discovery support.
pub mod service;

// ─── Convenience re-exports ─────────────────────────────────────────────────

/// Re-exported from [`discovery`]: CoinGecko and CoinPaprika ID lookup functions.
pub use discovery::{discover_coingecko_id, discover_coinpaprika_id};

/// Re-exported from [`service`]: the mapping service, its config, and the
/// repository trait.
pub use service::{CryptoMappingService, MappingConfig, MappingRepository};
