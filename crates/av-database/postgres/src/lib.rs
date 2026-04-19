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

//! # av-database-postgres
//!
//! TimescaleDB/PostgreSQL integration for the Alpha Vantage data pipeline.
//!
//! This crate is the database layer for the `alphavantage` workspace. It
//! provides Diesel ORM models, repository abstractions, and query helpers
//! optimized for storing and querying financial market data in a
//! PostgreSQL/TimescaleDB backend.
//!
//! ## Crate architecture
//!
//! ```text
//! av-database-postgres
//! ├── connection     → single-connection factory (establish_connection)
//! ├── schema         → Diesel table! macros (auto-generated from migrations)
//! ├── models/        → Diesel structs: Queryable, Insertable, AsChangeset
//! │   ├── security   → symbols, overviews, overviewexts, equity_details, symbol_mappings
//! │   ├── price      → intradayprices, summaryprices, topstats (TimescaleDB hypertables)
//! │   ├── news       → newsoverviews, feeds, articles, authors, sources, sentiment, topics
//! │   ├── crypto     → crypto_overview_basic/metrics, crypto_technical/social, crypto_api_map
//! │   ├── crypto_markets → crypto exchange/trading-pair market data
//! │   └── missing_symbols → unresolved symbol tracking & resolution workflow
//! ├── repository     → DbPool, RepositoryError, traits (Repository, CacheRepository, etc.)
//! └── repositories/  → concrete async repository implementations (SymbolRepository)
//! ```
//!
//! ## Key features
//!
//! - **Dual async strategy:**
//!   - [`models`] use `diesel-async` (`AsyncPgConnection`) for direct async queries.
//!   - [`repositories`] use `spawn_blocking` over an `r2d2` synchronous pool
//!     for the repository pattern layer.
//! - **TimescaleDB integration:** Hypertables for time-series data
//!   (`intradayprices`, `summaryprices`, `topstats`) with `time_bucket()`,
//!   `first()` / `last()`, and continuous aggregates.
//! - **Repository pattern:** [`DatabaseContext`] provides a single entry point
//!   for obtaining domain-specific repositories (overview, news, crypto).
//! - **Caching:** [`CacheRepository`] / [`CacheRepositoryExt`] traits for
//!   response caching with TTL.
//! - **Precision:** Financial values use `BigDecimal` (not `f64`) to avoid
//!   floating-point rounding.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use av_database_postgres::{DatabaseContext, RepositoryResult};
//!
//! fn main() -> RepositoryResult<()> {
//!     let db = DatabaseContext::new("postgres://user:pass@localhost/alphavantage")?;
//!     let conn = db.get_connection()?;
//!     println!("Connected to database");
//!     Ok(())
//! }
//! ```
//!
//! ## Module overview
//!
//! | Module           | Purpose                                                        |
//! |------------------|----------------------------------------------------------------|
//! | [`connection`]   | Bare `PgConnection` factory ([`establish_connection`])          |
//! | [`schema`]       | Auto-generated Diesel `table!` macros from SQL migrations      |
//! | [`models`]       | ORM structs for all database tables (query, insert, update)    |
//! | [`repository`]   | Pool management, error types, trait definitions, `DatabaseContext` |
//! | [`repositories`] | Concrete async repository implementations                      |
//!
//! ## Re-exports
//!
//! The `pub use` statements below hoist the most commonly used types to
//! the crate root so downstream code can write
//! `use av_database_postgres::{DatabaseContext, RepositoryResult}` etc.

/// Bare PostgreSQL connection factory.
///
/// Provides [`establish_connection`] for creating a single `PgConnection`
/// from a database URL. For pooled access, use
/// [`DatabaseContext`](crate::repository::DatabaseContext) instead.
pub mod connection;

/// Diesel ORM models for all database tables.
///
/// Organized by domain: [`models::security`], [`models::price`],
/// [`models::news`], [`models::crypto`], [`models::crypto_markets`],
/// [`models::missing_symbols`]. See the [`models`] module documentation
/// for the full type inventory.
pub mod models;

/// Concrete async repository implementations.
///
/// Currently contains [`SymbolRepository`]. See the [`repositories`]
/// module documentation for the architecture pattern.
pub mod repositories;

/// Database infrastructure: pool management, error types, and trait definitions.
///
/// Key exports: [`DbPool`](repository::DbPool),
/// [`DatabaseContext`](repository::DatabaseContext),
/// [`RepositoryError`](repository::RepositoryError),
/// [`RepositoryResult`](repository::RepositoryResult),
/// and domain traits ([`Repository`](repository::Repository),
/// [`OverviewRepository`](repository::OverviewRepository),
/// [`NewsRepository`](repository::NewsRepository),
/// [`CryptoRepository`](repository::CryptoRepository)).
pub mod repository;

/// Auto-generated Diesel `table!` macros.
///
/// Generated by `diesel print-schema` from the database migrations. Do not
/// edit manually — changes are overwritten on migration runs.
pub mod schema;

// ─── Convenience re-exports ─────────────────────────────────────────────────

/// Re-exported from [`connection`]: creates a bare `PgConnection`.
pub use connection::establish_connection;

/// Re-exported from [`diesel`]: brings Diesel query DSL into scope.
pub use diesel::prelude::*;

/// Re-exported from [`models::crypto`]: aggregate crypto mapping statistics.
pub use models::crypto::CryptoSummary;

/// Re-exported from [`repositories`]: async symbol CRUD repository.
pub use repositories::SymbolRepository;

/// Re-exported from [`repository`]: pool management, error types, traits,
/// and the [`DatabaseContext`] entry point.
pub use repository::{
  CacheRepository, CacheRepositoryExt, CryptoRepository, DatabaseContext, NewsRepository,
  OverviewRepository, OverviewSymbolFilter, Repository, RepositoryError, RepositoryResult,
  SymbolInfo, Transactional,
};
