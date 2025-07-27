use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{NaiveDateTime, Utc};
use tracing::{info, warn, error, debug};

use av_client::AlphaVantageClient;
use av_core::types::market::{SecurityIdentifier, SecurityType, Exchange};
use av_loaders::{
  SecurityLoader, SecurityLoaderInput, LoaderConfig, LoaderContext,
  process_tracker::ProcessTracker, DataLoader, SymbolMatchMode,
};

// Import diesel types
use diesel::prelude::*;
use diesel::PgConnection;

use crate::config::Config;

#[derive(Args)]
pub struct SecuritiesArgs {
  /// Path to NASDAQ CSV file
  #[arg(long, env = "NASDAQ_LISTED")]
  nasdaq_csv: Option<String>,

  /// Path to NYSE CSV file
  #[arg(long, env = "OTHER_LISTED")]
  nyse_csv: Option<String>,

  /// Maximum concurrent API requests
  #[arg(short, long, default_value = "5")]
  concurrent: usize,

  /// Skip database updates (dry run)
  #[arg(short, long)]
  dry_run: bool,

  /// Continue on errors
  #[arg(short = 'k', long)]
  continue_on_error: bool,

  /// Symbol matching mode
  #[arg(long, value_enum, default_value = "exact")]
  match_mode: MatchMode,

  /// Number of top matches to accept (only used with --match-mode=top)
  #[arg(long, default_value = "3")]
  top_matches: usize,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum MatchMode {
  /// Only accept exact symbol matches
  Exact,
  /// Accept all symbols returned from search
  All,
  /// Accept top N matches based on match score
  Top,
}

/// Maintains the next available raw_id for each security type
struct SidGenerator {
  next_raw_ids: HashMap<SecurityType, u32>,
}

impl SidGenerator {
  /// Initialize by reading max SIDs from database
  async fn new(conn: &mut PgConnection) -> Result<Self> {
    use av_database_postgres::schema::symbols::dsl::*;

    info!("Initializing SID generator - reading existing SIDs from database");

    // Get all existing SIDs
    let sids: Vec<i64> = symbols
        .select(sid)
        .load(conn)?;

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

    // Convert to next available IDs - create new HashMap with correct type
    let mut next_ids: HashMap<SecurityType, u32> = HashMap::new();
    for (security_type_val, max_id) in max_raw_ids {
      next_ids.insert(security_type_val, max_id + 1);
      debug!("SecurityType::{:?} next raw_id: {}", security_type_val, max_id + 1);
    }

    info!("SID generator initialized with {} security types", next_ids.len());

    Ok(Self {
      next_raw_ids: next_ids,
    })
  }

  /// Generate the next SID for a given security type
  fn next_sid(&mut self, security_type: SecurityType) -> i64 {
    let raw_id = self.next_raw_ids.entry(security_type).or_insert(1);
    let sid = SecurityType::encode(security_type, *raw_id);
    *raw_id += 1; // Increment for next use
    sid
  }
}

/// Establish database connection
fn establish_connection(database_url: &str) -> Result<PgConnection> {
  use diesel::Connection;

  PgConnection::establish(database_url)
      .map_err(|e| anyhow::anyhow!("Error connecting to database: {}", e))
}

pub async fn execute(args: SecuritiesArgs, config: Config) -> Result<()> {
  info!("Starting security symbol loader");

  // Set up database connection and SID generator
  let mut conn = if !args.dry_run {
    Some(establish_connection(&config.database_url)?)
  } else {
    info!("Dry run mode - no database updates will be performed");
    None
  };

  // Initialize SID generator once at startup
  let mut sid_generator = if let Some(conn) = &mut conn {
    Some(SidGenerator::new(conn).await?)
  } else {
    None
  };

  // Create API client
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: !args.dry_run,
    batch_size: 100,
  };

  // Create loader context
  let mut context = LoaderContext::new(client, loader_config);

  // Set up process tracking if not dry run
  if !args.dry_run {
    let tracker = ProcessTracker::new();
    context = context.with_process_tracker(tracker);
  }

