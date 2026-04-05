/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Social data loading for cryptocurrencies.

pub mod loader;

pub use loader::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSymbolForSocial, ProcessedSocialData, SocialLoader,
  SocialLoaderResult,
};
