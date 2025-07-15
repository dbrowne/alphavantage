use super::EndpointBase;
use crate::impl_endpoint_base;
use crate::transport::Transport;
use av_core::{FuncType, Result};
use av_models::fundamentals::*;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Fundamental data endpoints for company financial information
pub struct FundamentalsEndpoints {
  transport: Arc<Transport>,
  rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl FundamentalsEndpoints {
  /// Create a new fundamentals endpoints instance
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
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
  #[instrument(skip(self), fields(symbol))]
  pub async fn earnings(&self, symbol: &str) -> Result<Earnings> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), symbol.to_string());

    self.transport.get(FuncType::Earnings, params).await
  }

  /// Get top gainers, losers, and most actively traded stocks
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// # use av_client::FundamentalsEndpoints;
  /// # use std::sync::Arc;
  /// # let endpoints = FundamentalsEndpoints::new(Arc::new(transport), Arc::new(rate_limiter));
  /// let top_stats = endpoints.top_gainers_losers().await?;
  /// for gainer in &top_stats.top_gainers {
  ///     println!("Top Gainer: {} (+{}%)", gainer.ticker, gainer.change_percentage);
  /// }
  /// # Ok::<(), av_core::Error>(())
  /// ```
  #[instrument(skip(self))]
  pub async fn top_gainers_losers(&self) -> Result<TopGainersLosers> {
    self.wait_for_rate_limit().await?;

    let params = HashMap::new();

    self.transport.get(FuncType::TopGainersLosers, params).await
  }

  /// Get listing status (active or delisted) for securities
  ///
  /// # Arguments
  ///
  /// * `date` - Optional date in YYYY-MM-DD format
  /// * `state` - Optional state filter ("active" or "delisted")
  #[instrument(skip(self), fields(date, state))]
  pub async fn listing_status(
    &self,
    date: Option<&str>,
    state: Option<&str>,
  ) -> Result<ListingStatus> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    if let Some(date) = date {
      params.insert("date".to_string(), date.to_string());
    }
    if let Some(state) = state {
      params.insert("state".to_string(), state.to_string());
    }

    self.transport.get(FuncType::ListingStatus, params).await
  }

  /// Get earnings calendar data
  ///
  /// # Arguments
  ///
  /// * `symbol` - Optional symbol filter
  /// * `horizon` - Time horizon ("3month", "6month", or "12month")
  #[instrument(skip(self), fields(symbol, horizon))]
  pub async fn earnings_calendar(
    &self,
    symbol: Option<&str>,
    horizon: Option<&str>,
  ) -> Result<EarningsCalendar> {
    self.wait_for_rate_limit().await?;

    let mut params = HashMap::new();
    if let Some(symbol) = symbol {
      params.insert("symbol".to_string(), symbol.to_string());
    }
    if let Some(horizon) = horizon {
      params.insert("horizon".to_string(), horizon.to_string());
    }

    self.transport.get(FuncType::EarningsCalendar, params).await
  }

  /// Get IPO calendar data
  #[instrument(skip(self))]
  pub async fn ipo_calendar(&self) -> Result<IpoCalendar> {
    self.wait_for_rate_limit().await?;

    let params = HashMap::new();

    self.transport.get(FuncType::IpoCalendar, params).await
  }
}

impl_endpoint_base!(FundamentalsEndpoints);
