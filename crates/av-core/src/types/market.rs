//! Market-related types

use serde::{Deserialize, Serialize};

/// Stock exchange identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    /// New York Stock Exchange
    NYSE,
    /// NASDAQ
    NASDAQ,
    /// American Stock Exchange
    AMEX,
    /// Chicago Board of Trade
    CBOT,
    /// Chicago Mercantile Exchange
    CME,
    /// London Stock Exchange
    LSE,
    /// Toronto Stock Exchange
    TSX,
    /// Tokyo Stock Exchange
    TSE,
    /// Hong Kong Stock Exchange
    HKSE,
    /// Shanghai Stock Exchange
    SSE,
    /// Shenzhen Stock Exchange
    SZSE,
    /// Euronext
    EURONEXT,
    /// Frankfurt Stock Exchange
    FRA,
    /// Swiss Exchange
    SIX,
    /// Australian Securities Exchange
    ASX,
    /// Bombay Stock Exchange
    BSE,
    /// National Stock Exchange of India
    NSE,
    /// São Paulo Stock Exchange
    BOVESPA,
    /// Moscow Exchange
    MOEX,
    /// Korea Exchange
    KRX,
    /// Taiwan Stock Exchange
    TWSE,
    /// Singapore Exchange
    SGX,
    /// Johannesburg Stock Exchange
    JSE,
    /// Tel Aviv Stock Exchange
    TASE,
    /// Other/Unknown exchange
    OTHER,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::NYSE => write!(f, "NYSE"),
            Exchange::NASDAQ => write!(f, "NASDAQ"),
            Exchange::AMEX => write!(f, "AMEX"),
            Exchange::CBOT => write!(f, "CBOT"),
            Exchange::CME => write!(f, "CME"),
            Exchange::LSE => write!(f, "LSE"),
            Exchange::TSX => write!(f, "TSX"),
            Exchange::TSE => write!(f, "TSE"),
            Exchange::HKSE => write!(f, "HKSE"),
            Exchange::SSE => write!(f, "SSE"),
            Exchange::SZSE => write!(f, "SZSE"),
            Exchange::EURONEXT => write!(f, "EURONEXT"),
            Exchange::FRA => write!(f, "FRA"),
            Exchange::SIX => write!(f, "SIX"),
            Exchange::ASX => write!(f, "ASX"),
            Exchange::BSE => write!(f, "BSE"),
            Exchange::NSE => write!(f, "NSE"),
            Exchange::BOVESPA => write!(f, "BOVESPA"),
            Exchange::MOEX => write!(f, "MOEX"),
            Exchange::KRX => write!(f, "KRX"),
            Exchange::TWSE => write!(f, "TWSE"),
            Exchange::SGX => write!(f, "SGX"),
            Exchange::JSE => write!(f, "JSE"),
            Exchange::TASE => write!(f, "TASE"),
            Exchange::OTHER => write!(f, "OTHER"),
        }
    }
}

