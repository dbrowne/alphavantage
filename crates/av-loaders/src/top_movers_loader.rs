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

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use av_database_postgres::{
  establish_connection,
  models::price::NewTopStat,
  repository::{CacheRepositoryExt, NewsRepository},
};
use av_models::fundamentals::{StockMover, TopGainersLosers};
use diesel::PgConnection;

use crate::cache::{CacheConfigProvider, ttl};
use crate::{DataLoader, LoaderContext, LoaderError, LoaderResult, process_tracker::ProcessState};

const SOURCE_NAME: &str = "top_movers";
const API_SOURCE: &str = "alphavantage";

pub struct TopMoversLoader {
  config: TopMoversConfig,
  database_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TopMoversConfig {
  /// Whether to record missing symbols
  pub track_missing_symbols: bool,
  /// Enable caching of API responses
  pub enable_cache: bool,
  /// Cache TTL in hours
  pub cache_ttl_hours: i64,
  /// Force refresh (bypass cache)
  pub force_refresh: bool,
}

impl TopMoversConfig {
  pub fn with_database_url(self, _database_url: Option<String>) -> Self {
    self
  }
}

impl Default for TopMoversConfig {
  fn default() -> Self {
    Self {
      track_missing_symbols: true,
      enable_cache: true,
      cache_ttl_hours: ttl::TOP_MOVERS,
      force_refresh: false,
    }
  }
}

impl CacheConfigProvider for TopMoversConfig {
  fn cache_enabled(&self) -> bool {
    self.enable_cache
  }

  fn cache_ttl_hours(&self) -> i64 {
    self.cache_ttl_hours
  }

  fn force_refresh(&self) -> bool {
    self.force_refresh
  }
}

impl TopMoversLoader {
  pub fn new(config: TopMoversConfig, database_url: Option<String>) -> Self {
    Self { config, database_url }
  }

  /// Load all equity symbols from the database
  /// TODO: This should be moved to a SymbolRepository trait method
  fn load_all_equity_symbols(conn: &mut PgConnection) -> Result<HashMap<String, i64>, LoaderError> {
    use av_database_postgres::schema::symbols;
    use diesel::prelude::*;

    let results: Vec<(String, i64)> = symbols::table
      .filter(symbols::sec_type.eq("Equity"))
      .select((symbols::symbol, symbols::sid))
      .load(conn)
      .map_err(|e| LoaderError::DatabaseError(format!("Failed to load equity symbols: {}", e)))?;

    Ok(results.into_iter().collect())
  }

  /// Record missing symbols to the database
  async fn record_missing_symbols(
    &self,
    news_repo: &Arc<dyn NewsRepository>,
    symbols: &[String],
  ) -> Result<usize, LoaderError> {
    if !self.config.track_missing_symbols || symbols.is_empty() {
      return Ok(0);
    }

    let mut logged_count = 0;
    for symbol in symbols {
      match news_repo.record_missing_symbol(symbol, SOURCE_NAME).await {
        Ok(_) => {
          logged_count += 1;
          debug!("Recorded missing symbol: {}", symbol);
        }
        Err(e) => {
          warn!("Failed to record missing symbol {}: {}", symbol, e);
        }
      }
    }

    Ok(logged_count)
  }

  /// Parse stock mover data and convert to database format
  fn parse_mover_data(
    movers: &[StockMover],
    event_type: &str,
    symbol_map: &HashMap<String, i64>,
    missing_symbols: &mut Vec<String>,
  ) -> Vec<ParsedMoverData> {
    movers
      .iter()
      .filter_map(|mover| {
        let sid = match symbol_map.get(&mover.ticker) {
          Some(&sid) => sid,
          None => {
            missing_symbols.push(mover.ticker.clone());
            return None;
          }
        };

        let price = mover.price.parse::<f32>().unwrap_or_else(|e| {
          warn!("Failed to parse price '{}' for {}: {}", mover.price, mover.ticker, e);
          0.0
        });

        let change_val = mover.change_amount.parse::<f32>().unwrap_or_else(|e| {
          warn!(
            "Failed to parse change amount '{}' for {}: {}",
            mover.change_amount, mover.ticker, e
          );
          0.0
        });

        let change_pct =
          mover.change_percentage.trim_end_matches('%').parse::<f32>().unwrap_or_else(|e| {
            warn!(
              "Failed to parse change percentage '{}' for {}: {}",
              mover.change_percentage, mover.ticker, e
            );
            0.0
          });

        let volume = mover.volume.parse::<i64>().unwrap_or_else(|e| {
          warn!("Failed to parse volume '{}' for {}: {}", mover.volume, mover.ticker, e);
          0
        });

        Some(ParsedMoverData {
          sid,
          symbol: mover.ticker.clone(),
          price,
          change_val,
          change_pct,
          volume,
          event_type: event_type.to_string(),
        })
      })
      .collect()
  }

