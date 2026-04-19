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

//! Diesel ORM models for the Alpha Vantage TimescaleDB schema.
//!
//! This module is the public façade for all database model types in the
//! `av-database-postgres` crate. Each sub-module maps to a logical domain
//! area and contains:
//!
//! - **Query structs** — `#[derive(Queryable, Selectable, Identifiable)]` types
//!   that represent rows read from the database.
//! - **Insertable structs** — `#[derive(Insertable)]` types used to write new
//!   rows. These come in two flavors:
//!   - *Borrowed* (e.g., `NewSymbol<'a>`) — borrows string fields for zero-copy
//!     bulk inserts.
//!   - *Owned* (e.g., `NewSymbolOwned`) — owns all data, useful when the
//!     insertable must outlive the source data.
//! - **Changeset structs** — `#[derive(AsChangeset)]` types for partial updates.
//! - **Aggregation / query-result structs** — types used with raw SQL or
//!   `QueryableByName` for analytics queries (sentiment summaries, OHLC buckets,
//!   sector performance, etc.).
//!
//! # Sub-module overview
//!
//! ```text
//! models/
//! ├── mod.rs              ← this file (public façade, re-exports)
//! ├── crypto.rs           → cryptocurrency overview, technical, social, and API mapping
//! ├── crypto_markets.rs   → crypto exchange/trading-pair market data
//! ├── missing_symbols.rs  → unresolved symbol tracking and resolution workflow
//! ├── news.rs             → news articles, feeds, authors, sources, sentiment, topics
//! ├── price.rs            → intraday & summary OHLCV, top movers, sector performance
//! └── security.rs         → symbols, company overviews, equity details, symbol mappings
//! ```
//!
//! # Type inventory by sub-module
//!
//! ## [`crypto`] — Cryptocurrency fundamentals
//!
//! | Type                       | Role                                                          |
//! |----------------------------|---------------------------------------------------------------|
//! | `CryptoOverviewBasic`      | Core crypto data: market cap, price, circulating/max supply   |
//! | `CryptoOverviewMetrics`    | Performance metrics: 24h–1y price changes, ATH/ATL            |
//! | `CryptoOverviewFull`       | Combined view pairing `Basic` + `Metrics`                     |
//! | `CryptoTechnical`          | Blockchain data: consensus mechanism, hashing algo, GitHub stats |
//! | `CryptoSocial`             | Social media / community metrics                              |
//! | `CryptoApiMap`             | Maps Alpha Vantage crypto symbols to internal IDs             |
//! | `CryptoSummary`            | Aggregated crypto analytics result                            |
//! | `New*` variants            | Insertable structs for each of the above                      |
//!
//! ## [`crypto_markets`] — Crypto exchange data
//!
//! | Type                   | Role                                                          |
//! |------------------------|---------------------------------------------------------------|
//! | `CryptoMarket`         | Trading pair on an exchange: volume, liquidity, spread        |
//! | `NewCryptoMarket`      | Insertable for new market records                             |
//! | `UpdateCryptoMarket`   | Changeset for partial market updates                          |
//! | `CryptoMarketsSummary` | Aggregated market-level statistics                            |
//! | `ExchangeStats`        | Per-exchange aggregated metrics                               |
//! | `CryptoMarketInput`    | Input DTO for market data ingestion                           |
//!
//! ## [`missing_symbols`] — Symbol resolution tracking
//!
//! | Type                   | Role                                                          |
//! |------------------------|---------------------------------------------------------------|
//! | `ResolutionStatus`     | Enum: `Pending`, `Found`, `NotFound`, `Skipped`               |
//! | `MissingSymbol`        | Database record for an unresolved symbol reference            |
//! | `NewMissingSymbol`     | Insertable for recording a new missing symbol                 |
//! | `UpdateMissingSymbol`  | Changeset for updating resolution status                      |
//!
//! ## [`news`] — News articles and sentiment
//!
//! | Type                   | Role                                                          |
//! |------------------------|---------------------------------------------------------------|
//! | `NewsOverview`         | Top-level news record linked to a symbol                      |
//! | `Feed`                 | News feed source record                                       |
//! | `Article`              | Individual news article with metadata and sentiment           |
//! | `Author` / `AuthorMap` | Article author and article↔author junction table              |
//! | `Source` / `SourceMap` | News source and article↔source junction                       |
//! | `TickerSentiment`      | Per-ticker sentiment score for an article                     |
//! | `TopicRef` / `TopicMap`| Topic taxonomy and article↔topic junction                     |
//! | `SentimentSummary`     | Aggregated sentiment analytics (averages, distributions)      |
//! | `SentimentTrend`       | Time-bucketed sentiment trends                                |
//! | `TrendingTopic`        | Trending topic with mention counts and sentiment              |
//! | `ProcessedNewsStats`   | Ingestion pipeline statistics                                 |
//! | `NewsData` / `NewsItem`| Deserialization DTOs for API responses                        |
//! | `New*` / `New*Owned`   | Insertable structs (borrowed and owned variants)              |
//!
//! ## [`price`] — OHLCV price data
//!
//! | Type                | Role                                                          |
//! |---------------------|---------------------------------------------------------------|
//! | `IntradayPrice`     | Intraday OHLCV bar with volume                                |
//! | `SummaryPrice`      | Daily/weekly/monthly summary price record                     |
//! | `TopStat`           | Top gainer/loser/most-active snapshot                         |
//! | `OhlcBucket`        | TimescaleDB time-bucket aggregated OHLCV                      |
//! | `PriceWithMA`       | Price row augmented with moving average columns               |
//! | `HistoricalTopMover`| Historical top-mover query result                             |
//! | `SectorPerformance` | Per-sector aggregated performance metrics                     |
//! | `New*` / `New*Owned`| Insertable structs for each record type                       |
//!
//! ## [`security`] — Securities and company data
//!
//! | Type                | Role                                                          |
//! |---------------------|---------------------------------------------------------------|
//! | `Symbol`            | Core security record: ticker, type, region, currency          |
//! | `Overview`          | Company fundamentals: P/E, EBITDA, market cap, sector         |
//! | `Overviewext`       | Extended overview with additional fundamental fields          |
//! | `EquityDetail`      | Equity-specific detail record                                 |
//! | `SymbolMapping`     | Maps external identifiers to internal symbol IDs              |
//! | `New*` / `New*Owned`| Insertable structs (borrowed and owned variants)              |
//!
//! # Common patterns
//!
//! - **Diesel derives:** All query types implement `Queryable`, `Selectable`,
//!   and `Identifiable`. Insertable types implement `Insertable`.
//! - **Serde support:** Most types derive `Serialize` and `Deserialize` for
//!   JSON interop.
//! - **Precision:** Financial values use [`bigdecimal::BigDecimal`] to avoid
//!   floating-point rounding errors.
//! - **Timestamps:** `chrono::DateTime<Utc>` for timezone-aware columns,
//!   `chrono::NaiveDateTime` for timezone-naive columns.
//! - **Async:** The crate uses `diesel-async` for non-blocking database access.
//!
//! # Re-exports
//!
//! The `pub use` statements below hoist the most commonly used types to
//! `av_database_postgres::models::*`. The full sub-modules remain accessible
//! for types not re-exported here.

