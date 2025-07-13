//! Common types used across the API

use serde::{Deserialize, Serialize};

/// Data output format for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Json => write!(f, "json"),
            DataType::Csv => write!(f, "csv"),
        }
    }
}

/// Time interval for intraday data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Interval {
    Min1,
    Min5,
    Min15,
    Min30,
    Min60,
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Min1 => write!(f, "1min"),
            Interval::Min5 => write!(f, "5min"),
            Interval::Min15 => write!(f, "15min"),
            Interval::Min30 => write!(f, "30min"),
            Interval::Min60 => write!(f, "60min"),
        }
    }
}

impl Interval {
    /// Parse interval from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "1min" => Some(Interval::Min1),
            "5min" => Some(Interval::Min5),
            "15min" => Some(Interval::Min15),
            "30min" => Some(Interval::Min30),
            "60min" => Some(Interval::Min60),
            _ => None,
        }
    }

    /// Get interval duration in minutes
    pub fn minutes(&self) -> u32 {
        match self {
            Interval::Min1 => 1,
            Interval::Min5 => 5,
            Interval::Min15 => 15,
            Interval::Min30 => 30,
            Interval::Min60 => 60,
        }
    }
}

/// Output size for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputSize {
    /// Compact output (latest 100 data points)
    Compact,
    /// Full output (up to 20 years of data)
    Full,
}

impl std::fmt::Display for OutputSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputSize::Compact => write!(f, "compact"),
            OutputSize::Full => write!(f, "full"),
        }
    }
}

/// Sort order for API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    /// Latest first
    Latest,
    /// Earliest first
    Earliest,
    /// By relevance
    Relevance,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Latest => write!(f, "LATEST"),
            SortOrder::Earliest => write!(f, "EARLIEST"),
            SortOrder::Relevance => write!(f, "RELEVANCE"),
        }
    }
}

/// Time horizon for calendar data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeHorizon {
    /// 3 months
    ThreeMonth,
    /// 6 months
    SixMonth,
    /// 12 months
    TwelveMonth,
}

impl std::fmt::Display for TimeHorizon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeHorizon::ThreeMonth => write!(f, "3month"),
            TimeHorizon::SixMonth => write!(f, "6month"),
            TimeHorizon::TwelveMonth => write!(f, "12month"),
        }
    }
}

/// Listing state for securities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListingState {
    /// Currently listed securities
    Active,
    /// Delisted securities
    Delisted,
}

impl std::fmt::Display for ListingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingState::Active => write!(f, "active"),
            ListingState::Delisted => write!(f, "delisted"),
        }
    }
}

/// Sentiment label
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SentimentLabel {
    /// Bullish sentiment
    Bullish,
    /// Neutral sentiment
    Neutral,
    /// Bearish sentiment
    Bearish,
}

impl std::fmt::Display for SentimentLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SentimentLabel::Bullish => write!(f, "Bullish"),
            SentimentLabel::Neutral => write!(f, "Neutral"),
            SentimentLabel::Bearish => write!(f, "Bearish"),
        }
    }
}

impl SentimentLabel {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bullish" => Some(SentimentLabel::Bullish),
            "neutral" => Some(SentimentLabel::Neutral),
            "bearish" => Some(SentimentLabel::Bearish),
            _ => None,
        }
    }

    /// Get sentiment score range
    pub fn score_range(&self) -> (f64, f64) {
        match self {
            SentimentLabel::Bearish => (-1.0, -0.35),
            SentimentLabel::Neutral => (-0.35, 0.35),
            SentimentLabel::Bullish => (0.35, 1.0),
        }
    }
}

/// Currency codes commonly used in financial APIs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurrencyCode {
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    CAD,
    AUD,
    NZD,
    CNY,
    HKD,
    SGD,
    SEK,
    NOK,
    DKK,
    PLN,
    CZK,
    HUF,
    RUB,
    ZAR,
    BRL,
    MXN,
    INR,
    KRW,
    TRY,
    ILS,
    THB,
    MYR,
    PHP,
    IDR,
}

impl std::fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl CurrencyCode {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USD" => Some(CurrencyCode::USD),
            "EUR" => Some(CurrencyCode::EUR),
            "GBP" => Some(CurrencyCode::GBP),
            "JPY" => Some(CurrencyCode::JPY),
            "CHF" => Some(CurrencyCode::CHF),
            "CAD" => Some(CurrencyCode::CAD),
            "AUD" => Some(CurrencyCode::AUD),
            "NZD" => Some(CurrencyCode::NZD),
            "CNY" => Some(CurrencyCode::CNY),
            "HKD" => Some(CurrencyCode::HKD),
            "SGD" => Some(CurrencyCode::SGD),
            "SEK" => Some(CurrencyCode::SEK),
            "NOK" => Some(CurrencyCode::NOK),
            "DKK" => Some(CurrencyCode::DKK),
            "PLN" => Some(CurrencyCode::PLN),
            "CZK" => Some(CurrencyCode::CZK),
            "HUF" => Some(CurrencyCode::HUF),
            "RUB" => Some(CurrencyCode::RUB),
            "ZAR" => Some(CurrencyCode::ZAR),
            "BRL" => Some(CurrencyCode::BRL),
            "MXN" => Some(CurrencyCode::MXN),
            "INR" => Some(CurrencyCode::INR),
            "KRW" => Some(CurrencyCode::KRW),
            "TRY" => Some(CurrencyCode::TRY),
            "ILS" => Some(CurrencyCode::ILS),
            "THB" => Some(CurrencyCode::THB),
            "MYR" => Some(CurrencyCode::MYR),
            "PHP" => Some(CurrencyCode::PHP),
            "IDR" => Some(CurrencyCode::IDR),
            _ => None,
        }
    }

    /// Check if this is a major currency
    pub fn is_major(&self) -> bool {
        matches!(self,
            CurrencyCode::USD | CurrencyCode::EUR | CurrencyCode::GBP |
            CurrencyCode::JPY | CurrencyCode::CHF | CurrencyCode::CAD |
            CurrencyCode::AUD | CurrencyCode::NZD
        )
    }

    /// Get decimal places typically used for this currency
    pub fn decimal_places(&self) -> u8 {
        match self {
            CurrencyCode::JPY | CurrencyCode::KRW | CurrencyCode::HUF => 0,
            _ => 2,
        }
    }
}

