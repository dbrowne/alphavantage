use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::schema::{overviewexts, overviews, symbols};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = symbols)]
#[diesel(primary_key(sid))]
pub struct Symbol {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub sec_type: String,
  pub region: String,
  pub market_open: NaiveTime,
  pub market_close: NaiveTime,
  pub timezone: String,
  pub currency: String,
  pub overview: bool,
  pub intraday: bool,
  pub summary: bool,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = symbols)]
pub struct NewSymbol<'a> {
  pub sid: &'a i64,
  pub symbol: &'a String,
  pub name: &'a String,
  pub sec_type: &'a String,
  pub region: &'a String,
  pub market_open: &'a NaiveTime,
  pub market_close: &'a NaiveTime,
  pub timezone: &'a String,
  pub currency: &'a String,
  pub overview: &'a bool,
  pub intraday: &'a bool,
  pub summary: &'a bool,
  pub c_time: &'a NaiveDateTime,
  pub m_time: &'a NaiveDateTime,
}

// Add async methods
impl Symbol {
  pub async fn find_by_sid(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
  ) -> Result<Self, diesel::result::Error> {
    symbols::table.find(sid).first(conn).await
  }

  pub async fn find_by_symbol(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
  ) -> Result<Self, diesel::result::Error> {
    symbols::table
      .filter(symbols::symbol.eq(symbol))
      .first(conn)
      .await
  }

  pub async fn active_symbols(
    conn: &mut diesel_async::AsyncPgConnection,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    symbols::table
      .filter(symbols::overview.eq(true))
      .or_filter(symbols::intraday.eq(true))
      .or_filter(symbols::summary.eq(true))
      .load(conn)
      .await
  }
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone, Serialize)]
#[diesel(table_name = overviews)]
#[diesel(primary_key(sid))]
#[diesel(belongs_to(Symbol, foreign_key = sid))]
pub struct Overview {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub description: String,
  pub cik: String,
  pub exchange: String,
  pub currency: String,
  pub country: String,
  pub sector: String,
  pub industry: String,
  pub address: String,
  pub fiscal_year_end: String,
  pub latest_quarter: NaiveDate,
  pub market_capitalization: i64,
  pub ebitda: i64,
  pub pe_ratio: f32,
  pub peg_ratio: f32,
  pub book_value: f32,
  pub dividend_per_share: f32,
  pub dividend_yield: f32,
  pub eps: f32,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

// Add conversion methods
impl NewSymbolOwned {
  pub fn as_ref(&self) -> NewSymbol<'_> {
    NewSymbol {
      sid: &self.sid,
      symbol: &self.symbol,
      name: &self.name,
      sec_type: &self.sec_type,
      region: &self.region,
      market_open: &self.market_open,
      market_close: &self.market_close,
      timezone: &self.timezone,
      currency: &self.currency,
      overview: &self.overview,
      intraday: &self.intraday,
      summary: &self.summary,
      c_time: &self.c_time,
      m_time: &self.m_time,
    }
  }
}

impl NewOverviewOwned {
  pub fn as_ref(&self) -> NewOverview<'_> {
    NewOverview {
      sid: &self.sid,
      symbol: &self.symbol,
      name: &self.name,
      description: &self.description,
      cik: &self.cik,
      exchange: &self.exchange,
      currency: &self.currency,
      country: &self.country,
      sector: &self.sector,
      industry: &self.industry,
      address: &self.address,
      fiscal_year_end: &self.fiscal_year_end,
      latest_quarter: &self.latest_quarter,
      market_capitalization: &self.market_capitalization,
      ebitda: &self.ebitda,
      pe_ratio: &self.pe_ratio,
      peg_ratio: &self.peg_ratio,
      book_value: &self.book_value,
      dividend_per_share: &self.dividend_per_share,
      dividend_yield: &self.dividend_yield,
      eps: &self.eps,
      c_time: &self.c_time,
      m_time: &self.m_time,
    }
  }
}