/// Cryptocurrency fundamental data: overviews, technical blockchain metrics,
/// social metrics, and Alpha Vantage API symbol mapping.
pub mod crypto;

/// Crypto exchange and trading-pair market data: volume, liquidity, spreads,
/// and per-exchange aggregated statistics.
pub mod crypto_markets;

/// Tracks unresolved symbol references encountered during data ingestion.
/// Supports a resolution workflow with status transitions
/// (`Pending` → `Found` / `NotFound` / `Skipped`).
pub mod missing_symbols;

/// News articles, feeds, authors, sources, ticker-level sentiment scores,
/// topic taxonomy, and aggregated sentiment analytics.
pub mod news;

/// OHLCV price data: intraday bars, daily summaries, top gainers/losers,
/// TimescaleDB time-bucket aggregations, and sector performance.
pub mod price;

/// Core security records: ticker symbols, company overviews, extended
/// fundamentals, equity details, and external-to-internal symbol mappings.
pub mod security;

// ─── Convenience re-exports ─────────────────────────────────────────────────
//
// Hoist the most frequently used types so downstream code can import from
// `models::*` without spelling out individual sub-module paths.

/// Re-exported from [`crypto`]: overview, technical, social, and API-map types
/// with their insertable counterparts.
pub use crypto::{
  CryptoApiMap, CryptoOverviewBasic, CryptoOverviewFull, CryptoOverviewMetrics, CryptoSocial,
  CryptoTechnical, NewCryptoApiMap, NewCryptoOverviewBasic, NewCryptoOverviewMetrics,
  NewCryptoSocial, NewCryptoTechnical,
};

/// Re-exported from [`missing_symbols`]: resolution status enum, query/insert/update types.
pub use missing_symbols::{MissingSymbol, NewMissingSymbol, ResolutionStatus, UpdateMissingSymbol};

/// Re-exported from [`news`]: core article, feed, overview, and sentiment types.
pub use news::{Article, Feed, NewsOverview, TickerSentiment};

/// Re-exported from [`price`]: intraday and summary OHLCV, top-mover snapshots.
pub use price::{IntradayPrice, SummaryPrice, TopStat};

/// Re-exported from [`security`]: symbol records, company overviews (including
/// extended), symbol mappings, and their owned insertable variants.
pub use security::{
  NewOverviewOwned, NewOverviewextOwned, NewSymbol, NewSymbolMapping, NewSymbolOwned, Overview,
  Overviewext, Symbol, SymbolMapping,
};
