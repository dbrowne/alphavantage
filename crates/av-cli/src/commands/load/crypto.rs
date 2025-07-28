
use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use chrono::Utc;
use tracing::{info, warn, error, debug};

use av_core::types::market::{SecurityIdentifier, SecurityType};
use diesel::prelude::*;
use diesel::PgConnection;

use crate::config::Config;

#[derive(Args)]
pub struct CryptoArgs {
    /// Path to digital currency list CSV file
    #[arg(long, default_value = "./data/digital_currency_list.csv", env = "DIGITAL_CURRENCY_LIST")]
    crypto_csv: String,

    /// Skip database updates (dry run)
    #[arg(short, long)]
    dry_run: bool,

    /// Continue on errors
    #[arg(short = 'k', long)]
    continue_on_error: bool,

    /// Limit number of symbols to load (for debugging)
    #[arg(short, long)]
    limit: Option<usize>,
}

/// Cryptocurrency data from CSV
#[derive(Debug)]
struct CryptoData {
    symbol: String,
    name: String,
}

/// Maintains the next available raw_id for cryptocurrency type
struct CryptoSidGenerator {
    next_raw_id: u32,
}

impl CryptoSidGenerator {
    /// Initialize by reading max cryptocurrency SIDs from database
    fn new(conn: &mut PgConnection) -> Result<Self> {
        use av_database_postgres::schema::symbols::dsl::*;

        info!("Initializing crypto SID generator");

        // Get all existing cryptocurrency SIDs
        let crypto_sids: Vec<i64> = symbols
            .filter(sec_type.eq("Cryptocurrency"))
            .select(sid)
            .load(conn)?;

        let mut max_raw_id: u32 = 0;

        // Decode each SID to find max raw_id
        for sid_val in crypto_sids {
            if let Some(identifier) = SecurityIdentifier::decode(sid_val) {
                if identifier.security_type == SecurityType::Cryptocurrency && identifier.raw_id > max_raw_id {
                    max_raw_id = identifier.raw_id;
                }
            }
        }

        info!("Cryptocurrency next raw_id: {}", max_raw_id + 1);

        Ok(Self {
            next_raw_id: max_raw_id + 1,
        })
    }

    /// Generate the next SID for a cryptocurrency
    fn next_sid(&mut self) -> i64 {
        let sid = SecurityType::encode(SecurityType::Cryptocurrency, self.next_raw_id);
        self.next_raw_id += 1;
        sid
    }
}

/// Main execute function
pub async fn execute(args: CryptoArgs, config: Config) -> Result<()> {
    info!("Starting cryptocurrency loader");
    info!("Reading from: {}", args.crypto_csv);

    if args.dry_run {
        info!("Dry run mode - no database updates will be performed");
    }

    // Read and parse CSV file
    let cryptos = read_crypto_csv(&args.crypto_csv, args.limit)?;

    if cryptos.is_empty() {
        info!("No cryptocurrencies found in CSV");
        return Ok(());
    }

    info!("Found {} cryptocurrencies in CSV", cryptos.len());

    if args.dry_run {
        // Dry run - just show what would be loaded
        for crypto in &cryptos {
            info!("Would load: {} - {}", crypto.symbol, crypto.name);
        }
        info!("Dry run completed - {} cryptocurrencies would be loaded", cryptos.len());
        return Ok(());
    }

    // Save to database
    let saved_count = tokio::task::spawn_blocking({
        let database_url = config.database_url.clone();
        let continue_on_error = args.continue_on_error;
        move || save_cryptos_to_db(&database_url, &cryptos, continue_on_error)
    }).await??;

    info!("Cryptocurrency loading completed: {} symbols saved", saved_count);
    Ok(())
}

/// Read cryptocurrencies from CSV file
fn read_crypto_csv(path: &str, limit: Option<usize>) -> Result<Vec<CryptoData>> {
    use csv::Reader;
    use std::fs::File;

    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);

    let mut cryptos = Vec::new();

    for (index, result) in reader.records().enumerate() {
        // Apply limit if specified
        if let Some(limit) = limit {
            if index >= limit {
                break;
            }
        }

        let record = result?;

        // Expect: symbol,name
        if let (Some(symbol), Some(name)) = (record.get(0), record.get(1)) {
            let symbol = symbol.trim().to_string();
            let name = name.trim().to_string();

            // Skip empty entries
            if !symbol.is_empty() && !name.is_empty() {
                cryptos.push(CryptoData { symbol, name });
            }
        }
    }

    Ok(cryptos)
}

