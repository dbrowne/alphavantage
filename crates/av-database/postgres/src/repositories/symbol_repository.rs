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

//! Repository for symbol operations

use crate::models::security::{NewSymbol, Symbol};
use crate::repository::{RepositoryError, RepositoryResult};
use crate::schema::symbols;
use diesel::prelude::*;
use std::sync::Arc;

/// Repository for symbol CRUD operations
pub struct SymbolRepository {
  pool: Arc<crate::repository::DbPool>,
}

impl SymbolRepository {
  pub fn new(pool: Arc<crate::repository::DbPool>) -> Self {
    Self { pool }
  }

  /// Find symbol by symbol string
  pub async fn find_by_symbol(&self, symbol: &str) -> RepositoryResult<Option<Symbol>> {
    let pool = Arc::clone(&self.pool);
    let symbol = symbol.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let result =
        symbols::table.filter(symbols::symbol.eq(&symbol)).first::<Symbol>(&mut conn).optional()?;

      Ok(result)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Find symbol by SID
  pub async fn find_by_sid(&self, sid: i64) -> RepositoryResult<Option<Symbol>> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let result = symbols::table.find(sid).first::<Symbol>(&mut conn).optional()?;

      Ok(result)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Find symbols by type
  pub async fn find_by_type(
    &self,
    sec_type: &str,
    limit: Option<i64>,
  ) -> RepositoryResult<Vec<Symbol>> {
    let pool = Arc::clone(&self.pool);
    let sec_type = sec_type.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let mut query = symbols::table.filter(symbols::sec_type.eq(&sec_type)).into_boxed();

      if let Some(lim) = limit {
        query = query.limit(lim);
      }

      let results = query.load::<Symbol>(&mut conn)?;
      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Find symbols by region
  pub async fn find_by_region(
    &self,
    region: &str,
    limit: Option<i64>,
  ) -> RepositoryResult<Vec<Symbol>> {
    let pool = Arc::clone(&self.pool);
    let region = region.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let mut query = symbols::table.filter(symbols::region.eq(&region)).into_boxed();

      if let Some(lim) = limit {
        query = query.limit(lim);
      }

      let results = query.load::<Symbol>(&mut conn)?;
      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Insert a new symbol
  pub async fn insert(&self, new_symbol: &NewSymbol<'_>) -> RepositoryResult<Symbol> {
    let pool = Arc::clone(&self.pool);
    // Convert borrowed NewSymbol to owned version for async
    let owned_symbol = crate::models::security::NewSymbolOwned::from(new_symbol);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let symbol = diesel::insert_into(symbols::table)
        .values(&owned_symbol)
        .get_result::<Symbol>(&mut conn)?;

      Ok(symbol)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Batch insert symbols
  pub async fn insert_batch(
    &self,
    new_symbols: Vec<crate::models::security::NewSymbolOwned>,
  ) -> RepositoryResult<usize> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;

      // Use ON CONFLICT DO NOTHING to avoid errors on duplicates
      let count = diesel::insert_into(symbols::table)
        .values(&new_symbols)
        .on_conflict_do_nothing()
        .execute(&mut conn)?;

      Ok(count)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Update symbol
  pub async fn update(&self, sid: i64, name: &str, currency: &str) -> RepositoryResult<Symbol> {
    let pool = Arc::clone(&self.pool);
    let name = name.to_string();
    let currency = currency.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let now = chrono::Utc::now().naive_utc();

      let symbol = diesel::update(symbols::table.find(sid))
        .set((symbols::name.eq(&name), symbols::currency.eq(&currency), symbols::m_time.eq(now)))
        .get_result::<Symbol>(&mut conn)?;

      Ok(symbol)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Check if symbol exists
  pub async fn exists(&self, symbol: &str) -> RepositoryResult<bool> {
    let pool = Arc::clone(&self.pool);
    let symbol = symbol.to_string();

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let count =
        symbols::table.filter(symbols::symbol.eq(&symbol)).count().get_result::<i64>(&mut conn)?;

      Ok(count > 0)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Get symbols without overviews (for loading)
  pub async fn find_without_overviews(&self, limit: Option<i64>) -> RepositoryResult<Vec<Symbol>> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let mut query = symbols::table.filter(symbols::overview.eq(false)).into_boxed();

      if let Some(lim) = limit {
        query = query.limit(lim);
      }

      let results = query.load::<Symbol>(&mut conn)?;
      Ok(results)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Mark symbol as having overview data
  pub async fn mark_overview_loaded(&self, sid: i64) -> RepositoryResult<()> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let now = chrono::Utc::now().naive_utc();

      diesel::update(symbols::table.find(sid))
        .set((symbols::overview.eq(true), symbols::m_time.eq(now)))
        .execute(&mut conn)?;

      Ok(())
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }

  /// Count total symbols
  pub async fn count(&self) -> RepositoryResult<i64> {
    let pool = Arc::clone(&self.pool);

    tokio::task::spawn_blocking(move || {
      let mut conn = pool.get()?;
      let count = symbols::table.count().get_result::<i64>(&mut conn)?;
      Ok(count)
    })
    .await
    .map_err(|e| RepositoryError::QueryError(format!("Task join error: {}", e)))?
  }
}