  /// Generate cache key for top movers data
  fn generate_cache_key(date: &NaiveDate) -> String {
    format!("top_movers:{}", date.format("%Y-%m-%d"))
  }

  /// Try to get cached response
  async fn get_cached_response(
    &self,
    context: &LoaderContext,
    cache_key: &str,
  ) -> Result<Option<TopGainersLosers>, LoaderError> {
    if !self.config.enable_cache || self.config.force_refresh {
      return Ok(None);
    }

    let cache_repo = match &context.cache_repository {
      Some(repo) => repo,
      None => {
        debug!("Cache repository not available");
        return Ok(None);
      }
    };

    match cache_repo.get::<TopGainersLosers>(cache_key, API_SOURCE).await {
      Ok(Some(data)) => {
        info!("Cache hit for key: {}", cache_key);
        Ok(Some(data))
      }
      Ok(None) => {
        debug!("Cache miss for key: {}", cache_key);
        Ok(None)
      }
      Err(e) => {
        warn!("Cache retrieval error: {}", e);
        Ok(None)
      }
    }
  }

  /// Cache the response
  async fn cache_response(
    &self,
    context: &LoaderContext,
    cache_key: &str,
    data: &TopGainersLosers,
  ) -> Result<(), LoaderError> {
    if !self.config.enable_cache {
      return Ok(());
    }

    let cache_repo = match &context.cache_repository {
      Some(repo) => repo,
      None => {
        debug!("Cache repository not available");
        return Ok(());
      }
    };

    let endpoint_url = "TOP_GAINERS_LOSERS";
    match cache_repo
      .set(cache_key, API_SOURCE, endpoint_url, data, self.config.cache_ttl_hours)
      .await
    {
      Ok(_) => {
        debug!("Cached response with key: {}", cache_key);
        Ok(())
      }
      Err(e) => {
        warn!("Failed to cache response: {}", e);
        Ok(())
      }
    }
  }
}

/// Internal struct to hold parsed mover data
#[derive(Debug, Clone)]
struct ParsedMoverData {
  sid: i64,
  symbol: String,
  price: f32,
  change_val: f32,
  change_pct: f32,
  volume: i64,
  event_type: String,
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

    // Generate cache key
    let date = input.date.unwrap_or_else(|| Utc::now().date_naive());
    let cache_key = Self::generate_cache_key(&date);

    // Try to get from cache first
    let (api_data, from_cache) =
      if let Some(cached_data) = self.get_cached_response(context, &cache_key).await? {
        info!("Using cached top movers data");
        (cached_data, true)
      } else {
        // Fetch data from API
        info!("Fetching fresh top movers data from API");
        let data = context
          .client
          .fundamentals()
          .top_gainers_losers()
          .await
          .map_err(|e| LoaderError::ApiError(e.to_string()))?;

        // Cache the response
        self.cache_response(context, &cache_key, &data).await?;

        (data, false)
      };

    info!(
      "Fetched {} gainers, {} losers, {} most active",
      api_data.top_gainers.len(),
      api_data.top_losers.len(),
      api_data.most_actively_traded.len()
    );

    // Get news repository for missing symbols tracking
    let news_repo = context.news_repository.as_ref().ok_or_else(|| {
      LoaderError::ConfigurationError("News repository not configured".to_string())
    })?;

