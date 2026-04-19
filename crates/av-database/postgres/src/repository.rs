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

//! Database repository abstraction layer.
//!
//! This is the largest module in the `av-database-postgres` crate. It defines
//! the connection pool, error types, caching infrastructure, and all
//! domain-specific repository traits with their implementations.
//!
//! # Module contents
//!
//! ## Infrastructure
//!
//! | Item                    | Kind       | Description                                         |
//! |-------------------------|------------|-----------------------------------------------------|
//! | [`DbPool`]              | Type alias | `r2d2::Pool<ConnectionManager<PgConnection>>`       |
//! | [`DbConnection`]        | Type alias | Pooled connection (leased from `DbPool`)            |
//! | [`RepositoryError`]     | Enum       | Unified error type with `From` impls for Diesel, r2d2, serde |
//! | [`RepositoryResult<T>`] | Type alias | `Result<T, RepositoryError>`                        |
//! | [`DatabaseContext`]     | Struct     | Central entry point — owns the pool, vends repositories |
//!
//! ## Traits
//!
//! | Trait                  | Description                                              |
//! |------------------------|----------------------------------------------------------|
//! | [`Repository<T>`]      | Generic async CRUD trait (find, insert, update, delete)  |
//! | [`Transactional`]      | Synchronous transaction support                          |
//! | [`CacheRepository`]    | Object-safe async cache (JSONB-based, TTL-aware)         |
//! | [`CacheRepositoryExt`] | Generic (type-safe) extension over `CacheRepository`     |
//! | [`OverviewRepository`] | Company overview CRUD + ingestion queue                  |
//! | [`NewsRepository`]     | Symbol lookups and missing-symbol tracking for news      |
//! | [`CryptoRepository`]   | API mappings, metadata, social/technical data, markets   |
//!
//! ## DTOs
//!
//! | Type                    | Description                                            |
//! |-------------------------|--------------------------------------------------------|
//! | [`CachedResponse<T>`]   | Wrapper with `cached_at` / `expires_at` metadata       |
//! | [`SymbolInfo`]          | Lightweight `(sid, symbol)` pair for overview loading  |
//! | [`OverviewSymbolFilter`]| Multi-criteria filter for selecting symbols to ingest  |
//!
//! # Async strategy
//!
//! All repository methods are `async` but internally use
//! [`tokio::task::spawn_blocking`] because the underlying `r2d2` pool and
//! Diesel queries are synchronous. Each method:
//! 1. Clones `Arc<DbPool>` and owned copies of string arguments.
//! 2. Moves them into a `spawn_blocking` closure.
//! 3. Acquires a connection from the pool and runs the Diesel query.
//! 4. Maps `JoinError` to [`RepositoryError::QueryError`].
//!
//! # Obtaining repositories
//!
//! All repositories are obtained from [`DatabaseContext`]:
//!
//! ```rust,no_run
//! use av_database_postgres::DatabaseContext;
//!
//! let db = DatabaseContext::new("postgres://localhost/alphavantage").unwrap();
//!
//! let overview_repo = db.overview_repository();
//! let news_repo = db.news_repository();
//! let crypto_repo = db.crypto_repository();
//! let cache_repo = db.cache_repository();
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::Error as DieselError;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::models::crypto::CryptoSummary;

// ─── Pool type aliases ──────────────────────────────────────────────────────

/// An `r2d2` connection pool of synchronous PostgreSQL connections.
///
/// Shared across repositories via `Arc<DbPool>`.
pub type DbPool = Pool<ConnectionManager<PgConnection>>;

/// A single connection leased from the [`DbPool`].
///
/// Returned to the pool when dropped.
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

// ─── Pool defaults ──────────────────────────────────────────────────────────

/// Maximum number of connections in the pool.
const MAX_POOL_SIZE: u32 = 50;
/// Minimum number of idle connections kept open.
const MIN_POOL_IDLE: u32 = 10;
/// Connection acquisition timeout in seconds — the pool returns an error
/// instead of blocking indefinitely.
const CONNECTION_TIMEOUT_SECS: u64 = 30;

// ─── Error types ────────────────────────────────────────────────────────────

/// Unified error type for all repository operations.
///
/// Provides automatic `From` conversions for:
/// - [`DieselError`] — maps `NotFound` to [`NotFound`](RepositoryError::NotFound),
///   unique/FK violations to [`ConstraintViolation`](RepositoryError::ConstraintViolation),
///   and everything else to [`QueryError`](RepositoryError::QueryError).
/// - [`r2d2::PoolError`](diesel::r2d2::PoolError) → [`PoolError`](RepositoryError::PoolError).
/// - [`serde_json::Error`] → [`SerializationError`](RepositoryError::SerializationError).
#[derive(Error, Debug)]
pub enum RepositoryError {
  #[error("Connection pool error: {0}")]
  PoolError(String),

  #[error("Database query error: {0}")]
  QueryError(String),

  #[error("Insert error: {0}")]
  InsertError(String),

  #[error("Serialization error: {0}")]
  SerializationError(String),

  #[error("Not found: {0}")]
  NotFound(String),

  #[error("Constraint violation: {0}")]
  ConstraintViolation(String),

  #[error("Transaction error: {0}")]
  TransactionError(String),
}

impl From<DieselError> for RepositoryError {
  fn from(err: DieselError) -> Self {
    match err {
      DieselError::NotFound => RepositoryError::NotFound("Record not found".to_string()),
      DieselError::DatabaseError(kind, info) => match kind {
        diesel::result::DatabaseErrorKind::UniqueViolation => {
          RepositoryError::ConstraintViolation(info.message().to_string())
        }
        diesel::result::DatabaseErrorKind::ForeignKeyViolation => {
          RepositoryError::ConstraintViolation(info.message().to_string())
        }
        _ => RepositoryError::QueryError(info.message().to_string()),
      },
      _ => RepositoryError::QueryError(err.to_string()),
    }
  }
}

impl From<diesel::r2d2::PoolError> for RepositoryError {
  fn from(err: diesel::r2d2::PoolError) -> Self {
    RepositoryError::PoolError(err.to_string())
  }
}

impl From<serde_json::Error> for RepositoryError {
  fn from(err: serde_json::Error) -> Self {
    RepositoryError::SerializationError(err.to_string())
  }
}

/// Convenience alias: `Result<T, RepositoryError>`.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

// ─── Caching ────────────────────────────────────────────────────────────────

/// Wrapper that pairs cached data with TTL metadata.
///
/// Used by the generic [`CacheRepositoryExt::get`] / [`CacheRepositoryExt::set`]
/// methods. `expires_at` determines when the entry is considered stale and
/// eligible for cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse<T> {
  pub data: T,
  pub cached_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
}

/// Object-safe async trait for API response caching.
///
/// All values are stored as [`serde_json::Value`] (JSONB in PostgreSQL) to
/// keep the trait object-safe. For type-safe get/set with automatic
/// serde conversion, use the [`CacheRepositoryExt`] extension trait.
///
/// Backed by the `api_response_cache` table.
#[async_trait]
pub trait CacheRepository: Send + Sync {
  /// Get a cached response by key, returns JSON value
  async fn get_json(
    &self,
    cache_key: &str,
    api_source: &str,
  ) -> RepositoryResult<Option<serde_json::Value>>;

  /// Set a cached response from JSON value
  async fn set_json(
    &self,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    data: serde_json::Value,
    ttl_hours: i64,
  ) -> RepositoryResult<()>;

  /// Delete expired cache entries
  async fn cleanup_expired(&self, api_source: &str) -> RepositoryResult<usize>;

  /// Delete specific cache entry
  async fn delete(&self, cache_key: &str) -> RepositoryResult<bool>;

