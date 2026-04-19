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

//! Concrete repository implementations for domain entities.
//!
//! This module provides domain-specific repository structs that encapsulate
//! all database access for a particular entity type. Repositories sit between
//! the application/service layer and the raw Diesel model layer, offering a
//! clean async API with connection-pool management handled internally.
//!
//! # Architecture
//!
//! ```text
//! Application / Service layer
//!   └──► repositories (this module)
//!          └──► models     (Diesel structs & raw queries)
//!                └──► schema   (Diesel table! macros)
//!                      └──► PostgreSQL / TimescaleDB
//! ```
//!
//! Each repository:
//! - Holds an `Arc<DbPool>` for thread-safe pool sharing.
//! - Exposes **async** methods that internally use
//!   [`tokio::task::spawn_blocking`] to run synchronous Diesel queries.
//! - Returns [`RepositoryResult<T>`](crate::repository::RepositoryResult)
//!   (aliased to `Result<T, RepositoryError>`).
//!
//! # Relationship to `repository.rs`
//!
//! The sibling [`repository`](crate::repository) module defines the shared
//! infrastructure: [`DbPool`](crate::repository::DbPool),
//! [`RepositoryError`](crate::repository::RepositoryError),
//! [`RepositoryResult`](crate::repository::RepositoryResult), and pool
//! construction helpers. This `repositories` module contains the
//! entity-specific implementations that use those types.
//!
//! # Available repositories
//!
//! | Repository            | Entity    | Description                                |
//! |-----------------------|-----------|--------------------------------------------|
//! | [`SymbolRepository`]  | `Symbol`  | CRUD, batch insert, existence checks, ingestion queue queries |
//!
//! Additional repositories (e.g., for overviews, prices, news) can be added
//! here following the same pattern established by [`SymbolRepository`].

/// Async repository for [`Symbol`](crate::models::security::Symbol) CRUD
/// operations: lookup, insert, batch insert, update, existence checks, and
/// ingestion queue management.
pub mod symbol_repository;

/// Re-exported for convenience so callers can write
/// `use repositories::SymbolRepository` instead of the full sub-module path.
pub use symbol_repository::SymbolRepository;
