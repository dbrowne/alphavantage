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

//! Statistics reporting commands for `av-cli update stats`.
//!
//! This module provides read-only analytics over the cryptocurrency data stored
//! in the PostgreSQL database. Unlike the sibling crypto update modules (which
//! fetch from external APIs), everything here queries local data via Diesel ORM
//! and prints formatted reports to stdout.
//!
//! ## Subcommands
//!
//! ```text
//! av-cli update stats
//! ├── crypto-mapping     API symbol mapping coverage and staleness
//! ├── crypto-markets     Market data by exchange, volume, activity status
//! └── crypto-overview    High-level database overview with record counts
//! ```
//!
//! ## Database Tables Queried
//!
//! | Table              | Used by                                    |
//! |--------------------|--------------------------------------------|
//! | `symbols`          | All three subcommands (crypto symbol index) |
//! | `crypto_api_map`   | `crypto-mapping`, `crypto-overview`         |
//! | `crypto_markets`   | `crypto-markets`, `crypto-overview`         |
//!
//! ## Configuration
//!
//! This module receives the full CLI [`Config`](crate::config::Config) (not the
//! API-only [`av_core::Config`]), because it needs `database_url` to establish a
//! PostgreSQL connection. The conversion happens in
//! [`handle_update`](crate::handle_update) in `main.rs`.
//!
//! ## Usage
//!
//! ```bash
//! # Show mapping coverage for a specific API source
//! av-cli update stats crypto-mapping --source CoinGecko --detailed
//!
//! # Show unmapped and stale symbols (older than 60 days)
//! av-cli update stats crypto-mapping --unmapped --stale --stale-days 60
//!
//! # Show market data for a specific symbol, including volume
//! av-cli update stats crypto-markets --symbol BTC --volume
//!
//! # Show full database overview with extended stats
//! av-cli update stats crypto-overview --extended
//! ```

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::config::Config;
use av_database_postgres::{
  models::crypto::CryptoApiMap,
  schema::{crypto_api_map, crypto_markets, symbols},
};
use diesel::prelude::*;

/// Subcommands for `av-cli update stats`.
///
/// Three report types, each querying the PostgreSQL database directly via
/// Diesel ORM. All reports are read-only and produce formatted text output
/// on stdout.
#[derive(Subcommand, Debug)]
pub enum StatsCommands {
  /// Report on API symbol mapping coverage and staleness.
  ///
  /// Shows how many cryptocurrency symbols in the `symbols` table have
  /// corresponding entries in `crypto_api_map`, broken down by API source.
  /// Optionally lists unmapped symbols and mappings that have not been
  /// verified within the `--stale-days` threshold.
  CryptoMapping {
    /// Filter results to a specific API source (e.g., `"CoinGecko"`, `"SosoValue"`).
    ///
    /// When omitted, statistics are shown across all sources.
    #[arg(short, long)]
    source: Option<String>,

    /// Show a detailed table of the top 20 mappings ranked by `rank` (descending).
    ///
    /// Columns: Symbol, Source, API ID, Rank.
    #[arg(short, long)]
    detailed: bool,

    /// List symbols that have no active mapping in `crypto_api_map`.
    ///
    /// When `--source` is set, uses [`CryptoApiMap::get_symbols_needing_mapping`]
    /// to find symbols missing a mapping for that specific source. Otherwise,
    /// performs a `LEFT JOIN` to find symbols with no mapping at all.
    /// Output is capped at 10 entries.
    #[arg(short, long)]
    unmapped: bool,

    /// Show mappings whose `last_verified` date exceeds the `--stale-days` threshold.
    ///
    /// Requires `--source` to be set (stale detection is per-source). Uses
    /// [`CryptoApiMap::get_stale_mappings`]. Output is capped at 10 entries.
    #[arg(long)]
    stale: bool,

    /// Number of days after which a mapping is considered stale.
    ///
    /// Used with `--stale`. Defaults to 30 days.
    #[arg(long, default_value = "30")]
    stale_days: i32,
  },

  /// Report on cryptocurrency market data by exchange, symbol, and volume.
  ///
  /// Queries the `crypto_markets` table, optionally joined with `symbols`
  /// when filtering by symbol. By default, only active markets are shown;
  /// use `--inactive` to include inactive ones.
  CryptoMarkets {
    /// Filter to a specific cryptocurrency symbol (e.g., `BTC`).
    ///
    /// Joins `symbols` with `crypto_markets` on `sid` to resolve the symbol name.
    #[arg(short, long)]
    symbol: Option<String>,

    /// Filter to a specific exchange (e.g., `Binance`).
    #[arg(short, long)]
    exchange: Option<String>,

    /// Show aggregate 24-hour volume statistics.
    ///
    /// Computes `SUM(volume_24h)` across matching markets using
    /// [`bigdecimal::BigDecimal`] for precision.
    #[arg(long)]
    volume: bool,

    /// Include inactive markets in the results and show an active/inactive breakdown.
    ///
    /// When omitted, markets where `is_active != true` are excluded from all queries.
    #[arg(long)]
    inactive: bool,
  },

