//! Foreign exchange (forex) data models

use crate::common::{OhlcData, TimeSeriesData};
use serde::{Deserialize, Serialize};

/// Real-time exchange rate response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// Realtime currency exchange rate data
    #[serde(rename = "Realtime Currency Exchange Rate")]
    pub realtime_currency_exchange_rate: ExchangeRateData,
}

/// Exchange rate data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateData {
    /// From currency code
    #[serde(rename = "1. From_Currency Code")]
    pub from_currency_code: String,
    
    /// From currency name
    #[serde(rename = "2. From_Currency Name")]
    pub from_currency_name: String,
    
    /// To currency code
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

/// Forex intraday time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxIntraday {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: FxMetadata,
    
    /// Time series data
    #[serde(flatten)]
    pub time_series: TimeSeriesData<OhlcData>,
}

/// Forex daily time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxDaily {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: FxMetadata,
    
    /// Time series data
    #[serde(rename = "Time Series FX (Daily)")]
    pub time_series: TimeSeriesData<OhlcData>,
}

/// Forex weekly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxWeekly {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: FxMetadata,
    
    /// Time series data
    #[serde(rename = "Time Series FX (Weekly)")]
    pub time_series: TimeSeriesData<OhlcData>,
}

/// Forex monthly time series
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxMonthly {
    /// Metadata
    #[serde(rename = "Meta Data")]
    pub meta_data: FxMetadata,
    
    /// Time series data
    #[serde(rename = "Time Series FX (Monthly)")]
    pub time_series: TimeSeriesData<OhlcData>,
}

/// Forex metadata structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxMetadata {
    /// Information
    #[serde(rename = "1. Information")]
    pub information: String,
    
    /// From symbol
    #[serde(rename = "2. From Symbol")]
    pub from_symbol: String,
    
    /// To symbol
    #[serde(rename = "3. To Symbol")]
    pub to_symbol: String,
    
    /// Last refreshed
    #[serde(rename = "4. Last Refreshed")]
    pub last_refreshed: String,
    
    /// Interval (for intraday data)
    #[serde(rename = "5. Interval", skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    
    /// Output size (for intraday data)
    #[serde(rename = "6. Output Size", skip_serializing_if = "Option::is_none")]
    pub output_size: Option<String>,
    
    /// Time zone
    #[serde(rename = "7. Time Zone", skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

/// Currency pair information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyPair {
    /// Base currency
    pub base_currency: String,
    
    /// Quote currency
    pub quote_currency: String,
    
    /// Currency pair symbol (e.g., "EURUSD")
    pub symbol: String,
    
    /// Display name
    pub display_name: String,
    
    /// Decimal places for pricing
    pub decimal_places: u8,
}

/// Forex market session information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForexSession {
    /// Session name (e.g., "London", "New York", "Tokyo", "Sydney")
    pub name: String,
    
    /// Session timezone
    pub timezone: String,
    
    /// Session open time (UTC)
    pub open_time_utc: String,
    
    /// Session close time (UTC)
    pub close_time_utc: String,
    
    /// Whether session is currently active
    pub is_active: bool,
    
    /// Major currency pairs traded in this session
    pub major_pairs: Vec<String>,
}

/// Cross-currency rate calculation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRate {
    /// Base currency
    pub base_currency: String,
    
    /// Quote currency
    pub quote_currency: String,
    
    /// Intermediate currency (if used for calculation)
    pub intermediate_currency: Option<String>,
    
    /// Calculated rate
    pub rate: f64,
    
    /// Calculation method
    pub calculation_method: String,
    
    /// Timestamp of calculation
    pub calculated_at: String,
}

/// Volatility information for currency pair
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyVolatility {
    /// Currency pair
    pub currency_pair: String,
    
    /// Time period for calculation
    pub period: String,
    
    /// Historical volatility (annualized)
    pub historical_volatility: f64,
    
    /// Implied volatility (if available)
    pub implied_volatility: Option<f64>,
    
    /// Average true range
    pub average_true_range: f64,
    
    /// Calculation date
    pub calculated_on: String,
}

/// Economic indicator impact on currency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomicImpact {
    /// Currency affected
    pub currency: String,
    
    /// Economic indicator name
    pub indicator_name: String,
    
    /// Release date/time
    pub release_time: String,
    
    /// Expected value
    pub expected_value: Option<String>,
    
    /// Actual value
    pub actual_value: Option<String>,
    
    /// Previous value
    pub previous_value: Option<String>,
    
    /// Impact level (High/Medium/Low)
    pub impact_level: String,
    
    /// Currency reaction direction
    pub currency_reaction: Option<String>,
}

impl ExchangeRate {
    /// Get the actual exchange rate data
    pub fn rate(&self) -> &ExchangeRateData {
        &self.realtime_currency_exchange_rate
    }
}

impl ExchangeRateData {
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
    
