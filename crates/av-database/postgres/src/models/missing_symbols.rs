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

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::missing_symbols;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
  Pending,
  Found,
  NotFound,
  Skipped,
}

impl ResolutionStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      ResolutionStatus::Pending => "pending",
      ResolutionStatus::Found => "found",
      ResolutionStatus::NotFound => "not_found",
      ResolutionStatus::Skipped => "skipped",
    }
  }

  pub fn from_str(s: &str) -> Self {
    match s {
      "found" => ResolutionStatus::Found,
      "not_found" => ResolutionStatus::NotFound,
      "skipped" => ResolutionStatus::Skipped,
      _ => ResolutionStatus::Pending,
    }
  }
}

impl std::fmt::Display for ResolutionStatus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

/// Represents a missing symbol record
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

impl MissingSymbol {
  /// Get status as enum
  pub fn status(&self) -> ResolutionStatus {
    ResolutionStatus::from_str(&self.resolution_status)
  }

  /// Check if symbol is still pending
  pub fn is_pending(&self) -> bool {
    self.resolution_status == "pending"
  }

  /// Check if symbol was found
  pub fn is_found(&self) -> bool {
    self.resolution_status == "found"
  }
}

/// For inserting new missing symbols
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

/// For updating missing symbol status
#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = missing_symbols)]
pub struct UpdateMissingSymbol {
  pub last_seen_at: Option<NaiveDateTime>,
  pub seen_count: Option<i32>,
  pub resolution_status: Option<String>,
  pub sid: Option<Option<i64>>,
  pub resolution_details: Option<Option<String>>,
  pub resolved_at: Option<Option<NaiveDateTime>>,
}

impl MissingSymbol {
  /// Record a new missing symbol or increment seen count if it already exists
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

  /// Get all pending missing symbols
  pub fn get_pending(conn: &mut PgConnection) -> Result<Vec<Self>, diesel::result::Error> {
    missing_symbols::table
      .filter(missing_symbols::resolution_status.eq("pending"))
      .order_by(missing_symbols::seen_count.desc())
      .load::<Self>(conn)
  }

  /// Get pending symbols for a specific source
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

  /// Mark symbol as found
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

  /// Mark symbol as not found
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

  /// Mark symbol as skipped
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