impl Exchange {
    /// Parse exchange from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NYSE" | "NEW YORK STOCK EXCHANGE" => Some(Exchange::NYSE),
            "NASDAQ" => Some(Exchange::NASDAQ),
            "AMEX" | "AMERICAN STOCK EXCHANGE" => Some(Exchange::AMEX),
            "CBOT" => Some(Exchange::CBOT),
            "CME" => Some(Exchange::CME),
            "LSE" | "LONDON STOCK EXCHANGE" => Some(Exchange::LSE),
            "TSX" | "TORONTO STOCK EXCHANGE" => Some(Exchange::TSX),
            "TSE" | "TOKYO STOCK EXCHANGE" => Some(Exchange::TSE),
            "HKSE" | "HONG KONG STOCK EXCHANGE" => Some(Exchange::HKSE),
            "SSE" | "SHANGHAI STOCK EXCHANGE" => Some(Exchange::SSE),
            "SZSE" | "SHENZHEN STOCK EXCHANGE" => Some(Exchange::SZSE),
            "EURONEXT" => Some(Exchange::EURONEXT),
            "FRA" | "FRANKFURT STOCK EXCHANGE" => Some(Exchange::FRA),
            "SIX" | "SWISS EXCHANGE" => Some(Exchange::SIX),
            "ASX" | "AUSTRALIAN SECURITIES EXCHANGE" => Some(Exchange::ASX),
            "BSE" | "BOMBAY STOCK EXCHANGE" => Some(Exchange::BSE),
            "NSE" | "NATIONAL STOCK EXCHANGE OF INDIA" => Some(Exchange::NSE),
            "BOVESPA" => Some(Exchange::BOVESPA),
            "MOEX" | "MOSCOW EXCHANGE" => Some(Exchange::MOEX),
            "KRX" | "KOREA EXCHANGE" => Some(Exchange::KRX),
            "TWSE" | "TAIWAN STOCK EXCHANGE" => Some(Exchange::TWSE),
            "SGX" | "SINGAPORE EXCHANGE" => Some(Exchange::SGX),
            "JSE" | "JOHANNESBURG STOCK EXCHANGE" => Some(Exchange::JSE),
            "TASE" | "TEL AVIV STOCK EXCHANGE" => Some(Exchange::TASE),
            _ => Some(Exchange::OTHER),
        }
    }

    /// Get the full name of the exchange
    pub fn full_name(&self) -> &'static str {
        match self {
            Exchange::NYSE => "New York Stock Exchange",
            Exchange::NASDAQ => "NASDAQ",
            Exchange::AMEX => "American Stock Exchange",
            Exchange::CBOT => "Chicago Board of Trade",
            Exchange::CME => "Chicago Mercantile Exchange",
            Exchange::LSE => "London Stock Exchange",
            Exchange::TSX => "Toronto Stock Exchange",
            Exchange::TSE => "Tokyo Stock Exchange",
            Exchange::HKSE => "Hong Kong Stock Exchange",
            Exchange::SSE => "Shanghai Stock Exchange",
            Exchange::SZSE => "Shenzhen Stock Exchange",
            Exchange::EURONEXT => "Euronext",
            Exchange::FRA => "Frankfurt Stock Exchange",
            Exchange::SIX => "Swiss Exchange",
            Exchange::ASX => "Australian Securities Exchange",
            Exchange::BSE => "Bombay Stock Exchange",
            Exchange::NSE => "National Stock Exchange of India",
            Exchange::BOVESPA => "São Paulo Stock Exchange",
            Exchange::MOEX => "Moscow Exchange",
            Exchange::KRX => "Korea Exchange",
            Exchange::TWSE => "Taiwan Stock Exchange",
            Exchange::SGX => "Singapore Exchange",
            Exchange::JSE => "Johannesburg Stock Exchange",
            Exchange::TASE => "Tel Aviv Stock Exchange",
            Exchange::OTHER => "Other Exchange",
        }
    }

    /// Get the timezone for the exchange
    pub fn timezone(&self) -> &'static str {
        match self {
            Exchange::NYSE | Exchange::NASDAQ | Exchange::AMEX => "America/New_York",
            Exchange::CBOT | Exchange::CME => "America/Chicago",
            Exchange::LSE => "Europe/London",
            Exchange::TSX => "America/Toronto",
            Exchange::TSE => "Asia/Tokyo",
            Exchange::HKSE => "Asia/Hong_Kong",
            Exchange::SSE | Exchange::SZSE => "Asia/Shanghai",
            Exchange::EURONEXT => "Europe/Paris",
            Exchange::FRA => "Europe/Berlin",
            Exchange::SIX => "Europe/Zurich",
            Exchange::ASX => "Australia/Sydney",
            Exchange::BSE | Exchange::NSE => "Asia/Kolkata",
            Exchange::BOVESPA => "America/Sao_Paulo",
            Exchange::MOEX => "Europe/Moscow",
            Exchange::KRX => "Asia/Seoul",
            Exchange::TWSE => "Asia/Taipei",
            Exchange::SGX => "Asia/Singapore",
            Exchange::JSE => "Africa/Johannesburg",
            Exchange::TASE => "Asia/Jerusalem",
            Exchange::OTHER => "UTC",
        }
    }

    /// Get the primary currency for the exchange
    pub fn primary_currency(&self) -> &'static str {
        match self {
            Exchange::NYSE | Exchange::NASDAQ | Exchange::AMEX |
            Exchange::CBOT | Exchange::CME => "USD",
            Exchange::LSE => "GBP",
            Exchange::TSX => "CAD",
            Exchange::TSE => "JPY",
            Exchange::HKSE => "HKD",
            Exchange::SSE | Exchange::SZSE => "CNY",
            Exchange::EURONEXT => "EUR",
            Exchange::FRA => "EUR",
            Exchange::SIX => "CHF",
            Exchange::ASX => "AUD",
            Exchange::BSE | Exchange::NSE => "INR",
            Exchange::BOVESPA => "BRL",
            Exchange::MOEX => "RUB",
            Exchange::KRX => "KRW",
            Exchange::TWSE => "TWD",
            Exchange::SGX => "SGD",
            Exchange::JSE => "ZAR",
            Exchange::TASE => "ILS",
            Exchange::OTHER => "USD",
        }
    }

    /// Check if this is a major global exchange
    pub fn is_major(&self) -> bool {
        matches!(self,
            Exchange::NYSE | Exchange::NASDAQ | Exchange::LSE |
            Exchange::TSE | Exchange::HKSE | Exchange::EURONEXT |
            Exchange::SSE | Exchange::FRA
        )
    }
}

