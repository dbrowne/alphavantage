/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Social data loading for cryptocurrencies.
//!
//! This module provides types and a loader for fetching social media metrics
//! (Twitter followers, Telegram members, Reddit subscribers, etc.) and
//! composite scores (CoinGecko score, developer score, etc.) for
//! cryptocurrency projects.
//!
//! # Status
//!
//! The [`SocialLoader`] is currently a **stub** — `load_data()` returns
//! empty [`ProcessedSocialData`] records. For production social-data loading,
//! use [`CoinGeckoDetailsLoader`](crate::loaders::CoinGeckoDetailsLoader)
//! instead, which provides a full implementation.
//!
//! # Type inventory
//!
//! | Type                      | Purpose                                          |
//! |---------------------------|--------------------------------------------------|
//! | [`SocialLoader`]          | Stub loader (pending full implementation)        |
//! | [`CryptoSocialConfig`]    | API keys, rate limits, batch size, retry config  |
//! | [`CryptoSocialInput`]     | Input: symbols to load + update-existing flag    |
//! | [`CryptoSymbolForSocial`] | Input DTO: sid, symbol, name, coingecko_id       |
//! | [`ProcessedSocialData`]   | Output: flat social data with `Decimal` scores   |
//! | [`SocialLoaderResult<T>`] | Type alias for `Result<T, CryptoLoaderError>`    |

/// Social data types, configuration, and the [`SocialLoader`] stub.
pub mod loader;

pub use loader::{
  CryptoSocialConfig, CryptoSocialInput, CryptoSymbolForSocial, ProcessedSocialData, SocialLoader,
  SocialLoaderResult,
};