impl NewOverviewextOwned {
  pub fn as_ref(&self) -> NewOverviewext<'_> {
    NewOverviewext {
      sid: &self.sid,
      revenue_per_share_ttm: &self.revenue_per_share_ttm,
      profit_margin: &self.profit_margin,
      operating_margin_ttm: &self.operating_margin_ttm,
      return_on_assets_ttm: &self.return_on_assets_ttm,
      return_on_equity_ttm: &self.return_on_equity_ttm,
      revenue_ttm: &self.revenue_ttm,
      gross_profit_ttm: &self.gross_profit_ttm,
      diluted_eps_ttm: &self.diluted_eps_ttm,
      quarterly_earnings_growth_yoy: &self.quarterly_earnings_growth_yoy,
      quarterly_revenue_growth_yoy: &self.quarterly_revenue_growth_yoy,
      analyst_target_price: &self.analyst_target_price,
      trailing_pe: &self.trailing_pe,
      forward_pe: &self.forward_pe,
      price_to_sales_ratio_ttm: &self.price_to_sales_ratio_ttm,
      price_to_book_ratio: &self.price_to_book_ratio,
      ev_to_revenue: &self.ev_to_revenue,
      ev_to_ebitda: &self.ev_to_ebitda,
      beta: &self.beta,
      week_high_52: &self.week_high_52,
      week_low_52: &self.week_low_52,
      day_moving_average_50: &self.day_moving_average_50,
      day_moving_average_200: &self.day_moving_average_200,
      shares_outstanding: &self.shares_outstanding,
      dividend_date: self.dividend_date.as_ref(),
      ex_dividend_date: self.ex_dividend_date.as_ref(),
      c_time: &self.c_time,
      m_time: &self.m_time,
    }
  }
}

// Add async methods for Overview
impl Overview {
  pub async fn find_by_sid(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
  ) -> Result<Self, diesel::result::Error> {
    overviews::table.find(sid).first(conn).await
  }

  pub async fn find_by_symbol(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
  ) -> Result<Self, diesel::result::Error> {
    overviews::table
      .filter(overviews::symbol.eq(symbol))
      .first(conn)
      .await
  }

  pub async fn by_sector(
    conn: &mut diesel_async::AsyncPgConnection,
    sector: &str,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    overviews::table
      .filter(overviews::sector.eq(sector))
      .load(conn)
      .await
  }
}

// Add async methods for Overviewext
impl Overviewext {
  pub async fn find_by_sid(
    conn: &mut diesel_async::AsyncPgConnection,
    sid: i64,
  ) -> Result<Self, diesel::result::Error> {
    overviewexts::table.find(sid).first(conn).await
  }

  pub async fn with_high_beta(
    conn: &mut diesel_async::AsyncPgConnection,
    min_beta: f32,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    overviewexts::table
      .filter(overviewexts::beta.ge(min_beta))
      .load(conn)
      .await
  }

  pub async fn with_dividend(
    conn: &mut diesel_async::AsyncPgConnection,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    overviewexts::table
      .filter(overviewexts::dividend_date.is_not_null())
      .load(conn)
      .await
  }
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviews)]
pub struct NewOverview<'a> {
  pub sid: &'a i64,
  pub symbol: &'a String,
  pub name: &'a String,
  pub description: &'a String,
  pub cik: &'a String,
  pub exchange: &'a String,
  pub currency: &'a String,
  pub country: &'a String,
  pub sector: &'a String,
  pub industry: &'a String,
  pub address: &'a String,
  pub fiscal_year_end: &'a String,
  pub latest_quarter: &'a NaiveDate,
  pub market_capitalization: &'a i64,
  pub ebitda: &'a i64,
  pub pe_ratio: &'a f32,
  pub peg_ratio: &'a f32,
  pub book_value: &'a f32,
  pub dividend_per_share: &'a f32,
  pub dividend_yield: &'a f32,
  pub eps: &'a f32,
  pub c_time: &'a NaiveDateTime,
  pub m_time: &'a NaiveDateTime,
}

// Add owned versions for easier API integration
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = symbols)]
pub struct NewSymbolOwned {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub sec_type: String,
  pub region: String,
  pub market_open: NaiveTime,
  pub market_close: NaiveTime,
  pub timezone: String,
  pub currency: String,
  pub overview: bool,
  pub intraday: bool,
  pub summary: bool,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviews)]
