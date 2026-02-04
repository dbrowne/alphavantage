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

pub mod coingecko_details_loader;
pub mod crypto_news_loader;
pub mod database;
pub mod intraday_loader;
pub mod loader;
pub mod mapping_service;
pub mod markets_loader;
pub mod metadata_loader;
pub mod metadata_providers;
pub mod metadata_types;
pub mod social_loader;

pub mod sources;
pub mod types;

// Re-export the main loaders and types
pub use loader::CryptoSymbolLoader;
pub use markets_loader::{
  CryptoMarketData, CryptoMarketsConfig, CryptoMarketsInput, CryptoMarketsLoader,
  CryptoSymbolForMarkets,
};

pub use types::*;

pub use metadata_loader::CryptoMetadataLoader;
pub use metadata_types::{
  CryptoMetadataConfig, CryptoMetadataInput, CryptoMetadataOutput, CryptoSymbolForMetadata,
  MetadataSourceResult, ProcessedCryptoMetadata,
};

pub use intraday_loader::{
  CryptoIntradayConfig, CryptoIntradayInput, CryptoIntradayLoader, CryptoIntradayLoaderInput,
  CryptoIntradayLoaderOutput, CryptoIntradayOutput, CryptoIntradayPriceData,
  CryptoSymbolInfo as CryptoIntradaySymbolInfo,
};

pub use coingecko_details_loader::{
  CoinGeckoDetailsInput, CoinGeckoDetailsLoader, CoinGeckoDetailsOutput, CoinInfo,
  CryptoDetailedData, CryptoSocialData, CryptoTechnicalData,
};

pub use social_loader::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSocialLoader, CryptoSymbolForSocial,
  ProcessedSocialData, SocialLoaderResult,
};

pub use mapping_service::{
  CryptoMappingService, CryptoRepositoryMappingAdapter, MappingConfig, MappingRepository,
};

// Re-export error types from crypto-loaders
pub use crypto_loaders::{CryptoLoaderError, CryptoLoaderResult};

// Implement conversion from crypto-loaders error to local LoaderError
impl From<crypto_loaders::CryptoLoaderError> for crate::LoaderError {
  fn from(err: crypto_loaders::CryptoLoaderError) -> Self {
    use crypto_loaders::CryptoLoaderError as CLE;
    match err {
      CLE::RequestFailed(e) => crate::LoaderError::IoError(e.to_string()),
      CLE::JsonParseFailed(e) => crate::LoaderError::SerializationError(e.to_string()),
      CLE::RateLimitExceeded { retry_after_secs, .. } => {
        crate::LoaderError::RateLimitExceeded { retry_after: retry_after_secs.unwrap_or(60) }
      }
      CLE::ApiKeyMissing(source) => {
        crate::LoaderError::ConfigurationError(format!("API key missing for {}", source))
      }
      CLE::InvalidResponse { provider, message } => {
        crate::LoaderError::ApiError(format!("{}: {}", provider, message))
      }
      CLE::ServerError { provider, message } => {
        crate::LoaderError::ApiError(format!("{} server error: {}", provider, message))
      }
      CLE::AccessDenied { provider, message } => {
        crate::LoaderError::ApiError(format!("{} access denied: {}", provider, message))
      }
      CLE::SourceUnavailable(provider) => {
        crate::LoaderError::ApiError(format!("Provider unavailable: {}", provider))
      }
      CLE::NetworkError(msg) => crate::LoaderError::IoError(msg),
      CLE::ApiError { provider, message } => {
        crate::LoaderError::ApiError(format!("{}: {}", provider, message))
      }
      CLE::ParseError(msg) => crate::LoaderError::SerializationError(msg),
      CLE::CacheError(msg) => crate::LoaderError::DatabaseError(msg),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crypto_loader_error_conversion() {
    let crypto_err = CryptoLoaderError::ApiKeyMissing("CoinGecko".to_string());
    let loader_err: crate::LoaderError = crypto_err.into();

    assert!(
      matches!(loader_err, crate::LoaderError::ConfigurationError(ref msg) if msg.contains("CoinGecko")),
      "Expected ConfigurationError containing 'CoinGecko', got {:?}",
      loader_err
    );
  }

  #[test]
  fn test_rate_limit_error() {
    let crypto_err = CryptoLoaderError::RateLimitExceeded {
      provider: "CoinGecko".to_string(),
      retry_after_secs: Some(120),
    };
    let loader_err: crate::LoaderError = crypto_err.into();

    assert!(
      matches!(loader_err, crate::LoaderError::RateLimitExceeded { retry_after: 120 }),
      "Expected RateLimitExceeded with retry_after=120, got {:?}",
      loader_err
    );
  }

  #[test]
  fn test_rate_limit_error_default() {
    let crypto_err = CryptoLoaderError::RateLimitExceeded {
      provider: "CoinGecko".to_string(),
      retry_after_secs: None,
    };
    let loader_err: crate::LoaderError = crypto_err.into();

    assert!(
      matches!(loader_err, crate::LoaderError::RateLimitExceeded { retry_after: 60 }),
      "Expected RateLimitExceeded with default retry_after=60, got {:?}",
      loader_err
    );
  }

  #[test]
  fn test_api_error_conversion() {
    let crypto_err = CryptoLoaderError::ApiError {
      provider: "CoinMarketCap".to_string(),
      message: "invalid request".to_string(),
    };
    let loader_err: crate::LoaderError = crypto_err.into();

    assert!(
      matches!(loader_err, crate::LoaderError::ApiError(ref msg) if msg.contains("CoinMarketCap") && msg.contains("invalid request")),
      "Expected ApiError containing source and message, got {:?}",
      loader_err
    );
  }
}
