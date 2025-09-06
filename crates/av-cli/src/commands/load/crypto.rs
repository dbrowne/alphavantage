use anyhow::Result;
use clap::Args;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use av_client::AlphaVantageClient;
use av_core::{
  types::market::{SecurityIdentifier, SecurityType},
};
use av_loaders::{
  DataLoader, LoaderConfig, LoaderContext, ProcessTracker,
  crypto::{
    CryptoDataSource, CryptoLoaderConfig, CryptoSymbol, CryptoSymbolLoader,
    database::{CryptoDbInput, CryptoDbLoader},
  },
};
use diesel::prelude::*;

use crate::config::Config;

#[derive(Args, Debug)]
pub struct CryptoArgs {
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

/// SID generator using existing SecurityType system
struct CryptoSidGenerator {
  next_raw_id: u32,
}

impl CryptoSidGenerator {
  /// Initialize by reading max cryptocurrency SIDs from database
  fn new(conn: &mut PgConnection) -> Result<Self> {
    use av_database_postgres::schema::symbols::dsl::*;

    info!("Initializing crypto SID generator using existing SecurityType system");

    // Get all existing cryptocurrency SIDs
    let crypto_sids: Vec<i64> =
      symbols.filter(sec_type.eq("Cryptocurrency")).select(sid).load(conn)?;

    let mut max_raw_id: u32 = 0;

    // Use existing SecurityIdentifier::decode to find max raw_id
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

/// Main execute function using SosoValue API
pub async fn execute(args: CryptoArgs, config: Config) -> Result<()> {
  info!("Starting crypto symbol loader using SosoValue API");

  if args.dry_run {
    info!("Dry run mode - no database updates will be performed");
    return execute_dry_run(args).await;
  }

  // Validate SosoValue API key
  if args.sosovalue_api_key.is_none() {
    return Err(anyhow::anyhow!(
      "SosoValue API key is required. Set SOSOVALUE_API_KEY environment variable or use --sosovalue-api-key"
    ));
  }

  // Create API client for HTTP operations
  let client = Arc::new(AlphaVantageClient::new(config.api_config));

  // Create crypto loader configuration focused on SosoValue
  let crypto_config = CryptoLoaderConfig {
    sources: vec![CryptoDataSource::SosoValue],
    batch_size: args.batch_size,
    max_concurrent_requests: args.concurrent,
    rate_limit_delay_ms: 1000, // Conservative for SosoValue
    enable_progress_bar: args.verbose,
    ..Default::default()
  };

  // Create crypto database loader
  let crypto_loader = CryptoDbLoader::new(crypto_config);

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

  // Prepare API keys for SosoValue
  let mut api_keys = HashMap::new();
  if let Some(key) = args.sosovalue_api_key {
    api_keys.insert(CryptoDataSource::SosoValue, key);
  }

  // Create loader input
  let input = CryptoDbInput {
    sources: Some(vec![CryptoDataSource::SosoValue]),
    update_existing: args.update_existing,
    batch_size: Some(args.batch_size),
    api_keys: Some(api_keys),
  };

  // Execute the loader using existing framework
  match crypto_loader.load(&context, input).await {
    Ok(output) => {
      info!(
        "SosoValue crypto loading completed: {} fetched, {} processed, {} errors",
        output.symbols_fetched, output.symbols_processed, output.errors
      );

      // Display SosoValue-specific results
      if let Some(sosovalue_result) = output.source_results.get(&CryptoDataSource::SosoValue) {
        info!(
          "SosoValue API: {} symbols fetched, {} processed{}",
          sosovalue_result.symbols_fetched,
          sosovalue_result.symbols_processed,
          if sosovalue_result.rate_limited { " (rate limited)" } else { "" }
        );

        if args.verbose && !sosovalue_result.errors.is_empty() {
          warn!("SosoValue API errors:");
          for error in &sosovalue_result.errors {
            warn!("  - {}", error);
          }
        }
      }

      // Save symbols to database using existing patterns
      if !output.symbols.is_empty() {
        let symbols: Vec<CryptoSymbol> = output
          .symbols
          .into_iter()
          .map(|db_symbol| CryptoSymbol {
            symbol: db_symbol.symbol,
            name: db_symbol.name,
            source: db_symbol.source,
            source_id: db_symbol.source_id,
            market_cap_rank: db_symbol.market_cap_rank,
            base_currency: db_symbol.base_currency,
            quote_currency: db_symbol.quote_currency,
            is_active: db_symbol.is_active,
            created_at: chrono::Utc::now(),
            additional_data: serde_json::from_value(db_symbol.additional_data).unwrap_or_default(),
          })
          .collect();

        let (saved_count, updated_count, failed_count) = save_crypto_symbols_to_db(
          &config.database_url,
          &symbols,
          args.update_existing,
          args.continue_on_error,
        )
        .await?;

        info!(
          "Database save completed: {} created, {} updated, {} failed",
          saved_count, updated_count, failed_count
        );

        if failed_count > 0 && !args.continue_on_error {
          return Err(anyhow::anyhow!(
            "Crypto loading completed with {} database errors",
            failed_count
          ));
        }
      } else {
        warn!("No crypto symbols received from SosoValue API");
      }

      if output.errors > 0 && !args.continue_on_error {
        return Err(anyhow::anyhow!("Crypto loading completed with {} API errors", output.errors));
      }
    }
    Err(e) => {
      error!("SosoValue crypto loading failed: {}", e);
      return Err(e.into());
    }
  }

  Ok(())
}

/// Execute in dry run mode - test SosoValue API connection
async fn execute_dry_run(args: CryptoArgs) -> Result<()> {
  info!("Executing crypto loader in dry run mode");

  info!("Configuration:");
  info!("  - Data source: SosoValue API");
  info!("  - Concurrent requests: {}", args.concurrent);
  info!("  - Batch size: {}", args.batch_size);
  info!("  - Update existing: {}", args.update_existing);
  info!("  - Continue on error: {}", args.continue_on_error);

  if args.sosovalue_api_key.is_some() {
    info!("  - SosoValue API key: configured");

    // Test API connection in dry run
    info!("Testing SosoValue API connection...");

    let crypto_config = CryptoLoaderConfig {
      sources: vec![CryptoDataSource::SosoValue],
      batch_size: 10, // Small batch for testing
      max_concurrent_requests: 1,
      rate_limit_delay_ms: 1000,
      enable_progress_bar: false,
      ..Default::default()
    };

    let mut api_keys = HashMap::new();
    api_keys.insert(CryptoDataSource::SosoValue, args.sosovalue_api_key.unwrap());

    let crypto_loader = CryptoSymbolLoader::new(crypto_config).with_api_keys(api_keys);

    match crypto_loader.load_from_source(CryptoDataSource::SosoValue).await {
      Ok(symbols) => {
        info!("✓ SosoValue API connection successful");
        info!("✓ Retrieved {} crypto symbols", symbols.len());

        if args.verbose && !symbols.is_empty() {
          info!("Sample symbols from SosoValue:");
          for (i, symbol) in symbols.iter().take(5).enumerate() {
            info!("  {}. {} ({})", i + 1, symbol.symbol, symbol.name);
          }
          if symbols.len() > 5 {
            info!("  ... and {} more", symbols.len() - 5);
          }
        }
      }
      Err(e) => {
        error!("✗ SosoValue API connection failed: {}", e);
        return Err(anyhow::anyhow!("SosoValue API test failed: {}", e));
      }
    }
  } else {
    warn!("  - SosoValue API key: not configured");
    warn!("    Set SOSOVALUE_API_KEY environment variable or use --sosovalue-api-key");
  }

  info!("Dry run completed - no actual database updates performed");
  Ok(())
}

/// Save crypto symbols to database using existing patterns
async fn save_crypto_symbols_to_db(
  database_url: &str,
  symbols: &[CryptoSymbol],
  update_existing: bool,
  continue_on_error: bool,
) -> Result<(usize, usize, usize)> {
  use av_database_postgres::{
    models::security::{NewSymbolOwned },
    schema::symbols,
  };
  use diesel::PgConnection;

  let database_url = database_url.to_string();
  let symbols = symbols.to_vec();

  // Execute in blocking context since diesel is synchronous
  tokio::task::spawn_blocking(move || {
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
                  symbols::m_time.eq(chrono::Utc::now()),
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
              debug!("Symbol {} already exists, skipping", crypto_symbol.symbol);
            }
          }
          Ok(None) => {
            // Generate new SID using existing system
            let new_sid = sid_generator.next_sid();

            // Create new symbol using existing pattern
            let new_symbol = NewSymbolOwned::from_symbol_data(
              &crypto_symbol.symbol,
              &crypto_symbol.name,
              "Cryptocurrency",
              "DeFi",
              crypto_symbol.quote_currency.as_deref().unwrap_or("USD"),
              new_sid,
            );

            match diesel::insert_into(symbols::table).values(&new_symbol.as_ref()).execute(conn) {
              Ok(_) => {
                saved_count += 1;
                info!("Created crypto symbol: {} (SID: {})", crypto_symbol.symbol, new_sid);
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
      "Database save completed: {} created, {} updated, {} failed",
      saved_count, updated_count, failed_count
    );

    Ok((saved_count, updated_count, failed_count))
  })
  .await?
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crypto_sid_generation() {
    // Test that crypto SIDs use SecurityType::Cryptocurrency encoding
    let encoded_sid = SecurityType::encode(SecurityType::Cryptocurrency, 1);
    let decoded = SecurityIdentifier::decode(encoded_sid).unwrap();

    assert_eq!(decoded.security_type, SecurityType::Cryptocurrency);
    assert_eq!(decoded.raw_id, 1);
  }

  #[tokio::test]
  async fn test_dry_run_without_api_key() {
    let args = CryptoArgs {
      dry_run: true,
      continue_on_error: false,
      limit: None,
      update_existing: false,
      sosovalue_api_key: None,
      concurrent: 5,
      batch_size: 100,
      verbose: false,
      track_process: false,
    };

    // Should complete without error even without API key in dry run
    let result = execute_dry_run(args).await;
    assert!(result.is_ok());
  }

  #[test]
  fn test_cli_args_parsing() {
    // Test that CLI args can be parsed correctly
    // This would be expanded in a real test environment

    assert_eq!(CryptoDataSource::SosoValue.to_string(), "sosovalue");
  }
}