  /// High-level overview of all cryptocurrency data in the database.
  ///
  /// Shows total counts for symbols, API mappings, and markets, plus API
  /// mapping coverage percentage, source breakdown, and the top 5 exchanges
  /// by market count.
  CryptoOverview {
    /// Include extended statistics: top 10 symbols by market count and
    /// market type distribution.
    #[arg(short, long)]
    extended: bool,
  },
}

/// Dispatches `av-cli update stats` subcommands to their report generators.
///
/// Establishes a PostgreSQL connection using `config.database_url` and routes
/// to one of three private async functions based on the [`StatsCommands`]
/// variant.
///
/// # Arguments
///
/// * `cmd` — The parsed [`StatsCommands`] variant with its associated arguments.
/// * `config` — The full CLI [`Config`](crate::config::Config) containing `database_url`.
///
/// # Errors
///
/// Returns errors from:
/// - Database connection failure (invalid or unreachable `database_url`)
/// - Any Diesel query error within the individual report generators
pub async fn handle_stats(cmd: StatsCommands, config: Config) -> Result<()> {
  use diesel::pg::PgConnection;

  let mut conn = PgConnection::establish(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

  match cmd {
    StatsCommands::CryptoMapping { source, detailed, unmapped, stale, stale_days } => {
      execute_crypto_mapping_stats(&mut conn, source, detailed, unmapped, stale, stale_days).await
    }
    StatsCommands::CryptoMarkets { symbol, exchange, volume, inactive } => {
      execute_crypto_market_stats(&mut conn, symbol, exchange, volume, inactive).await
    }
    StatsCommands::CryptoOverview { extended } => {
      execute_crypto_overview_stats(&mut conn, extended).await
    }
  }
}

/// Generates the `crypto-mapping` report.
///
/// Produces up to four output sections depending on the flags:
///
/// 1. **Summary** (always) — Total crypto symbols, active mappings, and coverage
///    percentage. When `source_filter` is `None`, includes a per-source breakdown.
/// 2. **Unmapped symbols** (`show_unmapped`) — Lists symbols with no active mapping.
///    Per-source filtering uses [`CryptoApiMap::get_symbols_needing_mapping`];
///    all-source mode uses a `LEFT JOIN` on `crypto_api_map`. Capped at 10 entries.
/// 3. **Stale mappings** (`show_stale`) — Lists mappings whose `last_verified` date
///    exceeds `stale_days`. Requires `source_filter` to be set. Uses
///    [`CryptoApiMap::get_stale_mappings`]. Capped at 10 entries.
/// 4. **Detailed table** (`detailed`) — Top 20 mappings by `rank` (descending,
///    nulls last) showing Symbol, Source, API ID, and Rank.
async fn execute_crypto_mapping_stats(
  conn: &mut PgConnection,
  source_filter: Option<String>,
  detailed: bool,
  show_unmapped: bool,
  show_stale: bool,
  stale_days: i32,
) -> Result<()> {
  info!("Generating crypto API mapping statistics...");

  // Basic mapping statistics
  let total_mappings: i64 = if let Some(ref source) = source_filter {
    crypto_api_map::table
      .filter(crypto_api_map::api_source.eq(source))
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .count()
      .get_result(conn)?
  } else {
    crypto_api_map::table
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .count()
      .get_result(conn)?
  };

  let total_crypto_symbols: i64 =
    symbols::table.filter(symbols::sec_type.eq("Cryptocurrency")).count().get_result(conn)?;

  println!("📊 Crypto API Mapping Statistics");
  println!("================================");

  if let Some(ref source) = source_filter {
    println!("API Source: {}", source);
  } else {
    println!("All API Sources");
  }

  println!("Total cryptocurrency symbols: {}", total_crypto_symbols);
  println!("Total active mappings: {}", total_mappings);

  if total_crypto_symbols > 0 {
    let coverage = (total_mappings as f64 / total_crypto_symbols as f64) * 100.0;
    println!("Mapping coverage: {:.1}%", coverage);
  }

  // Source breakdown
  if source_filter.is_none() {
    let source_stats: Vec<(String, i64)> = crypto_api_map::table
      .filter(crypto_api_map::is_active.eq(Some(true)))
      .group_by(crypto_api_map::api_source)
      .select((crypto_api_map::api_source, diesel::dsl::count_star()))
      .load(conn)?;

    if !source_stats.is_empty() {
      println!("\n📋 Mappings by Source:");
      for (source, count) in source_stats {
        println!("  {}: {}", source, count);
      }
    }
  }

  // Show unmapped symbols
  if show_unmapped {
    let unmapped_symbols = if let Some(ref source) = source_filter {
      CryptoApiMap::get_symbols_needing_mapping(conn, source)?
    } else {
      // For all sources, show symbols with no mappings at all
      symbols::table
        .left_join(
          crypto_api_map::table
            .on(crypto_api_map::sid.eq(symbols::sid).and(crypto_api_map::is_active.eq(Some(true)))),
        )
        .filter(symbols::sec_type.eq("Cryptocurrency"))
        .filter(crypto_api_map::sid.is_null())
        .select((symbols::sid, symbols::symbol, symbols::name))
        .load::<(i64, String, String)>(conn)?
    };

    if !unmapped_symbols.is_empty() {
      println!("\n🔍 Unmapped Symbols ({}):", unmapped_symbols.len());
      for (sid, symbol, name) in unmapped_symbols.iter().take(10) {
        println!("  {} ({}) - {}", symbol, sid, name);
      }
      if unmapped_symbols.len() > 10 {
        println!("  ... and {} more", unmapped_symbols.len() - 10);
      }
    } else {
      println!("\n✅ All symbols are mapped!");
    }
  }

  // Show stale mappings
  if show_stale {
    if let Some(ref source) = source_filter {
      let stale_mappings = CryptoApiMap::get_stale_mappings(conn, source, stale_days)?;

      if !stale_mappings.is_empty() {
        println!("\n⚠️  Stale Mappings (older than {} days): {}", stale_days, stale_mappings.len());
        for mapping in stale_mappings.iter().take(10) {
          let verified_str = mapping
            .last_verified
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Never".to_string());
          println!("  API ID: {} - Last verified: {}", mapping.api_id, verified_str);
        }
        if stale_mappings.len() > 10 {
          println!("  ... and {} more", stale_mappings.len() - 10);
        }
      } else {
        println!("\n✅ No stale mappings found!");
      }
    }
  }

  // Detailed view
  if detailed {
    let mappings: Vec<(String, String, String, Option<String>, Option<i32>)> =
      if let Some(ref source) = source_filter {
        crypto_api_map::table
          .inner_join(symbols::table)
          .filter(crypto_api_map::api_source.eq(source))
          .filter(crypto_api_map::is_active.eq(Some(true)))
          .select((
            symbols::symbol,
            crypto_api_map::api_source,
            crypto_api_map::api_id,
            crypto_api_map::api_slug,
            crypto_api_map::rank,
          ))
          .order(crypto_api_map::rank.desc().nulls_last())
          .limit(20)
          .load(conn)?
      } else {
        crypto_api_map::table
          .inner_join(symbols::table)
          .filter(crypto_api_map::is_active.eq(Some(true)))
          .select((
            symbols::symbol,
            crypto_api_map::api_source,
            crypto_api_map::api_id,
            crypto_api_map::api_slug,
            crypto_api_map::rank,
          ))
          .order(crypto_api_map::rank.desc().nulls_last())
          .limit(20)
          .load(conn)?
      };

    if !mappings.is_empty() {
      println!("\n🔗 Detailed Mappings (top 20 by rank):");
      println!("Symbol\tSource\t\tAPI ID\t\t\tRank");
      println!("------\t------\t\t------\t\t\t----");
      for (symbol, source, api_id, _slug, rank) in mappings {
        let rank_str = rank.map(|r| r.to_string()).unwrap_or_else(|| "-".to_string());
        println!("{}\t{}\t\t{}\t\t{}", symbol, source, api_id, rank_str);
      }
    }
  }

  Ok(())
}

/// Generates the `crypto-markets` report.
///
/// Produces up to four output sections depending on the flags:
///
/// 1. **Total markets** (always) — Count of matching markets. When both
///    `symbol_filter` and `exchange_filter` are `None`, counts all active markets.
/// 2. **Exchange breakdown** (always) — Top 10 exchanges by market count for the
///    current filter set.
/// 3. **Volume statistics** (`show_volume`) — Aggregate `SUM(volume_24h)` across
///    matching markets. Uses [`bigdecimal::BigDecimal`] for precision and converts
///    to `f64` for display formatting.
/// 4. **Active/inactive breakdown** (`show_inactive`) — When enabled, inactive
///    markets are included in all queries and an active vs. inactive count is
///    printed. When disabled (default), all queries filter to `is_active = true`.
async fn execute_crypto_market_stats(
  conn: &mut PgConnection,
  symbol_filter: Option<String>,
  exchange_filter: Option<String>,
  show_volume: bool,
  show_inactive: bool,
) -> Result<()> {
  info!("Generating crypto market statistics...");

  println!("📈 Crypto Market Statistics");
  println!("===========================");

  // Build base query conditions
  let _active_filter = if show_inactive { None } else { Some(true) };

  let total_markets: i64 = if let Some(ref symbol) = symbol_filter {
    // Query with symbol filter using join
    let mut query = symbols::table
      .inner_join(crypto_markets::table)
      .filter(symbols::symbol.eq(symbol))
      .into_boxed();

    if let Some(ref exchange) = exchange_filter {
      query = query.filter(crypto_markets::exchange.eq(exchange));
    }

    if !show_inactive {
      query = query.filter(crypto_markets::is_active.eq(Some(true)));
    }

    query.count().get_result(conn)?
  } else {
    // Query without symbol filter
    let mut query = crypto_markets::table.into_boxed();

    if let Some(ref exchange) = exchange_filter {
      query = query.filter(crypto_markets::exchange.eq(exchange));
    }

    if !show_inactive {
      query = query.filter(crypto_markets::is_active.eq(Some(true)));
    }

    query.count().get_result(conn)?
  };

  println!("Total markets: {}", total_markets);

  // Exchange breakdown - simplified query
  let exchange_stats: Vec<(String, i64)> = if symbol_filter.is_some() {
    // For specific symbol, get exchange breakdown with join
    let mut query = symbols::table
      .inner_join(crypto_markets::table)
      .filter(symbols::symbol.eq(symbol_filter.as_ref().unwrap()))
      .group_by(crypto_markets::exchange)
      .select((crypto_markets::exchange, diesel::dsl::count_star()))
      .order(diesel::dsl::count_star().desc())
      .limit(10)
      .into_boxed();

    if !show_inactive {
      query = query.filter(crypto_markets::is_active.eq(Some(true)));
    }

    query.load(conn)?
  } else {
    // For all symbols, simpler query
    let mut query = crypto_markets::table
      .group_by(crypto_markets::exchange)
      .select((crypto_markets::exchange, diesel::dsl::count_star()))
      .order(diesel::dsl::count_star().desc())
      .limit(10)
      .into_boxed();

    if !show_inactive {
      query = query.filter(crypto_markets::is_active.eq(Some(true)));
    }

    query.load(conn)?
  };

  if !exchange_stats.is_empty() {
    println!("\n🏪 Top Exchanges by Market Count:");
    for (exchange, count) in exchange_stats {
      println!("  {}: {}", exchange, count);
    }
  }

  // Volume statistics - simplified
  if show_volume {
    use bigdecimal::ToPrimitive;
    use diesel::dsl::sum;

    let total_volume: Option<bigdecimal::BigDecimal> = if let Some(ref symbol) = symbol_filter {
      let mut query = symbols::table
        .inner_join(crypto_markets::table)
        .filter(symbols::symbol.eq(symbol))
        .select(sum(crypto_markets::volume_24h))
        .into_boxed();

      if !show_inactive {
        query = query.filter(crypto_markets::is_active.eq(Some(true)));
      }

      query.first(conn)?
    } else {
      let mut query = crypto_markets::table.select(sum(crypto_markets::volume_24h)).into_boxed();

      if !show_inactive {
        query = query.filter(crypto_markets::is_active.eq(Some(true)));
      }

      query.first(conn)?
    };

    if let Some(vol) = total_volume {
      if let Some(vol_f64) = vol.to_f64() {
        println!("\n💰 24h Volume Statistics:");
        println!("Total 24h volume: ${:.2}", vol_f64);
      }
    }
  }

  // Active vs inactive breakdown
  if show_inactive {
    let active_count: i64 = if let Some(ref symbol) = symbol_filter {
      symbols::table
        .inner_join(crypto_markets::table)
        .filter(symbols::symbol.eq(symbol))
        .filter(crypto_markets::is_active.eq(Some(true)))
        .count()
        .get_result(conn)?
    } else {
      crypto_markets::table
        .filter(crypto_markets::is_active.eq(Some(true)))
        .count()
        .get_result(conn)?
    };

    let inactive_count = total_markets - active_count;

    println!("\n📊 Market Status:");
    println!("Active markets: {}", active_count);
    println!("Inactive markets: {}", inactive_count);
  }

  Ok(())
}

/// Generates the `crypto-overview` report.
///
/// Produces a high-level summary of all cryptocurrency data in the database:
///
/// 1. **Basic counts** (always) — Total crypto symbols (from `symbols` where
///    `sec_type = "Cryptocurrency"`), active API mappings, active markets, and
///    mapping coverage percentage.
/// 2. **API source breakdown** (always) — Active mappings grouped by `api_source`,
///    sorted by count descending.
/// 3. **Top 5 exchanges** (always) — Exchanges with the most active markets.
/// 4. **Extended statistics** (`extended`) — Top 10 symbols by market count and
///    market type distribution (from `crypto_markets.market_type`). Null market
///    types are displayed as `"Unknown"`.
async fn execute_crypto_overview_stats(conn: &mut PgConnection, extended: bool) -> Result<()> {
  info!("Generating crypto overview statistics...");

  println!("🌐 Crypto Database Overview");
  println!("===========================");

  // Basic counts
  let total_crypto_symbols: i64 =
    symbols::table.filter(symbols::sec_type.eq("Cryptocurrency")).count().get_result(conn)?;

  let total_mappings: i64 = crypto_api_map::table
    .filter(crypto_api_map::is_active.eq(Some(true)))
    .count()
    .get_result(conn)?;

  let total_markets: i64 = crypto_markets::table
    .filter(crypto_markets::is_active.eq(Some(true)))
    .count()
    .get_result(conn)?;

  println!("Total cryptocurrency symbols: {}", total_crypto_symbols);
  println!("Total active API mappings: {}", total_mappings);
  println!("Total active markets: {}", total_markets);

  if total_crypto_symbols > 0 {
    let mapping_coverage = (total_mappings as f64 / total_crypto_symbols as f64) * 100.0;
    println!("API mapping coverage: {:.1}%", mapping_coverage);
  }

  // API source breakdown
  let api_sources: Vec<(String, i64)> = crypto_api_map::table
    .filter(crypto_api_map::is_active.eq(Some(true)))
    .group_by(crypto_api_map::api_source)
    .select((crypto_api_map::api_source, diesel::dsl::count_star()))
    .order(diesel::dsl::count_star().desc())
    .load(conn)?;

  if !api_sources.is_empty() {
    println!("\n📡 API Sources:");
    for (source, count) in api_sources {
      println!("  {}: {} mappings", source, count);
    }
  }

  // Top exchanges
  let top_exchanges: Vec<(String, i64)> = crypto_markets::table
    .filter(crypto_markets::is_active.eq(Some(true)))
    .group_by(crypto_markets::exchange)
    .select((crypto_markets::exchange, diesel::dsl::count_star()))
    .order(diesel::dsl::count_star().desc())
    .limit(5)
    .load(conn)?;

  if !top_exchanges.is_empty() {
    println!("\n🏪 Top 5 Exchanges by Market Count:");
    for (exchange, count) in top_exchanges {
      println!("  {}: {} markets", exchange, count);
    }
  }

  if extended {
    // Extended statistics
    println!("\n📈 Extended Statistics:");

    // Symbols with most markets
    let symbols_with_markets: Vec<(String, i64)> = symbols::table
      .inner_join(crypto_markets::table)
      .filter(symbols::sec_type.eq("Cryptocurrency"))
      .filter(crypto_markets::is_active.eq(Some(true)))
      .group_by((symbols::symbol, symbols::name))
      .select((symbols::symbol, diesel::dsl::count_star()))
      .order(diesel::dsl::count_star().desc())
      .limit(10)
      .load(conn)?;

    if !symbols_with_markets.is_empty() {
      println!("\n🔝 Top 10 Symbols by Market Count:");
      for (symbol, count) in symbols_with_markets {
        println!("  {}: {} markets", symbol, count);
      }
    }

    // Market type distribution if available
    let market_types: Vec<(Option<String>, i64)> = crypto_markets::table
      .filter(crypto_markets::is_active.eq(Some(true)))
      .group_by(crypto_markets::market_type)
      .select((crypto_markets::market_type, diesel::dsl::count_star()))
      .order(diesel::dsl::count_star().desc())
      .load(conn)?;

    if !market_types.is_empty() {
      println!("\n📊 Market Types:");
      for (market_type, count) in market_types {
        let type_name = market_type.unwrap_or_else(|| "Unknown".to_string());
        println!("  {}: {} markets", type_name, count);
      }
    }
  }

  Ok(())
}
