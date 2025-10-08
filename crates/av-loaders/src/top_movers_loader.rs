use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use tracing::{info, warn};
use diesel::prelude::*;
use diesel::PgConnection;
use std::collections::HashMap;

use av_models::fundamentals::{TopGainersLosers, StockMover};
use av_database_postgres::{
    establish_connection,
    models::price::NewTopStat,
    schema::{symbols, topstats},
};

use crate::{
    DataLoader, LoaderContext, LoaderResult, LoaderError,
    process_tracker::ProcessState,
};

pub struct TopMoversLoader {
    database_url: Option<String>,
}

impl TopMoversLoader {
    pub fn new(database_url: Option<String>) -> Self {
        Self { database_url }
    }

    fn resolve_symbol_ids(
        &self,
        conn: &mut PgConnection,
        tickers: &[String],
    ) -> Result<HashMap<String, i64>, diesel::result::Error> {
        // Filter for Equity securities only
        let results: Vec<(String, i64)> = symbols::table
            .filter(symbols::symbol.eq_any(tickers))
            .filter(symbols::sec_type.eq("Equity"))  // Only Equity type
            .select((symbols::symbol, symbols::sid))
            .load(conn)?;

        info!("Resolved {} equity symbols out of {} total tickers",
          results.len(), tickers.len());

        Ok(results.into_iter().collect())
    }
}

#[async_trait]
impl DataLoader for TopMoversLoader {
    type Input = TopMoversLoaderInput;
    type Output = TopMoversLoaderOutput;

    async fn load(&self, context: &LoaderContext, input: Self::Input) -> LoaderResult<Self::Output> {
        info!("Loading top gainers and losers");

        // Start process tracking if available
        if let Some(tracker) = &context.process_tracker {
            tracker.start("top_movers_loader").await?;
        }

        // Fetch data from API
        let api_data = context.client
            .fundamentals()
            .top_gainers_losers()
            .await
            .map_err(|e| LoaderError::ApiError(e.to_string()))?;

        info!(
            "Fetched {} gainers, {} losers, {} most active",
            api_data.top_gainers.len(),
            api_data.top_losers.len(),
            api_data.most_actively_traded.len()
        );

        // Convert NaiveDate to DateTime<Utc> for the database
        let date_time = input.date
            .unwrap_or_else(|| Utc::now().date_naive())
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let mut records_inserted = 0;
        let mut missing_symbols = Vec::new();

        // If database URL is provided, resolve symbols and save
        if let Some(db_url) = &self.database_url {
            let mut conn = establish_connection(db_url)
                .map_err(|e| LoaderError::DatabaseError(e.to_string()))?;

            // Collect all unique tickers
            let mut all_tickers = Vec::new();
            all_tickers.extend(api_data.top_gainers.iter().map(|m| m.ticker.clone()));
            all_tickers.extend(api_data.top_losers.iter().map(|m| m.ticker.clone()));
            all_tickers.extend(api_data.most_actively_traded.iter().map(|m| m.ticker.clone()));
            all_tickers.sort();
            all_tickers.dedup();

            // Resolve to SIDs
            let symbol_map = self.resolve_symbol_ids(&mut conn, &all_tickers)
                .map_err(|e| LoaderError::DatabaseError(e.to_string()))?;

            // Store parsed values to ensure they live long enough
            struct ParsedMoverData {
                sid: i64,
                symbol: String,
                price: f32,
                change_val: f32,
                change_pct: f32,
                volume: i64,
                event_type: String,
            }

            let mut parsed_data = Vec::new();

            // Helper function to parse mover data
            let parse_mover = |mover: &StockMover, event_type: &str| -> ParsedMoverData {
                let sid = symbol_map.get(&mover.ticker).copied().unwrap_or(0);
                ParsedMoverData {
                    sid,
                    symbol: mover.ticker.clone(),
                    price: mover.price.parse::<f32>().unwrap_or(0.0),
                    change_val: mover.change_amount.parse::<f32>().unwrap_or(0.0),
                    change_pct: mover.change_percentage
                        .trim_end_matches('%')
                        .parse::<f32>()
                        .unwrap_or(0.0),
                    volume: mover.volume.parse::<i64>().unwrap_or(0),
                    event_type: event_type.to_string(),
                }
            };

            // Process gainers
            for gainer in &api_data.top_gainers {
                let data = parse_mover(gainer, "gainers");
                if data.sid == 0 {
                    missing_symbols.push(gainer.ticker.clone());
                        continue;
                }
                parsed_data.push(data);
            }

            // Process losers
            for loser in &api_data.top_losers {
                let data = parse_mover(loser, "losers");
                if data.sid == 0 {
                    missing_symbols.push(loser.ticker.clone());
                        continue;
                }
                parsed_data.push(data);
            }

            // Process most active
            for active in &api_data.most_actively_traded {
                let data = parse_mover(active, "most_active");
                if data.sid == 0 {
                    missing_symbols.push(active.ticker.clone());
                        continue;
                }
                parsed_data.push(data);
            }

            // Remove duplicates from missing symbols
            missing_symbols.sort();
            missing_symbols.dedup();

            if !missing_symbols.is_empty() {
                warn!("Missing symbols in database: {:?}", missing_symbols);
            }

            // Create NewTopStat records with references to parsed_data
            let new_records: Vec<NewTopStat> = parsed_data.iter().map(|data| {
                NewTopStat {
                    date: &date_time,
                    event_type: &data.event_type,
                    sid: &data.sid,
                    symbol: &data.symbol,
                    price: &data.price,
                    change_val: &data.change_val,
                    change_pct: &data.change_pct,
                    volume: &data.volume,
                }
            }).collect();

            // Insert into database
            records_inserted = diesel::insert_into(topstats::table)
                .values(&new_records)
                .on_conflict_do_nothing()
                .execute(&mut conn)
                .map_err(|e| LoaderError::DatabaseError(e.to_string()))?;

            info!("Inserted {} records into topstats table", records_inserted);
        }

        // Complete process tracking
        if let Some(tracker) = &context.process_tracker {
            let state = if missing_symbols.is_empty() {
                ProcessState::Success
            } else {
                ProcessState::CompletedWithErrors
            };
            tracker.complete(state).await?;
        }

        Ok(TopMoversLoaderOutput {
            date: input.date.unwrap_or_else(|| Utc::now().date_naive()),
            last_updated: api_data.last_updated.clone(),
            gainers_count: api_data.top_gainers.len(),
            losers_count: api_data.top_losers.len(),
            most_active_count: api_data.most_actively_traded.len(),
            records_saved: records_inserted,
            missing_symbols,
            raw_data: api_data,
        })
    }

    fn name(&self) -> &'static str {
        "TopMoversLoader"
    }
}

#[derive(Debug, Default)]
pub struct TopMoversLoaderInput {
    pub date: Option<NaiveDate>,
}

#[derive(Debug)]
pub struct TopMoversLoaderOutput {
    pub date: NaiveDate,
    pub last_updated: String,
    pub gainers_count: usize,
    pub losers_count: usize,
    pub most_active_count: usize,
    pub records_saved: usize,
    pub missing_symbols: Vec<String>,
    pub raw_data: TopGainersLosers,
}