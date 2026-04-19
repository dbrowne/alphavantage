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


//! Process tracking models for ETL job monitoring.
//!
//! This module provides a lightweight process/job lifecycle tracker stored
//! across three tables. It enables monitoring which ETL jobs ran, when they
//! started and finished, whether they succeeded or failed, and how many
//! records were processed.
//!
//! # Database schema
//!
//! ```text
//! proctypes ──1:N──► procstates ──N:1──► states
//!   (what)             (runs)              (outcomes)
//! ```
//!
//! | Table        | Model          | Purpose                                    |
//! |--------------|----------------|--------------------------------------------|
//! | `proctypes`  | [`ProcType`]   | Registry of ETL process names              |
//! | `states`     | [`State`]      | Enumeration of outcome states              |
//! | `procstates` | [`ProcState`]  | Individual process execution records       |
//!
//! # Lifecycle
//!
//! 1. **Start:** Look up (or create) the process type via
//!    [`ProcType::find_or_create`], then insert a [`NewProcState`] with
//!    `start_time` set and `end_state`/`end_time` set to `None`.
//! 2. **Progress:** Optionally update `records_processed` via
//!    [`ProcState::update_records_processed`].
//! 3. **Complete:** Call [`ProcState::update_end_state`] with the
//!    appropriate [`state_ids`] constant and the finish timestamp.
//! 4. **Error:** Call [`ProcState::update_with_error`] to record both the
//!    state and a diagnostic error message.
//!
//! # State IDs
//!
//! The [`state_ids`] module provides constants matching the seed data from
//! the database migration:
//!
//! | Constant    | Value | Meaning                                      |
//! |-------------|-------|----------------------------------------------|
//! | `STARTED`   | 1     | Process is currently running                 |
//! | `COMPLETED` | 2     | Process finished successfully                |
//! | `FAILED`    | 3     | Process terminated with an error              |
//! | `CANCELLED` | 4     | Process was manually cancelled               |
//! | `RETRYING`  | 5     | Process failed but will be retried           |
//!
//! All query methods are **synchronous** (`&mut PgConnection`).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::{procstates, proctypes, states};

// ─── ProcType ───────────────────────────────────────────────────────────────

/// A registered ETL process type (e.g., `"intraday_load"`, `"news_ingest"`).
///
/// Maps to the `proctypes` table. Each unique process name is stored once;
/// use [`find_or_create`](ProcType::find_or_create) to ensure idempotent
/// registration.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = proctypes)]
pub struct ProcType {
  /// Auto-increment primary key.
  pub id: i32,
  /// Human-readable process name (unique).
  pub name: String,
}

/// Insertable (borrowed) form of [`ProcType`].
#[derive(Insertable, Debug)]
#[diesel(table_name = proctypes)]
pub struct NewProcType<'a> {
  pub name: &'a str,
}

// ─── State ──────────────────────────────────────────────────────────────────

/// An outcome state from the `states` lookup table.
///
/// Seeded by a database migration with the values listed in [`state_ids`].
/// Typically referenced by `id` rather than by name at runtime.
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = states)]
pub struct State {
  /// State ID (matches [`state_ids`] constants).
  pub id: i32,
  /// Human-readable state name (e.g., `"started"`, `"completed"`).
  pub name: String,
}

// ─── ProcState ──────────────────────────────────────────────────────────────

/// An individual execution record for an ETL process.
///
/// Maps to the `procstates` table with PK `spid` (state-process ID).
/// Each row tracks one run of a particular process type, including timing,
/// outcome, error details, and throughput.
///
/// # Fields
///
/// | Field               | Type                    | Description                               |
/// |---------------------|-------------------------|-------------------------------------------|
/// | `spid`              | `i32`                   | Auto-increment primary key                |
/// | `proc_id`           | `Option<i32>`           | FK to `proctypes.id`                      |
/// | `start_time`        | `NaiveDateTime`         | When the process started                  |
/// | `end_state`         | `Option<i32>`           | FK to `states.id` — `None` while running  |
/// | `end_time`          | `Option<NaiveDateTime>` | When the process finished — `None` while running |
/// | `error_msg`         | `Option<String>`        | Diagnostic message on failure             |
/// | `records_processed` | `Option<i32>`           | Number of records handled (optional metric) |
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = procstates)]
#[diesel(primary_key(spid))]
pub struct ProcState {
  pub spid: i32,
  pub proc_id: Option<i32>,
  pub start_time: NaiveDateTime,
  pub end_state: Option<i32>,
  pub end_time: Option<NaiveDateTime>,
  pub error_msg: Option<String>,
  pub records_processed: Option<i32>,
}

/// Insertable form of [`ProcState`].
///
/// To start tracking a new process run:
/// 1. Set `proc_id` to the [`ProcType::id`] (from [`ProcType::find_or_create`]).
/// 2. Set `start_time` to `chrono::Utc::now().naive_utc()`.
/// 3. Leave `end_state`, `end_time`, `error_msg`, and `records_processed`
///    as `None` — they are updated later via [`ProcState`] methods.
#[derive(Insertable, Debug)]
#[diesel(table_name = procstates)]
pub struct NewProcState {
  pub proc_id: Option<i32>,
  pub start_time: NaiveDateTime,
  pub end_state: Option<i32>,
  pub end_time: Option<NaiveDateTime>,
  pub error_msg: Option<String>,
  pub records_processed: Option<i32>,
}

