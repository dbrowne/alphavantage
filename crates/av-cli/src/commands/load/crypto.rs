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

use anyhow::Result;
use clap::Args;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use av_client::AlphaVantageClient;
use av_core::types::market::{SecurityIdentifier, SecurityType};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, ProcessTracker,
  crypto::{
    CryptoDataSource, CryptoLoaderConfig, CryptoSymbol,
    database::{CryptoDbInput, CryptoDbLoader},
  },
};
use diesel::prelude::*;

use crate::config::Config;

use av_database_postgres::models::crypto::NewCryptoApiMap;
use av_database_postgres::models::security::NewSymbolOwned;
use av_loaders::crypto::database::CryptoSymbolForDb;
const NO_PRIORITY: i32 = 9_999_999;

#[derive(Args, Debug)]
pub struct CryptoArgs {
  /// Data sources to use for crypto symbol loading
  #[arg(
    long,
    value_enum,
    default_values = ["coin-gecko", "coin-market-cap"],
    value_delimiter = ','
    )]
  sources: Vec<CryptoDataSourceArg>,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,

  /// Continue on errors
  #[arg(short = 'k', long)]
  continue_on_error: bool,

  /// Limit number of symbols to load (for debugging)
  #[arg(short, long)]
  limit: Option<usize>,

  /// Update existing symbols in database
  #[arg(long)]
  update_existing: bool,

  /// SosoValue API key (can also be set via SOSOVALUE_API_KEY env var)
  #[arg(long, env = "SOSOVALUE_API_KEY")]
  sosovalue_api_key: Option<String>,

  /// CoinGecko API key (optional, increases rate limits)
  #[arg(long, env = "COINGECKO_API_KEY")]
  coingecko_api_key: Option<String>,

  /// CoinMarketCap API key (optional, for CMC data source)
  #[arg(long, env = "CMC_API_KEY")]
  coinmarketcap_api_key: Option<String>,

  /// Maximum concurrent requests
  #[arg(long, default_value = "5")]
  concurrent: usize,

  /// Batch size for database operations
  #[arg(long, default_value = "100")]
  batch_size: usize,

  /// Show detailed progress information
  #[arg(long)]
  verbose: bool,

  /// Track the loading process in the database
  #[arg(long)]
  track_process: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum CryptoDataSourceArg {
  CoinGecko,
  CoinMarketCap,
  SosoValue,
}

impl From<CryptoDataSourceArg> for CryptoDataSource {
  fn from(arg: CryptoDataSourceArg) -> Self {
    match arg {
      CryptoDataSourceArg::CoinGecko => CryptoDataSource::CoinGecko,
      CryptoDataSourceArg::CoinMarketCap => CryptoDataSource::CoinMarketCap,
      CryptoDataSourceArg::SosoValue => CryptoDataSource::SosoValue,
    }
  }
}

/// SID generator using existing SecurityType system
struct CryptoSidGenerator {
  next_raw_id: u32,
}

impl CryptoSidGenerator {
  /// Initialize by reading max cryptocurrency SIDs from database
  fn new(conn: &mut PgConnection) -> Result<Self> {
    use av_database_postgres::schema::symbols::dsl::*;

    // Get all existing cryptocurrency SIDs
    let crypto_sids: Vec<i64> =
      symbols.filter(sec_type.eq("Cryptocurrency")).select(sid).load(conn)?;

    let mut max_raw_id: u32 = 0;

    for sid_val in crypto_sids {
      if let Some(identifier) = SecurityIdentifier::decode(sid_val) {
        if identifier.security_type == SecurityType::Cryptocurrency
          && identifier.raw_id > max_raw_id
        {
          max_raw_id = identifier.raw_id;
        }
      }
    }

    info!("Crypto next raw_id: {}", max_raw_id + 1);

    Ok(Self { next_raw_id: max_raw_id + 1 })
  }

  /// Generate the next SID using existing SecurityType::encode
  fn next_sid(&mut self) -> i64 {
    let sid = SecurityType::encode(SecurityType::Cryptocurrency, self.next_raw_id);
    self.next_raw_id += 1;
    sid
  }
}

