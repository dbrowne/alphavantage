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

//! Security ID (SID) generator for the bootstrap loaders.
//!
//! Every row in the `symbols` table is keyed by a 64-bit integer SID that
//! encodes both a [`SecurityType`] (Equity, Cryptocurrency, ETF, etc.) and a
//! sequential `raw_id` within that type. The encoding is performed by
//! [`SecurityType::encode`] / decoded by [`SecurityIdentifier::decode`].
//!
//! This module provides [`SidGenerator`], a stateful counter that:
//!
//! 1. Reads all existing SIDs from `symbols` on initialization.
//! 2. Decodes each SID to find the maximum `raw_id` per [`SecurityType`].
//! 3. Hands out the next sequential SID per type via [`SidGenerator::next_sid`].
//!
//! ## Used By
//!
//! - [`super::securities`] — Bootstrap NASDAQ/NYSE equity loader.
//! - [`super::missing_symbols`] — Resolves unknown symbols from news feeds
//!   and creates new `symbols` rows.
//!
//! [`super::crypto`] uses its own `CryptoSidGenerator` defined inline because
//! it has slightly different scanning semantics (cryptocurrency-only filter).
//!
//! ## Thread Safety and Lifetime
//!
//! The generator is **not thread-safe** — it maintains a mutable per-type
//! counter without locking. It must be used within a single thread (typically
//! inside a `spawn_blocking` task) and constructed at the start of a save
//! operation. Multiple concurrent generators would race and produce duplicate
//! SIDs.
//!
//! For the same reason, the generator should not be cached across operations:
//! its in-memory state can drift from the database if other processes (or
//! other generators in this process) insert new SIDs concurrently.

use anyhow::Result;
use av_core::types::market::{SecurityIdentifier, SecurityType};
use diesel::prelude::*;
use std::collections::HashMap;
use tracing::{debug, info};

///  SID generator that allocates monotonically increasing IDs per
/// [`SecurityType`].
///
/// Maintains an in-memory `HashMap<SecurityType, next_raw_id>`. The map is
/// seeded from the database during construction (via [`Self::new`]) and
/// updated by each call to [`Self::next_sid`].
///
/// ## Encoding
///
/// SIDs are 64-bit integers where the upper bits encode the [`SecurityType`]
/// and the lower bits hold the sequential `raw_id`. Encoding/decoding is
/// delegated to [`SecurityType::encode`] / [`SecurityIdentifier::decode`].
///
/// ## Thread Safety
///
/// **Not thread-safe.** Use within a single thread (typically inside a
/// `spawn_blocking` task). See the module-level docs for details.
pub struct SidGenerator {
  /// Per-type next-available `raw_id` counter. Missing types are treated as
  /// starting at `1` (see [`Self::next_sid`]).
  next_raw_ids: HashMap<SecurityType, u32>,
}

impl SidGenerator {
  /// Initializes a new generator by scanning every existing SID in the
  /// `symbols` table.
  ///
  /// For each SID, decodes the [`SecurityIdentifier`] and tracks the maximum
  /// `raw_id` per security type. The generator's internal counter is then set
  /// to `max_id + 1` for each type, so the first call to [`Self::next_sid`]
  /// returns a fresh, unused SID.
  ///
  /// SIDs that fail to decode (corrupt or zero) are silently skipped — they
  /// don't contribute to any type's max.
  ///
  /// # Errors
  ///
  /// Returns an error if the database query for existing SIDs fails.
  pub fn new(conn: &mut PgConnection) -> Result<Self> {
    use av_database_postgres::schema::symbols::dsl::*;

    info!("Initializing SID generator - reading existing SIDs from database");

    // Get all existing SIDs
    let sids: Vec<i64> = symbols.select(sid).load(conn)?;

    let mut max_raw_ids: HashMap<SecurityType, u32> = HashMap::new();

    // Decode each SID to find max raw_id per type
    for sid_val in sids {
      if let Some(identifier) = SecurityIdentifier::decode(sid_val) {
        let current_max = max_raw_ids.entry(identifier.security_type).or_insert(0);
        if identifier.raw_id > *current_max {
          *current_max = identifier.raw_id;
        }
      }
    }

    // Convert to next available IDs
    let mut next_ids: HashMap<SecurityType, u32> = HashMap::new();
    for (security_type_val, max_id) in max_raw_ids {
      next_ids.insert(security_type_val, max_id + 1);
      debug!("SecurityType::{:?} next raw_id: {}", security_type_val, max_id + 1);
    }

    info!("SID generator initialized with {} security types", next_ids.len());

    Ok(Self { next_raw_ids: next_ids })
  }

  /// Returns the next SID for the given [`SecurityType`] and advances the counter.
  ///
  /// If this is the first call for a security type that wasn't found in the
  /// database during initialization, the counter starts at `1` (so the first
  /// SID for that type encodes `raw_id = 1`).
  ///
  /// Each call:
  /// 1. Looks up (or initializes) the per-type counter.
  /// 2. Encodes the current `raw_id` with the type via [`SecurityType::encode`].
  /// 3. Increments the counter so the next call returns a different SID.
  pub fn next_sid(&mut self, security_type: SecurityType) -> i64 {
    let raw_id = self.next_raw_ids.entry(security_type).or_insert(1);
    let sid = SecurityType::encode(security_type, *raw_id);
    *raw_id += 1; // Increment for next use
    sid
  }
}