  // Create security loader with match mode
  let match_mode = match args.match_mode {
    MatchMode::Exact => SymbolMatchMode::ExactMatch,
    MatchMode::All => SymbolMatchMode::AllMatches,
    MatchMode::Top => SymbolMatchMode::TopMatches(args.top_matches),
  };

  let loader = SecurityLoader::new(args.concurrent)
      .with_match_mode(match_mode);

  let mut total_loaded = 0;
  let mut total_errors = 0;

  // Process NASDAQ file
  let nasdaq_path = args.nasdaq_csv.unwrap_or(config.nasdaq_csv_path);
  if std::path::Path::new(&nasdaq_path).exists() {
    info!("Loading NASDAQ symbols from: {}", nasdaq_path);

    let input = SecurityLoaderInput {
      file_path: nasdaq_path,
      exchange: "NASDAQ".to_string(),
    };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
                    "NASDAQ API calls complete: {} loaded, {} errors, {} skipped, {} duplicates prevented",
                    output.loaded_count, output.errors, output.skipped_count, output.duplicates_prevented
                );

        if !args.dry_run {
          let saved = save_symbols_to_db(
            &mut conn.as_mut().unwrap(),
            &output.data,
            &mut sid_generator.as_mut().unwrap()
          ).await?;
          info!("Saved {} NASDAQ symbols to database", saved);
          total_loaded += saved;
        }

        total_errors += output.errors;
      }
      Err(e) => {
        error!("Failed to load NASDAQ securities: {}", e);
        if !args.continue_on_error {
          return Err(e.into());
        }
      }
    }
  } else {
    warn!("NASDAQ CSV file not found: {}", nasdaq_path);
  }

  // Process NYSE file
  let nyse_path = args.nyse_csv.unwrap_or(config.nyse_csv_path);
  if std::path::Path::new(&nyse_path).exists() {
    info!("Loading NYSE symbols from: {}", nyse_path);

    let input = SecurityLoaderInput {
      file_path: nyse_path,
      exchange: "NYSE".to_string(),
    };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
                    "NYSE API calls complete: {} loaded, {} errors, {} skipped, {} duplicates prevented",
                    output.loaded_count, output.errors, output.skipped_count, output.duplicates_prevented
                );

        if !args.dry_run {
          let saved = save_symbols_to_db(
            &mut conn.as_mut().unwrap(),
            &output.data,
            &mut sid_generator.as_mut().unwrap()
          ).await?;
          info!("Saved {} NYSE symbols to database", saved);
          total_loaded += saved;
        }

        total_errors += output.errors;
      }
      Err(e) => {
        error!("Failed to load NYSE securities: {}", e);
        if !args.continue_on_error {
          return Err(e.into());
        }
      }
    }
  } else {
    warn!("NYSE CSV file not found: {}", nyse_path);
  }

  // Complete process tracking
  if let Some(tracker) = &context.process_tracker {
    let state = if total_errors > 0 {
      av_loaders::process_tracker::ProcessState::CompletedWithErrors
    } else {
      av_loaders::process_tracker::ProcessState::Success
    };
    tracker.complete(state).await?;
  }

  info!(
        "Symbol loading completed: {} symbols saved, {} errors",
        total_loaded, total_errors
    );
  Ok(())
}


