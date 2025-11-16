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

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::config::Config;
use av_database_postgres::{
  models::crypto::CryptoApiMap,
  schema::{crypto_api_map, crypto_markets, symbols},
};
use diesel::prelude::*;

#[derive(Subcommand, Debug)]
pub enum StatsCommands {
  /// Show crypto API mapping statistics
  CryptoMapping {
    /// API source to analyze (e.g., "CoinGecko", "SosoValue")
    #[arg(short, long)]
    source: Option<String>,

    /// Show detailed mapping information
    #[arg(short, long)]
    detailed: bool,

    /// Show unmapped symbols
    #[arg(short, long)]
    unmapped: bool,

    /// Show symbols needing verification
    #[arg(long)]
    stale: bool,

    /// Days threshold for stale mappings
    #[arg(long, default_value = "30")]
    stale_days: i32,
  },

  /// Show crypto market statistics
  CryptoMarkets {
    /// Symbol to analyze (show all exchanges/markets for this symbol)
    #[arg(short, long)]
    symbol: Option<String>,

    /// Exchange to analyze
    #[arg(short, long)]
    exchange: Option<String>,

    /// Show volume statistics
    #[arg(long)]
    volume: bool,

    /// Show inactive markets
    #[arg(long)]
    inactive: bool,
  },

  /// Show overall crypto database statistics
  CryptoOverview {
    /// Include extended statistics
    #[arg(short, long)]
    extended: bool,
  },
}

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