/// Type of security
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
    CommonStock,
    PreferredStock,
    ETF,
    MutualFund,
    REIT,
    ADR,
    CD,
    Bond,
    GovernmentBond,
    CorporateBond,
    MunicipalBond,
    TreasuryBill,
    Option,
    Future,
    Warrant,
    Index,
    Currency,
    Commodity,
    Cryptocurrency,
    Other,
}

impl std::fmt::Display for SecurityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityType::CommonStock => write!(f, "Common Stock"),
            SecurityType::PreferredStock => write!(f, "Preferred Stock"),
            SecurityType::ETF => write!(f, "ETF"),
            SecurityType::MutualFund => write!(f, "Mutual Fund"),
            SecurityType::REIT => write!(f, "REIT"),
            SecurityType::ADR => write!(f, "ADR"),
            SecurityType::CD => write!(f, "Certificate of Deposit"),
            SecurityType::Bond => write!(f, "Bond"),
            SecurityType::GovernmentBond => write!(f, "Government Bond"),
            SecurityType::CorporateBond => write!(f, "Corporate Bond"),
            SecurityType::MunicipalBond => write!(f, "Municipal Bond"),
            SecurityType::TreasuryBill => write!(f, "Treasury Bill"),
            SecurityType::Option => write!(f, "Option"),
            SecurityType::Future => write!(f, "Future"),
            SecurityType::Warrant => write!(f, "Warrant"),
            SecurityType::Index => write!(f, "Index"),
            SecurityType::Currency => write!(f, "Currency"),
            SecurityType::Commodity => write!(f, "Commodity"),
            SecurityType::Cryptocurrency => write!(f, "Cryptocurrency"),
            SecurityType::Other => write!(f, "Other"),
        }
    }
}