async fn save_symbols_to_db(
  conn: &mut PgConnection,
  securities: &[av_loaders::SecurityData],
  sid_generator: &mut SidGenerator,
) -> Result<usize> {
  use av_database_postgres::schema::symbols;
  use av_database_postgres::models::security::NewSymbol;

  let progress = ProgressBar::new(securities.len() as u64);
  progress.set_style(
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("##-"),
  );
  progress.set_message("Saving symbols to database");

  let mut saved_count = 0;
  let mut updated_count = 0;
  let mut symbol_map = HashMap::new(); // Track symbols we're processing in this batch

  for security_data in securities {
    // Only save if we got valid data from the API
    if security_data.symbol.is_empty() || security_data.symbol == "None" {
      continue;
    }

    // Check for duplicates within this batch
    if symbol_map.contains_key(&security_data.symbol.to_uppercase()) {
      debug!("Duplicate symbol {} found in batch, skipping", security_data.symbol);
      continue;
    }
    symbol_map.insert(security_data.symbol.to_uppercase(), true);

    // Log if the matched symbol differs from original query
    if let Some(original) = &security_data.original_query {
      if !original.eq_ignore_ascii_case(&security_data.symbol) {
        info!("Symbol {} mapped to {} (match score: {:?})",
              original, security_data.symbol, security_data.match_score);
      }
    }

    // Use the av-core mapping function for security type
    let security_type = SecurityType::from_alpha_vantage(&security_data.stock_type);

    if security_type == SecurityType::Other {
      warn!(
        "Unknown asset type '{}' for symbol {}, mapping to Other",
        security_data.stock_type, security_data.symbol
      );
    }

    // Parse market hours from the security data
    let market_open = chrono::NaiveTime::parse_from_str(&security_data.market_open, "%H:%M")
        .unwrap_or_else(|_| chrono::NaiveTime::parse_from_str("09:30:00", "%H:%M:%S").unwrap());
    let market_close = chrono::NaiveTime::parse_from_str(&security_data.market_close, "%H:%M")
        .unwrap_or_else(|_| chrono::NaiveTime::parse_from_str("16:00:00", "%H:%M:%S").unwrap());

    // Use the timezone from the security data or fall back to Exchange lookup
    let timezone = if !security_data.timezone.is_empty() {
      security_data.timezone.clone()
    } else {
      Exchange::from_str(&security_data.exchange)
          .map(|ex| ex.timezone().to_string())
          .unwrap_or_else(|| "US/Eastern".to_string())
    };

    // Check if symbol already exists
    let existing: Option<(i64, String)> = symbols::table
        .filter(symbols::symbol.eq(&security_data.symbol))
        .select((symbols::sid, symbols::sec_type))
        .first(conn)
        .optional()?;

    match existing {
      Some((sid_val, existing_sec_type)) => {
        // Symbol exists, verify security type hasn't changed
        let existing_security_type = SecurityIdentifier::decode(sid_val)
            .map(|si| si.security_type)
            .unwrap_or(SecurityType::Other);

        if format!("{:?}", existing_security_type) != existing_sec_type {
          warn!(
            "Security type mismatch for {}: database has '{}', API returned '{}'",
            security_data.symbol, existing_sec_type, security_data.stock_type
          );
        }

        // Update the symbol data
        let updated = diesel::update(symbols::table.find(sid_val))
            .set((
              symbols::name.eq(&security_data.name),
              symbols::region.eq(&security_data.region),
              symbols::currency.eq(&security_data.currency),
              symbols::timezone.eq(&timezone),
              symbols::m_time.eq(diesel::dsl::now),
            ))
            .execute(conn)?;

        if updated > 0 {
          updated_count += 1;
          debug!("Updated symbol {} with SID {}", security_data.symbol, sid_val);
        }
      }
      None => {
        // New symbol, generate SID using our in-memory generator
        let new_sid = sid_generator.next_sid(security_type);
        let now_t = Utc::now().naive_local();

        let new_symbol = NewSymbol {
          sid: &new_sid,
          symbol: &security_data.symbol,
          name: &security_data.name,
          sec_type: &format!("{:?}", security_type),
          region: &security_data.region,
          market_open: &market_open,
          market_close: &market_close,
          timezone: &timezone,
          currency: &security_data.currency,
          overview: &false,
          intraday: &false,
          summary: &false,
          c_time: &now_t,
          m_time: &now_t,
        };

        diesel::insert_into(symbols::table)
            .values(&new_symbol)
            .execute(conn)?;

        saved_count += 1;
        debug!("Saved new symbol {} with SID {}", security_data.symbol, new_sid);
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message(format!("Saved {} symbols, updated {}", saved_count, updated_count));

  Ok(saved_count)
}
