/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Cryptocurrency ID mapping and discovery.
//!
//! This module provides functionality for discovering and mapping cryptocurrency
//! identifiers across different data sources (CoinGecko, CoinPaprika, etc.).

pub mod discovery;
pub mod service;

pub use discovery::{discover_coingecko_id, discover_coinpaprika_id};
pub use service::{CryptoMappingService, MappingConfig, MappingRepository};