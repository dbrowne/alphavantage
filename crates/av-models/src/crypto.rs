//! Cryptocurrency data models

use crate::common::{OhlcData, TimeSeriesData};
use serde::{Deserialize, Serialize};

/// Cryptocurrency exchange rate response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchangeRate {
    /// Realtime currency exchange rate
    #[serde(rename = "Realtime Currency Exchange Rate")]
    pub realtime_currency_exchange_rate: CryptoExchangeRateData,
}

/// Cryptocurrency exchange rate data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchangeRateData {
    /// From currency code (crypto symbol)
    #[serde(rename = "1. From_Currency Code")]
    pub from_currency_code: String,
    
    /// From currency name
    #[serde(rename = "2. From_Currency Name")]
    pub from_currency_name: String,
    
    /// To currency code (fiat or crypto)
    #[serde(rename = "3. To_Currency Code")]
    pub to_currency_code: String,
    
    /// To currency name
    #[serde(rename = "4. To_Currency Name")]
    pub to_currency_name: String,
    
    /// Exchange rate
    #[serde(rename = "5. Exchange Rate")]
    pub exchange_rate: String,
    
    /// Last refreshed timestamp
    #[serde(rename = "6. Last Refreshed")]
    pub last_refreshed: String,
    
    /// Timezone
    #[serde(rename = "7. Time Zone")]
    pub time_zone: String,
    
    /// Bid price
    #[serde(rename = "8. Bid Price")]
    pub bid_price: String,
    
    /// Ask price
    #[serde(rename = "9. Ask Price")]
    pub ask_price: String,
}

/// Cryptocurrency OHLCV data with additional crypto-specific fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoOhlcvData {
    /// Opening price
    #[serde(rename = "1a. open (USD)")]
    pub open_usd: String,
    
    /// Highest price
    #[serde(rename = "2a. high (USD)")]
    pub high_usd: String,
    
    /// Lowest price
    #[serde(rename = "3a. low (USD)")]
    pub low_usd: String,
    
    /// Closing price
    #[serde(rename = "4a. close (USD)")]
    pub close_usd: String,
    
    /// Volume in USD
    #[serde(rename = "5. volume")]
    pub volume: String,
    
    /// Market cap (if available)
    #[serde(rename = "6. market cap (USD)", skip_serializing_if = "Option::is_none")]
    pub market_cap_usd: Option<String>,
}

/// Alternative OHLCV structure for some crypto endpoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoOhlcvDataAlt {
    /// Opening price
    #[serde(rename = "1. open")]
    pub open: String,
    
    /// Highest price
    #[serde(rename = "2. high")]
    pub high: String,
    
    /// Lowest price
    #[serde(rename = "3. low")]
    pub low: String,
    
    /// Closing price
    #[serde(rename = "4. close")]
    pub close: String,
    
    /// Volume
    #[serde(rename = "5. volume")]
    pub volume: String,
}

/// Cryptocurrency intraday time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoIntraday {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: CryptoMetadata,
    
    /// Time series data
    #[serde(flatten)]
    pub time_series: TimeSeriesData<CryptoOhlcvDataAlt>,
}

/// Cryptocurrency daily time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoDaily {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: CryptoMetadata,
    
    /// Time series data (USD)
    #[serde(rename = "Time Series (Digital Currency Daily)")]
    pub time_series: TimeSeriesData<CryptoOhlcvData>,
}

/// Cryptocurrency weekly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoWeekly {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: CryptoMetadata,
    
    /// Time series data
    #[serde(rename = "Time Series (Digital Currency Weekly)")]
    pub time_series: TimeSeriesData<CryptoOhlcvData>,
}

/// Cryptocurrency monthly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoMonthly {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: CryptoMetadata,
    
    /// Time series data
    #[serde(rename = "Time Series (Digital Currency Monthly)")]
    pub time_series: TimeSeriesData<CryptoOhlcvData>,
}

