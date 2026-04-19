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

//! Tracks unresolved symbol references encountered during data ingestion.
//!
//! When the ingestion pipeline encounters a ticker symbol that does not exist
//! in the `symbols` table, a record is created (or incremented) in the
//! `missing_symbols` table. This allows operators to review which symbols
//! are missing, how often they appear, and resolve them through a simple
//! status-based workflow.
//!
//! # Resolution workflow
//!
//! ```text
//!   ┌─────────┐
//!   │ Pending │──── symbol found ────► Found   (sid populated)
//!   └────┬────┘
//!        ├──── confirmed absent ────► NotFound
//!        └──── intentionally ignored ► Skipped
//! ```
//!
//! The [`ResolutionStatus`] enum models these four states. Database records
//! store the status as a lowercase string (`"pending"`, `"found"`, etc.) and
//! the enum provides bidirectional conversion via [`FromStr`] / [`Display`].
//!
//! # Struct inventory
//!
//! | Type                   | Role                                                      |
//! |------------------------|-----------------------------------------------------------|
//! | [`ResolutionStatus`]   | Enum of resolution states with string round-tripping      |
//! | [`MissingSymbol`]      | Queryable row from `missing_symbols`                      |
//! | [`NewMissingSymbol`]   | Insertable struct for recording a new missing symbol      |
//! | [`UpdateMissingSymbol`]| `AsChangeset` struct for partial status updates           |
//!
//! # Key operations
//!
//! All methods on [`MissingSymbol`] are synchronous (`&mut PgConnection`):
//!
//! - **Record:** [`record_or_increment`](MissingSymbol::record_or_increment) —
//!   insert-or-update in one call.
//! - **Query:** [`get_pending`](MissingSymbol::get_pending),
//!   [`get_pending_for_source`](MissingSymbol::get_pending_for_source).
//! - **Resolve:** [`mark_found`](MissingSymbol::mark_found),
//!   [`mark_not_found`](MissingSymbol::mark_not_found),
//!   [`mark_skipped`](MissingSymbol::mark_skipped).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use std::str::FromStr;

use crate::schema::missing_symbols;

// ─── Resolution status enum ────────────────────────────────────────────────

/// The resolution state of a missing symbol record.
///
/// Stored in the database as a lowercase string in the `resolution_status`
/// column. The lifecycle is: `Pending` → one of `Found`, `NotFound`, or
/// `Skipped`.
///
/// # String representation
///
/// | Variant    | String       | Meaning                                        |
/// |------------|--------------|------------------------------------------------|
/// | `Pending`  | `"pending"`  | Awaiting resolution — default for new records  |
/// | `Found`    | `"found"`    | Symbol was located and linked to an `sid`      |
/// | `NotFound` | `"not_found"`| Confirmed that the symbol does not exist       |
/// | `Skipped`  | `"skipped"`  | Intentionally ignored (e.g., test ticker)      |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
  /// Awaiting resolution (initial state for all new records).
  Pending,
  /// The symbol was located in the `symbols` table or an external source
  /// and linked via `sid`.
  Found,
  /// The symbol was confirmed to not exist in any known source.
  NotFound,
  /// The symbol was intentionally skipped (e.g., a test ticker, invalid data).
  Skipped,
}

/// Parses a resolution status from its database string representation.
///
/// Unrecognized strings default to [`Pending`](ResolutionStatus::Pending)
/// rather than returning an error — this makes the conversion infallible.
impl FromStr for ResolutionStatus {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "found" => Ok(ResolutionStatus::Found),
      "not_found" => Ok(ResolutionStatus::NotFound),
      "skipped" => Ok(ResolutionStatus::Skipped),
      _ => Ok(ResolutionStatus::Pending),
    }
  }
}

impl ResolutionStatus {
  /// Returns the lowercase string representation suitable for database storage.
  pub fn as_str(&self) -> &'static str {
    match self {
      ResolutionStatus::Pending => "pending",
      ResolutionStatus::Found => "found",
      ResolutionStatus::NotFound => "not_found",
      ResolutionStatus::Skipped => "skipped",
    }
  }
}

/// Formats as the lowercase database string (delegates to [`as_str`](ResolutionStatus::as_str)).
impl std::fmt::Display for ResolutionStatus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

// ─── Queryable model ────────────────────────────────────────────────────────