/// Synchronous query methods for [`ProcType`].
impl ProcType {
  /// Looks up a process type by name, creating it if it doesn't exist
  /// (find-or-create pattern).
  ///
  /// Returns the existing or newly-created [`ProcType`] with its `id`.
  pub fn find_or_create(
    conn: &mut PgConnection,
    process_name: &str,
  ) -> Result<Self, diesel::result::Error> {
    use crate::schema::proctypes::dsl::*;

    // Try to find existing
    match proctypes.filter(name.eq(process_name)).first::<ProcType>(conn).optional()? {
      Some(proc_type) => Ok(proc_type),
      None => {
        // Create new
        diesel::insert_into(proctypes).values(NewProcType { name: process_name }).get_result(conn)
      }
    }
  }

  /// Looks up a process type by its numeric ID.
  pub fn find_by_id(conn: &mut PgConnection, proc_id: i32) -> Result<Self, diesel::result::Error> {
    use crate::schema::proctypes::dsl::*;

    proctypes.find(proc_id).first(conn)
  }
}

impl NewProcState {
  /// Inserts this process state record and returns the created [`ProcState`]
  /// (including the auto-generated `spid`).
  ///
  /// Consumes `self` because the record is moved into the database.
  pub fn insert(self, conn: &mut PgConnection) -> Result<ProcState, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::insert_into(procstates).values(&self).get_result(conn)
  }
}

/// Synchronous mutation and query methods for [`ProcState`].
impl ProcState {
  /// Marks a process as finished by setting `end_state` and `end_time`.
  ///
  /// Use [`state_ids::COMPLETED`] for success or [`state_ids::FAILED`] /
  /// [`state_ids::CANCELLED`] for other outcomes. For failures with a
  /// diagnostic message, prefer [`update_with_error`](Self::update_with_error).
  pub fn update_end_state(
    conn: &mut PgConnection,
    spid_val: i32,
    end_state_val: i32,
    end_time_val: NaiveDateTime,
  ) -> Result<usize, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::update(procstates.find(spid_val))
      .set((end_state.eq(Some(end_state_val)), end_time.eq(Some(end_time_val))))
      .execute(conn)
  }

  /// Marks a process as finished with an error message.
  ///
  /// Sets `end_state`, `end_time`, and `error_msg` in a single update.
  /// Typically called with [`state_ids::FAILED`].
  pub fn update_with_error(
    conn: &mut PgConnection,
    spid_val: i32,
    end_state_val: i32,
    end_time_val: NaiveDateTime,
    error: &str,
  ) -> Result<usize, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::update(procstates.find(spid_val))
      .set((
        end_state.eq(Some(end_state_val)),
        end_time.eq(Some(end_time_val)),
        error_msg.eq(Some(error)),
      ))
      .execute(conn)
  }

  /// Updates the `records_processed` counter for a running process.
  ///
  /// Can be called multiple times during a long-running job to report
  /// progress.
  pub fn update_records_processed(
    conn: &mut PgConnection,
    spid_val: i32,
    count: i32,
  ) -> Result<usize, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::update(procstates.find(spid_val)).set(records_processed.eq(Some(count))).execute(conn)
  }

  /// Returns all currently-running processes (those with `end_state IS NULL`),
  /// ordered by `start_time` descending (most recently started first).
  pub fn get_active(conn: &mut PgConnection) -> Result<Vec<Self>, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    procstates.filter(end_state.is_null()).order(start_time.desc()).load(conn)
  }
}

impl State {
  /// Looks up a state by its human-readable name (e.g., `"completed"`).
  pub fn find_by_name(
    conn: &mut PgConnection,
    state_name: &str,
  ) -> Result<Self, diesel::result::Error> {
    use crate::schema::states::dsl::*;

    states.filter(name.eq(state_name)).first(conn)
  }
}

/// Well-known state IDs matching the `states` table seed data.
///
/// These constants correspond to the rows inserted by the database migration.
/// Use them with [`ProcState::update_end_state`] and
/// [`ProcState::update_with_error`] to avoid hardcoding magic numbers.
///
/// # Example
///
/// ```rust,no_run
/// use av_database_postgres::models::process::{ProcState, state_ids};
///
/// // Mark a process as successfully completed
/// let now = chrono::Utc::now().naive_utc();
/// ProcState::update_end_state(&mut conn, spid, state_ids::COMPLETED, now).unwrap();
/// ```
pub mod state_ids {
  /// The process is currently running.
  pub const STARTED: i32 = 1;
  /// The process finished successfully.
  pub const COMPLETED: i32 = 2;
  /// The process terminated with an error.
  pub const FAILED: i32 = 3;
  /// The process was manually cancelled.
  pub const CANCELLED: i32 = 4;
  /// The process failed but is scheduled for retry.
  pub const RETRYING: i32 = 5;
}