/// Cryptocurrency metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoMetadata {
    /// Information
    #[serde(rename = "1. Information")]
    pub information: String,
    
    /// Digital currency code
    #[serde(rename = "2. Digital Currency Code")]
    pub digital_currency_code: String,
    
    /// Digital currency name
    #[serde(rename = "3. Digital Currency Name")]
    pub digital_currency_name: String,
    
    /// Market code
    #[serde(rename = "4. Market Code")]
    pub market_code: String,
    
    /// Market name
    #[serde(rename = "5. Market Name")]
    pub market_name: String,
    
    /// Last refreshed
    #[serde(rename = "6. Last Refreshed")]
    pub last_refreshed: String,
    
    /// Interval (for intraday)
    #[serde(rename = "7. Interval", skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    
    /// Output size (for intraday)
    #[serde(rename = "8. Output Size", skip_serializing_if = "Option::is_none")]
    pub output_size: Option<String>,
    
    /// Time zone
    #[serde(rename = "9. Time Zone")]
    pub time_zone: String,
}

/// Cryptocurrency market information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoMarketInfo {
    /// Symbol
    pub symbol: String,
    
    /// Full name
    pub name: String,
    
    /// Current price in USD
    pub price_usd: f64,
    
    /// 24h price change
    pub change_24h: f64,
    
    /// 24h price change percentage
    pub change_percent_24h: f64,
    
    /// Market cap
    pub market_cap: f64,
    
    /// 24h volume
    pub volume_24h: f64,
    
    /// Circulating supply
    pub circulating_supply: f64,
    
    /// Total supply
    pub total_supply: Option<f64>,
    
    /// Max supply
    pub max_supply: Option<f64>,
    
    /// Last updated
    pub last_updated: String,
}

/// Cryptocurrency exchange information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchange {
    /// Exchange name
    pub name: String,
    
    /// Exchange ID
    pub id: String,
    
    /// 24h volume
    pub volume_24h_usd: f64,
    
    /// Number of active trading pairs
    pub active_pairs: u32,
    
    /// Website URL
    pub website_url: String,
    
    /// Country of operation
    pub country: Option<String>,
    
    /// Trust score
    pub trust_score: Option<f64>,
}

/// Trading pair information on an exchange
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradingPair {
    /// Base currency
    pub base_currency: String,
    
    /// Quote currency
    pub quote_currency: String,
    
    /// Pair symbol
    pub symbol: String,
    
    /// Current price
    pub price: f64,
    
    /// 24h volume
    pub volume_24h: f64,
    
    /// Bid price
    pub bid: f64,
    
    /// Ask price
    pub ask: f64,
    
    /// Spread percentage
    pub spread_percentage: f64,
    
    /// Exchange where pair is traded
    pub exchange: String,
    
    /// Last updated
    pub last_updated: String,
}

/// Cryptocurrency health index (custom calculation)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoHealthIndex {
    /// Cryptocurrency symbol
    pub symbol: String,
    
    /// Market (e.g., "USD")
    pub market: String,
    
    /// Price trend analysis
    pub price_trend: String,
    
    /// Volume trend analysis
    pub volume_trend: String,
    
    /// Volatility percentage
    pub volatility: f64,
    
    /// Last updated timestamp
    pub last_updated: String,
}

/// DeFi (Decentralized Finance) protocol information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefiProtocol {
    /// Protocol name
    pub name: String,
    
    /// Native token symbol
    pub token_symbol: String,
    
    /// Total Value Locked (TVL)
    pub tvl_usd: f64,
    
    /// 24h TVL change
    pub tvl_change_24h: f64,
    
    /// Protocol category
    pub category: String,
    
    /// Blockchain network
    pub network: String,
    
    /// Website URL
    pub website: String,
    
    /// Description
    pub description: String,
}

