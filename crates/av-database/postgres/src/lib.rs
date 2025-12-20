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
//! TimescaleDB/PostgreSQL integration for AlphaVantage time-series data.
//!
//! This crate provides async database operations using Diesel ORM with BB8 connection
//! pooling, optimized for storing and querying financial market data.
//!
//! ## Features
//!
//! - **Async Support**: Uses `diesel-async` with BB8 connection pool
//! - **TimescaleDB**: Optimized for time-series data with hypertables
//! - **Repository Pattern**: Clean abstractions for data access
//! - **Caching**: Built-in response caching layer
//!
//! ## Example
//!
//! ```ignore
//! use av_database_postgres::{establish_connection, Repository};
//!
//! let pool = establish_connection(&database_url).await?;
//! let repo = Repository::new(pool);
//! ```

pub mod connection;
pub mod models;
pub mod repositories;
pub mod repository;
pub mod schema;

// Re-export commonly used items
pub use connection::establish_connection;
pub use diesel::prelude::*;
pub use models::crypto::CryptoSummary;
pub use repositories::SymbolRepository;
pub use repository::{
  CacheRepository, CacheRepositoryExt, CryptoRepository, DatabaseContext, NewsRepository,
  OverviewRepository, OverviewSymbolFilter, Repository, RepositoryError, RepositoryResult,
  SymbolInfo, Transactional,
};
