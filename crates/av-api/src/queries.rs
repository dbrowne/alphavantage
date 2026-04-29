/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Pre-built queries for common data retrieval patterns.
//!
//! These queries join across multiple tables to return rich, composite
//! results in a single database round-trip.

use crate::error::Result;
use av_database_postgres::DatabaseContext;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Date, Float4, Nullable, Text};
use serde::{Deserialize, Serialize};

/// A point-in-time snapshot of a security: identity, description, and
/// most recent closing price.
///
/// Returned by [`security_snapshot`] and [`security_snapshot_by_sid`].
///
/// # Fields
///
/// | Field              | Source table    | Description                        |
/// |--------------------|----------------|------------------------------------|
/// | `sid`              | `symbols`      | Bitmap-encoded security ID         |
/// | `symbol`           | `symbols`      | Ticker (e.g., `"AAPL"`)           |
/// | `name`             | `symbols`      | Short name (e.g., `"Apple Inc"`)   |
/// | `sec_type`         | `symbols`      | Security type (e.g., `"Equity"`)   |
/// | `exchange`         | `overviews`    | Listed exchange (e.g., `"NASDAQ"`) |
/// | `sector`           | `overviews`    | GICS sector                        |
/// | `description`      | `overviews`    | Full company description           |
/// | `market_cap`       | `overviews`    | Market capitalisation (USD)        |
/// | `last_close`       | `summaryprices`| Most recent closing price          |
/// | `last_volume`      | `summaryprices`| Most recent trading volume         |
/// | `last_price_date`  | `summaryprices`| Date of the most recent price bar  |
#[derive(Debug, Clone, QueryableByName, Serialize, Deserialize)]
pub struct SecuritySnapshot {
  #[diesel(sql_type = BigInt)]
  pub sid: i64,

  #[diesel(sql_type = Text)]
  pub symbol: String,

  #[diesel(sql_type = Text)]
  pub name: String,

  #[diesel(sql_type = Text)]
  pub sec_type: String,

  #[diesel(sql_type = Nullable<Text>)]
  pub exchange: Option<String>,

  #[diesel(sql_type = Nullable<Text>)]
  pub sector: Option<String>,

  #[diesel(sql_type = Nullable<Text>)]
  pub description: Option<String>,

  #[diesel(sql_type = Nullable<BigInt>)]
  pub market_cap: Option<i64>,

  #[diesel(sql_type = Nullable<Float4>)]
  pub last_close: Option<f32>,

  #[diesel(sql_type = Nullable<BigInt>)]
  pub last_volume: Option<i64>,

  #[diesel(sql_type = Nullable<Date>)]
  pub last_price_date: Option<NaiveDate>,
}

/// The SQL query used by both lookup functions.
///
/// Joins `symbols` with `overviews` (LEFT — overview may not exist yet)
/// and a lateral subquery on `summaryprices` to get only the most recent
/// price bar. Using `LATERAL` avoids pulling the entire price history.
const SNAPSHOT_SQL: &str = r#"
    SELECT
        s.sid,
        s.symbol,
        s.name,
        s.sec_type,
        o.exchange,
        o.sector,
        o.description,
        o.market_capitalization AS market_cap,
        p.close              AS last_close,
        p.volume             AS last_volume,
        p.date               AS last_price_date
    FROM symbols s
    LEFT JOIN overviews o ON o.sid = s.sid
    LEFT JOIN LATERAL (
        SELECT close, volume, date
        FROM summaryprices
        WHERE sid = s.sid
        ORDER BY date DESC
        LIMIT 1
    ) p ON true
"#;