/// NFT (Non-Fungible Token) collection information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NftCollection {
    /// Collection name
    pub name: String,
    
    /// Collection symbol
    pub symbol: String,
    
    /// Blockchain network
    pub network: String,
    
    /// Floor price in ETH
    pub floor_price_eth: f64,
    
    /// Total volume in ETH
    pub total_volume_eth: f64,
    
    /// Number of items
    pub total_supply: u32,
    
    /// Number of owners
    pub num_owners: u32,
    
    /// 24h sales count
    pub sales_24h: u32,
    
    /// 24h volume change
    pub volume_change_24h: f64,
}

impl CryptoExchangeRate {
    /// Get the exchange rate data
    pub fn rate(&self) -> &CryptoExchangeRateData {
        &self.realtime_currency_exchange_rate
    }
}

impl CryptoExchangeRateData {
    /// Parse exchange rate as f64
    pub fn rate_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.exchange_rate.parse()
    }
    
    /// Parse bid price as f64
    pub fn bid_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.bid_price.parse()
    }
    
    /// Parse ask price as f64
    pub fn ask_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.ask_price.parse()
    }
    
    /// Calculate bid-ask spread
    pub fn spread(&self) -> Result<f64, std::num::ParseFloatError> {
        let bid = self.bid_as_f64()?;
        let ask = self.ask_as_f64()?;
        Ok(ask - bid)
    }
    
    /// Get the trading pair symbol
    pub fn pair_symbol(&self) -> String {
        format!("{}{}", self.from_currency_code, self.to_currency_code)
    }
}

impl CryptoOhlcvData {
    /// Parse opening price as f64
    pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.open_usd.parse()
    }
    
    /// Parse closing price as f64
    pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.close_usd.parse()
    }
    
    /// Parse volume as f64
    pub fn volume_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.volume.parse()
    }
    
    /// Calculate price change
    pub fn price_change(&self) -> Result<f64, std::num::ParseFloatError> {
        let open = self.open_as_f64()?;
        let close = self.close_as_f64()?;
        Ok(close - open)
    }
    
    /// Calculate percentage change
    pub fn percentage_change(&self) -> Result<f64, std::num::ParseFloatError> {
        let open = self.open_as_f64()?;
        let close = self.close_as_f64()?;
        if open == 0.0 {
            Ok(0.0)
        } else {
            Ok(((close - open) / open) * 100.0)
        }
    }
    
    /// Parse market cap as f64 if available
    pub fn market_cap_as_f64(&self) -> Option<Result<f64, std::num::ParseFloatError>> {
        self.market_cap_usd.as_ref().map(|mc| mc.parse())
    }
}

impl CryptoOhlcvDataAlt {
    /// Parse opening price as f64
    pub fn open_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.open.parse()
    }
    
    /// Parse closing price as f64
    pub fn close_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.close.parse()
    }
    
    /// Parse volume as f64
    pub fn volume_as_f64(&self) -> Result<f64, std::num::ParseFloatError> {
        self.volume.parse()
    }
}

impl CryptoDaily {
    /// Get the latest data point
    pub fn latest(&self) -> Option<(&String, &CryptoOhlcvData)> {
        self.time_series.iter().next()
    }
    
    /// Get the number of data points
    pub fn len(&self) -> usize {
        self.time_series.len()
    }
    
    /// Check if the time series is empty
    pub fn is_empty(&self) -> bool {
        self.time_series.is_empty()
    }
    
    /// Calculate average daily volume
    pub fn average_volume(&self) -> Result<f64, std::num::ParseFloatError> {
        let volumes: Result<Vec<f64>, _> = self.time_series
            .values()
            .map(|data| data.volume_as_f64())
            .collect();
        
        let volumes = volumes?;
        if volumes.is_empty() {
            Ok(0.0)
        } else {
            Ok(volumes.iter().sum::<f64>() / volumes.len() as f64)
        }
    }
    