pub struct NewOverviewOwned {
  pub sid: i64,
  pub symbol: String,
  pub name: String,
  pub description: String,
  pub cik: String,
  pub exchange: String,
  pub currency: String,
  pub country: String,
  pub sector: String,
  pub industry: String,
  pub address: String,
  pub fiscal_year_end: String,
  pub latest_quarter: NaiveDate,
  pub market_capitalization: i64,
  pub ebitda: i64,
  pub pe_ratio: f32,
  pub peg_ratio: f32,
  pub book_value: f32,
  pub dividend_per_share: f32,
  pub dividend_yield: f32,
  pub eps: f32,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviewexts)]
pub struct NewOverviewextOwned {
  pub sid: i64,
  pub revenue_per_share_ttm: f32,
  pub profit_margin: f32,
  pub operating_margin_ttm: f32,
  pub return_on_assets_ttm: f32,
  pub return_on_equity_ttm: f32,
  pub revenue_ttm: i64,
  pub gross_profit_ttm: i64,
  pub diluted_eps_ttm: f32,
  pub quarterly_earnings_growth_yoy: f32,
  pub quarterly_revenue_growth_yoy: f32,
  pub analyst_target_price: f32,
  pub trailing_pe: f32,
  pub forward_pe: f32,
  pub price_to_sales_ratio_ttm: f32,
  pub price_to_book_ratio: f32,
  pub ev_to_revenue: f32,
  pub ev_to_ebitda: f32,
  pub beta: f32,
  pub week_high_52: f32,
  pub week_low_52: f32,
  pub day_moving_average_50: f32,
  pub day_moving_average_200: f32,
  pub shares_outstanding: i64,
  pub dividend_date: Option<NaiveDate>,
  pub ex_dividend_date: Option<NaiveDate>,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone, Serialize)]
#[diesel(table_name = overviewexts)]
#[diesel(primary_key(sid))]
#[diesel(belongs_to(Symbol, foreign_key = sid))]
pub struct Overviewext {
  pub sid: i64,
  pub revenue_per_share_ttm: f32,
  pub profit_margin: f32,
  pub operating_margin_ttm: f32,
  pub return_on_assets_ttm: f32,
  pub return_on_equity_ttm: f32,
  pub revenue_ttm: i64,
  pub gross_profit_ttm: i64,
  pub diluted_eps_ttm: f32,
  pub quarterly_earnings_growth_yoy: f32,
  pub quarterly_revenue_growth_yoy: f32,
  pub analyst_target_price: f32,
  pub trailing_pe: f32,
  pub forward_pe: f32,
  pub price_to_sales_ratio_ttm: f32,
  pub price_to_book_ratio: f32,
  pub ev_to_revenue: f32,
  pub ev_to_ebitda: f32,
  pub beta: f32,
  pub week_high_52: f32,
  pub week_low_52: f32,
  pub day_moving_average_50: f32,
  pub day_moving_average_200: f32,
  pub shares_outstanding: i64,
  pub dividend_date: Option<NaiveDate>,
  pub ex_dividend_date: Option<NaiveDate>,
  pub c_time: NaiveDateTime,
  pub m_time: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviewexts)]
pub struct NewOverviewext<'a> {
  pub sid: &'a i64,
  pub revenue_per_share_ttm: &'a f32,
  pub profit_margin: &'a f32,
  pub operating_margin_ttm: &'a f32,
  pub return_on_assets_ttm: &'a f32,
  pub return_on_equity_ttm: &'a f32,
  pub revenue_ttm: &'a i64,
  pub gross_profit_ttm: &'a i64,
  pub diluted_eps_ttm: &'a f32,
  pub quarterly_earnings_growth_yoy: &'a f32,
  pub quarterly_revenue_growth_yoy: &'a f32,
  pub analyst_target_price: &'a f32,
  pub trailing_pe: &'a f32,
  pub forward_pe: &'a f32,
  pub price_to_sales_ratio_ttm: &'a f32,
  pub price_to_book_ratio: &'a f32,
  pub ev_to_revenue: &'a f32,
  pub ev_to_ebitda: &'a f32,
  pub beta: &'a f32,
  pub week_high_52: &'a f32,
  pub week_low_52: &'a f32,
  pub day_moving_average_50: &'a f32,
  pub day_moving_average_200: &'a f32,
  pub shares_outstanding: &'a i64,
  pub dividend_date: Option<&'a NaiveDate>,
  pub ex_dividend_date: Option<&'a NaiveDate>,
  pub c_time: &'a NaiveDateTime,
  pub m_time: &'a NaiveDateTime,
}
