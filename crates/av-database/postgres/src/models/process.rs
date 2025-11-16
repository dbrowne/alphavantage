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


//! Process tracking models for ETL job monitoring

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// Note: The schema module should already have these tables defined from migrations
use crate::schema::{procstates, proctypes, states};

// ===== ProcType =====
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = proctypes)]
pub struct ProcType {
  pub id: i32,
  pub name: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = proctypes)]
pub struct NewProcType<'a> {
  pub name: &'a str,
}

// ===== State =====
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = states)]
pub struct State {
  pub id: i32,
  pub name: String,
}

// ===== ProcState =====
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

// Implementation methods
impl ProcType {
  /// Find existing process type or create a new one
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

  /// Get process type by ID
  pub fn find_by_id(conn: &mut PgConnection, proc_id: i32) -> Result<Self, diesel::result::Error> {
    use crate::schema::proctypes::dsl::*;

    proctypes.find(proc_id).first(conn)
  }
}

impl NewProcState {
  /// Insert a new process state record
  pub fn insert(self, conn: &mut PgConnection) -> Result<ProcState, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::insert_into(procstates).values(&self).get_result(conn)
  }
}

impl ProcState {
  /// Update the end state and time for a process
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

  /// Update with error message
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

  /// Update records processed count
  pub fn update_records_processed(
    conn: &mut PgConnection,
    spid_val: i32,
    count: i32,
  ) -> Result<usize, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    diesel::update(procstates.find(spid_val)).set(records_processed.eq(Some(count))).execute(conn)
  }

  /// Get active processes (not completed)
  pub fn get_active(conn: &mut PgConnection) -> Result<Vec<Self>, diesel::result::Error> {
    use crate::schema::procstates::dsl::*;

    procstates.filter(end_state.is_null()).order(start_time.desc()).load(conn)
  }
}

impl State {
  /// Get state by name
  pub fn find_by_name(
    conn: &mut PgConnection,
    state_name: &str,
  ) -> Result<Self, diesel::result::Error> {
    use crate::schema::states::dsl::*;

    states.filter(name.eq(state_name)).first(conn)
  }
}

// Constants for state IDs (based on migration data)
pub mod state_ids {
  pub const STARTED: i32 = 1;
  pub const COMPLETED: i32 = 2;
  pub const FAILED: i32 = 3;
  pub const CANCELLED: i32 = 4;
  pub const RETRYING: i32 = 5;
}
