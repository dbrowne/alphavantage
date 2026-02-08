/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

use anyhow::Result;
use av_database_postgres::repository::DatabaseContext;
use av_loaders::crypto::sources::CacheRepositoryAdapter;
use bigdecimal::BigDecimal;
use chrono::{Timelike, Utc};
use clap::Args;
use diesel::prelude::*;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::config::Config;
use crypto_loaders::{TopMoverCoin, TopMoversConfig, TopMoversLoader};

const PROVIDER: &str = "CoinGecko";

#[derive(Args, Debug)]
pub struct CryptoTopMoversArgs {
  /// CoinGecko API key
  #[arg(long, env = "COINGECKO_API_KEY")]
  api_key: Option<String>,

  /// Duration for top movers: 1h, 24h, 7d, 14d, 30d, 200d, 1y
  #[arg(long, default_value = "24h")]
  duration: String,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,
}

#[instrument(name = "CryptoTopMovers", skip(args, config), fields(loader = "CryptoTopMovers"))]
pub async fn execute(args: CryptoTopMoversArgs, config: Config) -> Result<()> {
  let api_key = args.api_key.ok_or_else(|| {
    anyhow::anyhow!("CoinGecko API key required (--api-key or COINGECKO_API_KEY)")
  })?;

  let loader_config = TopMoversConfig { duration: args.duration.clone(), cache_ttl_hours: 1 };

  // Set up cache via DatabaseContext → CacheRepository → CacheRepositoryAdapter
  let db_context = DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;
  let cache_repo = Arc::new(db_context.cache_repository());
  let cache_adapter = CacheRepositoryAdapter::as_arc(cache_repo);

  let loader = TopMoversLoader::new(api_key, loader_config).with_cache(cache_adapter);

  // Fetch top movers from CoinGecko
  let output = loader.load().await.map_err(|e| anyhow::anyhow!("{}", e))?;

  info!(
    gainers = output.gainers.len(),
    losers = output.losers.len(),
    from_cache = output.from_cache,
    "Top movers loaded"
  );

  if args.dry_run {
    info!("Dry run mode — skipping database writes");
    for coin in &output.gainers {
      info!(
        symbol = %coin.symbol,
        rank = ?coin.market_cap_rank,
        price = ?coin.usd,
        change_24h = ?coin.usd_24h_change,
        "Top gainer"
      );
    }
    for coin in &output.losers {
      info!(
        symbol = %coin.symbol,
        rank = ?coin.market_cap_rank,
        price = ?coin.usd,
        change_24h = ?coin.usd_24h_change,
        "Top loser"
      );
    }
    return Ok(());
  }

  // Build owned list of (coin, event_type) for the blocking closure
  let num_gainers = output.gainers.len();
  let num_losers = output.losers.len();
  let from_cache = output.from_cache;

  let all_coins: Vec<(TopMoverCoin, String)> = output
    .gainers
    .into_iter()
    .map(|c| (c, "top_gainer".to_string()))
    .chain(output.losers.into_iter().map(|c| (c, "top_loser".to_string())))
    .collect();

  let coingecko_ids: Vec<String> = all_coins.iter().map(|(c, _)| c.id.clone()).collect();

  let id_to_sid = resolve_ids_to_sids(&config.database_url, &coingecko_ids)?;

  info!(resolved = id_to_sid.len(), total = coingecko_ids.len(), "Resolved CoinGecko IDs to SIDs");

  // Truncate timestamp to hour boundary so re-runs within the same hour
  // are deduplicated by the composite PK (tstamp, sid, api_source, event_type)
  let now = Utc::now().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
  let database_url = config.database_url.clone();

  let (inserted, skipped, rank_updates, unknown) =
    tokio::task::spawn_blocking(move || -> Result<(usize, usize, usize, usize)> {
      let mut conn = diesel::PgConnection::establish(&database_url)?;
      let mut inserted = 0usize;
      let mut skipped = 0usize;
      let mut rank_updates = 0usize;
      let mut unknown = 0usize;

      for (coin, event_type) in &all_coins {
        let Some(&sid) = id_to_sid.get(&coin.id) else {
          debug!(id = %coin.id, symbol = %coin.symbol, "No SID mapping found — skipping");
          unknown += 1;
          continue;
        };

        // Insert into crypto_top_movers (returns false if duplicate)
        if insert_top_mover(&mut conn, &now, sid, coin, event_type)? {
          inserted += 1;

          // Only update ranks when a new row was actually inserted
          if let Some(rank) = coin.market_cap_rank {
            if update_rank(&mut conn, sid, &coin.id, rank)? {
              rank_updates += 1;
            }
          }
        } else {
          skipped += 1;
        }
      }

      Ok((inserted, skipped, rank_updates, unknown))
    })
    .await??;

  // Print summary
  println!("\n╔════════════════════════════════════════════╗");
  println!("║     CRYPTO TOP MOVERS LOADING SUMMARY       ║");
  println!("╠════════════════════════════════════════════╣");
  println!("║ Top Gainers:        {:<24} ║", num_gainers);
  println!("║ Top Losers:         {:<24} ║", num_losers);
  println!("║ Inserted:           {:<24} ║", inserted);
  println!("║ Skipped (dup):      {:<24} ║", skipped);
  println!("║ Rank Updates:       {:<24} ║", rank_updates);
  println!("║ Unknown (no SID):   {:<24} ║", unknown);
  println!("║ From Cache:         {:<24} ║", from_cache);
  println!("╚════════════════════════════════════════════╝");

  info!(
    inserted = inserted,
    skipped = skipped,
    rank_updates = rank_updates,
    unknown = unknown,
    "Loading complete"
  );

  Ok(())
}

