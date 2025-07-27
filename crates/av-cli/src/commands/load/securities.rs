use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{NaiveDateTime, Utc};
use diesel::dsl::now;
use tracing::{info, warn, error, debug};

use av_client::AlphaVantageClient;
use av_core::types::market::{SecurityIdentifier, SecurityType};
use av_loaders::{
  SecurityLoader, SecurityLoaderInput, LoaderConfig, LoaderContext,
  process_tracker::ProcessTracker, DataLoader,
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

  // Create security loader
  let loader = SecurityLoader::new(args.concurrent);

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
                    "NASDAQ API calls complete: {} loaded, {} errors",
                    output.loaded_count, output.errors
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
                    "NYSE API calls complete: {} loaded, {} errors",
                    output.loaded_count, output.errors
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

  for security_data in securities {
    let overview = &security_data.overview;

    // Only save if we got valid data from the API
    if overview.symbol.is_empty() || overview.symbol == "None" {
      continue;
    }

    // Use the av-core mapping function
    let security_type = SecurityType::from_alpha_vantage(&overview.asset_type);

    if security_type == SecurityType::Other {
      warn!(
                "Unknown asset type '{}' for symbol {}, mapping to Other",
                overview.asset_type, overview.symbol
            );
    }

    // Parse market hours with defaults
    let market_open = chrono::NaiveTime::parse_from_str("09:30:00", "%H:%M:%S").unwrap();
    let market_close = chrono::NaiveTime::parse_from_str("16:00:00", "%H:%M:%S").unwrap();

    // Determine timezone - handle Option<String> properly
    let timezone = <std::string::String AsRef<T>>::as_ref(&overview.exchange)
        .map(|_ex| "US/Eastern".to_string()) // Use _ex to avoid unused variable warning
        .unwrap_or_else(|| "US/Eastern".to_string());

    // Check if symbol already exists
    let existing: Option<(i64, String)> = symbols::table
        .filter(symbols::symbol.eq(&overview.symbol))
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
                        overview.symbol, existing_sec_type, overview.asset_type
                    );
        }

        // Update the symbol data
        let updated = diesel::update(symbols::table.find(sid_val))
            .set((
              symbols::name.eq(&overview.name),
              symbols::region.eq(&overview.country),
              symbols::currency.eq(&overview.currency),
              symbols::timezone.eq(&timezone),
              symbols::m_time.eq(diesel::dsl::now),
            ))
            .execute(conn)?;

        if updated > 0 {
          updated_count += 1;
          debug!("Updated symbol {} with SID {}", overview.symbol, sid_val);
        }
      }
      None => {
        // New symbol, generate SID using our in-memory generator
        let new_sid = sid_generator.next_sid(security_type);
        let now_t :NaiveDateTime = Utc::now().naive_local();

        let new_symbol = NewSymbol {
          sid: &new_sid,  // Changed to reference
          symbol: &overview.symbol,
          name: &overview.name,
          sec_type: &format!("{:?}", security_type),
          region: &overview.country,
          market_open: &market_open,
          market_close: &market_close,
          timezone: &timezone,
          currency: &overview.currency,
          overview: &false,  // Changed to reference
          intraday: &false,  // Changed to reference
          summary: &false,   // Changed to reference
          c_time: &now_t,
          m_time: &now_t,
        };

        // Insert with explicit SID
        diesel::insert_into(symbols::table)
            .values(&new_symbol)
            .execute(conn)?;

        saved_count += 1;
        info!(
                    "Inserted new symbol {} with SID {} (type: {:?})",
                    overview.symbol, new_sid, security_type
                );
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message(format!(
    "Symbol save complete: {} new, {} updated",
    saved_count, updated_count
  ));

  Ok(saved_count + updated_count)
}
