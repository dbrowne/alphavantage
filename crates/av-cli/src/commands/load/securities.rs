use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use av_client::AlphaVantageClient;
use av_core::types::market::{Exchange, SecurityIdentifier, SecurityType};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, SecurityLoader, SecurityLoaderInput, SymbolMatchMode,
  process_tracker::ProcessTracker,
};

// Import diesel types
use diesel::PgConnection;
use diesel::prelude::*;

use crate::config::Config;

#[derive(Args, Debug)]
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
  #[arg(long, value_enum, default_value = "all")]
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
#[derive(Clone)]
struct SidGenerator {
  next_raw_ids: HashMap<SecurityType, u32>,
}

impl SidGenerator {
  /// Initialize by reading max SIDs from database (synchronous version)
  fn new(conn: &mut PgConnection) -> Result<Self> {
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

  /// Generate the next SID for a given security type
  fn next_sid(&mut self, security_type: SecurityType) -> i64 {
    let raw_id = self.next_raw_ids.entry(security_type).or_insert(1);
    let sid = SecurityType::encode(security_type, *raw_id);
    *raw_id += 1; // Increment for next use
    sid
  }
}
/// Normalize region names to abbreviated forms.
///
/// Converts common region names to their standard abbreviated forms used
/// throughout the application. Unknown regions are returned unchanged.

pub fn normalize_alpha_region(region: &str) -> String {
  let normalized = match region {
    "United States" => "USA",
    "United Kingdom" => "UK",
    "Frankfurt" => "Frank",
    "Toronto" | "Toronto Venture" => "TOR",
    "India/Bombay" | "India" | "Bombay" => "Bomb",
    "Brazil/Sao Paolo" | "Brazil" | "Sao Paolo" => "SaoP",
    "Amsterdam" => "AMS",
    "XETRA" => "XETRA",
    "Shanghai" => "SH",
    "Hong Kong" => "HK",
    "Tokyo" => "TYO",
    "London" => "LON",
    "Paris" => "PAR",
    "Singapore" => "SG",
    "Sydney" => "SYD",
    "Mexico" => "MEX",
    "Canada" => "CAN",
    "Germany" => "DE",
    "Switzerland" => "CH",
    "Japan" => "JP",
    "Australia" => "AU",
    "Netherlands" => "NL",
    _ => region,
  };

  // Ensure the result fits in VARCHAR(10)
  if normalized.len() > 10 {
    warn!("Region '{}' exceeds 10 characters, truncating to '{}'", normalized, &normalized[..10]);
    normalized[..10].to_string()
  } else {
    normalized.to_string()
  }
}

/// Main execute function
pub async fn execute(args: SecuritiesArgs, config: Config) -> Result<()> {
  info!("Starting security symbol loader");

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    return execute_dry_run(args, config).await;
  }

