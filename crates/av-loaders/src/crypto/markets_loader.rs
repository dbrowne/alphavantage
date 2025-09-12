use crate::{
    LoaderResult, LoaderError,
};
use crate::crypto::CryptoDataSource;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use bigdecimal::{BigDecimal, ToPrimitive};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use indicatif::{ProgressBar, ProgressStyle};
// Define the struct locally to match what the CLI expects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSymbolForMarkets {
    pub sid: i64,
    pub symbol: String,
    pub name: String,
    pub coingecko_id: Option<String>,
    pub alphavantage_symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CryptoMarketData {
    pub sid: i64,
    pub exchange: String,
    pub base: String,
    pub target: String,
    pub market_type: Option<String>,
    pub volume_24h: Option<BigDecimal>,
    pub volume_percentage: Option<BigDecimal>,
    pub bid_ask_spread_pct: Option<BigDecimal>,
    pub liquidity_score: Option<String>,
    pub trust_score: Option<String>,
    pub is_active: bool,
    pub is_anomaly: bool,
    pub is_stale: bool,
    pub last_price: Option<f64>,
    pub last_traded_at: Option<String>,
    pub last_fetch_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CryptoMarketsConfig {
    pub coingecko_api_key: Option<String>,
    pub delay_ms: u64,
    pub batch_size: usize,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub max_concurrent_requests: usize,
    pub rate_limit_delay_ms: u64,
    pub enable_progress_bar: bool,
    pub alphavantage_api_key: Option<String>,
    pub fetch_all_exchanges: bool,
    pub min_volume_threshold: Option<f64>,
    pub max_markets_per_symbol: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct CryptoMarketsInput {
    pub symbols: Option<Vec<CryptoSymbolForMarkets>>,
    pub exchange_filter: Option<Vec<String>>,
    pub update_existing: bool,
    pub sources: Vec<CryptoDataSource>,
    pub batch_size: Option<usize>,
}

pub struct CryptoMarketsLoader {
    config: CryptoMarketsConfig,
    client: Client,
}

impl CryptoMarketsLoader {
    pub fn new(config: CryptoMarketsConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("AlphaVantage-Rust-Client/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    pub async fn load(
        &self,
        _context: &crate::LoaderContext,
        input: CryptoMarketsInput,
    ) -> LoaderResult<Vec<CryptoMarketData>> {
        let symbols = input.symbols.unwrap_or_default();

        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        info!("Starting market data fetch for {} symbols", symbols.len());

        let mut all_market_data = Vec::new();

        // Setup progress bar if enabled
        let progress = if self.config.enable_progress_bar {
            let pb = ProgressBar::new(symbols.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                    .expect("Invalid progress template")
                    .progress_chars("##-"),
            );
            Some(pb)
        } else {
            None
        };

        // Process symbols in batches
        let batch_size = input.batch_size.unwrap_or(self.config.batch_size);
        let symbol_chunks: Vec<_> = symbols.chunks(batch_size).collect();

        for (chunk_index, symbol_chunk) in symbol_chunks.iter().enumerate() {
            info!("Processing batch {}/{}", chunk_index + 1, symbol_chunks.len());

            // Execute batch tasks with concurrency control
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_requests));
            let mut handles = Vec::new();

            for symbol in symbol_chunk.iter() {
                let sem = semaphore.clone();
                let symbol_clone = symbol.clone();
                let config = self.config.clone();
                let client = self.client.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("Semaphore acquire failed");

                    // Create a temporary loader for this task
                    let temp_loader = CryptoMarketsLoader { config, client };
                    temp_loader.fetch_market_data_for_symbol(symbol_clone).await
                });
                handles.push(handle);
            }

            // Collect results from the batch
            for handle in handles {
                match handle.await {
                    Ok(Ok(mut market_data)) => {
                        all_market_data.append(&mut market_data);
                    }
                    Ok(Err(e)) => {
                        warn!("Failed to fetch market data: {}", e);
                    }
                    Err(e) => {
                        error!("Task join error: {}", e);
                    }
                }

                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            }

            // Rate limiting between batches
            if chunk_index < symbol_chunks.len() - 1 {
                sleep(Duration::from_millis(self.config.rate_limit_delay_ms)).await;
            }
        }

        if let Some(pb) = progress {
            pb.finish_with_message("Market data fetch complete");
        }

        info!("Completed market data fetch. Retrieved {} market entries", all_market_data.len());
        Ok(all_market_data)
    }



    async fn fetch_market_data_for_symbol(
        &self,
        symbol: CryptoSymbolForMarkets,
    ) -> LoaderResult<Vec<CryptoMarketData>> {
        info!("🔍 Processing symbol: {} ({})", symbol.symbol, symbol.name);
        info!("🔍 CoinGecko ID: {:?}", symbol.coingecko_id);

        let mut market_data = Vec::new();

        // Check if we have a CoinGecko ID
        if let Some(ref coingecko_id) = symbol.coingecko_id {
            info!("✅ {} has CoinGecko ID: {}", symbol.symbol, coingecko_id);

            match self.fetch_coingecko_markets(coingecko_id, &symbol).await {
                Ok(mut data) => {
                    info!("✅ CoinGecko returned {} markets for {}", data.len(), symbol.symbol);
                    market_data.append(&mut data);
                }
                Err(e) => {
                    error!("❌ CoinGecko fetch failed for {}: {}", symbol.symbol, e);
                    // Don't return error, just log it and continue with empty results
                }
            }
        } else {
            warn!("⚠️  {} has no CoinGecko ID - skipping", symbol.symbol);
        }

        // Apply filters
        let original_count = market_data.len();

        if let Some(min_volume) = self.config.min_volume_threshold {
            let before_filter = market_data.len();
            market_data.retain(|m| {
                if let Some(ref volume) = m.volume_24h {
                    volume.to_f64().unwrap_or(0.0) >= min_volume
                } else {
                    false
                }
            });
            let after_filter = market_data.len();
            info!("📊 Volume filter for {}: {} -> {} markets (min: ${:.0})",
                 symbol.symbol, before_filter, after_filter, min_volume);
        }

        if let Some(max_markets) = self.config.max_markets_per_symbol {
            if market_data.len() > max_markets {
                market_data.truncate(max_markets);
                info!("📊 Truncated {} markets to {} (max per symbol)",
                     symbol.symbol, max_markets);
            }
        }

        info!("✅ Final result for {}: {} markets (from {} original)",
             symbol.symbol, market_data.len(), original_count);

        Ok(market_data)
    }

    async fn fetch_coingecko_markets(
        &self,
        coingecko_id: &str,
        symbol: &CryptoSymbolForMarkets,
    ) -> LoaderResult<Vec<CryptoMarketData>> {
        let base_url = "https://api.coingecko.com/api/v3";
        let mut url = format!("{}/coins/{}/tickers", base_url, coingecko_id);

        // Add API key if available
        if let Some(ref api_key) = self.config.coingecko_api_key {
            // Determine the correct parameter name based on API key format
            let auth_param = if api_key.starts_with("CG-") {
                // Pro API key
                url = format!("https://pro-api.coingecko.com/api/v3/coins/{}/tickers", coingecko_id);
                "x_cg_pro_api_key"
            } else {
                // Demo API key
                "x_cg_demo_api_key"
            };
            url = format!("{}?{}={}", url, auth_param, api_key);
            info!("🔑 Using {} API key for {}",
                 if api_key.starts_with("CG-") { "Pro" } else { "Demo" }, symbol.symbol);
        } else {
            warn!("⚠️  No CoinGecko API key provided for {} - using free tier (very limited)", symbol.symbol);
        }

        info!("🌐 API URL for {}: {}", symbol.symbol, url);

        let mut retries = 0;
        while retries < self.config.max_retries {
            info!("📡 Making API request for {} (attempt {}/{})",
                 symbol.symbol, retries + 1, self.config.max_retries);

            match self.client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    info!("📡 HTTP Status for {}: {}", symbol.symbol, status);

                    if status.is_success() {
                        let response_text = response.text().await.map_err(|e| {
                            error!("Failed to read response body for {}: {}", symbol.symbol, e);
                            LoaderError::IoError(format!("Failed to read response: {}", e))
                        })?;

                        info!("📄 Response length for {}: {} chars", symbol.symbol, response_text.len());

                        // Log first 200 chars for debugging
                        if response_text.len() > 200 {
                            info!("📄 Response preview for {}: {}...", symbol.symbol, &response_text[..200]);
                        } else if response_text.len() > 0 {
                            info!("📄 Full response for {}: {}", symbol.symbol, response_text);
                        }

                        match serde_json::from_str::<CoinGeckoTickersResponse>(&response_text) {
                            Ok(tickers_response) => {
                                info!("✅ Successfully parsed JSON for {}: {} tickers",
                                     symbol.symbol, tickers_response.tickers.len());
                                return self.parse_coingecko_markets(tickers_response, symbol);
                            }
                            Err(e) => {
                                error!("❌ JSON parse error for {}: {}", symbol.symbol, e);
                                error!("❌ Problematic response: {}", response_text);
                                return Err(LoaderError::SerializationError(format!(
                                    "Failed to parse CoinGecko response for {}: {}", symbol.symbol, e
                                )));
                            }
                        }
                    } else if status == 429 {
                        let delay = Duration::from_millis(self.config.rate_limit_delay_ms * (retries + 1) as u64);
                        warn!("⏱️  Rate limited for {}, waiting {:?} (attempt {})",
                             symbol.symbol, delay, retries + 1);
                        sleep(delay).await;
                        retries += 1;
                        continue;
                    } else if status == 404 {
                        warn!("❌ CoinGecko ID '{}' not found for {}", coingecko_id, symbol.symbol);
                        return Ok(Vec::new()); // Return empty results for 404
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        error!("❌ API error for {}: HTTP {} - {}", symbol.symbol, status, error_text);
                        return Err(LoaderError::ApiError(format!(
                            "CoinGecko API error for {}: HTTP {} - {}", symbol.symbol, status, error_text
                        )));
                    }
                }
                Err(e) => {
                    error!("❌ Network error for {}: {}", symbol.symbol, e);
                    return Err(LoaderError::IoError(format!(
                        "Request failed for {}: {}", symbol.symbol, e
                    )));
                }
            }
        }

        error!("❌ Max retries exceeded for {}", symbol.symbol);
        Err(LoaderError::ApiError(format!("Max retries exceeded for {}", symbol.symbol)))
    }

    fn parse_coingecko_markets(
        &self,
        response: CoinGeckoTickersResponse,
        symbol: &CryptoSymbolForMarkets,
    ) -> LoaderResult<Vec<CryptoMarketData>> {
        let mut markets = Vec::new();

        for ticker in response.tickers {
            let market_data = CryptoMarketData {
                sid: symbol.sid,
                exchange: ticker.market.name,
                base: ticker.base,
                target: ticker.target,
                market_type: Some("spot".to_string()),
                volume_24h: ticker.volume.map(|v| BigDecimal::try_from(v).unwrap_or_default()),
                volume_percentage: None, // Not provided by CoinGecko tickers
                bid_ask_spread_pct: ticker.bid_ask_spread_percentage.map(|s| BigDecimal::try_from(s).unwrap_or_default()),
                liquidity_score: None,
                trust_score: ticker.trust_score.map(|s| s.to_string()),
                is_active: ticker.market.has_trading_incentive.unwrap_or(true),
                is_anomaly: ticker.is_anomaly.unwrap_or(false),
                is_stale: ticker.is_stale.unwrap_or(false),
                last_price: ticker.last,
                last_traded_at: ticker.last_traded_at,
                last_fetch_at: Some(chrono::Utc::now().to_rfc3339()),
            };
            markets.push(market_data);
        }

        Ok(markets)
    }
}

// CoinGecko API response structures
#[derive(Debug, Deserialize)]
struct CoinGeckoTickersResponse {
    name: String,
    tickers: Vec<CoinGeckoTicker>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoTicker {
    base: String,
    target: String,
    market: CoinGeckoMarket,
    last: Option<f64>,
    volume: Option<f64>,
    trust_score: Option<String>,
    bid_ask_spread_percentage: Option<f64>,
    timestamp: Option<String>,
    last_traded_at: Option<String>,
    last_fetch_at: Option<String>,
    is_anomaly: Option<bool>,
    is_stale: Option<bool>,
    trade_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoMarket {
    name: String,
    identifier: String,
    has_trading_incentive: Option<bool>,
}