    /// Calculate volatility (standard deviation of daily returns)
    pub fn calculate_volatility(&self, days: usize) -> Result<f64, std::num::ParseFloatError> {
        let prices: Result<Vec<f64>, _> = self.time_series
            .values()
            .take(days)
            .map(|data| data.close_as_f64())
            .collect();
        
        let prices = prices?;
        if prices.len() < 2 {
            return Ok(0.0);
        }
        
        // Calculate daily returns
        let mut returns = Vec::new();
        for i in 1..prices.len() {
            let return_rate = (prices[i-1] / prices[i] - 1.0) * 100.0;
            returns.push(return_rate);
        }
        
        // Calculate standard deviation
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        Ok(variance.sqrt())
    }
}

impl CryptoMarketInfo {
    /// Calculate market cap from price and circulating supply
    pub fn calculate_market_cap(&self) -> f64 {
        self.price_usd * self.circulating_supply
    }
    
    /// Check if this is a stablecoin (price close to $1)
    pub fn is_stablecoin(&self) -> bool {
        (self.price_usd - 1.0).abs() < 0.1
    }
    
    /// Get supply utilization (circulating / max supply)
    pub fn supply_utilization(&self) -> Option<f64> {
        self.max_supply.map(|max| {
            if max > 0.0 {
                self.circulating_supply / max
            } else {
                0.0
            }
        })
    }
}

impl TradingPair {
    /// Calculate mid price from bid and ask
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }
    
    /// Check if spread is reasonable for trading
    pub fn is_liquid(&self) -> bool {
        self.spread_percentage < 1.0 // Less than 1% spread
    }
    
    /// Get the pair in standard format
    pub fn standard_symbol(&self) -> String {
        format!("{}/{}", self.base_currency, self.quote_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crypto_exchange_rate() {
        let rate_data = CryptoExchangeRateData {
            from_currency_code: "BTC".to_string(),
            from_currency_name: "Bitcoin".to_string(),
            to_currency_code: "USD".to_string(),
            to_currency_name: "United States Dollar".to_string(),
            exchange_rate: "45000.00".to_string(),
            last_refreshed: "2024-01-15 16:00:00".to_string(),
            time_zone: "UTC".to_string(),
            bid_price: "44950.00".to_string(),
            ask_price: "45050.00".to_string(),
        };
        
        assert_eq!(rate_data.rate_as_f64().unwrap(), 45000.00);
        assert_eq!(rate_data.spread().unwrap(), 100.00);
        assert_eq!(rate_data.pair_symbol(), "BTCUSD");
    }
    
    #[test]
    fn test_crypto_ohlcv_calculations() {
        let data = CryptoOhlcvData {
            open_usd: "44000.00".to_string(),
            high_usd: "45500.00".to_string(),
            low_usd: "43500.00".to_string(),
            close_usd: "45000.00".to_string(),
            volume: "1000.5".to_string(),
            market_cap_usd: Some("850000000000".to_string()),
        };
        
        assert_eq!(data.open_as_f64().unwrap(), 44000.00);
        assert_eq!(data.close_as_f64().unwrap(), 45000.00);
        assert_eq!(data.price_change().unwrap(), 1000.00);
        
        let pct_change = data.percentage_change().unwrap();
        assert!((pct_change - 2.272727).abs() < 0.001); // Approximately 2.27%
        
        assert_eq!(data.market_cap_as_f64().unwrap().unwrap(), 850000000000.0);
    }
    
    #[test]
    fn test_crypto_market_info() {
        let market_info = CryptoMarketInfo {
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            price_usd: 45000.0,
            change_24h: 1000.0,
            change_percent_24h: 2.27,
            market_cap: 850000000000.0,
            volume_24h: 25000000000.0,
            circulating_supply: 19000000.0,
            total_supply: Some(19000000.0),
            max_supply: Some(21000000.0),
            last_updated: "2024-01-15T16:00:00Z".to_string(),
        };
        
        assert!(!market_info.is_stablecoin());
        
        let utilization = market_info.supply_utilization().unwrap();
        assert!((utilization - 0.904762).abs() < 0.001); // Approximately 90.48%
    }
}
