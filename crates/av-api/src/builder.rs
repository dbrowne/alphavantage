/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Single entry point for the Alpha Vantage API.
//!
//! [`AlphaVantage`] wraps configuration and lazily constructs the HTTP
//! client (and optionally a database context) so that external programs
//! have one import and one construction site.
//!
//! # Examples
//!
//! ```rust,no_run
//! use av_api::prelude::*;
//!
//! # fn main() -> av_api::error::Result<()> {
//! // Minimal — just the API client
//! let av = AlphaVantage::from_env()?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "client")]
use crate::error::ApiError;
use crate::error::Result;
use av_core::Config;

/// Central entry point for the Alpha Vantage API.
///
/// Holds a [`Config`] and lazily constructs the underlying HTTP client on
/// first access. When the `database` feature is enabled, an optional
/// [`DatabaseContext`](av_database_postgres::DatabaseContext) can be
/// attached via [`with_database`](Self::with_database).
///
/// `AlphaVantage` is `Send + Sync` and can be shared across tasks via `Arc`.
pub struct AlphaVantage {
  config: Config,

  #[cfg(feature = "client")]
  client: std::sync::OnceLock<av_client::AlphaVantageClient>,

  #[cfg(feature = "database")]
  db: Option<av_database_postgres::DatabaseContext>,
}

impl AlphaVantage {
  /// Creates a new instance from an explicit [`Config`].
  pub fn new(config: Config) -> Self {
    Self {
      config,
      #[cfg(feature = "client")]
      client: std::sync::OnceLock::new(),
      #[cfg(feature = "database")]
      db: None,
    }
  }

  /// Creates a new instance by loading configuration from environment
  /// variables (delegates to [`Config::from_env`]).
  pub fn from_env() -> Result<Self> {
    let config = Config::from_env()?;
    Ok(Self::new(config))
  }

  /// Returns a reference to the underlying [`Config`].
  pub fn config(&self) -> &Config {
    &self.config
  }

  // ── Client (feature = "client") ─────────────────────────────────────

  /// Returns a reference to the lazily-initialized HTTP client.
  ///
  /// The client is created on first call and reused thereafter.
  #[cfg(feature = "client")]
  pub fn client(&self) -> Result<&av_client::AlphaVantageClient> {
    // OnceLock::get_or_try_init is unstable, so we init-then-get.
    if self.client.get().is_none() {
      let c = av_client::AlphaVantageClient::new(self.config.clone()).map_err(ApiError::Core)?;
      // Another thread may have raced us — that's fine, set returns Err
      // containing the value but we just discard it.
      let _ = self.client.set(c);
    }
    Ok(self.client.get().expect("client was just initialized"))
  }

  // ── Database (feature = "database") ─────────────────────────────────

  /// Attaches a database context by connecting to the given URL.
  ///
  /// Uses the default pool configuration (max 50 connections, 10 idle,
  /// 30s timeout). Fails fast if the database is unreachable.
  #[cfg(feature = "database")]
  pub fn with_database(mut self, database_url: &str) -> Result<Self> {
    let db = av_database_postgres::DatabaseContext::new(database_url)?;
    self.db = Some(db);
    Ok(self)
  }

  /// Attaches a pre-built [`DatabaseContext`](av_database_postgres::DatabaseContext).
  #[cfg(feature = "database")]
  pub fn with_database_context(mut self, db: av_database_postgres::DatabaseContext) -> Self {
    self.db = Some(db);
    self
  }

  /// Returns the attached database context, if any.
  #[cfg(feature = "database")]
  pub fn database(&self) -> Option<&av_database_postgres::DatabaseContext> {
    self.db.as_ref()
  }
}
