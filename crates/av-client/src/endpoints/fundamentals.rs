//! Fundamental data endpoints for company analysis
//!
//! This module provides access to AlphaVantage's fundamental data including:
//! - Company overviews with key metrics
//! - Income statements
//! - Balance sheets  
//! - Cash flow statements
//! - Earnings data
//! - Top gainers/losers/most active stocks

use super::{impl_endpoint_base, EndpointBase};
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::fundamentals::*;
use governor::RateLimiter;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Fundamental data endpoints for company financial information
pub struct FundamentalsEndpoints {
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
}

impl FundamentalsEndpoints {
    /// Create a new fundamentals endpoints instance
    pub fn new(
        transport: Arc<Transport>,
        rate_limiter: Arc<RateLimiter<governor::clock::DefaultClock, governor::state::InMemoryState>>,
    ) -> Self {
        Self { transport, rate_limiter }
    }

    /// Get company overview with key financial metrics
    ///
    /// Returns comprehensive company information including market cap, P/E ratio,
    /// dividend information, analyst targets, and other key metrics.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol (e.g., "AAPL", "MSFT")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let overview = endpoints.company_overview("AAPL").await?;
    /// println!("Market Cap: {}", overview.market_capitalization);
    /// println!("P/E Ratio: {}", overview.pe_ratio);
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn company_overview(&self, symbol: &str) -> Result<CompanyOverview> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());

        self.transport.get(FuncType::Overview, params).await
    }

    /// Get annual and quarterly income statements
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let income_statement = endpoints.income_statement("AAPL").await?;
    /// for report in &income_statement.annual_reports {
    ///     println!("Year: {}, Revenue: {}", report.fiscal_date_ending, report.total_revenue);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn income_statement(&self, symbol: &str) -> Result<IncomeStatement> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());

        self.transport.get(FuncType::IncomeStatement, params).await
    }

    /// Get annual and quarterly balance sheets
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let balance_sheet = endpoints.balance_sheet("AAPL").await?;
    /// for report in &balance_sheet.annual_reports {
    ///     println!("Year: {}, Total Assets: {}", report.fiscal_date_ending, report.total_assets);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn balance_sheet(&self, symbol: &str) -> Result<BalanceSheet> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());

        self.transport.get(FuncType::BalanceSheet, params).await
    }

    /// Get annual and quarterly cash flow statements
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let cash_flow = endpoints.cash_flow("AAPL").await?;
    /// for report in &cash_flow.annual_reports {
    ///     println!("Year: {}, Operating Cash Flow: {}", 
    ///              report.fiscal_date_ending, report.operating_cashflow);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn cash_flow(&self, symbol: &str) -> Result<CashFlow> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());

        self.transport.get(FuncType::CashFlow, params).await
    }

    /// Get annual and quarterly earnings data
    ///
    /// # Arguments
    ///
    /// * `symbol` - The stock symbol
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let earnings = endpoints.earnings("AAPL").await?;
    /// for report in &earnings.annual_earnings {
    ///     println!("Year: {}, EPS: {}", report.fiscal_date_ending, report.reported_eps);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol))]
    pub async fn earnings(&self, symbol: &str) -> Result<Earnings> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());

        self.transport.get(FuncType::Earnings, params).await
    }

    /// Get top gainers, losers, and most actively traded stocks in the US market
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let top_movers = endpoints.top_gainers_losers().await?;
    /// 
    /// println!("Top Gainers:");
    /// for gainer in &top_movers.top_gainers {
    ///     println!("  {} ({}): +{}%", gainer.ticker, gainer.price, gainer.change_percentage);
    /// }
    /// 
    /// println!("Top Losers:");
    /// for loser in &top_movers.top_losers {
    ///     println!("  {} ({}): {}%", loser.ticker, loser.price, loser.change_percentage);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self))]
    pub async fn top_gainers_losers(&self) -> Result<TopGainersLosers> {
        self.wait_for_rate_limit().await?;

        let params = HashMap::new();
        self.transport.get(FuncType::TopGainersLosers, params).await
    }

    /// Get listing status of all active or delisted US stocks and ETFs
    ///
    /// # Arguments
    ///
    /// * `date` - Optional date in YYYY-MM-DD format. If None, returns active listings.
    /// * `state` - "active" for currently listed securities, "delisted" for historical
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get all active listings
    /// let active_listings = endpoints.listing_status(None, "active").await?;
    /// 
    /// // Get delisted securities as of a specific date
    /// let delisted = endpoints.listing_status(Some("2023-12-31"), "delisted").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(date, state))]
    pub async fn listing_status(&self, date: Option<&str>, state: &str) -> Result<ListingStatus> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("state".to_string(), state.to_string());
        
        if let Some(date) = date {
            params.insert("date".to_string(), date.to_string());
        }

        self.transport.get(FuncType::ListingStatus, params).await
    }

    /// Get earnings calendar for the next few quarters
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by. If None, returns broad market calendar.
    /// * `horizon` - Time horizon: "3month", "6month", "12month"
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// // Get earnings calendar for Apple
    /// let apple_earnings = endpoints.earnings_calendar(Some("AAPL"), "3month").await?;
    /// 
    /// // Get broad market earnings calendar
    /// let market_earnings = endpoints.earnings_calendar(None, "3month").await?;
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self), fields(symbol, horizon))]
    pub async fn earnings_calendar(&self, symbol: Option<&str>, horizon: &str) -> Result<EarningsCalendar> {
        self.wait_for_rate_limit().await?;

        let mut params = HashMap::new();
        params.insert("horizon".to_string(), horizon.to_string());
        
        if let Some(symbol) = symbol {
            params.insert("symbol".to_string(), symbol.to_string());
        }

        self.transport.get(FuncType::EarningsCalendar, params).await
    }

    /// Get IPO calendar for upcoming and recent IPOs
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use av_client::FundamentalsEndpoints;
    /// # use std::sync::Arc;
    /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
    /// let ipo_calendar = endpoints.ipo_calendar().await?;
    /// for ipo in &ipo_calendar.data {
    ///     println!("Company: {}, Date: {}, Price Range: {}-{}", 
    ///              ipo.name, ipo.ipo_date, ipo.price_range_low, ipo.price_range_high);
    /// }
    /// # Ok::<(), av_core::Error>(())
    /// ```
    #[instrument(skip(self))]
    pub async fn ipo_calendar(&self) -> Result<IpoCalendar> {
        self.wait_for_rate_limit().await?;

        let params = HashMap::new();
        self.transport.get(FuncType::IpoCalendar, params).await
    }
}

impl_endpoint_base!(FundamentalsEndpoints);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;

    fn create_test_endpoints() -> FundamentalsEndpoints {
        let transport = Arc::new(Transport::new_mock());
        let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        FundamentalsEndpoints::new(transport, rate_limiter)
    }

    #[test]
    fn test_endpoints_creation() {
        let endpoints = create_test_endpoints();
        assert_eq!(endpoints.transport.base_url(), "https://mock.alphavantage.co");
    }

    #[tokio::test]
    async fn test_rate_limit_wait() {
        let endpoints = create_test_endpoints();
        let result = endpoints.wait_for_rate_limit().await;
        assert!(result.is_ok());
    }
}