/// Main execute function supporting multiple APIs
pub async fn execute(args: CryptoArgs, config: Config) -> Result<()> {
  info!("Starting crypto symbol loader with sources: {:?}", args.sources);

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    return execute_dry_run(args).await;
  }

  // Convert sources
  let sources: Vec<CryptoDataSource> = args.sources.iter().map(|s| s.clone().into()).collect();

  // Validate API keys for selected sources
  validate_api_keys(&sources, &args)?;

  // Create database context and crypto repository
  let db_context = av_database_postgres::repository::DatabaseContext::new(&config.database_url)
    .map_err(|e| anyhow::anyhow!("Failed to create database context: {}", e))?;
  let _crypto_repo: Arc<dyn av_database_postgres::repository::CryptoRepository> =
    Arc::new(db_context.crypto_repository()); // todo: Fix this!!!

  // Create cache repository for API response caching
  let cache_repo: Arc<dyn av_database_postgres::repository::CacheRepository> =
    Arc::new(db_context.cache_repository());

  // Create API client for HTTP operations
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create crypto loader configuration with multiple sources
  let crypto_config = CryptoLoaderConfig {
    sources: sources.clone(),
    batch_size: args.batch_size,
    max_concurrent_requests: args.concurrent,
    rate_limit_delay_ms: 1000, // Conservative for public APIs
    enable_progress_bar: args.verbose,
    ..Default::default()
  };

  // Create crypto database loader with cache repository
  let crypto_loader = CryptoDbLoader::new(crypto_config).with_cache_repository(cache_repo);

  // Create loader context
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: args.verbose,
    track_process: args.track_process,
    batch_size: args.batch_size,
  };

  let mut context = LoaderContext::new(client, loader_config);

  // Set up process tracking if requested
  if args.track_process {
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Prepare API keys
  let mut api_keys = HashMap::new();
  if let Some(key) = args.sosovalue_api_key {
    api_keys.insert(CryptoDataSource::SosoValue, key);
  }
  if let Some(key) = args.coingecko_api_key {
    api_keys.insert(CryptoDataSource::CoinGecko, key);
  }
  if let Some(key) = args.coinmarketcap_api_key {
    api_keys.insert(CryptoDataSource::CoinMarketCap, key);
  }

  // Create loader input
  let input = CryptoDbInput {
    sources: Some(sources.clone()),
    update_existing: args.update_existing,
    batch_size: Some(args.batch_size),
    api_keys: if api_keys.is_empty() { None } else { Some(api_keys) },
  };

  // Execute the loader
  match crypto_loader.load(&context, input).await {
    Ok(output) => {
      info!(
        "Crypto loading completed: {} fetched, {} processed",
        output.symbols_fetched, output.symbols_processed
      );

      if !output.symbols.is_empty() {
        // Save each symbol to database - this allows multiple tokens per trading symbol
        let mut saved_count = 0;
        let mut error_count = 0;

        for db_symbol in output.symbols {
          match save_crypto_symbol_to_database(
            &config.database_url,
            &db_symbol,
            args.update_existing,
          )
          .await
          {
            Ok(sid) => {
              saved_count += 1;
              debug!("Saved {} '{}' with SID: {}", db_symbol.symbol, db_symbol.name, sid);
            }
            Err(e) => {
              error_count += 1;
              error!("Failed to save {} '{}': {}", db_symbol.symbol, db_symbol.name, e);

              if !args.continue_on_error {
                return Err(e);
              }
            }
          }
        }

        info!("Database operations: {} saved, {} errors", saved_count, error_count);
      }
    }
    Err(e) => {
      error!("Crypto loading failed: {}", e);
      return Err(e.into());
    }
  }

  Ok(())
}
fn update_existing_token_in_db(
  conn: &mut PgConnection,
  sid: i64,
  db_symbol: &CryptoSymbolForDb,
) -> Result<()> {
  use av_database_postgres::schema::{crypto_api_map, symbol_mappings, symbols};
  use diesel::prelude::*;

  // Update symbols table
  diesel::update(symbols::table.find(sid))
    .set((symbols::name.eq(&db_symbol.name), symbols::m_time.eq(chrono::Utc::now())))
    .execute(conn)?;

  // Update crypto_api_map table
  diesel::update(
    crypto_api_map::table
      .filter(crypto_api_map::sid.eq(sid))
      .filter(crypto_api_map::api_source.eq(db_symbol.source.to_string())),
  )
  .set((
    crypto_api_map::api_id.eq(&db_symbol.source_id),
    crypto_api_map::api_symbol.eq(Some(&db_symbol.symbol)),
    crypto_api_map::rank.eq(db_symbol.market_cap_rank.map(|r| r as i32)),
    crypto_api_map::last_verified.eq(Some(chrono::Utc::now())),
    crypto_api_map::m_time.eq(chrono::Utc::now()),
  ))
  .execute(conn)?;

  // Only update symbol_mappings for PRIMARY tokens (priority != 9999999)
  if db_symbol.priority != NO_PRIORITY {
    let source_name = db_symbol.source.to_string();

    diesel::insert_into(symbol_mappings::table)
      .values((
        symbol_mappings::sid.eq(sid),
        symbol_mappings::source_name.eq(&source_name),
        symbol_mappings::source_identifier.eq(&db_symbol.source_id),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .on_conflict((symbol_mappings::sid, symbol_mappings::source_name))
      .do_update()
      .set((
        symbol_mappings::source_identifier.eq(&db_symbol.source_id),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .execute(conn)?;

    debug!(
      "Updated symbol_mappings entry for PRIMARY token: sid={} → {}:{}",
      sid, source_name, db_symbol.source_id
    );
  }

  Ok(())
}

/// Save individual crypto symbol to database, allowing multiple tokens per trading symbol
async fn save_crypto_symbol_to_database(
  database_url: &str,
  db_symbol: &CryptoSymbolForDb,
  update_existing: bool,
) -> Result<i64> {
  use diesel::prelude::*;

  let mut conn = PgConnection::establish(database_url)
    .map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;

  // Check if this exact token (same symbol + source + source_id) already exists
  let existing_sid = find_existing_token_in_db(
    &mut conn,
    &db_symbol.symbol,
    &db_symbol.source,
    &db_symbol.source_id,
  )?;

  if let Some(sid) = existing_sid {
    if update_existing {
      // Update existing token
      update_existing_token_in_db(&mut conn, sid, db_symbol)?;
      info!("Updated existing token: {} '{}' (SID: {})", db_symbol.symbol, db_symbol.name, sid);
      Ok(sid)
    } else {
      // Skip updating the symbol itself, but still ensure symbol_mappings is populated for PRIMARY tokens
      if db_symbol.priority != NO_PRIORITY {
        use av_database_postgres::schema::symbol_mappings;
        use diesel::prelude::*;

        let source_name = db_symbol.source.to_string();

        diesel::insert_into(symbol_mappings::table)
          .values((
            symbol_mappings::sid.eq(sid),
            symbol_mappings::source_name.eq(&source_name),
            symbol_mappings::source_identifier.eq(&db_symbol.source_id),
            symbol_mappings::verified.eq(true),
            symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
          ))
          .on_conflict((symbol_mappings::sid, symbol_mappings::source_name))
          .do_nothing() // Don't overwrite existing mapping
          .execute(&mut conn)?;

        debug!(
          "Ensured symbol_mappings entry exists for PRIMARY token: sid={} → {}:{}",
          sid, source_name, db_symbol.source_id
        );
      }
      info!("Skipped existing token: {} '{}' (SID: {})", db_symbol.symbol, db_symbol.name, sid);
      Ok(sid)
    }
  } else {
    // Insert new token - this allows multiple SOL variants
    let sid = insert_new_token_in_db(&mut conn, db_symbol)?;
    info!("Inserted new token: {} '{}' (SID: {})", db_symbol.symbol, db_symbol.name, sid);
    Ok(sid)
  }
}

/// Find existing token by checking symbols + crypto_api_map tables
fn find_existing_token_in_db(
  conn: &mut PgConnection,
  symbol: &str,
  source: &CryptoDataSource,
  source_id: &str,
) -> Result<Option<i64>> {
  use av_database_postgres::schema::{crypto_api_map, symbols};
  use diesel::prelude::*;

  let existing_sid: Option<i64> = symbols::table
    .inner_join(crypto_api_map::table.on(crypto_api_map::sid.eq(symbols::sid)))
    .filter(symbols::symbol.eq(symbol))
    .filter(crypto_api_map::api_source.eq(source.to_string()))
    .filter(crypto_api_map::api_id.eq(source_id))
    .select(symbols::sid)
    .first::<i64>(conn)
    .optional()?;

  Ok(existing_sid)
}

fn insert_new_token_in_db(conn: &mut PgConnection, db_symbol: &CryptoSymbolForDb) -> Result<i64> {
  use av_database_postgres::schema::{crypto_api_map, symbol_mappings, symbols};
  use diesel::prelude::*;

  // Use CryptoSidGenerator directly - no need for wrapper function
  let mut sid_generator = CryptoSidGenerator::new(conn)?;
  let new_sid = sid_generator.next_sid();

  // Create NewSymbolOwned with priority field
  let mut new_symbol = NewSymbolOwned::from_symbol_data(
    &db_symbol.symbol,
    db_symbol.priority, // Trading symbol: "SOL"
    &db_symbol.name,    // Full name: "Solana" or "Allbridge Bridged SOL"
    "Cryptocurrency",
    "Global",
    "USD",
    new_sid,
  );

  // Set the priority from the processed db_symbol
  new_symbol.priority = db_symbol.priority;

  diesel::insert_into(symbols::table).values(&new_symbol).execute(conn)?;

  // Insert API mapping to link symbol to source (legacy crypto_api_map table)
  let api_mapping = NewCryptoApiMap {
    sid: new_sid,
    api_source: db_symbol.source.to_string(),
    api_id: db_symbol.source_id.clone(),
    api_slug: None,
    api_symbol: Some(db_symbol.symbol.clone()),
    rank: db_symbol.market_cap_rank.map(|r| r as i32),
    is_active: Some(true),
    last_verified: Some(chrono::Utc::now()),
    c_time: chrono::Utc::now(),
    m_time: chrono::Utc::now(),
  };

  diesel::insert_into(crypto_api_map::table).values(&api_mapping).execute(conn)?;

  // Only insert into symbol_mappings for PRIMARY tokens (priority != NO_PRIORITY)
  // This ensures we only map the canonical version of each symbol, not wrapped/bridged variants
  if db_symbol.priority != NO_PRIORITY {
    let source_name = db_symbol.source.to_string();

    diesel::insert_into(symbol_mappings::table)
      .values((
        symbol_mappings::sid.eq(new_sid),
        symbol_mappings::source_name.eq(&source_name),
        symbol_mappings::source_identifier.eq(&db_symbol.source_id),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .on_conflict((symbol_mappings::sid, symbol_mappings::source_name))
      .do_update()
      .set((
        symbol_mappings::source_identifier.eq(&db_symbol.source_id),
        symbol_mappings::verified.eq(true),
        symbol_mappings::last_verified_at.eq(chrono::Utc::now().naive_utc()),
      ))
      .execute(conn)?;

    info!(
      "Created symbol_mappings entry for PRIMARY token: sid={} → {}:{} (priority={})",
      new_sid, source_name, db_symbol.source_id, db_symbol.priority
    );
  } else {
    debug!(
      "Skipped symbol_mappings for non-primary token: sid={} {} '{}' (priority=9999999)",
      new_sid, db_symbol.symbol, db_symbol.name
    );
  }

  Ok(new_sid)
}
/// Execute in dry run mode - test API connections
async fn execute_dry_run(args: CryptoArgs) -> Result<()> {
  info!("Executing crypto loader in dry run mode");

  info!("Configuration:");
  info!("  - Data sources: {:?}", args.sources);
  info!("  - Concurrent requests: {}", args.concurrent);
  info!("  - Batch size: {}", args.batch_size);
  info!("  - Update existing: {}", args.update_existing);
  info!("  - Continue on error: {}", args.continue_on_error);

  let sources: Vec<CryptoDataSource> = args.sources.iter().map(|s| s.clone().into()).collect();

  for source in &sources {
    match source {
      CryptoDataSource::CoinGecko => {
        info!("Testing CoinGecko API connection...");
        if args.coingecko_api_key.is_some() {
          info!("  ✓ CoinGecko API key: configured");
        } else {
          info!("  - CoinGecko API key: not configured (will use free tier)");
        }
        info!("  ✓ CoinGecko connection test would run here");
      }
      CryptoDataSource::SosoValue => {
        info!("Testing SosoValue API connection...");
        if args.sosovalue_api_key.is_some() {
          info!("  ✓ SosoValue API key: configured");
        } else {
          warn!("  ✗ SosoValue API key: not configured");
          warn!("    Set SOSOVALUE_API_KEY environment variable");
        }
        info!("  ✓ SosoValue connection test would run here");
      }
      _ => {
        warn!("  - Source {:?}: not implemented in dry run", source);
      }
    }
  }

  info!("Dry run completed - no actual symbol loading or database updates performed");
  Ok(())
}

fn validate_api_keys(sources: &[CryptoDataSource], args: &CryptoArgs) -> Result<()> {
  for source in sources {
    match source {
      CryptoDataSource::SosoValue => {
        if args.sosovalue_api_key.is_none() {
          return Err(anyhow::anyhow!(
            "SosoValue API key is required for SosoValue source. Set SOSOVALUE_API_KEY environment variable or use --sosovalue-api-key"
          ));
        }
      }
      CryptoDataSource::CoinGecko => {
        // CoinGecko API key is optional
        if args.coingecko_api_key.is_none() {
          warn!("CoinGecko API key not provided - using free tier with rate limits");
        }
      }
      _ => {
        warn!("Source {:?} validation not implemented", source);
      }
    }
  }
  Ok(())
}

/// Save crypto symbols to database using existing patterns
#[allow(dead_code)] // Retained for potential direct database operations bypassing repository
async fn save_crypto_symbols_to_db(
  database_url: &str,
  symbols: &[CryptoSymbol],
  update_existing: bool,
  continue_on_error: bool,
) -> Result<(usize, usize, usize)> {
  use av_database_postgres::{models::security::NewSymbolOwned, schema::symbols};
  use diesel::PgConnection;

  let database_url = database_url.to_string();
  let symbols = symbols.to_vec();

  // Execute in blocking context since diesel is synchronous
  tokio::task::spawn_blocking(move || -> Result<(usize, usize, usize)> {
    let mut conn = PgConnection::establish(&database_url)
      .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    let mut saved_count = 0;
    let mut updated_count = 0;
    let mut failed_count = 0;

    // Start transaction
    conn.transaction(|conn| -> Result<(), anyhow::Error> {
      // Initialize SID generator using existing system
      let mut sid_generator = CryptoSidGenerator::new(conn)?;

      for crypto_symbol in &symbols {
        // Validate symbol data
        if crypto_symbol.symbol.is_empty() || crypto_symbol.name.is_empty() {
          if continue_on_error {
            failed_count += 1;
            continue;
          } else {
            return Err(anyhow::anyhow!("Invalid symbol data"));
          }
        }

        // Check length constraints
        if crypto_symbol.symbol.len() > 20 {
          if continue_on_error {
            warn!("Symbol too long: {}, skipping", crypto_symbol.symbol);
            failed_count += 1;
            continue;
          } else {
            return Err(anyhow::anyhow!("Symbol too long: {}", crypto_symbol.symbol));
          }
        }

        // Check if symbol already exists
        let existing_result = symbols::table
          .filter(symbols::symbol.eq(&crypto_symbol.symbol))
          .filter(symbols::sec_type.eq("Cryptocurrency"))
          .select((symbols::sid, symbols::sec_type))
          .first::<(i64, String)>(conn)
          .optional();

        match existing_result {
          Ok(Some((sid_val, _))) => {
            if update_existing {
              // Update existing symbol
              match diesel::update(symbols::table.find(sid_val))
                .set((
                  symbols::name.eq(&crypto_symbol.name),
                  symbols::m_time.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn)
              {
                Ok(_) => {
                  updated_count += 1;
                  debug!("Updated cryptocurrency {} (SID {})", crypto_symbol.symbol, sid_val);
                }
                Err(e) => {
                  error!("Failed to update cryptocurrency {}: {}", crypto_symbol.symbol, e);
                  failed_count += 1;
                  if !continue_on_error {
                    return Err(e.into());
                  }
                }
              }
            } else {
              debug!("Cryptocurrency {} already exists, skipping", crypto_symbol.symbol);
            }
          }
          Ok(None) => {
            // Insert new symbol
            let new_sid = sid_generator.next_sid();

            let new_symbol = NewSymbolOwned {
              sid: new_sid,
              symbol: crypto_symbol.symbol.clone(),
              priority: crypto_symbol.priority,
              name: crypto_symbol.name.clone(),
              sec_type: "Cryptocurrency".to_string(),
              region: "Global".to_string(), // ADD this line
              currency: "USD".to_string(),  // ADD this line
              overview: false,
              intraday: false,
              summary: false,
              c_time: chrono::Utc::now().naive_utc(), // FIX: add .naive_utc()
              m_time: chrono::Utc::now().naive_utc(), // FIX: add .naive_utc()
            };

            match diesel::insert_into(symbols::table).values(&new_symbol).execute(conn) {
              Ok(_) => {
                saved_count += 1;
                debug!("Inserted cryptocurrency {} (SID {})", crypto_symbol.symbol, new_sid);

                // Insert API mapping if available
                if let Err(e) = insert_api_mapping(conn, new_sid, crypto_symbol) {
                  warn!("Failed to insert API mapping for {}: {}", crypto_symbol.symbol, e);
                  // Don't fail the whole operation for mapping errors
                }
              }
              Err(e) => {
                error!("Failed to insert cryptocurrency {}: {}", crypto_symbol.symbol, e);
                failed_count += 1;
                if !continue_on_error {
                  return Err(e.into());
                }
              }
            }
          }
          Err(e) => {
            error!("Database error checking symbol {}: {}", crypto_symbol.symbol, e);
            failed_count += 1;
            if !continue_on_error {
              return Err(e.into());
            }
          }
        }
      }
      Ok(())
    })?;

    info!(
      "Symbol insertion completed: {} new, {} updated, {} failed",
      saved_count, updated_count, failed_count
    );

    Ok((saved_count, updated_count, failed_count))
  })
  .await?
}

/// Insert API mapping for a cryptocurrency
fn insert_api_mapping(
  conn: &mut PgConnection,
  sid: i64,
  crypto_symbol: &CryptoSymbol,
) -> Result<()> {
  use av_database_postgres::{models::crypto::NewCryptoApiMap, schema::crypto_api_map};

  let api_source = match crypto_symbol.source {
    CryptoDataSource::CoinGecko => "CoinGecko",
    CryptoDataSource::SosoValue => "SosoValue",
    _ => return Ok(()), // Skip unknown sources
  };

  let new_mapping = NewCryptoApiMap {
    sid,
    api_source: api_source.to_string(),
    api_id: crypto_symbol.source_id.clone(),
    api_slug: None,
    api_symbol: Some(crypto_symbol.symbol.clone()),
    rank: crypto_symbol.market_cap_rank.map(|r| r as i32),
    is_active: Some(crypto_symbol.is_active),
    last_verified: Some(crypto_symbol.created_at),
    c_time: chrono::Utc::now(),
    m_time: chrono::Utc::now(),
  };

  diesel::insert_into(crypto_api_map::table)
    .values(&new_mapping)
    .execute(conn)
    .map_err(|e| anyhow::anyhow!("Failed to insert API mapping: {}", e))?;

  debug!(
    "Inserted {} API mapping for {} ({})",
    api_source, crypto_symbol.symbol, crypto_symbol.source_id
  );
  Ok(())
}