  /// Check if cache entry exists and is not expired
  async fn exists(&self, cache_key: &str) -> RepositoryResult<bool>;
}

/// Type-safe extension trait for [`CacheRepository`].
///
/// Adds generic `get<T>` / `set<T>` methods that automatically serialize
/// and deserialize via `serde_json`. Blanket-implemented for all
/// `CacheRepository` implementors — no manual implementation needed.
#[allow(async_fn_in_trait)]
pub trait CacheRepositoryExt: CacheRepository {
  /// Get a cached response with automatic deserialization
  async fn get<T>(&self, cache_key: &str, api_source: &str) -> RepositoryResult<Option<T>>
  where
    T: for<'de> Deserialize<'de> + Send + 'static,
  {
    match self.get_json(cache_key, api_source).await? {
      Some(json) => {
        let data: T = serde_json::from_value(json)?;
        Ok(Some(data))
      }
      None => Ok(None),
    }
  }

  /// Set a cached response with automatic serialization
  async fn set<T>(
    &self,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    data: &T,
    ttl_hours: i64,
  ) -> RepositoryResult<()>
  where
    T: Serialize + Send + Sync,
  {
    let json = serde_json::to_value(data)?;
    self.set_json(cache_key, api_source, endpoint_url, json, ttl_hours).await
  }
}

// Automatically implement CacheRepositoryExt for all types implementing CacheRepository
impl<T: CacheRepository + ?Sized> CacheRepositoryExt for T {}

// ─── Generic CRUD trait ─────────────────────────────────────────────────────

/// Generic async CRUD repository trait.
///
/// Defines the standard operations (`find_by_id`, `find_all`, `insert`,
/// `insert_batch`, `update`, `delete`, `count`) for any entity type `T`.
/// Not currently implemented by any concrete type in this module — included
/// as a contract for future domain repositories.
#[async_trait]
pub trait Repository<T>: Send + Sync
where
  T: Send + Sync,
{
  /// Find a single record by ID
  async fn find_by_id(&self, id: i64) -> RepositoryResult<Option<T>>;

  /// Find all records with optional filtering
  async fn find_all(&self, limit: Option<i64>) -> RepositoryResult<Vec<T>>;

  /// Insert a single record
  async fn insert(&self, entity: &T) -> RepositoryResult<T>;

  /// Insert multiple records in a batch
  async fn insert_batch(&self, entities: &[T]) -> RepositoryResult<usize>;

  /// Update a record
  async fn update(&self, id: i64, entity: &T) -> RepositoryResult<T>;

  /// Delete a record
  async fn delete(&self, id: i64) -> RepositoryResult<bool>;

  /// Count total records
  async fn count(&self) -> RepositoryResult<i64>;
}

/// Synchronous transaction support trait.
///
/// Provides a `with_transaction` method that runs a closure within a
/// database transaction. For the `DatabaseContext` equivalent, use
/// [`DatabaseContext::transaction`].
pub trait Transactional {
  /// Execute operations within a transaction
  fn with_transaction<F, R>(&self, f: F) -> RepositoryResult<R>
  where
    F: FnOnce(&mut DbConnection) -> RepositoryResult<R>;
}

// ─── DatabaseContext ────────────────────────────────────────────────────────

/// Central entry point for all database operations.
///
/// Owns an `Arc<DbPool>` and provides factory methods to obtain
/// domain-specific repositories:
///
/// | Method                  | Returns                    |
/// |-------------------------|----------------------------|
/// | [`overview_repository`] | `impl OverviewRepository`  |
/// | [`news_repository`]     | `impl NewsRepository`      |
/// | [`crypto_repository`]   | `impl CryptoRepository`    |
/// | [`cache_repository`]    | `impl CacheRepository`     |
///
/// Also provides direct pool access ([`get_connection`], [`pool`]),
/// transaction support ([`transaction`]), and an async helper ([`run`]).
///
/// `DatabaseContext` is `Clone` (cheap — only clones the `Arc`).
///
/// [`overview_repository`]: DatabaseContext::overview_repository
/// [`news_repository`]: DatabaseContext::news_repository
/// [`crypto_repository`]: DatabaseContext::crypto_repository
/// [`cache_repository`]: DatabaseContext::cache_repository
/// [`get_connection`]: DatabaseContext::get_connection
/// [`pool`]: DatabaseContext::pool
/// [`transaction`]: DatabaseContext::transaction
/// [`run`]: DatabaseContext::run
#[derive(Clone)]
pub struct DatabaseContext {
  pool: Arc<DbPool>,
}

impl DatabaseContext {
  /// Creates a new database context with an `r2d2` connection pool.
  ///
  /// **Fails fast:** tests the connection *before* building the pool.
  /// If the database is unreachable, returns an error immediately rather
  /// than letting `r2d2` spawn background retry threads.
  ///
  /// Uses default pool settings: max size = 50, min idle = 10,
  /// timeout = 30s. For custom settings, use [`with_pool_config`](Self::with_pool_config).
  pub fn new(database_url: &str) -> RepositoryResult<Self> {
    // Test connection BEFORE creating the pool to fail fast without background retry noise
    PgConnection::establish(database_url)
      .map_err(|e| RepositoryError::PoolError(format!("Failed to connect to database: {}", e)))?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
      .max_size(MAX_POOL_SIZE)
      .min_idle(Some(MIN_POOL_IDLE))
      .connection_timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS))
      .build(manager)
      .map_err(|e| RepositoryError::PoolError(e.to_string()))?;

    Ok(Self { pool: Arc::new(pool) })
  }

  /// Creates a context with custom pool size, using the default timeout.
  ///
  /// Fails fast (see [`new`](Self::new)).
  pub fn with_pool_config(
    database_url: &str,
    max_size: u32,
    min_idle: u32,
  ) -> RepositoryResult<Self> {
    Self::with_pool_config_and_timeout(database_url, max_size, min_idle, CONNECTION_TIMEOUT_SECS)
  }

  /// Creates a context with fully custom pool configuration.
  ///
  /// Fails fast (see [`new`](Self::new)).
  ///
  /// # Arguments
  ///
  /// - `max_size` — maximum connections in the pool.
  /// - `min_idle` — minimum idle connections kept open.
  /// - `timeout_secs` — connection acquisition timeout.
  pub fn with_pool_config_and_timeout(
    database_url: &str,
    max_size: u32,
    min_idle: u32,
    timeout_secs: u64,
  ) -> RepositoryResult<Self> {
    // Test connection BEFORE creating the pool to fail fast without background retry noise
    PgConnection::establish(database_url)
      .map_err(|e| RepositoryError::PoolError(format!("Failed to connect to database: {}", e)))?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
      .max_size(max_size)
      .min_idle(Some(min_idle))
      .connection_timeout(Duration::from_secs(timeout_secs))
      .build(manager)
      .map_err(|e| RepositoryError::PoolError(e.to_string()))?;

    Ok(Self { pool: Arc::new(pool) })
  }

  /// Leases a connection from the pool.
  ///
  /// Blocks up to the configured timeout. Returns
  /// [`RepositoryError::PoolError`] if the pool is exhausted.
  pub fn get_connection(&self) -> RepositoryResult<DbConnection> {
    self.pool.get().map_err(|e| RepositoryError::PoolError(e.to_string()))
  }

  /// Returns a reference to the underlying `r2d2` pool.
  ///
  /// Useful for passing to [`SymbolRepository::new`](crate::SymbolRepository::new)
  /// or other consumers that need `Arc<DbPool>`.
  pub fn pool(&self) -> &DbPool {
    &self.pool
  }

  /// Returns a [`CacheRepository`] backed by the `api_response_cache` table.
  pub fn cache_repository(&self) -> impl CacheRepository {
    CacheRepositoryImpl { pool: Arc::clone(&self.pool) }
  }

  /// Executes a closure within a database transaction.
  ///
  /// Acquires a connection, begins a transaction, calls `f`, and commits
  /// on `Ok` or rolls back on `Err`.
  pub fn transaction<F, R>(&self, f: F) -> RepositoryResult<R>
  where
    F: FnOnce(&mut DbConnection) -> RepositoryResult<R>,
  {
    let mut conn = self.get_connection()?;
    conn.transaction(|conn| f(conn)).map_err(|e| RepositoryError::TransactionError(e.to_string()))
  }

  /// Runs a synchronous database closure on a blocking thread.
  ///
  /// This is the building block used by all repository methods. The closure
  /// receives a `&mut DbConnection` and can run arbitrary Diesel queries.
  pub async fn run<F, R>(&self, f: F) -> RepositoryResult<R>
  where
    F: FnOnce(&mut DbConnection) -> RepositoryResult<R> + Send + 'static,
    R: Send + 'static,
  {
    let pool = Arc::clone(&self.pool);
    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get().map_err(|e| RepositoryError::PoolError(e.to_string()))?;
      f(&mut conn)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}

