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

//! Bare PostgreSQL connection factory.
//!
//! This module provides a single function, [`establish_connection`], that
//! creates an unpooled, synchronous [`PgConnection`] from a database URL.
//! It is the simplest way to connect to the database and is primarily
//! intended for:
//!
//! - **Diesel CLI migrations** and one-off scripts.
//! - **Tests** that need an isolated connection.
//! - **Quick prototyping** where pool overhead is unnecessary.
//!
//! For production and service code, prefer
//! [`DatabaseContext`](crate::repository::DatabaseContext), which manages
//! an `r2d2` connection pool with configurable size, idle connections, and
//! timeouts.
//!
//! # Example
//!
//! ```rust,no_run
//! use av_database_postgres::establish_connection;
//!
//! let conn = establish_connection("postgres://user:pass@localhost/alphavantage")
//!     .expect("Failed to connect to database");
//! ```

use diesel::pg::PgConnection;
use diesel::prelude::*;

/// Creates a single, unpooled [`PgConnection`] to a PostgreSQL database.
///
/// # Arguments
///
/// - `database_url` — a PostgreSQL connection string in the format
///   `postgres://user:password@host:port/database`. Supports all
///   [`libpq` connection parameters](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING).
///
/// # Errors
///
/// Returns [`diesel::ConnectionError`] if the connection cannot be
/// established (e.g., invalid URL, server unreachable, authentication
/// failure).
///
/// # Notes
///
/// This function is **synchronous** and blocks the calling thread until
/// the TCP connection and authentication handshake complete. It creates
/// a **single connection** with no pooling — each call opens a new socket.
/// For pooled, reusable connections use
/// [`DatabaseContext::new`](crate::repository::DatabaseContext::new).
pub fn establish_connection(database_url: &str) -> Result<PgConnection, diesel::ConnectionError> {
  PgConnection::establish(database_url)
}