    // Load all equity symbols from database if database URL is provided
    let symbol_map = if let Some(db_url) = &self.database_url {
      let mut conn = establish_connection(db_url)
        .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect to database: {}", e)))?;
      Self::load_all_equity_symbols(&mut conn)?
    } else {
      HashMap::new()
    };

    debug!("Loaded {} equity symbols from database", symbol_map.len());

    let mut missing_symbols = Vec::new();
    let mut all_parsed_data = Vec::new();

    // Process gainers
    let gainers_data =
      Self::parse_mover_data(&api_data.top_gainers, "gainers", &symbol_map, &mut missing_symbols);
    all_parsed_data.extend(gainers_data);

    // Process losers
    let losers_data =
      Self::parse_mover_data(&api_data.top_losers, "losers", &symbol_map, &mut missing_symbols);
    all_parsed_data.extend(losers_data);

    // Process most active
    let active_data = Self::parse_mover_data(
      &api_data.most_actively_traded,
      "most_active",
      &symbol_map,
      &mut missing_symbols,
    );
    all_parsed_data.extend(active_data);

    // Remove duplicates from missing symbols
    missing_symbols.sort();
    missing_symbols.dedup();

    let resolved_count = all_parsed_data.len();
    let total_count =
      api_data.top_gainers.len() + api_data.top_losers.len() + api_data.most_actively_traded.len();

    info!("Resolved {} equity symbols out of {} total tickers", resolved_count, total_count);

    // Record missing symbols if tracking is enabled
    let missing_recorded = if !missing_symbols.is_empty() {
      warn!("Missing symbols in database: {:?}", missing_symbols);
      self.record_missing_symbols(news_repo, &missing_symbols).await?
    } else {
      0
    };

    if missing_recorded > 0 {
      info!("Recorded {} missing symbols to database", missing_recorded);
    }

    // Save to database if we have data
    let records_inserted = if !all_parsed_data.is_empty() && self.database_url.is_some() {
      // Convert NaiveDate to DateTime<Utc> for the database
      let date_time = input
        .date
        .unwrap_or_else(|| Utc::now().date_naive())
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| LoaderError::InvalidData("Invalid date/time".to_string()))?
        .and_local_timezone(Utc)
        .single()
        .ok_or_else(|| LoaderError::InvalidData("Ambiguous date/time".to_string()))?;

      // Use direct database access for now
      // TODO: This should be refactored to use a PriceRepository trait
      use av_database_postgres::schema::topstats;
      use diesel::prelude::*;

      let db_url = self.database_url.as_ref().unwrap();
      let mut conn = establish_connection(db_url)
        .map_err(|e| LoaderError::DatabaseError(format!("Failed to connect to database: {}", e)))?;

      // Create NewTopStat records
      let new_records: Vec<NewTopStat> = all_parsed_data
        .iter()
        .map(|data| NewTopStat {
          date: &date_time,
          event_type: &data.event_type,
          sid: &data.sid,
          symbol: &data.symbol,
          price: &data.price,
          change_val: &data.change_val,
          change_pct: &data.change_pct,
          volume: &data.volume,
        })
        .collect();

      // Insert into database
      diesel::insert_into(topstats::table)
        .values(&new_records)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .map_err(|e| LoaderError::DatabaseError(format!("Failed to insert records: {}", e)))?
    } else {
      if all_parsed_data.is_empty() {
        warn!("No valid equity symbols found to save");
      } else {
        warn!("Database URL not provided, skipping database save");
      }
      0
    };

    info!("Inserted {} records into topstats table", records_inserted);

    // Complete process tracking
    if let Some(tracker) = &context.process_tracker {
      let state = if missing_symbols.is_empty() && records_inserted > 0 {
        ProcessState::Success
      } else if records_inserted > 0 {
        ProcessState::CompletedWithErrors
      } else {
        ProcessState::Failed
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
      missing_recorded,
      raw_data: api_data,
      from_cache,
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
  pub missing_recorded: usize,
  pub raw_data: TopGainersLosers,
  pub from_cache: bool,
}