/// Save cryptocurrencies to database
fn save_cryptos_to_db(
    database_url: &str,
    cryptos: &[CryptoData],
    continue_on_error: bool,
) -> Result<usize> {
    use av_database_postgres::schema::symbols;
    use av_database_postgres::models::security::NewSymbol;

    let mut conn = PgConnection::establish(database_url)
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    // Initialize SID generator
    let mut sid_generator = CryptoSidGenerator::new(&mut conn)?;

    let progress = ProgressBar::new(cryptos.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    progress.set_message("Saving cryptocurrencies to database");

    let mut saved_count = 0;
    let mut updated_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;

    // Fixed values for all cryptocurrencies
    let security_type = SecurityType::Cryptocurrency;
    let region = "DeFi".to_string();
    let market_open = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let market_close = chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap();
    let timezone = "UTC".to_string();
    let currency = "USD".to_string();
    let now_t = chrono::Utc::now().naive_utc();

    for crypto in cryptos {
        // Check if symbol already exists
        let existing_result = symbols::table
            .filter(symbols::symbol.eq(&crypto.symbol))
            .select((symbols::sid, symbols::sec_type))
            .first::<(i64, String)>(&mut conn)
            .optional();

        match existing_result {
            Ok(Some((sid_val, existing_sec_type))) => {
                // Symbol exists
                if existing_sec_type == "Cryptocurrency" {
                    // Update name and other fields if needed
                    match diesel::update(symbols::table.find(sid_val))
                        .set((
                            symbols::name.eq(&crypto.name),
                            symbols::region.eq(&region),
                            symbols::market_open.eq(&market_open),
                            symbols::market_close.eq(&market_close),
                            symbols::timezone.eq(&timezone),
                            symbols::currency.eq(&currency),
                            symbols::m_time.eq(&now_t),
                        ))
                        .execute(&mut conn) {
                        Ok(rows_affected) => {
                            if rows_affected > 0 {
                                updated_count += 1;
                                debug!("Updated cryptocurrency {} (SID {})", crypto.symbol, sid_val);
                            } else {
                                skipped_count += 1;
                                debug!("No changes for cryptocurrency {} (SID {})", crypto.symbol, sid_val);
                            }
                        }
                        Err(e) => {
                            error!("Failed to update cryptocurrency {}: {}", crypto.symbol, e);
                            failed_count += 1;
                            if !continue_on_error {
                                return Err(e.into());
                            }
                        }
                    }
                } else {
                    warn!("Symbol {} exists as {} type, not Cryptocurrency. Skipping.",
                          crypto.symbol, existing_sec_type);
                    skipped_count += 1;
                }
            }
            Ok(None) => {
                // New cryptocurrency, generate SID and insert
                let new_sid = sid_generator.next_sid();

                let new_symbol = NewSymbol {
                    sid: &new_sid,
                    symbol: &crypto.symbol,
                    name: &crypto.name,
                    sec_type: &format!("{:?}", security_type),
                    region: &region,
                    market_open: &market_open,
                    market_close: &market_close,
                    timezone: &timezone,
                    currency: &currency,
                    overview: &false,
                    intraday: &false,
                    summary: &false,
                    c_time: &now_t,
                    m_time: &now_t,
                };

                match diesel::insert_into(symbols::table)
                    .values(&new_symbol)
                    .execute(&mut conn) {
                    Ok(_) => {
                        saved_count += 1;
                        info!("Saved cryptocurrency {} - {} (SID {})",
                              crypto.symbol, crypto.name, new_sid);
                    }
                    Err(e) => {
                        error!("Failed to insert cryptocurrency {}: {}", crypto.symbol, e);
                        failed_count += 1;
                        if !continue_on_error {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Database error checking cryptocurrency {}: {}", crypto.symbol, e);
                failed_count += 1;
                if !continue_on_error {
                    return Err(e.into());
                }
            }
        }

        progress.inc(1);
    }

    progress.finish_with_message(format!(
        "Completed: {} saved, {} updated, {} skipped, {} failed",
        saved_count, updated_count, skipped_count, failed_count
    ));

    if failed_count > 0 {
        warn!("Failed to process {} cryptocurrencies", failed_count);
    }

    Ok(saved_count + updated_count)
}