/// Cryptocurrency symbols commonly used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoSymbol {
    BTC,
    ETH,
    BNB,
    ADA,
    SOL,
    XRP,
    DOT,
    DOGE,
    AVAX,
    MATIC,
    LINK,
    LTC,
    BCH,
    XLM,
    VET,
    ICP,
    FIL,
    TRX,
    ETC,
    XMR,
}

impl std::fmt::Display for CryptoSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl CryptoSymbol {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BTC" => Some(CryptoSymbol::BTC),
            "ETH" => Some(CryptoSymbol::ETH),
            "BNB" => Some(CryptoSymbol::BNB),
            "ADA" => Some(CryptoSymbol::ADA),
            "SOL" => Some(CryptoSymbol::SOL),
            "XRP" => Some(CryptoSymbol::XRP),
            "DOT" => Some(CryptoSymbol::DOT),
            "DOGE" => Some(CryptoSymbol::DOGE),
            "AVAX" => Some(CryptoSymbol::AVAX),
            "MATIC" => Some(CryptoSymbol::MATIC),
            "LINK" => Some(CryptoSymbol::LINK),
            "LTC" => Some(CryptoSymbol::LTC),
            "BCH" => Some(CryptoSymbol::BCH),
            "XLM" => Some(CryptoSymbol::XLM),
            "VET" => Some(CryptoSymbol::VET),
            "ICP" => Some(CryptoSymbol::ICP),
            "FIL" => Some(CryptoSymbol::FIL),
            "TRX" => Some(CryptoSymbol::TRX),
            "ETC" => Some(CryptoSymbol::ETC),
            "XMR" => Some(CryptoSymbol::XMR),
            _ => None,
        }
    }

    /// Check if this is a major cryptocurrency
    pub fn is_major(&self) -> bool {
        matches!(self,
            CryptoSymbol::BTC | CryptoSymbol::ETH | CryptoSymbol::BNB |
            CryptoSymbol::ADA | CryptoSymbol::SOL | CryptoSymbol::XRP |
            CryptoSymbol::DOT | CryptoSymbol::LINK
        )
    }

    /// Get the full name of the cryptocurrency
    pub fn full_name(&self) -> &'static str {
        match self {
            CryptoSymbol::BTC => "Bitcoin",
            CryptoSymbol::ETH => "Ethereum",
            CryptoSymbol::BNB => "Binance Coin",
            CryptoSymbol::ADA => "Cardano",
            CryptoSymbol::SOL => "Solana",
            CryptoSymbol::XRP => "XRP",
            CryptoSymbol::DOT => "Polkadot",
            CryptoSymbol::DOGE => "Dogecoin",
            CryptoSymbol::AVAX => "Avalanche",
            CryptoSymbol::MATIC => "Polygon",
            CryptoSymbol::LINK => "Chainlink",
            CryptoSymbol::LTC => "Litecoin",
            CryptoSymbol::BCH => "Bitcoin Cash",
            CryptoSymbol::XLM => "Stellar",
            CryptoSymbol::VET => "VeChain",
            CryptoSymbol::ICP => "Internet Computer",
            CryptoSymbol::FIL => "Filecoin",
            CryptoSymbol::TRX => "TRON",
            CryptoSymbol::ETC => "Ethereum Classic",
            CryptoSymbol::XMR => "Monero",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_parsing() {
        assert_eq!(Interval::from_str("5min"), Some(Interval::Min5));
        assert_eq!(Interval::from_str("invalid"), None);
        assert_eq!(Interval::Min15.minutes(), 15);
    }

    #[test]
    fn test_currency_code_parsing() {
        assert_eq!(CurrencyCode::from_str("USD"), Some(CurrencyCode::USD));
        assert_eq!(CurrencyCode::from_str("usd"), Some(CurrencyCode::USD));
        assert!(CurrencyCode::USD.is_major());
        assert_eq!(CurrencyCode::USD.decimal_places(), 2);
        assert_eq!(CurrencyCode::JPY.decimal_places(), 0);
    }

    #[test]
    fn test_crypto_symbol_parsing() {
        assert_eq!(CryptoSymbol::from_str("BTC"), Some(CryptoSymbol::BTC));
        assert_eq!(CryptoSymbol::from_str("btc"), Some(CryptoSymbol::BTC));
        assert!(CryptoSymbol::BTC.is_major());
        assert_eq!(CryptoSymbol::BTC.full_name(), "Bitcoin");
    }

    #[test]
    fn test_sentiment_label() {
        assert_eq!(SentimentLabel::from_str("Bullish"), Some(SentimentLabel::Bullish));
        assert_eq!(SentimentLabel::from_str("bullish"), Some(SentimentLabel::Bullish));

        let (min, max) = SentimentLabel::Bullish.score_range();
        assert_eq!(min, 0.35);
        assert_eq!(max, 1.0);
    }
}
