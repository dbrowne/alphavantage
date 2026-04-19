/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
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

//! # crypto-loaders
//!
//! Cryptocurrency data loading library for the `alphavantage` workspace.
//!
//! This crate aggregates cryptocurrency data from 5 external providers,
//! normalizes it into a unified symbol list, and provides loaders for
//! detailed social/technical data and cross-provider ID mappings.
//!
//! # Crate architecture
//!
//! ```text
//! crypto-loaders
//! ├── providers/       → CryptoDataProvider impls for 5 APIs
//! │   ├── coingecko    (Pro/Demo, 2-step: coin list + market rankings)
//! │   ├── coinmarketcap (API key required, single-request listings)
//! │   ├── coinpaprika  (free, active-coin filtering)
//! │   ├── coincap      (free, paginated assets)
//! │   └── sosovalue    (POST-based, API key required)
//! ├── loaders/         → High-level orchestration
//! │   ├── symbol_loader    → multi-provider symbol aggregation + dedup
//! │   └── details_loader   → CoinGecko detailed coin data (social + technical)
//! ├── mapping/         → Cross-provider ID discovery and persistence
//! │   ├── discovery        → stateless CoinGecko/CoinPaprika ID lookup
//! │   └── service          → orchestration with DB persistence + rate limiting
//! ├── metadata/        → Per-coin metadata loading with caching
//! │   ├── types            → config, input/output DTOs
//! │   └── coingecko_provider → CoinGecko /coins/{id} metadata
//! ├── social/          → Social data loading (stub — use details_loader)
//! ├── traits/          → CryptoDataProvider, CryptoCache trait definitions
//! ├── types/           → CryptoSymbol, CryptoDataSource, config structs
//! └── error/           → CryptoLoaderError enum
//! ```
//!
//! # Supported providers
//!
//! | Provider       | API key env var       | Key required? | Rate delay |
//! |----------------|-----------------------|---------------|------------|
//! | CoinGecko      | `COINGECKO_API_KEY`   | Yes           | 2000ms     |
//! | CoinMarketCap  | `CMC_API_KEY`         | Yes           | 300ms      |
//! | SosoValue      | `SOSOVALUE_API_KEY`   | Yes           | 500ms      |
//! | CoinPaprika    | —                     | No            | 500ms      |
//! | CoinCap        | —                     | No            | 200ms      |
//!
//! # Quick start
//!
//! ```rust,no_run
//! use crypto_loaders::prelude::*;
//!
//! # async fn example() -> Result<(), CryptoLoaderError> {
//! // Load symbols from all configured providers
//! let config = CryptoLoaderConfig::default();
//! let loader = CryptoSymbolLoader::new(config);
//! let result = loader.load_all_symbols().await?;
//!
//! println!("Loaded {} unique symbols", result.symbols_loaded);
//! # Ok(())
//! # }
//! ```
//!
//! # Prelude
//!
//! The [`prelude`] module re-exports the most commonly used types for
//! glob import: all 5 providers, core traits, config, error, symbol/source
//! types, and the symbol loader.

/// Error types: [`CryptoLoaderError`] enum with variants for API, parse,
/// rate-limit, network, and configuration errors.
pub mod error;

/// High-level loaders: [`CryptoSymbolLoader`] (multi-provider aggregation)
/// and [`CoinGeckoDetailsLoader`] (detailed social + technical data).
pub mod loaders;

/// Cross-provider ID mapping: [`discover_coingecko_id`] / [`discover_coinpaprika_id`]
/// functions and the [`CryptoMappingService`] orchestrator.
pub mod mapping;

/// Per-coin metadata loading: [`CoinGeckoMetadataProvider`] with caching
/// and configurable rate limiting.
pub mod metadata;

/// [`CryptoDataProvider`] implementations for 5 external APIs.
pub mod providers;

/// Social data loading (stub). For production use, prefer
/// [`CoinGeckoDetailsLoader`](loaders::CoinGeckoDetailsLoader).
pub mod social;

/// Core trait definitions: [`CryptoDataProvider`] and [`CryptoCache`].
pub mod traits;

/// Shared types: [`CryptoSymbol`], [`CryptoDataSource`],
/// [`CryptoLoaderConfig`], [`SourceResult`].
pub mod types;

// ─── Re-exports ─────────────────────────────────────────────────────────────

/// Error types.
pub use error::{CryptoLoaderError, CryptoLoaderResult};

/// Loader types: symbol loader + details loader with all associated structs.
pub use loaders::{
  CoinGeckoDetailedCoin, CoinGeckoDetailsLoader, CoinGeckoDetailsOutput, CoinInfo,
  CryptoDetailedData, CryptoSocialData, CryptoSymbolLoader, CryptoTechnicalData,
  DetailsLoaderConfig, LoadAllSymbolsResult,
};

/// Core traits.
pub use traits::{CryptoCache, CryptoDataProvider};

/// Shared types.
pub use types::{
  CryptoDataSource, CryptoLoaderConfig, CryptoLoaderResult as LoaderResult, CryptoSymbol,
  SourceResult,
};

/// All 5 provider implementations.
pub use providers::{
  CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider, SosoValueProvider,
};

/// Social data types and loader (stub).
pub use social::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSymbolForSocial, ProcessedSocialData, SocialLoader,
  SocialLoaderResult,
};

/// Mapping discovery functions and service.
pub use mapping::{
  CryptoMappingService, MappingConfig, MappingRepository, discover_coingecko_id,
  discover_coinpaprika_id,
};

/// Metadata loading types and CoinGecko provider.
pub use metadata::{
  CoinGeckoMetadataProvider, CryptoMetadataConfig, CryptoMetadataOutput, CryptoSymbolForMetadata,
  MetadataSourceResult, ProcessedCryptoMetadata,
};

/// Convenience prelude for glob imports.
///
/// ```rust
/// use crypto_loaders::prelude::*;
/// ```
///
/// Includes all 5 providers, core traits (`CryptoDataProvider`, `CryptoCache`),
/// config, error/result types, symbol/source types, and the symbol loader.
pub mod prelude {
  pub use crate::providers::{
    CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider,
    SosoValueProvider,
  };
  pub use crate::{
    CryptoCache, CryptoDataProvider, CryptoDataSource, CryptoLoaderConfig, CryptoLoaderError,
    CryptoLoaderResult, CryptoSymbol, CryptoSymbolLoader, LoadAllSymbolsResult, SourceResult,
  };
}