/// A record tracking a symbol that was referenced but not found in `symbols`.
///
/// Maps to the `missing_symbols` table with auto-increment `id` primary key.
///
/// # Key fields
///
/// | Field                | Type                    | Description                                    |
/// |----------------------|-------------------------|------------------------------------------------|
/// | `id`                 | `i32`                   | Auto-increment primary key                     |
/// | `symbol`             | `String`                | The ticker string that was not found           |
/// | `source`             | `String`                | Which pipeline/endpoint encountered it         |
/// | `first_seen_at`      | `NaiveDateTime`         | When this symbol was first encountered         |
/// | `last_seen_at`       | `NaiveDateTime`         | Most recent occurrence                         |
/// | `seen_count`         | `i32`                   | Total number of times encountered              |
/// | `resolution_status`  | `String`                | Current status (see [`ResolutionStatus`])       |
/// | `sid`                | `Option<i64>`           | Linked security ID (populated when `Found`)    |
/// | `resolution_details` | `Option<String>`        | Free-text notes about the resolution           |
/// | `resolved_at`        | `Option<NaiveDateTime>` | When the status was changed from `Pending`     |
/// | `created_at` / `updated_at` | `NaiveDateTime`  | Row-level audit timestamps                     |
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = missing_symbols)]
#[diesel(primary_key(id))]
pub struct MissingSymbol {
  pub id: i32,
  pub symbol: String,
  pub source: String,
  pub first_seen_at: NaiveDateTime,
  pub last_seen_at: NaiveDateTime,
  pub seen_count: i32,
  pub resolution_status: String,
  pub sid: Option<i64>,
  pub resolution_details: Option<String>,
  pub resolved_at: Option<NaiveDateTime>,
  pub created_at: NaiveDateTime,
  pub updated_at: NaiveDateTime,
}

/// Convenience accessors for [`MissingSymbol`].
impl MissingSymbol {
  /// Parses the `resolution_status` string column into a [`ResolutionStatus`] enum.
  ///
  /// Defaults to [`Pending`](ResolutionStatus::Pending) if the string is
  /// unrecognized.
  pub fn status(&self) -> ResolutionStatus {
    self.resolution_status.parse().unwrap_or(ResolutionStatus::Pending)
  }

  /// Returns `true` if `resolution_status` is `"pending"`.
  pub fn is_pending(&self) -> bool {
    self.resolution_status == "pending"
  }

  /// Returns `true` if `resolution_status` is `"found"`.
  pub fn is_found(&self) -> bool {
    self.resolution_status == "found"
  }
}

// ─── Insertable model ───────────────────────────────────────────────────────

/// Insertable form of [`MissingSymbol`].
///
/// Only `symbol` and `source` are required; all other fields are `Option`
/// and will use database defaults when `None`:
/// - `first_seen_at` / `last_seen_at` — default to `NOW()`.
/// - `seen_count` — defaults to `1`.
/// - `resolution_status` — defaults to `"pending"`.
///
/// Use [`NewMissingSymbol::new`] for the common case of recording a symbol
/// with all defaults. For the atomic upsert pattern, prefer
/// [`MissingSymbol::record_or_increment`] instead.
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = missing_symbols)]
pub struct NewMissingSymbol {
  pub symbol: String,
  pub source: String,
  pub first_seen_at: Option<NaiveDateTime>,
  pub last_seen_at: Option<NaiveDateTime>,
  pub seen_count: Option<i32>,
  pub resolution_status: Option<String>,
}

impl NewMissingSymbol {
  /// Creates a new missing symbol record with all optional fields set to `None`
  /// (database defaults apply on insert).
  pub fn new(symbol: String, source: String) -> Self {
    Self {
      symbol,
      source,
      first_seen_at: None,
      last_seen_at: None,
      seen_count: None,
      resolution_status: None,
    }
  }
}

// ─── Changeset model ────────────────────────────────────────────────────────

/// Partial-update changeset for [`MissingSymbol`].
///
/// Uses Diesel's `Option<Option<T>>` pattern for nullable columns:
/// - `None` — leave the column unchanged.
/// - `Some(None)` — set the column to `NULL`.
/// - `Some(Some(value))` — set the column to `value`.
///
/// This pattern applies to `sid`, `resolution_details`, and `resolved_at`.
#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = missing_symbols)]
pub struct UpdateMissingSymbol {
  pub last_seen_at: Option<NaiveDateTime>,
  pub seen_count: Option<i32>,
  pub resolution_status: Option<String>,
  /// `Option<Option<i64>>`: outer `None` = don't update, `Some(None)` = set NULL,
  /// `Some(Some(sid))` = link to a security.
  pub sid: Option<Option<i64>>,
  /// `Option<Option<String>>`: outer `None` = don't update.
  pub resolution_details: Option<Option<String>>,
  /// `Option<Option<NaiveDateTime>>`: outer `None` = don't update.
  pub resolved_at: Option<Option<NaiveDateTime>>,
}