/// Private implementation of [`CacheRepository`] backed by
/// the `api_response_cache` table. Uses raw SQL queries via
/// `diesel::sql_query` for JSONB operations.
struct CacheRepositoryImpl {
  pool: Arc<DbPool>,
}

#[async_trait]
impl CacheRepository for CacheRepositoryImpl {
  async fn get_json(
    &self,
    cache_key: &str,
    api_source: &str,
  ) -> RepositoryResult<Option<serde_json::Value>> {
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();
    let api_source = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::{Jsonb, Text};

      let mut conn = pool.get()?;

      #[derive(QueryableByName)]
      struct CacheEntry {
        #[diesel(sql_type = Jsonb)]
        response_data: serde_json::Value,
      }

      let result: Option<CacheEntry> = sql_query(
        "SELECT response_data FROM api_response_cache
         WHERE cache_key = $1 AND api_source = $2 AND expires_at > NOW()",
      )
      .bind::<Text, _>(&cache_key)
      .bind::<Text, _>(&api_source)
      .get_result(&mut conn)
      .optional()?;

      Ok(result.map(|entry| entry.response_data))
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn set_json(
    &self,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    data: serde_json::Value,
    ttl_hours: i64,
  ) -> RepositoryResult<()> {
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();
    let api_source = api_source.to_string();
    let endpoint_url = endpoint_url.to_string();
    let expires_at = Utc::now() + chrono::Duration::hours(ttl_hours);

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::{Integer, Jsonb, Text, Timestamptz};

      let mut conn = pool.get()?;

      sql_query(
        "INSERT INTO api_response_cache
         (cache_key, api_source, endpoint_url, response_data, status_code, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (cache_key) DO UPDATE SET
           response_data = EXCLUDED.response_data,
           status_code = EXCLUDED.status_code,
           expires_at = EXCLUDED.expires_at,
           cached_at = NOW()",
      )
      .bind::<Text, _>(&cache_key)
      .bind::<Text, _>(&api_source)
      .bind::<Text, _>(&endpoint_url)
      .bind::<Jsonb, _>(&data)
      .bind::<Integer, _>(200)
      .bind::<Timestamptz, _>(expires_at)
      .execute(&mut conn)?;

      Ok(())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn cleanup_expired(&self, api_source: &str) -> RepositoryResult<usize> {
    let pool = Arc::clone(&self.pool);
    let api_source = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::Text;

      let mut conn = pool.get()?;

      let deleted = sql_query(
        "DELETE FROM api_response_cache
         WHERE api_source = $1 AND expires_at < NOW()",
      )
      .bind::<Text, _>(&api_source)
      .execute(&mut conn)?;

      Ok(deleted)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn delete(&self, cache_key: &str) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::Text;

      let mut conn = pool.get()?;

      let deleted = sql_query("DELETE FROM api_response_cache WHERE cache_key = $1")
        .bind::<Text, _>(&cache_key)
        .execute(&mut conn)?;

      Ok(deleted > 0)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn exists(&self, cache_key: &str) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::{BigInt, Text};

      let mut conn = pool.get()?;

      #[derive(QueryableByName)]
      struct CountResult {
        #[diesel(sql_type = BigInt)]
        count: i64,
      }

      let result: CountResult = sql_query(
        "SELECT COUNT(*) as count FROM api_response_cache
         WHERE cache_key = $1 AND expires_at > NOW()",
      )
      .bind::<Text, _>(&cache_key)
      .get_result(&mut conn)?;

      Ok(result.count > 0)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}

// ─── DTOs ───────────────────────────────────────────────────────────────────

/// Lightweight symbol reference returned by [`OverviewRepository::get_symbols_to_load`].
///
/// Contains only the `sid` and `symbol` string — sufficient for the
/// overview ingestion pipeline to identify what to fetch.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
  pub sid: i64,
  pub symbol: String,
}

/// Multi-criteria filter for selecting which symbols need overview ingestion.
///
/// Used by [`OverviewRepository::get_symbols_to_load`]. All criteria are
/// combined with `AND`. The [`Default`] implementation targets U.S. equities
/// without existing overviews.
///
/// # Fields
///
/// | Field                    | Type                  | Default                | Description                      |
/// |--------------------------|-----------------------|------------------------|----------------------------------|
/// | `symbols`                | `Option<Vec<String>>` | `None`                 | Restrict to specific tickers     |
/// | `sec_type`               | `Option<String>`      | `Some("Equity")`       | Security type filter             |
/// | `region`                 | `Option<String>`      | `Some("USA")`          | Region filter                    |
/// | `missing_overviews_only` | `bool`                | `true`                 | Only symbols with `overview = false` |
/// | `limit`                  | `Option<usize>`       | `None`                 | Max results                      |
#[derive(Debug, Clone)]
pub struct OverviewSymbolFilter {
  /// Restrict to specific ticker strings. If `None`, all symbols matching
  /// other criteria are returned.
  pub symbols: Option<Vec<String>>,
  /// Filter by security type (e.g., `"Equity"`, `"ETF"`).
  pub sec_type: Option<String>,
  /// Filter by geographic region (e.g., `"USA"`).
  pub region: Option<String>,
  /// When `true`, only return symbols where `overview = false`.
  pub missing_overviews_only: bool,
  /// Cap the number of results returned.
  pub limit: Option<usize>,
}

impl Default for OverviewSymbolFilter {
  fn default() -> Self {
    Self {
      symbols: None,
      sec_type: Some("Equity".to_string()),
      region: Some("USA".to_string()),
      missing_overviews_only: true,
      limit: None,
    }
  }
}

// ─── Overview repository ────────────────────────────────────────────────────

/// Async trait for company overview ingestion and queries.
///
/// Obtained via [`DatabaseContext::overview_repository`]. Handles the
/// full overview lifecycle: selecting symbols to ingest, saving overview
/// + extended overview pairs (with upsert), and tracking which symbols
/// have been processed.
#[async_trait]
pub trait OverviewRepository: Send + Sync {
  /// Get symbols that need overviews based on filter criteria
  async fn get_symbols_to_load(
    &self,
    filter: &OverviewSymbolFilter,
  ) -> RepositoryResult<Vec<SymbolInfo>>;

  /// Save a single overview (both main and extended records)
  /// Returns true if saved, false if skipped due to constraints
  async fn save_overview(
    &self,
    overview: &crate::models::security::NewOverviewOwned,
    overview_ext: &crate::models::security::NewOverviewextOwned,
  ) -> RepositoryResult<bool>;

  /// Save multiple overviews in a single transaction
  /// Returns the number of overviews successfully saved
  async fn batch_save_overviews(
    &self,
    overviews: &[(
      crate::models::security::NewOverviewOwned,
      crate::models::security::NewOverviewextOwned,
    )],
  ) -> RepositoryResult<usize>;

  /// Check if a symbol has an overview
  async fn has_overview(&self, sid: i64) -> RepositoryResult<bool>;

  /// Mark symbol as having overview data
  async fn mark_symbol_has_overview(&self, sid: i64) -> RepositoryResult<bool>;
}

/// Private implementation of [`OverviewRepository`].
struct OverviewRepositoryImpl {
  pool: Arc<DbPool>,
}

#[async_trait]
impl OverviewRepository for OverviewRepositoryImpl {
  async fn get_symbols_to_load(
    &self,
    filter: &OverviewSymbolFilter,
  ) -> RepositoryResult<Vec<SymbolInfo>> {
    let pool = Arc::clone(&self.pool);
    let filter = filter.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::symbols::dsl::*;

      let mut conn = pool.get()?;

      let mut query = symbols.into_boxed();

      // Apply filters
      if let Some(ref symbol_list) = filter.symbols {
        query = query.filter(symbol.eq_any(symbol_list));
      }

      if let Some(ref sec_type_val) = filter.sec_type {
        query = query.filter(sec_type.eq(sec_type_val));
      }

      if let Some(ref region_val) = filter.region {
        query = query.filter(region.eq(region_val));
      }

      if filter.missing_overviews_only {
        query = query.filter(overview.eq(false));
      }

      query = query.order(symbol.asc());

      if let Some(limit_val) = filter.limit {
        query = query.limit(limit_val as i64);
      }

      let results: Vec<crate::models::Symbol> = query.load(&mut conn)?;

      Ok(results.into_iter().map(|s| SymbolInfo { sid: s.sid, symbol: s.symbol }).collect())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn save_overview(
    &self,
    overview: &crate::models::security::NewOverviewOwned,
    overview_ext: &crate::models::security::NewOverviewextOwned,
  ) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);
    let overview = overview.clone();
    let overview_ext = overview_ext.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{overviewexts, overviews, symbols};

      let mut conn = pool.get()?;

      conn.transaction(|conn| {
        // Save main overview
        diesel::insert_into(overviews::table)
          .values(&overview)
          .on_conflict(overviews::sid)
          .do_update()
          .set(&overview)
          .execute(conn)?;

        // Save extended overview
        diesel::insert_into(overviewexts::table)
          .values(&overview_ext)
          .on_conflict(overviewexts::sid)
          .do_update()
          .set(&overview_ext)
          .execute(conn)?;

        // Update symbols table
        diesel::update(symbols::table.filter(symbols::sid.eq(overview.sid)))
          .set(symbols::overview.eq(true))
          .execute(conn)?;

        Ok(true)
      })
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn batch_save_overviews(
    &self,
    overviews: &[(
      crate::models::security::NewOverviewOwned,
      crate::models::security::NewOverviewextOwned,
    )],
  ) -> RepositoryResult<usize> {
    let pool = Arc::clone(&self.pool);
    let overviews = overviews.to_vec();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{overviewexts, overviews as overviews_table, symbols};
      use diesel::upsert::excluded;

      let mut conn = pool.get()?;

      // PostgreSQL has a limit of 65535 parameters per query.
      // NewOverviewOwned has ~22 columns, NewOverviewextOwned has ~27 columns.
      // Use batch size of 500 to stay well under the limit (500 * 27 = 13,500 params).
      // todo!:: Add progress bar when running attached to a terminal
      const BATCH_SIZE: usize = 500;

      conn.transaction(|conn| {
        let total = overviews.len();

        for chunk in overviews.chunks(BATCH_SIZE) {
          let overview_records: Vec<_> = chunk.iter().map(|(ov, _)| ov.clone()).collect();
          let overview_ext_records: Vec<_> =
            chunk.iter().map(|(_, ov_ext)| ov_ext.clone()).collect();
          let sids: Vec<i64> = chunk.iter().map(|(ov, _)| ov.sid).collect();

          // Batch insert/update overviews
          diesel::insert_into(overviews_table::table)
            .values(&overview_records)
            .on_conflict(overviews_table::sid)
            .do_update()
            .set((
              overviews_table::symbol.eq(excluded(overviews_table::symbol)),
              overviews_table::name.eq(excluded(overviews_table::name)),
              overviews_table::description.eq(excluded(overviews_table::description)),
              overviews_table::cik.eq(excluded(overviews_table::cik)),
              overviews_table::exchange.eq(excluded(overviews_table::exchange)),
              overviews_table::currency.eq(excluded(overviews_table::currency)),
              overviews_table::country.eq(excluded(overviews_table::country)),
              overviews_table::sector.eq(excluded(overviews_table::sector)),
              overviews_table::industry.eq(excluded(overviews_table::industry)),
              overviews_table::address.eq(excluded(overviews_table::address)),
              overviews_table::fiscal_year_end.eq(excluded(overviews_table::fiscal_year_end)),
              overviews_table::latest_quarter.eq(excluded(overviews_table::latest_quarter)),
              overviews_table::market_capitalization
                .eq(excluded(overviews_table::market_capitalization)),
              overviews_table::ebitda.eq(excluded(overviews_table::ebitda)),
              overviews_table::pe_ratio.eq(excluded(overviews_table::pe_ratio)),
              overviews_table::peg_ratio.eq(excluded(overviews_table::peg_ratio)),
              overviews_table::book_value.eq(excluded(overviews_table::book_value)),
              overviews_table::dividend_per_share.eq(excluded(overviews_table::dividend_per_share)),
              overviews_table::dividend_yield.eq(excluded(overviews_table::dividend_yield)),
              overviews_table::eps.eq(excluded(overviews_table::eps)),
            ))
            .execute(conn)?;

          // Batch insert/update overview extensions
          diesel::insert_into(overviewexts::table)
            .values(&overview_ext_records)
            .on_conflict(overviewexts::sid)
            .do_update()
            .set((
              overviewexts::revenue_per_share_ttm.eq(excluded(overviewexts::revenue_per_share_ttm)),
              overviewexts::profit_margin.eq(excluded(overviewexts::profit_margin)),
              overviewexts::operating_margin_ttm.eq(excluded(overviewexts::operating_margin_ttm)),
              overviewexts::return_on_assets_ttm.eq(excluded(overviewexts::return_on_assets_ttm)),
              overviewexts::return_on_equity_ttm.eq(excluded(overviewexts::return_on_equity_ttm)),
              overviewexts::revenue_ttm.eq(excluded(overviewexts::revenue_ttm)),
              overviewexts::gross_profit_ttm.eq(excluded(overviewexts::gross_profit_ttm)),
              overviewexts::diluted_eps_ttm.eq(excluded(overviewexts::diluted_eps_ttm)),
              overviewexts::quarterly_earnings_growth_yoy
                .eq(excluded(overviewexts::quarterly_earnings_growth_yoy)),
              overviewexts::quarterly_revenue_growth_yoy
                .eq(excluded(overviewexts::quarterly_revenue_growth_yoy)),
              overviewexts::analyst_target_price.eq(excluded(overviewexts::analyst_target_price)),
              overviewexts::trailing_pe.eq(excluded(overviewexts::trailing_pe)),
              overviewexts::forward_pe.eq(excluded(overviewexts::forward_pe)),
              overviewexts::price_to_sales_ratio_ttm
                .eq(excluded(overviewexts::price_to_sales_ratio_ttm)),
              overviewexts::price_to_book_ratio.eq(excluded(overviewexts::price_to_book_ratio)),
              overviewexts::ev_to_revenue.eq(excluded(overviewexts::ev_to_revenue)),
              overviewexts::ev_to_ebitda.eq(excluded(overviewexts::ev_to_ebitda)),
              overviewexts::beta.eq(excluded(overviewexts::beta)),
              overviewexts::week_high_52.eq(excluded(overviewexts::week_high_52)),
              overviewexts::week_low_52.eq(excluded(overviewexts::week_low_52)),
              overviewexts::day_moving_average_50.eq(excluded(overviewexts::day_moving_average_50)),
              overviewexts::day_moving_average_200
                .eq(excluded(overviewexts::day_moving_average_200)),
              overviewexts::shares_outstanding.eq(excluded(overviewexts::shares_outstanding)),
              overviewexts::dividend_date.eq(excluded(overviewexts::dividend_date)),
              overviewexts::ex_dividend_date.eq(excluded(overviewexts::ex_dividend_date)),
            ))
            .execute(conn)?;

          // Batch update symbols table
          diesel::update(symbols::table.filter(symbols::sid.eq_any(&sids)))
            .set(symbols::overview.eq(true))
            .execute(conn)?;
        }

        Ok(total)
      })
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn has_overview(&self, sid: i64) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      use crate::schema::symbols::dsl;

      let mut conn = pool.get()?;

      let overview_flag: bool = dsl::symbols
        .filter(dsl::sid.eq(sid))
        .select(dsl::overview)
        .first(&mut conn)
        .optional()?
        .unwrap_or(false);

      Ok(overview_flag)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn mark_symbol_has_overview(&self, sid: i64) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      use crate::schema::symbols;

      let mut conn = pool.get()?;

      let updated = diesel::update(symbols::table.filter(symbols::sid.eq(sid)))
        .set(symbols::overview.eq(true))
        .execute(&mut conn)?;

      Ok(updated > 0)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}

impl DatabaseContext {
  /// Returns an [`OverviewRepository`] for company overview operations.
  pub fn overview_repository(&self) -> impl OverviewRepository {
    OverviewRepositoryImpl { pool: Arc::clone(&self.pool) }
  }

  /// Returns a [`NewsRepository`] for news-related symbol lookups.
  pub fn news_repository(&self) -> impl NewsRepository {
    NewsRepositoryImpl { pool: Arc::clone(&self.pool) }
  }
}

// ─── News repository ────────────────────────────────────────────────────────

/// Async trait for news-related symbol lookups and missing-symbol tracking.
///
/// Obtained via [`DatabaseContext::news_repository`]. Provides the symbol
/// mapping that the news ingestion pipeline needs to resolve ticker
/// mentions to internal `sid` values.
#[async_trait]
pub trait NewsRepository: Send + Sync {
  /// Get all symbols as a mapping from symbol string to SID
  /// Used for mapping news ticker sentiments to database symbols
  async fn get_all_symbols(&self) -> RepositoryResult<HashMap<String, i64>>;

  /// Get equity symbols that have overview=true
  /// Returns list of (sid, symbol) pairs
  async fn get_equity_symbols_with_overview(&self) -> RepositoryResult<Vec<(i64, String)>>;

  /// Record or increment count for a ticker that was mentioned in news but not found in database
  /// Returns true if recorded/incremented successfully
  async fn record_missing_symbol(&self, ticker: &str, source: &str) -> RepositoryResult<bool>;

  /// Get missing symbols statistics
  /// Returns list of (symbol, source, seen_count, first_seen_at, last_seen_at)
  async fn get_missing_symbols(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(String, String, i32, chrono::NaiveDateTime, chrono::NaiveDateTime)>>;
}

/// Private implementation of [`NewsRepository`].
struct NewsRepositoryImpl {
  pool: Arc<DbPool>,
}

#[async_trait]
impl NewsRepository for NewsRepositoryImpl {
  async fn get_all_symbols(&self) -> RepositoryResult<HashMap<String, i64>> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      use crate::schema::symbols::dsl::*;

      let mut conn = pool.get()?;

      let results: Vec<(String, i64)> = symbols.select((symbol, sid)).load(&mut conn)?;

      Ok(results.into_iter().collect())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_equity_symbols_with_overview(&self) -> RepositoryResult<Vec<(i64, String)>> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      use crate::schema::symbols::dsl::*;

      let mut conn = pool.get()?;

      let results: Vec<(i64, String)> = symbols
        .filter(overview.eq(true))
        .filter(sec_type.eq("Equity"))
        .select((sid, symbol))
        .load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn record_missing_symbol(&self, ticker: &str, source: &str) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);
    let ticker = ticker.to_string();
    let source = source.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;

      crate::models::MissingSymbol::record_or_increment(&mut conn, &ticker, &source).map_err(
        |e| RepositoryError::InsertError(format!("Failed to record missing symbol: {}", e)),
      )?;

      Ok(true)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_missing_symbols(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(String, String, i32, chrono::NaiveDateTime, chrono::NaiveDateTime)>>
  {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      use crate::schema::missing_symbols::dsl::*;

      let mut conn = pool.get()?;

      let mut query = missing_symbols
        .select((symbol, source, seen_count, first_seen_at, last_seen_at))
        .order(seen_count.desc())
        .into_boxed();

      if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
      }

      let results: Vec<(String, String, i32, chrono::NaiveDateTime, chrono::NaiveDateTime)> =
        query.load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}

// ─── Crypto repository ──────────────────────────────────────────────────────

/// Async trait for cryptocurrency-specific database operations.
///
/// Obtained via [`DatabaseContext::crypto_repository`]. Covers API mappings,
/// metadata, social/technical data upserts, market data, and summary
/// statistics.
///
/// # Operation groups
///
/// - **API mapping:** `get_api_id`, `get_symbols_needing_mapping`,
///   `upsert_api_mapping`, `get_crypto_symbols_with_mappings`.
/// - **Metadata:** `has_metadata`, `upsert_metadata`,
///   `get_symbols_without_metadata`.
/// - **Social / technical:** `upsert_social_data_full`, `has_social_data`,
///   `batch_upsert_social`, `batch_upsert_technical`.
/// - **Market data:** `upsert_market_data`.
/// - **Analytics:** `get_crypto_summary`, `get_cryptos_with_coingecko_ids`.
#[async_trait]
pub trait CryptoRepository: Send + Sync {
  // API Mapping operations
  /// Get API ID for a symbol from a specific source (e.g., "CoinGecko")
  async fn get_api_id(&self, sid: i64, api_source: &str) -> RepositoryResult<Option<String>>;

  /// Get symbols that need API mapping for a specific source
  async fn get_symbols_needing_mapping(
    &self,
    api_source: &str,
  ) -> RepositoryResult<Vec<(i64, String, String)>>;

  /// Insert or update an API mapping
  async fn upsert_api_mapping(
    &self,
    sid: i64,
    api_source: &str,
    api_id: &str,
    api_slug: Option<&str>,
    api_symbol: Option<&str>,
    is_active: Option<bool>,
  ) -> RepositoryResult<()>;

  // Crypto symbol operations
  /// Get all cryptocurrency symbols with their mappings
  async fn get_crypto_symbols_with_mappings(
    &self,
    api_source: &str,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String, String, Option<String>)>>;

  /// Get cryptocurrency symbols without metadata
  async fn get_symbols_without_metadata(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String)>>;

  // Metadata operations
  /// Check if metadata exists for a symbol
  async fn has_metadata(&self, sid: i64) -> RepositoryResult<bool>;

  /// Save or update cryptocurrency metadata
  async fn upsert_metadata(
    &self,
    sid: i64,
    source: &str,
    source_id: &str,
    market_cap_rank: Option<i32>,
    base_currency: Option<&str>,
    quote_currency: Option<&str>,
    is_active: bool,
    additional_data: Option<serde_json::Value>,
  ) -> RepositoryResult<()>;

  // Social data operations
  /// Save or update comprehensive social data for a symbol
  async fn upsert_social_data_full(
    &self,
    sid: i64,
    website_url: Option<String>,
    whitepaper_url: Option<String>,
    github_url: Option<String>,
    twitter_handle: Option<String>,
    twitter_followers: Option<i32>,
    telegram_url: Option<String>,
    telegram_members: Option<i32>,
    discord_url: Option<String>,
    discord_members: Option<i32>,
    reddit_url: Option<String>,
    reddit_subscribers: Option<i32>,
    facebook_url: Option<String>,
    facebook_likes: Option<i32>,
    coingecko_score: Option<bigdecimal::BigDecimal>,
    developer_score: Option<bigdecimal::BigDecimal>,
    community_score: Option<bigdecimal::BigDecimal>,
    liquidity_score: Option<bigdecimal::BigDecimal>,
    public_interest_score: Option<bigdecimal::BigDecimal>,
    sentiment_votes_up_pct: Option<bigdecimal::BigDecimal>,
    sentiment_votes_down_pct: Option<bigdecimal::BigDecimal>,
  ) -> RepositoryResult<()>;

  /// Check if social data exists for a symbol
  async fn has_social_data(&self, sid: i64) -> RepositoryResult<bool>;

  // Summary and statistics
  /// Get cryptocurrency mapping summary statistics
  async fn get_crypto_summary(&self) -> RepositoryResult<CryptoSummary>;

  // Market data operations
  /// Batch upsert crypto market data
  async fn upsert_market_data(
    &self,
    market_data: &[crate::models::crypto_markets::NewCryptoMarket],
  ) -> RepositoryResult<(usize, usize)>;

  // Detailed data operations
  /// Batch upsert crypto social data
  async fn batch_upsert_social(
    &self,
    social_data: &[crate::models::crypto::NewCryptoSocial],
  ) -> RepositoryResult<usize>;

  /// Batch upsert crypto technical data
  async fn batch_upsert_technical(
    &self,
    technical_data: &[crate::models::crypto::NewCryptoTechnical],
  ) -> RepositoryResult<usize>;

  /// Get cryptos with CoinGecko mappings (for details loading)
  async fn get_cryptos_with_coingecko_ids(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String, String)>>; // (sid, symbol, coingecko_id)
}

/// PostgreSQL implementation of [`CryptoRepository`].
///
/// Public (unlike the other `*Impl` structs) so it can be constructed
/// directly by consumers that don't use [`DatabaseContext`].
pub struct CryptoRepositoryImpl {
  pool: Arc<DbPool>,
}

impl CryptoRepositoryImpl {
  pub fn new(pool: Arc<DbPool>) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CryptoRepository for CryptoRepositoryImpl {
  async fn get_api_id(&self, sid: i64, api_source: &str) -> RepositoryResult<Option<String>> {
    let pool = self.pool.clone();
    let sid_val = sid;
    let api_source_val = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_api_map::dsl::*;
      let mut conn = pool.get()?;

      let result: Option<String> = crypto_api_map
        .filter(crate::schema::crypto_api_map::sid.eq(sid_val))
        .filter(crate::schema::crypto_api_map::api_source.eq(&api_source_val))
        .select(api_id)
        .first(&mut conn)
        .optional()?;

      Ok(result)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_symbols_needing_mapping(
    &self,
    api_source: &str,
  ) -> RepositoryResult<Vec<(i64, String, String)>> {
    let pool = self.pool.clone();
    let api_source = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{crypto_api_map, symbols};
      let mut conn = pool.get()?;

      let results: Vec<(i64, String, String)> =
        symbols::table
          .left_join(crypto_api_map::table.on(
            symbols::sid.eq(crypto_api_map::sid).and(crypto_api_map::api_source.eq(&api_source)),
          ))
          .filter(symbols::sec_type.eq("Cryptocurrency"))
          .filter(crypto_api_map::api_id.is_null())
          .select((symbols::sid, symbols::symbol, symbols::name))
          .load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn upsert_api_mapping(
    &self,
    sid: i64,
    api_source: &str,
    api_id_val: &str,
    api_slug_val: Option<&str>,
    api_symbol_val: Option<&str>,
    is_active_val: Option<bool>,
  ) -> RepositoryResult<()> {
    let pool = self.pool.clone();
    let api_source = api_source.to_string();
    let api_id_val = api_id_val.to_string();
    let api_slug_val = api_slug_val.map(|s| s.to_string());
    let api_symbol_val = api_symbol_val.map(|s| s.to_string());

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_api_map;
      let mut conn = pool.get()?;

      diesel::insert_into(crypto_api_map::table)
        .values((
          crypto_api_map::sid.eq(sid),
          crypto_api_map::api_source.eq(&api_source),
          crypto_api_map::api_id.eq(&api_id_val),
          crypto_api_map::api_slug.eq(api_slug_val.as_deref()),
          crypto_api_map::api_symbol.eq(api_symbol_val.as_deref()),
          crypto_api_map::is_active.eq(is_active_val),
        ))
        .on_conflict((crypto_api_map::sid, crypto_api_map::api_source))
        .do_update()
        .set((
          crypto_api_map::api_id.eq(&api_id_val),
          crypto_api_map::api_slug.eq(api_slug_val.as_deref()),
          crypto_api_map::api_symbol.eq(api_symbol_val.as_deref()),
          crypto_api_map::is_active.eq(is_active_val),
          crypto_api_map::m_time.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)?;

      Ok(())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_crypto_symbols_with_mappings(
    &self,
    api_source: &str,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String, String, Option<String>)>> {
    let pool = self.pool.clone();
    let api_source = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{crypto_api_map, symbols};
      let mut conn = pool.get()?;

      let mut query =
        symbols::table
          .left_join(crypto_api_map::table.on(
            symbols::sid.eq(crypto_api_map::sid).and(crypto_api_map::api_source.eq(&api_source)),
          ))
          .filter(symbols::sec_type.eq("Cryptocurrency"))
          .select((symbols::sid, symbols::symbol, symbols::name, crypto_api_map::api_id.nullable()))
          .into_boxed();

      if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
      }

      let results: Vec<(i64, String, String, Option<String>)> = query.load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_symbols_without_metadata(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String)>> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{crypto_metadata, symbols};
      let mut conn = pool.get()?;

      let mut query = symbols::table
        .left_join(crypto_metadata::table.on(symbols::sid.eq(crypto_metadata::sid)))
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .filter(crypto_metadata::sid.is_null())
        .select((symbols::sid, symbols::symbol))
        .into_boxed();

      if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
      }

      let results: Vec<(i64, String)> = query.load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn has_metadata(&self, sid: i64) -> RepositoryResult<bool> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_metadata;
      let mut conn = pool.get()?;

      let exists = diesel::select(diesel::dsl::exists(
        crypto_metadata::table.filter(crypto_metadata::sid.eq(sid)),
      ))
      .get_result(&mut conn)?;

      Ok(exists)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn upsert_metadata(
    &self,
    sid: i64,
    source: &str,
    source_id: &str,
    market_cap_rank: Option<i32>,
    base_currency: Option<&str>,
    quote_currency: Option<&str>,
    is_active: bool,
    additional_data: Option<serde_json::Value>,
  ) -> RepositoryResult<()> {
    let pool = self.pool.clone();
    let source = source.to_string();
    let source_id = source_id.to_string();
    let base_currency = base_currency.map(|s| s.to_string());
    let quote_currency = quote_currency.map(|s| s.to_string());

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_metadata;
      let mut conn = pool.get()?;

      diesel::insert_into(crypto_metadata::table)
        .values((
          crypto_metadata::sid.eq(sid),
          crypto_metadata::source.eq(&source),
          crypto_metadata::source_id.eq(&source_id),
          crypto_metadata::market_cap_rank.eq(market_cap_rank),
          crypto_metadata::base_currency.eq(base_currency.as_deref()),
          crypto_metadata::quote_currency.eq(quote_currency.as_deref()),
          crypto_metadata::is_active.eq(is_active),
          crypto_metadata::additional_data.eq(&additional_data),
          crypto_metadata::last_updated.eq(diesel::dsl::now),
        ))
        .on_conflict(crypto_metadata::sid)
        .do_update()
        .set((
          crypto_metadata::source.eq(&source),
          crypto_metadata::source_id.eq(&source_id),
          crypto_metadata::market_cap_rank.eq(market_cap_rank),
          crypto_metadata::base_currency.eq(base_currency.as_deref()),
          crypto_metadata::quote_currency.eq(quote_currency.as_deref()),
          crypto_metadata::is_active.eq(is_active),
          crypto_metadata::additional_data.eq(&additional_data),
          crypto_metadata::last_updated.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)?;

      Ok(())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn upsert_social_data_full(
    &self,
    sid: i64,
    website_url: Option<String>,
    whitepaper_url: Option<String>,
    github_url: Option<String>,
    twitter_handle: Option<String>,
    twitter_followers: Option<i32>,
    telegram_url: Option<String>,
    telegram_members: Option<i32>,
    discord_url: Option<String>,
    discord_members: Option<i32>,
    reddit_url: Option<String>,
    reddit_subscribers: Option<i32>,
    facebook_url: Option<String>,
    facebook_likes: Option<i32>,
    coingecko_score: Option<bigdecimal::BigDecimal>,
    developer_score: Option<bigdecimal::BigDecimal>,
    community_score: Option<bigdecimal::BigDecimal>,
    liquidity_score: Option<bigdecimal::BigDecimal>,
    public_interest_score: Option<bigdecimal::BigDecimal>,
    sentiment_votes_up_pct: Option<bigdecimal::BigDecimal>,
    sentiment_votes_down_pct: Option<bigdecimal::BigDecimal>,
  ) -> RepositoryResult<()> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_social;
      let mut conn = pool.get()?;

      diesel::insert_into(crypto_social::table)
        .values((
          crypto_social::sid.eq(sid),
          crypto_social::website_url.eq(website_url.as_deref()),
          crypto_social::whitepaper_url.eq(whitepaper_url.as_deref()),
          crypto_social::github_url.eq(github_url.as_deref()),
          crypto_social::twitter_handle.eq(twitter_handle.as_deref()),
          crypto_social::twitter_followers.eq(twitter_followers),
          crypto_social::telegram_url.eq(telegram_url.as_deref()),
          crypto_social::telegram_members.eq(telegram_members),
          crypto_social::discord_url.eq(discord_url.as_deref()),
          crypto_social::discord_members.eq(discord_members),
          crypto_social::reddit_url.eq(reddit_url.as_deref()),
          crypto_social::reddit_subscribers.eq(reddit_subscribers),
          crypto_social::facebook_url.eq(facebook_url.as_deref()),
          crypto_social::facebook_likes.eq(facebook_likes),
          crypto_social::coingecko_score.eq(coingecko_score.as_ref()),
          crypto_social::developer_score.eq(developer_score.as_ref()),
          crypto_social::community_score.eq(community_score.as_ref()),
          crypto_social::liquidity_score.eq(liquidity_score.as_ref()),
          crypto_social::public_interest_score.eq(public_interest_score.as_ref()),
          crypto_social::sentiment_votes_up_pct.eq(sentiment_votes_up_pct.as_ref()),
          crypto_social::sentiment_votes_down_pct.eq(sentiment_votes_down_pct.as_ref()),
        ))
        .on_conflict(crypto_social::sid)
        .do_update()
        .set((
          crypto_social::website_url.eq(website_url.as_deref()),
          crypto_social::whitepaper_url.eq(whitepaper_url.as_deref()),
          crypto_social::github_url.eq(github_url.as_deref()),
          crypto_social::twitter_handle.eq(twitter_handle.as_deref()),
          crypto_social::twitter_followers.eq(twitter_followers),
          crypto_social::telegram_url.eq(telegram_url.as_deref()),
          crypto_social::telegram_members.eq(telegram_members),
          crypto_social::discord_url.eq(discord_url.as_deref()),
          crypto_social::discord_members.eq(discord_members),
          crypto_social::reddit_url.eq(reddit_url.as_deref()),
          crypto_social::reddit_subscribers.eq(reddit_subscribers),
          crypto_social::facebook_url.eq(facebook_url.as_deref()),
          crypto_social::facebook_likes.eq(facebook_likes),
          crypto_social::coingecko_score.eq(coingecko_score.as_ref()),
          crypto_social::developer_score.eq(developer_score.as_ref()),
          crypto_social::community_score.eq(community_score.as_ref()),
          crypto_social::liquidity_score.eq(liquidity_score.as_ref()),
          crypto_social::public_interest_score.eq(public_interest_score.as_ref()),
          crypto_social::sentiment_votes_up_pct.eq(sentiment_votes_up_pct.as_ref()),
          crypto_social::sentiment_votes_down_pct.eq(sentiment_votes_down_pct.as_ref()),
          crypto_social::m_time.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)?;

      Ok(())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn has_social_data(&self, sid: i64) -> RepositoryResult<bool> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_social;
      let mut conn = pool.get()?;

      let exists = diesel::select(diesel::dsl::exists(
        crypto_social::table.filter(crypto_social::sid.eq(sid)),
      ))
      .get_result(&mut conn)?;

      Ok(exists)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_crypto_summary(&self) -> RepositoryResult<CryptoSummary> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{crypto_api_map, crypto_markets, symbols};
      let mut conn = pool.get()?;

      let total_cryptos: i64 = symbols::table
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .count()
        .get_result(&mut conn)?;

      let active_cryptos: i64 = symbols::table
        .inner_join(crypto_markets::table.on(crypto_markets::sid.eq(symbols::sid)))
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .filter(crypto_markets::is_active.eq(Some(true)))
        .count()
        .get_result(&mut conn)?;

      let mapped_coingecko: i64 = crypto_api_map::table
        .filter(crypto_api_map::api_source.eq("CoinGecko"))
        .filter(crypto_api_map::is_active.eq(Some(true)))
        .count()
        .get_result(&mut conn)?;

      let mapped_coinpaprika: i64 = crypto_api_map::table
        .filter(crypto_api_map::api_source.eq("CoinPaprika"))
        .filter(crypto_api_map::is_active.eq(Some(true)))
        .count()
        .get_result(&mut conn)?;

      Ok(CryptoSummary { total_cryptos, active_cryptos, mapped_coingecko, mapped_coinpaprika })
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn upsert_market_data(
    &self,
    market_data: &[crate::models::crypto_markets::NewCryptoMarket],
  ) -> RepositoryResult<(usize, usize)> {
    let pool = self.pool.clone();
    let market_data = market_data.to_vec();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_markets::dsl::*;
      use diesel::upsert::excluded;
      let mut conn = pool.get()?;

      let mut total_affected = 0;

      // Process individually using the same pattern as CryptoMarket::upsert_markets
      for market in &market_data {
        let result = diesel::insert_into(crypto_markets)
          .values(market)
          .on_conflict((sid, exchange, base, target))
          .do_update()
          .set((
            market_type.eq(excluded(market_type)),
            volume_24h.eq(excluded(volume_24h)),
            volume_percentage.eq(excluded(volume_percentage)),
            bid_ask_spread_pct.eq(excluded(bid_ask_spread_pct)),
            liquidity_score.eq(excluded(liquidity_score)),
            is_active.eq(excluded(is_active)),
            is_anomaly.eq(excluded(is_anomaly)),
            is_stale.eq(excluded(is_stale)),
            trust_score.eq(excluded(trust_score)),
            last_traded_at.eq(excluded(last_traded_at)),
            last_fetch_at.eq(excluded(last_fetch_at)),
          ))
          .execute(&mut conn);

        match result {
          Ok(_) => total_affected += 1,
          Err(e) => {
            error!("Error upserting market {:?}/{:?}: {}", market.exchange, market.base, e)
          }
        }
      }

      // Return total affected rows (we can't easily distinguish inserts from updates with ON CONFLICT)
      Ok((total_affected, 0))
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn batch_upsert_social(
    &self,
    social_data: &[crate::models::crypto::NewCryptoSocial],
  ) -> RepositoryResult<usize> {
    let pool = self.pool.clone();
    let social_data = social_data.to_vec();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_social;
      use diesel::upsert::excluded;
      let mut conn = pool.get()?;

      let mut saved_count = 0;

      for social in &social_data {
        let result = diesel::insert_into(crypto_social::table)
          .values(social)
          .on_conflict(crypto_social::sid)
          .do_update()
          .set((
            crypto_social::website_url.eq(excluded(crypto_social::website_url)),
            crypto_social::whitepaper_url.eq(excluded(crypto_social::whitepaper_url)),
            crypto_social::github_url.eq(excluded(crypto_social::github_url)),
            crypto_social::twitter_handle.eq(excluded(crypto_social::twitter_handle)),
            crypto_social::twitter_followers.eq(excluded(crypto_social::twitter_followers)),
            crypto_social::telegram_url.eq(excluded(crypto_social::telegram_url)),
            crypto_social::telegram_members.eq(excluded(crypto_social::telegram_members)),
            crypto_social::discord_url.eq(excluded(crypto_social::discord_url)),
            crypto_social::discord_members.eq(excluded(crypto_social::discord_members)),
            crypto_social::reddit_url.eq(excluded(crypto_social::reddit_url)),
            crypto_social::reddit_subscribers.eq(excluded(crypto_social::reddit_subscribers)),
            crypto_social::facebook_url.eq(excluded(crypto_social::facebook_url)),
            crypto_social::facebook_likes.eq(excluded(crypto_social::facebook_likes)),
            crypto_social::coingecko_score.eq(excluded(crypto_social::coingecko_score)),
            crypto_social::developer_score.eq(excluded(crypto_social::developer_score)),
            crypto_social::community_score.eq(excluded(crypto_social::community_score)),
            crypto_social::liquidity_score.eq(excluded(crypto_social::liquidity_score)),
            crypto_social::public_interest_score.eq(excluded(crypto_social::public_interest_score)),
            crypto_social::sentiment_votes_up_pct
              .eq(excluded(crypto_social::sentiment_votes_up_pct)),
            crypto_social::sentiment_votes_down_pct
              .eq(excluded(crypto_social::sentiment_votes_down_pct)),
            crypto_social::m_time.eq(diesel::dsl::now),
          ))
          .execute(&mut conn);

        match result {
          Ok(_) => saved_count += 1,
          Err(e) => error!("Error upserting social data for sid {}: {}", social.sid, e),
        }
      }

      Ok(saved_count)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn batch_upsert_technical(
    &self,
    technical_data: &[crate::models::crypto::NewCryptoTechnical],
  ) -> RepositoryResult<usize> {
    let pool = self.pool.clone();
    let technical_data = technical_data.to_vec();

    tokio::task::spawn_blocking(move || {
      use crate::schema::crypto_technical;
      use diesel::upsert::excluded;
      let mut conn = pool.get()?;

      let mut saved_count = 0;

      for technical in &technical_data {
        let result = diesel::insert_into(crypto_technical::table)
          .values(technical)
          .on_conflict(crypto_technical::sid)
          .do_update()
          .set((
            crypto_technical::blockchain_platform
              .eq(excluded(crypto_technical::blockchain_platform)),
            crypto_technical::token_standard.eq(excluded(crypto_technical::token_standard)),
            crypto_technical::github_forks.eq(excluded(crypto_technical::github_forks)),
            crypto_technical::github_stars.eq(excluded(crypto_technical::github_stars)),
            crypto_technical::github_subscribers.eq(excluded(crypto_technical::github_subscribers)),
            crypto_technical::github_total_issues
              .eq(excluded(crypto_technical::github_total_issues)),
            crypto_technical::github_closed_issues
              .eq(excluded(crypto_technical::github_closed_issues)),
            crypto_technical::github_pull_requests
              .eq(excluded(crypto_technical::github_pull_requests)),
            crypto_technical::github_contributors
              .eq(excluded(crypto_technical::github_contributors)),
            crypto_technical::github_commits_4_weeks
              .eq(excluded(crypto_technical::github_commits_4_weeks)),
            crypto_technical::is_defi.eq(excluded(crypto_technical::is_defi)),
            crypto_technical::is_stablecoin.eq(excluded(crypto_technical::is_stablecoin)),
            crypto_technical::is_nft_platform.eq(excluded(crypto_technical::is_nft_platform)),
            crypto_technical::is_exchange_token.eq(excluded(crypto_technical::is_exchange_token)),
            crypto_technical::is_gaming.eq(excluded(crypto_technical::is_gaming)),
            crypto_technical::is_metaverse.eq(excluded(crypto_technical::is_metaverse)),
            crypto_technical::is_privacy_coin.eq(excluded(crypto_technical::is_privacy_coin)),
            crypto_technical::is_layer2.eq(excluded(crypto_technical::is_layer2)),
            crypto_technical::is_wrapped.eq(excluded(crypto_technical::is_wrapped)),
            crypto_technical::genesis_date.eq(excluded(crypto_technical::genesis_date)),
            crypto_technical::m_time.eq(diesel::dsl::now),
          ))
          .execute(&mut conn);

        match result {
          Ok(_) => saved_count += 1,
          Err(e) => error!("Error upserting technical data for sid {}: {}", technical.sid, e),
        }
      }

      Ok(saved_count)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  async fn get_cryptos_with_coingecko_ids(
    &self,
    limit: Option<usize>,
  ) -> RepositoryResult<Vec<(i64, String, String)>> {
    let pool = self.pool.clone();

    tokio::task::spawn_blocking(move || {
      use crate::schema::{symbol_mappings, symbols};
      let mut conn = pool.get()?;

      let mut query = symbols::table
        .inner_join(symbol_mappings::table.on(
          symbols::sid.eq(symbol_mappings::sid).and(symbol_mappings::source_name.eq("coingecko")),
        ))
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .select((symbols::sid, symbols::symbol, symbol_mappings::source_identifier))
        .into_boxed();

      if let Some(limit_val) = limit {
        query = query.limit(limit_val as i64);
      }

      let results: Vec<(i64, String, String)> = query.load(&mut conn)?;

      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}

impl DatabaseContext {
  /// Returns a [`CryptoRepository`] for cryptocurrency operations.
  pub fn crypto_repository(&self) -> impl CryptoRepository {
    CryptoRepositoryImpl::new(Arc::clone(&self.pool))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_repository_error_conversion() {
    let diesel_error = DieselError::NotFound;
    let repo_error: RepositoryError = diesel_error.into();

    assert!(matches!(repo_error, RepositoryError::NotFound(_)));
  }

  #[tokio::test]
  #[ignore] // Requires database connection
  async fn test_database_context_creation() {
    let db_url = std::env::var("DATABASE_URL")
      .unwrap_or_else(|_| "postgresql://ts_user:dev_pw@localhost:6433/sec_master".to_string());

    let context = DatabaseContext::new(&db_url);
    assert!(context.is_ok());
  }
}