  // Create API client
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create loader configuration
  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: true,
    batch_size: 100,
  };

  // Create loader context
  let mut context = LoaderContext::new(client, loader_config);

  // Set up process tracking
  let tracker = ProcessTracker::new();
  context = context.with_process_tracker(tracker);

  // Create security loader with match mode
  let match_mode = match args.match_mode {
    MatchMode::Exact => SymbolMatchMode::ExactMatch,
    MatchMode::All => SymbolMatchMode::AllMatches,
    MatchMode::Top => SymbolMatchMode::TopMatches(args.top_matches),
  };

  let loader = SecurityLoader::new(args.concurrent).with_match_mode(match_mode);

  // Collect all securities first, then save in one transaction
  let mut all_securities = Vec::new();

  // Process NASDAQ file
  let nasdaq_path = args.nasdaq_csv.unwrap_or(config.nasdaq_csv_path);
  if std::path::Path::new(&nasdaq_path).exists() {
    info!("Loading NASDAQ symbols from: {}", nasdaq_path);

    let input = SecurityLoaderInput { file_path: nasdaq_path, exchange: "NASDAQ".to_string() };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
          "NASDAQ API calls complete: {} loaded, {} errors, {} skipped",
          output.loaded_count, output.errors, output.skipped_count
        );

        // Collect securities for later saving
        all_securities.extend(output.data);
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

    let input = SecurityLoaderInput { file_path: nyse_path, exchange: "NYSE".to_string() };

    match loader.load(&context, input).await {
      Ok(output) => {
        info!(
          "NYSE API calls complete: {} loaded, {} errors, {} skipped",
          output.loaded_count, output.errors, output.skipped_count
        );

        // Collect securities for later saving
        all_securities.extend(output.data);
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

  // Save all securities in one blocking operation
  let total_loaded = if !all_securities.is_empty() {
    let db_url = config.database_url.clone();

    tokio::task::spawn_blocking(move || -> Result<usize> {
      // Establish connection in the blocking context
      let mut conn = PgConnection::establish(&db_url)
        .map_err(|e| anyhow::anyhow!("Error connecting to database: {}", e))?;

      // Initialize SID generator
      let mut sid_generator = SidGenerator::new(&mut conn)?;

      // Save all symbols
      save_symbols_to_db(&mut conn, &all_securities, &mut sid_generator)
    })
    .await??
  } else {
    0
  };

  // Complete process tracking
  if let Some(tracker) = &context.process_tracker {
    let state = if total_loaded == 0 {
      av_loaders::process_tracker::ProcessState::CompletedWithErrors
    } else {
      av_loaders::process_tracker::ProcessState::Success
    };
    tracker.complete(state).await?;
  }

  info!("Symbol loading completed: {} symbols saved", total_loaded);
  Ok(())
}

/// Dry run version that doesn't need database
async fn execute_dry_run(args: SecuritiesArgs, config: Config) -> Result<()> {
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  let loader_config = LoaderConfig {
    max_concurrent_requests: args.concurrent,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    show_progress: true,
    track_process: false,
    batch_size: 100,
  };

  let context = LoaderContext::new(client, loader_config);

  let match_mode = match args.match_mode {
    MatchMode::Exact => SymbolMatchMode::ExactMatch,
    MatchMode::All => SymbolMatchMode::AllMatches,
    MatchMode::Top => SymbolMatchMode::TopMatches(args.top_matches),
  };

  let loader = SecurityLoader::new(args.concurrent).with_match_mode(match_mode);

  let mut total_loaded = 0;
  let mut total_errors = 0;
  let mut total_skipped = 0;

  // Process files
  for (path, exchange) in [
    (args.nasdaq_csv.unwrap_or(config.nasdaq_csv_path), "NASDAQ"),
    (args.nyse_csv.unwrap_or(config.nyse_csv_path), "NYSE"),
  ] {
    if std::path::Path::new(&path).exists() {
      info!("Loading {} symbols from: {}", exchange, path);

      let input = SecurityLoaderInput { file_path: path, exchange: exchange.to_string() };

      match loader.load(&context, input).await {
        Ok(output) => {
          info!(
            "{} API calls complete (DRY RUN): {} loaded, {} errors, {} skipped",
            exchange, output.loaded_count, output.errors, output.skipped_count
          );
          total_loaded += output.loaded_count;
          total_errors += output.errors;
          total_skipped += output.skipped_count;
        }
        Err(e) => {
          error!("Failed to load {} securities: {}", exchange, e);
          if !args.continue_on_error {
            return Err(e.into());
          }
        }
      }
    }
  }

  info!(
    "Dry run completed: {} symbols found, {} errors, {} skipped",
    total_loaded, total_errors, total_skipped
  );
  Ok(())
}

// Complete fix for save_symbols_to_db function in crates/av-cli/src/commands/load/securities.rs

fn save_symbols_to_db(
  conn: &mut PgConnection,
  securities: &[av_loaders::SecurityData],
  sid_generator: &mut SidGenerator,
) -> Result<usize> {
  use av_database_postgres::models::security::{NewEquityDetailOwned, NewSymbol};
  use av_database_postgres::schema::{equity_details, symbols};
  use diesel::result::DatabaseErrorKind;

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
  let mut failed_count = 0;
  let mut skipped_count = 0;
  let mut symbol_map = HashMap::new();

  // Process each symbol individually
  for security_data in securities {
    // Only save if we got valid data from the API
    if security_data.symbol.is_empty() || security_data.symbol == "None" {
      skipped_count += 1;
      continue;
    }

    // Check for duplicates within this batch
    if symbol_map.contains_key(&security_data.symbol.to_uppercase()) {
      debug!("Duplicate symbol {} found in batch, skipping", security_data.symbol);
      skipped_count += 1;
      continue;
    }
    symbol_map.insert(security_data.symbol.to_uppercase(), true);

    // Log if the matched symbol differs from original query
    if let Some(original) = &security_data.original_query {
      if !original.eq_ignore_ascii_case(&security_data.symbol) {
        info!(
          "Processing symbol {} (from search: {}, score: {:?})",
          security_data.symbol, original, security_data.match_score
        );
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
      .unwrap_or_else(|_| chrono::NaiveTime::parse_from_str("09:30", "%H:%M").unwrap());
    let market_close = chrono::NaiveTime::parse_from_str(&security_data.market_close, "%H:%M")
      .unwrap_or_else(|_| chrono::NaiveTime::parse_from_str("16:00", "%H:%M").unwrap());

    // Use the timezone from the security data or fall back to Exchange lookup
    let timezone = if !security_data.timezone.is_empty() {
      security_data.timezone.clone()
    } else {
      Exchange::from_str(&security_data.exchange)
        .map(|ex| ex.timezone().to_string())
        .unwrap_or_else(|| "US/Eastern".to_string())
    };

    // Normalize the region before saving with enhanced mapping
    let normalized_region = normalize_alpha_region(&security_data.region);

    // Check if THIS EXACT symbol already exists
    let existing_result = symbols::table
      .filter(symbols::symbol.eq(&security_data.symbol))
      .select((symbols::sid, symbols::region))
      .first::<(i64, String)>(conn)
      .optional();

    match existing_result {
      Ok(Some((sid_val, existing_region))) => {
        // Symbol already exists in database
        debug!(
          "Symbol {} already exists with SID {} in region {}",
          security_data.symbol, sid_val, existing_region
        );

        // Only update if it's the same region, otherwise skip
        if existing_region == normalized_region {
          // Use Diesel's built-in change detection by comparing all fields
          // This will only execute if at least one field is different
          match diesel::update(symbols::table.find(sid_val))
            .set((
              symbols::name.eq(&security_data.name),
              symbols::currency.eq(&security_data.currency),
              symbols::m_time.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
          {
            Ok(rows_affected) => {
              if rows_affected > 0 {
                updated_count += 1;
                info!("Updated symbol {} (SID {}) - data changed", security_data.symbol, sid_val);
              } else {
                // No rows affected means no changes
                debug!(
                  "No changes for symbol {} (SID {}), skipped update",
                  security_data.symbol, sid_val
                );
                skipped_count += 1;
              }
            }
            Err(e) => {
              error!("Failed to update symbol {}: {}", security_data.symbol, e);
              failed_count += 1;
            }
          }
        } else {
          // Different region for same symbol - this is a problem
          warn!(
            "Symbol {} already exists with different region (existing: {}, new: {}). Skipping.",
            security_data.symbol, existing_region, normalized_region
          );
          skipped_count += 1;
        }
      }
      Ok(None) => {
        // New symbol, generate SID and insert
        let new_sid = sid_generator.next_sid(security_type);
        let now_t = chrono::Utc::now().naive_utc();

        // Truncate name if needed
        let truncated_name = if security_data.name.len() > 255 {
          warn!("Truncating name for {}: '{}'", security_data.symbol, security_data.name);
          security_data.name.chars().take(255).collect()
        } else {
          security_data.name.clone()
        };

        let new_symbol = NewSymbol {
          sid: &new_sid,
          symbol: &security_data.symbol,
          priority: &9999999,
          name: &truncated_name,
          sec_type: &format!("{:?}", security_type),
          region: &normalized_region,
          currency: &security_data.currency,
          overview: &false,
          intraday: &false,
          summary: &false,
          c_time: &now_t,
          m_time: &now_t,
        };

        // Validate before insert
        let mut validation_failed = false;
        if security_data.symbol.len() > 20 {
          error!("Symbol '{}' exceeds VARCHAR(20) limit!", security_data.symbol);
          validation_failed = true;
        }
        if normalized_region.len() > 10 {
          error!("Region '{}' exceeds VARCHAR(10) limit!", normalized_region);
          validation_failed = true;
        }
        if security_data.currency.len() > 10 {
          error!("Currency '{}' exceeds VARCHAR(10) limit!", security_data.currency);
          validation_failed = true;
        }

        if validation_failed {
          failed_count += 1;
          continue;
        }

        // Try to insert
        match diesel::insert_into(symbols::table).values(&new_symbol).execute(conn) {
          Ok(_) => {
            saved_count += 1;
            info!(
              "Saved new symbol {} with SID {} in region {}",
              security_data.symbol, new_sid, normalized_region
            );
          }
          Err(e) => {
            // Check if it's a unique constraint violation
            if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = e {
              warn!(
                "Symbol {} already exists (concurrent insert?), skipping",
                security_data.symbol
              );
              skipped_count += 1;
            } else {
              error!("Failed to insert symbol {}: {}", security_data.symbol, e);
              failed_count += 1;
            }
          }
        }
        if security_type != SecurityType::Cryptocurrency {
          let new_equity_detail = NewEquityDetailOwned {
            sid: new_sid,
            exchange: security_data.exchange.clone(),
            market_open,
            market_close,
            timezone: timezone.clone(),
            c_time: now_t,
            m_time: now_t,
          };

          match diesel::insert_into(equity_details::table).values(&new_equity_detail).execute(conn)
          {
            Ok(_) => {
              debug!("Created equity details for {} (SID {})", security_data.symbol, new_sid);
            }
            Err(e) => {
              error!("Failed to create equity details for {}: {}", security_data.symbol, e);
              // Optionally handle this error - maybe roll back the symbol insert?
            }
          }
        }
      }
      Err(e) => {
        error!("Database error checking symbol {}: {}", security_data.symbol, e);
        failed_count += 1;
      }
    }

    progress.inc(1);
  }

  progress.finish_with_message(format!(
    "Completed: {} saved, {} updated, {} skipped, {} failed",
    saved_count, updated_count, skipped_count, failed_count
  ));

  if failed_count > 0 {
    warn!("Failed to process {} symbols", failed_count);
  }
  if skipped_count > 0 {
    info!("Skipped {} symbols (duplicates or no changes)", skipped_count);
  }

  // Return total successful operations
  Ok(saved_count + updated_count)
}