impl SecurityType {
    /// Parse security type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
            "COMMONSTOCK" | "EQUITY" | "STOCK" => Some(SecurityType::CommonStock),
            "PREFERREDSTOCK" | "PREFERRED" => Some(SecurityType::PreferredStock),
            "ETF" | "EXCHANGETRADEDFUND" => Some(SecurityType::ETF),
            "MUTUALFUND" | "FUND" => Some(SecurityType::MutualFund),
            "REIT" | "REALESTATEINVESTMENTTRUST" => Some(SecurityType::REIT),
            "ADR" | "AMERICANDEPOSITARYRECEIPT" => Some(SecurityType::ADR),
            "CD" | "CERTIFICATEOFDEPOSIT" => Some(SecurityType::CD),
            "BOND" => Some(SecurityType::Bond),
            "GOVERNMENTBOND" | "GOVBOND" => Some(SecurityType::GovernmentBond),
            "CORPORATEBOND" | "CORPBOND" => Some(SecurityType::CorporateBond),
            "MUNICIPALBOND" | "MUNIBOND" => Some(SecurityType::MunicipalBond),
            "TREASURYBILL" | "TBILL" => Some(SecurityType::TreasuryBill),
            "OPTION" => Some(SecurityType::Option),
            "FUTURE" | "FUTURES" => Some(SecurityType::Future),
            "WARRANT" => Some(SecurityType::Warrant),
            "INDEX" => Some(SecurityType::Index),
            "CURRENCY" | "FX" | "FOREX" => Some(SecurityType::Currency),
            "COMMODITY" => Some(SecurityType::Commodity),
            "CRYPTOCURRENCY" | "CRYPTO" => Some(SecurityType::Cryptocurrency),
            _ => Some(SecurityType::Other),
        }
    }

    /// Check if this security type represents equity
    pub fn is_equity(&self) -> bool {
        matches!(self,
            SecurityType::CommonStock | SecurityType::PreferredStock |
            SecurityType::ETF | SecurityType::REIT | SecurityType::ADR
        )
    }

    /// Check if this security type represents fixed income
    pub fn is_fixed_income(&self) -> bool {
        matches!(self,
            SecurityType::Bond | SecurityType::GovernmentBond |
            SecurityType::CorporateBond | SecurityType::MunicipalBond |
            SecurityType::TreasuryBill | SecurityType::CD
        )
    }

    /// Check if this security type represents derivatives
    pub fn is_derivative(&self) -> bool {
        matches!(self,
            SecurityType::Option | SecurityType::Future | SecurityType::Warrant
        )
    }

    /// Get the typical settlement period in days
    pub fn settlement_days(&self) -> u8 {
        match self {
            SecurityType::CommonStock | SecurityType::PreferredStock |
            SecurityType::ETF | SecurityType::REIT | SecurityType::ADR => 2, // T+2
            SecurityType::MutualFund => 1, // T+1
            SecurityType::Bond | SecurityType::GovernmentBond |
            SecurityType::CorporateBond | SecurityType::MunicipalBond => 1, // T+1
            SecurityType::TreasuryBill => 1, // T+1
            SecurityType::Option => 1, // T+1
            SecurityType::Future => 0, // Daily mark-to-market
            SecurityType::Currency => 2, // T+2
            SecurityType::Commodity => 0, // Immediate
            SecurityType::Cryptocurrency => 0, // Immediate
            _ => 2, // Default T+2
        }
    }
  }


/// Top movers type (gainers, losers, most active)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TopType {
    /// Top gaining stocks
    Gainers,
    /// Top losing stocks
    Losers,
    /// Most actively traded stocks
    MostActive,
}

impl std::fmt::Display for TopType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopType::Gainers => write!(f, "gainers"),
            TopType::Losers => write!(f, "losers"),
            TopType::MostActive => write!(f, "most_active"),
        }
    }
}

impl TopType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace([' ', '-', '_'], "").as_str() {
            "gainers" | "topgainers" | "winners" => Some(TopType::Gainers),
            "losers" | "toplosers" | "decliners" => Some(TopType::Losers),
            "mostactive" | "active" | "volume" => Some(TopType::MostActive),
            _ => None,
        }
    }
}

/// Market sector classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sector {
    /// Technology sector
    Technology,
    /// Healthcare sector
    Healthcare,
    /// Financial Services
    FinancialServices,
    /// Consumer Discretionary
    ConsumerDiscretionary,
    /// Consumer Staples
    ConsumerStaples,
    /// Industrials
    Industrials,
    /// Energy
    Energy,
    /// Materials
    Materials,
    /// Real Estate
    RealEstate,
    /// Utilities
    Utilities,
    /// Communication Services
    CommunicationServices,
    /// Other/Unknown
    Other,
}

impl std::fmt::Display for Sector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sector::Technology => write!(f, "Technology"),
            Sector::Healthcare => write!(f, "Healthcare"),
            Sector::FinancialServices => write!(f, "Financial Services"),
            Sector::ConsumerDiscretionary => write!(f, "Consumer Discretionary"),
            Sector::ConsumerStaples => write!(f, "Consumer Staples"),
            Sector::Industrials => write!(f, "Industrials"),
            Sector::Energy => write!(f, "Energy"),
            Sector::Materials => write!(f, "Materials"),
            Sector::RealEstate => write!(f, "Real Estate"),
            Sector::Utilities => write!(f, "Utilities"),
            Sector::CommunicationServices => write!(f, "Communication Services"),
            Sector::Other => write!(f, "Other"),
        }
    }
}