/// Fetches a [`SecuritySnapshot`] by ticker symbol (e.g., `"AAPL"`).
///
/// Returns `Ok(None)` if the symbol doesn't exist.
///
/// # Example
///
/// ```rust,no_run
/// # async fn run(db: &av_database_postgres::DatabaseContext) -> av_api::error::Result<()> {
/// use av_api::queries::security_snapshot;
///
/// if let Some(snap) = security_snapshot(db, "AAPL").await? {
///     println!("{} ({}) — last close: {:?}", snap.name, snap.symbol, snap.last_close);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn security_snapshot(
  db: &DatabaseContext,
  ticker: &str,
) -> Result<Option<SecuritySnapshot>> {
  let ticker = ticker.to_uppercase();
  let result = db
    .run(move |conn| {
      let query = format!("{} WHERE UPPER(s.symbol) = $1", SNAPSHOT_SQL);
      diesel::sql_query(query)
        .bind::<Text, _>(&ticker)
        .get_result::<SecuritySnapshot>(conn)
        .optional()
        .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

/// Fetches a [`SecuritySnapshot`] by security ID (`sid`).
///
/// Returns `Ok(None)` if the SID doesn't exist.
pub async fn security_snapshot_by_sid(
  db: &DatabaseContext,
  sid: i64,
) -> Result<Option<SecuritySnapshot>> {
  let result = db
    .run(move |conn| {
      let query = format!("{} WHERE s.sid = $1", SNAPSHOT_SQL);
      diesel::sql_query(query)
        .bind::<BigInt, _>(sid)
        .get_result::<SecuritySnapshot>(conn)
        .optional()
        .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

/// Fetches [`SecuritySnapshot`]s for multiple tickers in a single query.
///
/// Tickers are matched case-insensitively. The result order is not
/// guaranteed to match the input order.
pub async fn security_snapshots(
  db: &DatabaseContext,
  tickers: &[&str],
) -> Result<Vec<SecuritySnapshot>> {
  let tickers: Vec<String> = tickers.iter().map(|t| t.to_uppercase()).collect();
  let result = db
    .run(move |conn| {
      let query = format!("{} WHERE UPPER(s.symbol) = ANY($1)", SNAPSHOT_SQL);
      diesel::sql_query(query)
        .bind::<diesel::sql_types::Array<Text>, _>(&tickers)
        .load::<SecuritySnapshot>(conn)
        .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

/// Fetches [`SecuritySnapshot`]s for all securities in a given sector.
///
/// `sector` is matched case-insensitively (e.g., `"TECHNOLOGY"`).
pub async fn security_snapshots_by_sector(
  db: &DatabaseContext,
  sector: &str,
) -> Result<Vec<SecuritySnapshot>> {
  let sector = sector.to_uppercase();
  let result = db
    .run(move |conn| {
      let query = format!(
        "{} WHERE UPPER(o.sector) = $1 ORDER BY o.market_capitalization DESC",
        SNAPSHOT_SQL
      );
      diesel::sql_query(query)
        .bind::<Text, _>(&sector)
        .load::<SecuritySnapshot>(conn)
        .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

// ─── SID lookup (handles duplicate symbols) ─────────────────────────────────

/// A lightweight record identifying a security by SID, with enough context
/// to disambiguate duplicate ticker symbols (common in crypto — e.g., multiple
/// coins share the ticker `"ONE"` or `"LUNA"`).
///
/// Returned by [`get_sids`].
#[derive(Debug, Clone, QueryableByName, Serialize, Deserialize)]
pub struct SidEntry {
  /// Bitmap-encoded security ID.
  #[diesel(sql_type = BigInt)]
  pub sid: i64,

  /// Ticker symbol (e.g., `"BTC"`, `"LUNA"`).
  #[diesel(sql_type = Text)]
  pub symbol: String,

  /// Full security name — the primary way to distinguish duplicates
  /// (e.g., `"Terra"` vs. `"Terra Classic"` for `"LUNA"`).
  #[diesel(sql_type = Text)]
  pub name: String,

  /// Security type (e.g., `"Equity"`, `"Cryptocurrency"`).
  #[diesel(sql_type = Text)]
  pub sec_type: String,

  /// Ingestion priority — lower values indicate higher priority.
  /// For crypto symbols loaded from multiple providers, the provider
  /// with the best data (e.g., CoinGecko) gets a lower priority number.
  #[diesel(sql_type = diesel::sql_types::Integer)]
  pub priority: i32,
}

/// Fetches all SIDs matching a ticker symbol, ordered by priority (best first).
///
/// Unlike [`security_snapshot`], this function returns **all** rows that
/// share the same ticker. This is essential for cryptocurrencies where
/// the same symbol may map to multiple coins (e.g., `"LUNA"` → Terra
/// and Terra Classic, `"ONE"` → Harmony and BigONE).
///
/// Results are ordered by `priority ASC` (lowest = best), then by `name`
/// for stable ordering among equal-priority entries.
///
/// # Examples
///
/// ```rust,no_run
/// # async fn run(db: &av_database_postgres::DatabaseContext) -> av_api::error::Result<()> {
/// use av_api::queries::get_sids;
///
/// let entries = get_sids(db, "LUNA").await?;
/// for e in &entries {
///     println!("SID {} — {} ({}) priority={}", e.sid, e.name, e.sec_type, e.priority);
/// }
/// // Might print:
/// //   SID 12345 — Terra (Cryptocurrency) priority=1
/// //   SID 67890 — Terra Classic (Cryptocurrency) priority=4
/// # Ok(())
/// # }
/// ```
pub async fn get_sids(
  db: &DatabaseContext,
  ticker: &str,
) -> Result<Vec<SidEntry>> {
  let ticker = ticker.to_uppercase();
  let result = db
    .run(move |conn| {
      diesel::sql_query(
        "SELECT sid, symbol, name, sec_type, priority
         FROM symbols
         WHERE UPPER(symbol) = $1
         ORDER BY priority ASC, name ASC",
      )
      .bind::<Text, _>(&ticker)
      .load::<SidEntry>(conn)
      .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

/// Fetches all SIDs matching a ticker, filtered to a specific security type.
///
/// Useful when you know you want the crypto version (or the equity version)
/// of an ambiguous ticker.
///
/// ```rust,no_run
/// # async fn run(db: &av_database_postgres::DatabaseContext) -> av_api::error::Result<()> {
/// use av_api::queries::get_sids_by_type;
///
/// // Only crypto entries for "ONE"
/// let cryptos = get_sids_by_type(db, "ONE", "Cryptocurrency").await?;
/// # Ok(())
/// # }
/// ```
pub async fn get_sids_by_type(
  db: &DatabaseContext,
  ticker: &str,
  sec_type: &str,
) -> Result<Vec<SidEntry>> {
  let ticker = ticker.to_uppercase();
  let sec_type = sec_type.to_string();
  let result = db
    .run(move |conn| {
      diesel::sql_query(
        "SELECT sid, symbol, name, sec_type, priority
         FROM symbols
         WHERE UPPER(symbol) = $1 AND sec_type = $2
         ORDER BY priority ASC, name ASC",
      )
      .bind::<Text, _>(&ticker)
      .bind::<Text, _>(&sec_type)
      .load::<SidEntry>(conn)
      .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

/// Returns the **best** (lowest-priority) SID for a ticker, or `None` if
/// the ticker doesn't exist.
///
/// This is the "give me the most important one" shorthand — equivalent to
/// `get_sids(db, ticker).await?.first().map(|e| e.sid)` but done in a
/// single `LIMIT 1` query.
pub async fn get_best_sid(
  db: &DatabaseContext,
  ticker: &str,
) -> Result<Option<SidEntry>> {
  let ticker = ticker.to_uppercase();
  let result = db
    .run(move |conn| {
      diesel::sql_query(
        "SELECT sid, symbol, name, sec_type, priority
         FROM symbols
         WHERE UPPER(symbol) = $1
         ORDER BY priority ASC, name ASC
         LIMIT 1",
      )
      .bind::<Text, _>(&ticker)
      .get_result::<SidEntry>(conn)
      .optional()
      .map_err(Into::into)
    })
    .await?;
  Ok(result)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  /// Helper: connect to the test database or skip.
  fn test_db() -> DatabaseContext {
    let url = std::env::var("DATABASE_URL")
      .unwrap_or_else(|_| "postgresql://ts_user:dev_pw@localhost:6433/sec_master".to_string());
    DatabaseContext::new(&url).expect("Failed to connect to test database")
  }

  // ── Unit tests (no database required) ───────────────────────────────

  #[test]
  fn test_snapshot_sql_is_valid_fragment() {
    // The base SQL should contain the expected joins.
    assert!(SNAPSHOT_SQL.contains("FROM symbols s"));
    assert!(SNAPSHOT_SQL.contains("LEFT JOIN overviews o"));
    assert!(SNAPSHOT_SQL.contains("LEFT JOIN LATERAL"));
    // The fragment should end with the lateral join so callers can append WHERE.
    // (The LATERAL subquery itself contains WHERE, but the top-level does not.)
    let after_lateral = SNAPSHOT_SQL.rsplit_once("p ON true").unwrap().1.trim();
    assert!(after_lateral.is_empty(), "Unexpected trailing SQL after lateral join: {:?}", after_lateral);
  }

  #[test]
  fn test_security_snapshot_serde_roundtrip() {
    let snap = SecuritySnapshot {
      sid: 12345,
      symbol: "AAPL".to_string(),
      name: "Apple Inc".to_string(),
      sec_type: "Equity".to_string(),
      exchange: Some("NASDAQ".to_string()),
      sector: Some("TECHNOLOGY".to_string()),
      description: Some("Apple Inc designs and manufactures...".to_string()),
      market_cap: Some(3_000_000_000_000),
      last_close: Some(182.63),
      last_volume: Some(50_000_000),
      last_price_date: Some(NaiveDate::from_ymd_opt(2025, 4, 18).unwrap()),
    };

    let json = serde_json::to_string(&snap).unwrap();
    let deserialized: SecuritySnapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.sid, snap.sid);
    assert_eq!(deserialized.symbol, snap.symbol);
    assert_eq!(deserialized.name, snap.name);
    assert_eq!(deserialized.sec_type, snap.sec_type);
    assert_eq!(deserialized.exchange, snap.exchange);
    assert_eq!(deserialized.sector, snap.sector);
    assert_eq!(deserialized.description, snap.description);
    assert_eq!(deserialized.market_cap, snap.market_cap);
    assert_eq!(deserialized.last_close, snap.last_close);
    assert_eq!(deserialized.last_volume, snap.last_volume);
    assert_eq!(deserialized.last_price_date, snap.last_price_date);
  }

  #[test]
  fn test_security_snapshot_with_all_none_optionals() {
    // Simulates a symbol with no overview and no price history.
    let snap = SecuritySnapshot {
      sid: 99999,
      symbol: "UNKNOWN".to_string(),
      name: "Unknown Corp".to_string(),
      sec_type: "Equity".to_string(),
      exchange: None,
      sector: None,
      description: None,
      market_cap: None,
      last_close: None,
      last_volume: None,
      last_price_date: None,
    };

    let json = serde_json::to_string(&snap).unwrap();
    assert!(json.contains("\"last_close\":null"));

    let deserialized: SecuritySnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.exchange, None);
    assert_eq!(deserialized.last_close, None);
    assert_eq!(deserialized.last_price_date, None);
  }

  #[test]
  fn test_security_snapshot_clone() {
    let snap = SecuritySnapshot {
      sid: 1,
      symbol: "X".to_string(),
      name: "X Corp".to_string(),
      sec_type: "Equity".to_string(),
      exchange: None,
      sector: None,
      description: None,
      market_cap: None,
      last_close: Some(10.0),
      last_volume: None,
      last_price_date: None,
    };

    let cloned = snap.clone();
    assert_eq!(cloned.sid, snap.sid);
    assert_eq!(cloned.last_close, snap.last_close);
  }

  // ── Integration tests (require live database) ───────────────────────

  #[tokio::test]
  #[ignore] // Requires DATABASE_URL or local TimescaleDB at localhost:6433
  async fn test_snapshot_existing_symbol() {
    let db = test_db();

    // AAPL is expected to exist in a populated database.
    let result = security_snapshot(&db, "AAPL").await;
    assert!(result.is_ok(), "Query should not error: {:?}", result.err());

    if let Ok(Some(snap)) = result {
      assert_eq!(snap.symbol, "AAPL");
      assert!(!snap.name.is_empty());
      assert_eq!(snap.sec_type, "Equity");
      // If overview data was loaded, these should be populated.
      if snap.exchange.is_some() {
        assert!(!snap.exchange.as_ref().unwrap().is_empty());
      }
    }
    // Ok(None) is also acceptable if DB is empty — test just verifies no errors.
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshot_case_insensitive() {
    let db = test_db();

    let lower = security_snapshot(&db, "aapl").await.unwrap();
    let upper = security_snapshot(&db, "AAPL").await.unwrap();
    let mixed = security_snapshot(&db, "aApL").await.unwrap();

    // All three should resolve to the same symbol (or all None).
    match (&lower, &upper, &mixed) {
      (Some(l), Some(u), Some(m)) => {
        assert_eq!(l.sid, u.sid);
        assert_eq!(u.sid, m.sid);
      }
      (None, None, None) => {} // DB empty — acceptable
      _ => panic!("Case-insensitive lookup returned inconsistent results"),
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshot_nonexistent_symbol() {
    let db = test_db();

    let result = security_snapshot(&db, "ZZZZZNOTREAL").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshot_by_sid_nonexistent() {
    let db = test_db();

    let result = security_snapshot_by_sid(&db, -999).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshot_by_sid_matches_ticker_lookup() {
    let db = test_db();

    // Look up by ticker first, then verify by-SID returns the same data.
    if let Some(by_ticker) = security_snapshot(&db, "AAPL").await.unwrap() {
      let by_sid = security_snapshot_by_sid(&db, by_ticker.sid).await.unwrap();
      assert!(by_sid.is_some());

      let by_sid = by_sid.unwrap();
      assert_eq!(by_sid.sid, by_ticker.sid);
      assert_eq!(by_sid.symbol, by_ticker.symbol);
      assert_eq!(by_sid.last_close, by_ticker.last_close);
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshots_multiple_tickers() {
    let db = test_db();

    let results = security_snapshots(&db, &["AAPL", "MSFT", "ZZZNOTREAL"]).await;
    assert!(results.is_ok(), "Batch query should not error: {:?}", results.err());

    let snaps = results.unwrap();
    // ZZZNOTREAL should not appear; AAPL and MSFT may or may not be in DB.
    for snap in &snaps {
      assert_ne!(snap.symbol, "ZZZNOTREAL");
    }
    // No duplicates.
    let sids: Vec<i64> = snaps.iter().map(|s| s.sid).collect();
    let unique: std::collections::HashSet<i64> = sids.iter().copied().collect();
    assert_eq!(sids.len(), unique.len(), "Batch query returned duplicates");
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshots_empty_input() {
    let db = test_db();

    let results = security_snapshots(&db, &[]).await;
    assert!(results.is_ok());
    assert!(results.unwrap().is_empty());
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshots_by_sector() {
    let db = test_db();

    let results = security_snapshots_by_sector(&db, "TECHNOLOGY").await;
    assert!(results.is_ok(), "Sector query should not error: {:?}", results.err());

    let snaps = results.unwrap();
    // All results should belong to the requested sector (case-insensitive).
    for snap in &snaps {
      if let Some(ref sector) = snap.sector {
        assert_eq!(
          sector.to_uppercase(),
          "TECHNOLOGY",
          "Sector mismatch: {} for {}",
          sector,
          snap.symbol
        );
      }
    }

    // Results should be ordered by market_cap descending.
    for pair in snaps.windows(2) {
      match (pair[0].market_cap, pair[1].market_cap) {
        (Some(a), Some(b)) => assert!(a >= b, "Not ordered by market_cap DESC"),
        _ => {} // NULLs can appear anywhere in SQL ORDER BY
      }
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshots_by_sector_nonexistent() {
    let db = test_db();

    let results = security_snapshots_by_sector(&db, "UNDERWATER_BASKET_WEAVING").await;
    assert!(results.is_ok());
    assert!(results.unwrap().is_empty());
  }

  #[tokio::test]
  #[ignore]
  async fn test_snapshot_has_price_data_when_available() {
    let db = test_db();

    // Find any symbol that has summary price data.
    if let Some(snap) = security_snapshot(&db, "AAPL").await.unwrap() {
      if snap.last_close.is_some() {
        // If there's a close price, there should also be a date and volume.
        assert!(snap.last_price_date.is_some(), "Close present but date missing");
        assert!(snap.last_volume.is_some(), "Close present but volume missing");
        // Price should be positive.
        assert!(snap.last_close.unwrap() > 0.0, "Close price should be positive");
      }
    }
  }

  // ── SidEntry unit tests ─────────────────────────────────────────────

  #[test]
  fn test_sid_entry_serde_roundtrip() {
    let entry = SidEntry {
      sid: 42,
      symbol: "LUNA".to_string(),
      name: "Terra".to_string(),
      sec_type: "Cryptocurrency".to_string(),
      priority: 1,
    };

    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: SidEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.sid, 42);
    assert_eq!(deserialized.symbol, "LUNA");
    assert_eq!(deserialized.name, "Terra");
    assert_eq!(deserialized.sec_type, "Cryptocurrency");
    assert_eq!(deserialized.priority, 1);
  }

  #[test]
  fn test_sid_entry_clone() {
    let entry = SidEntry {
      sid: 1,
      symbol: "X".to_string(),
      name: "X Token".to_string(),
      sec_type: "Cryptocurrency".to_string(),
      priority: 99,
    };
    let cloned = entry.clone();
    assert_eq!(cloned.sid, entry.sid);
    assert_eq!(cloned.priority, entry.priority);
  }

  // ── SID lookup integration tests ────────────────────────────────────

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_single_match() {
    let db = test_db();

    // AAPL should have exactly one entry (equity, no crypto duplicate).
    let entries = get_sids(&db, "AAPL").await;
    assert!(entries.is_ok(), "get_sids should not error: {:?}", entries.err());

    let entries = entries.unwrap();
    if !entries.is_empty() {
      assert_eq!(entries[0].symbol, "AAPL");
      assert_eq!(entries[0].sec_type, "Equity");
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_case_insensitive() {
    let db = test_db();

    let lower = get_sids(&db, "aapl").await.unwrap();
    let upper = get_sids(&db, "AAPL").await.unwrap();

    assert_eq!(lower.len(), upper.len());
    for (l, u) in lower.iter().zip(upper.iter()) {
      assert_eq!(l.sid, u.sid);
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_nonexistent() {
    let db = test_db();

    let entries = get_sids(&db, "ZZZNOTREAL").await.unwrap();
    assert!(entries.is_empty());
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_ordered_by_priority() {
    let db = test_db();

    // Use a symbol likely to have entries; verify ordering.
    let entries = get_sids(&db, "BTC").await.unwrap();
    for pair in entries.windows(2) {
      assert!(
        pair[0].priority <= pair[1].priority,
        "Expected priority ascending: {} ({}) should come before {} ({})",
        pair[0].name,
        pair[0].priority,
        pair[1].name,
        pair[1].priority,
      );
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_by_type_filters_correctly() {
    let db = test_db();

    let cryptos = get_sids_by_type(&db, "BTC", "Cryptocurrency").await.unwrap();
    for entry in &cryptos {
      assert_eq!(entry.sec_type, "Cryptocurrency",
        "Expected Cryptocurrency but got {} for SID {}", entry.sec_type, entry.sid);
    }

    let equities = get_sids_by_type(&db, "BTC", "Equity").await.unwrap();
    for entry in &equities {
      assert_eq!(entry.sec_type, "Equity");
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_sids_by_type_nonexistent_type() {
    let db = test_db();

    let entries = get_sids_by_type(&db, "AAPL", "Cryptocurrency").await.unwrap();
    // AAPL is an equity, not a crypto — should return empty.
    assert!(entries.is_empty(), "AAPL should not appear as Cryptocurrency");
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_best_sid_returns_lowest_priority() {
    let db = test_db();

    let all = get_sids(&db, "BTC").await.unwrap();
    let best = get_best_sid(&db, "BTC").await.unwrap();

    if let Some(best_entry) = &best {
      // Best should match the first entry from the full list.
      assert!(!all.is_empty());
      assert_eq!(best_entry.sid, all[0].sid);
      assert_eq!(best_entry.priority, all[0].priority);
    } else {
      // If best is None, the full list should also be empty.
      assert!(all.is_empty());
    }
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_best_sid_nonexistent() {
    let db = test_db();

    let best = get_best_sid(&db, "ZZZNOTREAL").await.unwrap();
    assert!(best.is_none());
  }

  #[tokio::test]
  #[ignore]
  async fn test_get_best_sid_consistent_with_snapshot() {
    let db = test_db();

    // The best SID for a ticker should match the snapshot lookup.
    if let Some(best) = get_best_sid(&db, "AAPL").await.unwrap() {
      if let Some(snap) = security_snapshot(&db, "AAPL").await.unwrap() {
        assert_eq!(best.sid, snap.sid,
          "get_best_sid and security_snapshot should agree on SID");
      }
    }
  }
}