/// Batch-resolve CoinGecko IDs to SIDs via crypto_api_map
fn resolve_ids_to_sids(
  database_url: &str,
  coingecko_ids: &[String],
) -> Result<HashMap<String, i64>> {
  use av_database_postgres::schema::crypto_api_map;

  let mut conn = diesel::PgConnection::establish(database_url)?;

  let mappings: Vec<(String, i64)> = crypto_api_map::table
    .filter(crypto_api_map::api_source.eq(PROVIDER))
    .filter(crypto_api_map::api_id.eq_any(coingecko_ids))
    .filter(crypto_api_map::is_active.eq(Some(true)))
    .select((crypto_api_map::api_id, crypto_api_map::sid))
    .load(&mut conn)?;

  Ok(mappings.into_iter().collect())
}

/// Insert a single top mover record. Returns true if a row was inserted, false if skipped (duplicate).
fn insert_top_mover(
  conn: &mut PgConnection,
  tstamp: &chrono::DateTime<Utc>,
  sid: i64,
  coin: &TopMoverCoin,
  event_type: &str,
) -> Result<bool> {
  use av_database_postgres::schema::crypto_top_movers;

  let price_usd = coin.usd.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());
  let volume_24h =
    coin.usd_24h_vol.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default());

  let rows_affected = diesel::insert_into(crypto_top_movers::table)
    .values((
      crypto_top_movers::tstamp.eq(tstamp),
      crypto_top_movers::sid.eq(sid),
      crypto_top_movers::api_source.eq(PROVIDER),
      crypto_top_movers::event_type.eq(event_type),
      crypto_top_movers::price_usd.eq(&price_usd),
      crypto_top_movers::volume_24h.eq(&volume_24h),
      crypto_top_movers::change_pct_1h.eq(coin.usd_1h_change),
      crypto_top_movers::change_pct_24h.eq(coin.usd_24h_change),
      crypto_top_movers::change_pct_7d.eq(coin.usd_7d_change),
      crypto_top_movers::change_pct_14d.eq(coin.usd_14d_change),
      crypto_top_movers::change_pct_30d.eq(coin.usd_30d_change),
      crypto_top_movers::change_pct_200d.eq(coin.usd_200d_change),
      crypto_top_movers::change_pct_1y.eq(coin.usd_1y_change),
    ))
    .on_conflict_do_nothing()
    .execute(conn)?;

  if rows_affected > 0 {
    debug!(
      sid = sid,
      symbol = %coin.symbol,
      event_type = event_type,
      price = ?coin.usd,
      "Inserted top mover"
    );
  }

  Ok(rows_affected > 0)
}

/// Update symbols.priority and crypto_api_map.rank only if the value changed.
/// Returns true if any row was actually updated.
fn update_rank(conn: &mut PgConnection, sid: i64, api_id: &str, rank: i32) -> Result<bool> {
  use av_database_postgres::schema::{crypto_api_map, symbols};

  // Only update symbols.priority if it differs
  let priority_updated =
    diesel::update(symbols::table.filter(symbols::sid.eq(sid)).filter(symbols::priority.ne(rank)))
      .set(symbols::priority.eq(rank))
      .execute(conn)?;

  // Only update crypto_api_map.rank if it differs
  let rank_updated = diesel::update(
    crypto_api_map::table
      .filter(crypto_api_map::sid.eq(sid))
      .filter(crypto_api_map::api_id.eq(api_id))
      .filter(crypto_api_map::rank.is_null().or(crypto_api_map::rank.ne(Some(rank)))),
  )
  .set((crypto_api_map::rank.eq(Some(rank)), crypto_api_map::m_time.eq(Utc::now())))
  .execute(conn)?;

  let changed = priority_updated > 0 || rank_updated > 0;
  if changed {
    debug!(sid = sid, rank = rank, "Updated rank");
  }

  Ok(changed)
}