    /// Calculate spread as percentage of mid price
    pub fn spread_percentage(&self) -> Result<f64, std::num::ParseFloatError> {
        let bid = self.bid_as_f64()?;
        let ask = self.ask_as_f64()?;
        let spread = ask - bid;
        let mid = (bid + ask) / 2.0;
        
        if mid == 0.0 {
            Ok(0.0)
        } else {
            Ok((spread / mid) * 100.0)
        }
    }
    
    /// Get currency pair symbol
    pub fn pair_symbol(&self) -> String {
        format!("{}{}", self.from_currency_code, self.to_currency_code)
    }
}

impl FxIntraday {
    /// Get the latest data point
    pub fn latest(&self) -> Option<(&String, &OhlcData)> {
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
    
    /// Calculate volatility over the time period
    pub fn calculate_volatility(&self) -> Result<f64, std::num::ParseFloatError> {
        let closes: Result<Vec<f64>, _> = self.time_series
            .values()
            .map(|data| data.close.parse::<f64>())
            .collect();
        
        let closes = closes?;
        if closes.len() < 2 {
            return Ok(0.0);
        }
        
        // Calculate returns
        let mut returns = Vec::new();
        for i in 1..closes.len() {
            let return_rate = (closes[i-1] / closes[i] - 1.0) * 100.0;
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

impl FxDaily {
    /// Get the latest data point
    pub fn latest(&self) -> Option<(&String, &OhlcData)> {
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
    
    /// Calculate simple moving average
    pub fn simple_moving_average(&self, periods: usize) -> Result<Vec<(String, f64)>, std::num::ParseFloatError> {
        let mut result = Vec::new();
        let data_points: Vec<_> = self.time_series.iter().collect();
        
        if data_points.len() < periods {
            return Ok(result);
        }
        
        for i in (periods-1)..data_points.len() {
            let window = &data_points[i-(periods-1)..=i];
            let sum: Result<f64, _> = window.iter()
                .map(|(_, data)| data.close.parse::<f64>())
                .sum();
            
            let average = sum? / periods as f64;
            result.push((data_points[i].0.clone(), average));
        }
        
        Ok(result)
    }
}

impl CurrencyPair {
    /// Create a new currency pair
    pub fn new(base: &str, quote: &str) -> Self {
        let symbol = format!("{}{}", base, quote);
        let display_name = format!("{}/{}", base, quote);
        
        Self {
            base_currency: base.to_uppercase(),
            quote_currency: quote.to_uppercase(),
            symbol,
            display_name,
            decimal_places: if base == "JPY" || quote == "JPY" { 3 } else { 5 },
        }
    }
    
    /// Check if this is a major currency pair
    pub fn is_major(&self) -> bool {
        let majors = [
            "EURUSD", "USDJPY", "GBPUSD", "USDCHF",
            "AUDUSD", "USDCAD", "NZDUSD"
        ];
        majors.contains(&self.symbol.as_str())
    }
    
    /// Check if this is a cross currency pair (no USD)
    pub fn is_cross(&self) -> bool {
        self.base_currency != "USD" && self.quote_currency != "USD"
    }
    
    /// Get the inverse pair
    pub fn inverse(&self) -> Self {
        CurrencyPair::new(&self.quote_currency, &self.base_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_exchange_rate_calculations() {
        let rate_data = ExchangeRateData {
            from_currency_code: "EUR".to_string(),
            from_currency_name: "Euro".to_string(),
            to_currency_code: "USD".to_string(),
            to_currency_name: "United States Dollar".to_string(),
            exchange_rate: "1.0850".to_string(),
            last_refreshed: "2024-01-15 16:00:00".to_string(),
            time_zone: "UTC".to_string(),
            bid_price: "1.0849".to_string(),
            ask_price: "1.0851".to_string(),
        };
        
        assert_eq!(rate_data.rate_as_f64().unwrap(), 1.0850);
        assert_eq!(rate_data.bid_as_f64().unwrap(), 1.0849);
        assert_eq!(rate_data.ask_as_f64().unwrap(), 1.0851);
        assert_eq!(rate_data.spread().unwrap(), 0.0002);
        assert_eq!(rate_data.pair_symbol(), "EURUSD");
        
        let spread_pct = rate_data.spread_percentage().unwrap();
        assert!((spread_pct - 0.0184).abs() < 0.001); // Approximately 0.0184%
    }
    
    #[test]
    fn test_currency_pair() {
        let pair = CurrencyPair::new("EUR", "USD");
        
        assert_eq!(pair.base_currency, "EUR");
        assert_eq!(pair.quote_currency, "USD");
        assert_eq!(pair.symbol, "EURUSD");
        assert_eq!(pair.display_name, "EUR/USD");
        assert_eq!(pair.decimal_places, 5);
        assert!(pair.is_major());
        assert!(!pair.is_cross());
        
        let inverse = pair.inverse();
        assert_eq!(inverse.symbol, "USDEUR");
    }
    
    #[test]
    fn test_jpy_pair_decimals() {
        let pair = CurrencyPair::new("USD", "JPY");
        assert_eq!(pair.decimal_places, 3);
    }
}