impl Sector {
    /// Parse sector from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().replace([' ', '-', '_'], "").as_str() {
            "TECHNOLOGY" | "TECH" | "IT" | "INFORMATIONTECHNOLOGY" => Some(Sector::Technology),
            "HEALTHCARE" | "HEALTH" | "MEDICAL" | "PHARMA" | "PHARMACEUTICAL" => Some(Sector::Healthcare),
            "FINANCIALSERVICES" | "FINANCIAL" | "FINANCE" | "BANKING" | "FINTECH" => Some(Sector::FinancialServices),
            "CONSUMERDISCRETIONARY" | "CONSUMER" | "RETAIL" | "DISCRETIONARY" => Some(Sector::ConsumerDiscretionary),
            "CONSUMERSTAPLES" | "STAPLES" | "DEFENSIVE" => Some(Sector::ConsumerStaples),
            "INDUSTRIALS" | "INDUSTRIAL" | "MANUFACTURING" => Some(Sector::Industrials),
            "ENERGY" | "OIL" | "GAS" | "PETROLEUM" => Some(Sector::Energy),
            "MATERIALS" | "BASIC" | "BASICMATERIALS" | "MINING" => Some(Sector::Materials),
            "REALESTATE" | "PROPERTY" | "REIT" => Some(Sector::RealEstate),
            "UTILITIES" | "UTILITY" | "POWER" | "ELECTRIC" => Some(Sector::Utilities),
            "COMMUNICATIONSERVICES" | "COMMUNICATION" | "TELECOM" | "MEDIA" => Some(Sector::CommunicationServices),
            _ => Some(Sector::Other),
        }
    }

    /// Check if this is a cyclical sector
    pub fn is_cyclical(&self) -> bool {
        matches!(self,
            Sector::Technology | Sector::ConsumerDiscretionary |
            Sector::Industrials | Sector::Energy | Sector::Materials |
            Sector::FinancialServices
        )
    }

    /// Check if this is a defensive sector
    pub fn is_defensive(&self) -> bool {
        matches!(self,
            Sector::Healthcare | Sector::ConsumerStaples | Sector::Utilities
        )
    }

    /// Get typical P/E ratio range for the sector
    pub fn typical_pe_range(&self) -> (f64, f64) {
        match self {
            Sector::Technology => (15.0, 35.0),
            Sector::Healthcare => (12.0, 25.0),
            Sector::FinancialServices => (8.0, 15.0),
            Sector::ConsumerDiscretionary => (12.0, 25.0),
            Sector::ConsumerStaples => (15.0, 25.0),
            Sector::Industrials => (12.0, 20.0),
            Sector::Energy => (8.0, 15.0),
            Sector::Materials => (10.0, 18.0),
            Sector::RealEstate => (15.0, 30.0),
            Sector::Utilities => (12.0, 20.0),
            Sector::CommunicationServices => (10.0, 25.0),
            Sector::Other => (10.0, 25.0),
        }
    }
}

/// Market cap classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketCap {
    /// Nano cap (< $50M)
    NanoCap,
    /// Micro cap ($50M - $300M)
    MicroCap,
    /// Small cap ($300M - $2B)
    SmallCap,
    /// Mid cap ($2B - $10B)
    MidCap,
    /// Large cap ($10B - $200B)
    LargeCap,
    /// Mega cap (> $200B)
    MegaCap,
}

impl std::fmt::Display for MarketCap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketCap::NanoCap => write!(f, "Nano Cap"),
            MarketCap::MicroCap => write!(f, "Micro Cap"),
            MarketCap::SmallCap => write!(f, "Small Cap"),
            MarketCap::MidCap => write!(f, "Mid Cap"),
            MarketCap::LargeCap => write!(f, "Large Cap"),
            MarketCap::MegaCap => write!(f, "Mega Cap"),
        }
    }
}

