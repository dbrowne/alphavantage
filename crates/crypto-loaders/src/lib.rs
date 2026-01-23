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
//! Cryptocurrency data loading library for various exchanges and data providers.
//!
//! This crate provides data loaders for cryptocurrency data from sources like:
//! - CoinGecko
//! - CoinMarketCap
//! - CoinPaprika
//! - CoinCap
//! - SosoValue
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crypto_loaders::{
//!     CryptoDataProvider, CryptoSymbol, CryptoDataSource,
//!     providers::CoinGeckoProvider,
//! };
//!
//! // Create a provider
//! let provider = CoinGeckoProvider::new(Some("your-api-key".to_string()));
//!
//! // Fetch symbols
//! let client = reqwest::Client::new();
//! let symbols = provider.fetch_symbols(&client, None).await?;
//! ```

pub mod error;
pub mod loaders;
pub mod mapping;
pub mod metadata;
pub mod providers;
pub mod social;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use error::{CryptoLoaderError, CryptoLoaderResult};
pub use loaders::{
  CoinGeckoDetailsLoader, CoinGeckoDetailsOutput, CoinGeckoDetailedCoin, CoinInfo,
  CryptoDetailedData, CryptoSocialData, CryptoSymbolLoader, CryptoTechnicalData,
  DetailsLoaderConfig, LoadAllSymbolsResult,
};
pub use traits::{CryptoCache, CryptoDataProvider};
pub use types::{CryptoDataSource, CryptoLoaderConfig, CryptoLoaderResult as LoaderResult, CryptoSymbol, SourceResult};

// Re-export providers
pub use providers::{
  CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider, SosoValueProvider,
};

// Re-export social types
pub use social::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSymbolForSocial, ProcessedSocialData, SocialLoader,
  SocialLoaderResult,
};

// Re-export mapping types
pub use mapping::{
  discover_coingecko_id, discover_coinpaprika_id, CryptoMappingService, MappingConfig,
  MappingRepository,
};

// Re-export metadata types
pub use metadata::{
  CoinGeckoMetadataProvider, CryptoMetadataConfig, CryptoMetadataOutput, CryptoSymbolForMetadata,
  MetadataSourceResult, ProcessedCryptoMetadata,
};

pub mod prelude {
  pub use crate::{
    CryptoCache, CryptoDataProvider, CryptoDataSource, CryptoLoaderConfig, CryptoLoaderError,
    CryptoLoaderResult, CryptoSymbol, CryptoSymbolLoader, LoadAllSymbolsResult, SourceResult,
  };
  pub use crate::providers::{
    CoinCapProvider, CoinGeckoProvider, CoinMarketCapProvider, CoinPaprikaProvider,
    SosoValueProvider,
  };
}