/// Synchronous database operations for the missing-symbols resolution workflow.
///
/// All methods take `&mut PgConnection` and execute synchronously.
impl MissingSymbol {
  /// Records a missing symbol or increments its `seen_count` if it already exists.
  ///
  /// Performs a SELECT-then-INSERT-or-UPDATE pattern:
  /// 1. Queries for an existing `(symbol, source)` pair.
  /// 2. If found: updates `last_seen_at` to now and increments `seen_count`.
  /// 3. If not found: inserts a new record with `seen_count = 1`,
  ///    `resolution_status = "pending"`, and both timestamps set to now.
  ///
  /// Returns the inserted or updated row.
  pub fn record_or_increment(
    conn: &mut PgConnection,
    symbol: &str,
    source: &str,
  ) -> Result<Self, diesel::result::Error> {
    use diesel::prelude::*;

    let now = chrono::Utc::now().naive_utc();

    // Try to find existing record
    let existing = missing_symbols::table
      .filter(missing_symbols::symbol.eq(symbol))
      .filter(missing_symbols::source.eq(source))
      .first::<Self>(conn)
      .optional()?;

    if let Some(existing_record) = existing {
      // Update existing record
      diesel::update(missing_symbols::table.find(existing_record.id))
        .set((
          missing_symbols::last_seen_at.eq(now),
          missing_symbols::seen_count.eq(existing_record.seen_count + 1),
        ))
        .get_result(conn)
    } else {
      // Insert new record
      let new_record = NewMissingSymbol {
        symbol: symbol.to_string(),
        source: source.to_string(),
        first_seen_at: Some(now),
        last_seen_at: Some(now),
        seen_count: Some(1),
        resolution_status: Some("pending".to_string()),
      };

      diesel::insert_into(missing_symbols::table).values(&new_record).get_result(conn)
    }
  }

  /// Returns all records with `resolution_status = "pending"`, ordered by
  /// `seen_count` descending (most frequently encountered first).
  pub fn get_pending(conn: &mut PgConnection) -> Result<Vec<Self>, diesel::result::Error> {
    missing_symbols::table
      .filter(missing_symbols::resolution_status.eq("pending"))
      .order_by(missing_symbols::seen_count.desc())
      .load::<Self>(conn)
  }

  /// Returns pending records filtered to a specific `source` pipeline,
  /// ordered by `seen_count` descending.
  pub fn get_pending_for_source(
    conn: &mut PgConnection,
    source: &str,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    missing_symbols::table
      .filter(missing_symbols::resolution_status.eq("pending"))
      .filter(missing_symbols::source.eq(source))
      .order_by(missing_symbols::seen_count.desc())
      .load::<Self>(conn)
  }

  /// Transitions a record to [`Found`](ResolutionStatus::Found) status.
  ///
  /// Sets `resolution_status = "found"`, links the record to the given `sid`,
  /// stores optional `details`, and timestamps `resolved_at` to now.
  /// Returns the updated row.
  pub fn mark_found(
    conn: &mut PgConnection,
    id: i32,
    sid: i64,
    details: Option<String>,
  ) -> Result<Self, diesel::result::Error> {
    let now = chrono::Utc::now().naive_utc();

    diesel::update(missing_symbols::table.find(id))
      .set((
        missing_symbols::resolution_status.eq("found"),
        missing_symbols::sid.eq(Some(sid)),
        missing_symbols::resolution_details.eq(details),
        missing_symbols::resolved_at.eq(Some(now)),
      ))
      .get_result(conn)
  }

  /// Transitions a record to [`NotFound`](ResolutionStatus::NotFound) status.
  ///
  /// Sets `resolution_status = "not_found"`, stores optional `details`, and
  /// timestamps `resolved_at` to now. `sid` is left unchanged (typically `NULL`).
  pub fn mark_not_found(
    conn: &mut PgConnection,
    id: i32,
    details: Option<String>,
  ) -> Result<Self, diesel::result::Error> {
    let now = chrono::Utc::now().naive_utc();

    diesel::update(missing_symbols::table.find(id))
      .set((
        missing_symbols::resolution_status.eq("not_found"),
        missing_symbols::resolution_details.eq(details),
        missing_symbols::resolved_at.eq(Some(now)),
      ))
      .get_result(conn)
  }

  /// Transitions a record to [`Skipped`](ResolutionStatus::Skipped) status.
  ///
  /// Sets `resolution_status = "skipped"`, stores an optional `reason`, and
  /// timestamps `resolved_at` to now. Use this for test tickers, known-invalid
  /// symbols, or symbols intentionally excluded from ingestion.
  pub fn mark_skipped(
    conn: &mut PgConnection,
    id: i32,
    reason: Option<String>,
  ) -> Result<Self, diesel::result::Error> {
    let now = chrono::Utc::now().naive_utc();

    diesel::update(missing_symbols::table.find(id))
      .set((
        missing_symbols::resolution_status.eq("skipped"),
        missing_symbols::resolution_details.eq(reason),
        missing_symbols::resolved_at.eq(Some(now)),
      ))
      .get_result(conn)
  }
}