impl MarketCap {
    /// Classify market cap from value in USD
    pub fn from_value(market_cap_usd: f64) -> Self {
        if market_cap_usd < 50_000_000.0 {
            MarketCap::NanoCap
        } else if market_cap_usd < 300_000_000.0 {
            MarketCap::MicroCap
        } else if market_cap_usd < 2_000_000_000.0 {
            MarketCap::SmallCap
        } else if market_cap_usd < 10_000_000_000.0 {
            MarketCap::MidCap
        } else if market_cap_usd < 200_000_000_000.0 {
            MarketCap::LargeCap
        } else {
            MarketCap::MegaCap
        }
    }

    /// Get the range for this market cap category
    pub fn range(&self) -> (f64, Option<f64>) {
        match self {
            MarketCap::NanoCap => (0.0, Some(50_000_000.0)),
            MarketCap::MicroCap => (50_000_000.0, Some(300_000_000.0)),
            MarketCap::SmallCap => (300_000_000.0, Some(2_000_000_000.0)),
            MarketCap::MidCap => (2_000_000_000.0, Some(10_000_000_000.0)),
            MarketCap::LargeCap => (10_000_000_000.0, Some(200_000_000_000.0)),
            MarketCap::MegaCap => (200_000_000_000.0, None),
        }
    }

    /// Check if this is considered a large company
    pub fn is_large(&self) -> bool {
        matches!(self, MarketCap::LargeCap | MarketCap::MegaCap)
    }

    /// Check if this is considered a small company
    pub fn is_small(&self) -> bool {
        matches!(self, MarketCap::NanoCap | MarketCap::MicroCap | MarketCap::SmallCap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_parsing() {
        assert_eq!(Exchange::from_str("NYSE"), Some(Exchange::NYSE));
        assert_eq!(Exchange::from_str("nasdaq"), Some(Exchange::NASDAQ));
        assert_eq!(Exchange::NYSE.full_name(), "New York Stock Exchange");
        assert_eq!(Exchange::NYSE.timezone(), "America/New_York");
        assert_eq!(Exchange::NYSE.primary_currency(), "USD");
        assert!(Exchange::NYSE.is_major());
    }

    #[test]
    fn test_security_type_parsing() {
        assert_eq!(SecurityType::from_str("Common Stock"), Some(SecurityType::CommonStock));
        assert_eq!(SecurityType::from_str("ETF"), Some(SecurityType::ETF));
        assert!(SecurityType::CommonStock.is_equity());
        assert!(SecurityType::Bond.is_fixed_income());
        assert!(SecurityType::Option.is_derivative());
        assert_eq!(SecurityType::CommonStock.settlement_days(), 2);
    }

    #[test]
    fn test_sector_parsing() {
        assert_eq!(Sector::from_str("Technology"), Some(Sector::Technology));
        assert_eq!(Sector::from_str("TECH"), Some(Sector::Technology));
        assert!(Sector::Technology.is_cyclical());
        assert!(Sector::Healthcare.is_defensive());

        let (min, max) = Sector::Technology.typical_pe_range();
        assert_eq!(min, 15.0);
        assert_eq!(max, 35.0);
    }

    #[test]
    fn test_market_cap_classification() {
        assert_eq!(MarketCap::from_value(100_000_000.0), MarketCap::MicroCap);
        assert_eq!(MarketCap::from_value(5_000_000_000.0), MarketCap::MidCap);
        assert_eq!(MarketCap::from_value(50_000_000_000.0), MarketCap::LargeCap);
        assert_eq!(MarketCap::from_value(500_000_000_000.0), MarketCap::MegaCap);

        assert!(MarketCap::LargeCap.is_large());
        assert!(MarketCap::SmallCap.is_small());

        let (min, max) = MarketCap::MidCap.range();
        assert_eq!(min, 2_000_000_000.0);
        assert_eq!(max, Some(10_000_000_000.0));
    }

    #[test]
    fn test_top_type_parsing() {
        assert_eq!(TopType::from_str("gainers"), Some(TopType::Gainers));
        assert_eq!(TopType::from_str("most_active"), Some(TopType::MostActive));
        assert_eq!(TopType::from_str("invalid"), None);
    }
}
