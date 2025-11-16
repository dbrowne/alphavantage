/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

//! Database repository abstraction layer
//!
//! Provides a clean abstraction over database operations for use in loaders.
//! Supports connection pooling, caching, transactions, and common CRUD operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Database repository errors
#[derive(Error, Debug)]
pub enum RepositoryError {
  #[error("Connection pool error: {0}")]
  PoolError(String),

  #[error("Database query error: {0}")]
  QueryError(String),

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

pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Cached response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse<T> {
  pub data: T,
  pub cached_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
}

/// Cache repository trait for API response caching
#[async_trait]
pub trait CacheRepository: Send + Sync {
  /// Get a cached response by key
  async fn get<T>(&self, cache_key: &str, api_source: &str) -> RepositoryResult<Option<T>>
  where
    T: for<'de> Deserialize<'de> + Send + 'static;

  /// Set a cached response
  async fn set<T>(
    &self,
    cache_key: &str,
    api_source: &str,
    endpoint_url: &str,
    data: &T,
    ttl_hours: i64,
  ) -> RepositoryResult<()>
  where
    T: Serialize + Send + Sync;

  /// Delete expired cache entries
  async fn cleanup_expired(&self, api_source: &str) -> RepositoryResult<usize>;

  /// Delete specific cache entry
  async fn delete(&self, cache_key: &str) -> RepositoryResult<bool>;

  /// Check if cache entry exists and is not expired
  async fn exists(&self, cache_key: &str) -> RepositoryResult<bool>;
}

/// Generic repository trait for CRUD operations
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

/// Transaction support
pub trait Transactional {
  /// Execute operations within a transaction
  fn with_transaction<F, R>(&self, f: F) -> RepositoryResult<R>
  where
    F: FnOnce(&mut DbConnection) -> RepositoryResult<R>;
}

/// Database context that provides access to repositories and connection pool
#[derive(Clone)]
pub struct DatabaseContext {
  pool: Arc<DbPool>,
}

impl DatabaseContext {
  /// Create a new database context with connection pooling
  pub fn new(database_url: &str) -> RepositoryResult<Self> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
      .max_size(10)
      .min_idle(Some(2))
      .build(manager)
      .map_err(|e| RepositoryError::PoolError(e.to_string()))?;

    Ok(Self { pool: Arc::new(pool) })
  }

  /// Create with custom pool configuration
  pub fn with_pool_config(
    database_url: &str,
    max_size: u32,
    min_idle: u32,
  ) -> RepositoryResult<Self> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
      .max_size(max_size)
      .min_idle(Some(min_idle))
      .build(manager)
      .map_err(|e| RepositoryError::PoolError(e.to_string()))?;

    Ok(Self { pool: Arc::new(pool) })
  }

  /// Get a connection from the pool
  pub fn get_connection(&self) -> RepositoryResult<DbConnection> {
    self.pool.get().map_err(|e| RepositoryError::PoolError(e.to_string()))
  }

  /// Get the underlying pool
  pub fn pool(&self) -> &DbPool {
    &self.pool
  }

  /// Create a cache repository instance
  pub fn cache_repository(&self) -> impl CacheRepository {
    CacheRepositoryImpl { pool: Arc::clone(&self.pool) }
  }

  /// Execute operations within a transaction
  pub fn transaction<F, R>(&self, f: F) -> RepositoryResult<R>
  where
    F: FnOnce(&mut DbConnection) -> RepositoryResult<R>,
  {
    let mut conn = self.get_connection()?;
    conn.transaction(|conn| f(conn)).map_err(|e| RepositoryError::TransactionError(e.to_string()))
  }

  /// Execute a blocking database operation asynchronously
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

/// Implementation of cache repository
struct CacheRepositoryImpl {
  pool: Arc<DbPool>,
}

#[async_trait]
impl CacheRepository for CacheRepositoryImpl {
  async fn get<T>(&self, cache_key: &str, api_source: &str) -> RepositoryResult<Option<T>>
  where
    T: for<'de> Deserialize<'de> + Send + 'static,
  {
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();
    let api_source = api_source.to_string();

    tokio::task::spawn_blocking(move || {
      use diesel::sql_query;
      use diesel::sql_types::{Jsonb, Text, Timestamptz};

      let mut conn = pool.get()?;

      #[derive(QueryableByName)]
      struct CacheEntry {
        #[diesel(sql_type = Jsonb)]
        response_data: serde_json::Value,
        #[diesel(sql_type = Timestamptz)]
        expires_at: DateTime<Utc>,
      }

      let result: Option<CacheEntry> = sql_query(
        "SELECT response_data, expires_at FROM api_response_cache
         WHERE cache_key = $1 AND api_source = $2 AND expires_at > NOW()",
      )
      .bind::<Text, _>(&cache_key)
      .bind::<Text, _>(&api_source)
      .get_result(&mut conn)
      .optional()?;

      match result {
        Some(entry) => {
          let data: T = serde_json::from_value(entry.response_data)?;
          Ok(Some(data))
        }
        None => Ok(None),
      }
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

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
    let pool = Arc::clone(&self.pool);
    let cache_key = cache_key.to_string();
    let api_source = api_source.to_string();
    let endpoint_url = endpoint_url.to_string();
    let json_data = serde_json::to_value(data)?;
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
      .bind::<Jsonb, _>(&json_data)
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
