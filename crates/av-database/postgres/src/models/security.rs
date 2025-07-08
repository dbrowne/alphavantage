use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::schema::{symbols, overviews, overviewexts};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = symbols)]
#[diesel(primary_key(sid))]
pub struct Symbol {
    pub sid: i64,
    pub symbol: String,
    pub name: String,
    pub sec_type: String,
    pub region: String,
    pub marketopen: NaiveTime,
    pub marketclose: NaiveTime,
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
    pub symbol: &'a  String,
    pub name: &'a  String,
    pub sec_type: &'a  String,
    pub region: &'a  String,
    pub marketopen: &'a  NaiveTime,
    pub marketclose: &'a  NaiveTime,
    pub timezone: &'a  String,
    pub currency: &'a  String,
    pub overview: &'a  bool,
    pub intraday: &'a  bool,
    pub summary:  &'a bool,
    pub c_time:  &'a NaiveDateTime,
    pub m_time: &'a  NaiveDateTime,
}

// Add async methods
impl Symbol {
    pub async fn find_by_sid(
        conn: &mut diesel_async::AsyncPgConnection,
        sid: i64,
    ) -> Result<Self, diesel::result::Error> {
        symbols::table
            .find(sid)
            .first(conn)
            .await
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
    pub exch: String,
    pub curr: String,
    pub country: String,
    pub sector: String,
    pub industry: String,
    pub address: String,
    pub fiscalyearend: String,
    pub latestquarter: NaiveDate,
    pub marketcapitalization: i64,
    pub ebitda: i64,
    pub peratio: f32,
    pub pegratio: f32,
    pub bookvalue: f64,
    pub dividendpershare: f32,
    pub dividendyield: f32,
    pub eps: f32,
    pub c_time: NaiveDateTime,
    pub mod_time: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviews)]
pub struct NewOverview <'a> {
    pub sid: &'a  i64,
    pub symbol: &'a  String,
    pub name: &'a  String,
    pub description: &'a  String,
    pub cik: &'a  String,
    pub exch: &'a  String,
    pub curr: &'a  String,
    pub country: &'a  String,
    pub sector:  &'a String,
    pub industry: &'a  String,
    pub address:  &'a String,
    pub fiscalyearend: &'a  String,
    pub latestquarter: &'a  NaiveDate,
    pub marketcapitalization: &'a  i64,
    pub ebitda: &'a  i64,
    pub peratio:  &'a f32,
    pub pegratio:  &'a f32,
    pub bookvalue: &'a  f64,1G
    pub dividendpershare: &'a  f32,
    pub dividendyield: &'a  f32,
    pub eps: &'a  f32,
    pub c_time:  &'a NaiveDateTime,
    pub mod_time: &'a  NaiveDateTime,
}
#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone, Serialize)]
#[diesel(table_name = overviewext)]
#[diesel(primary_key(sid))]
#[diesel(belongs_to(Symbol, foreign_key = sid))]
pub struct Overviewext {
  pub sid: i64,
  pub revenuepersharettm: f32,
  pub profitmargin: f32,
  pub operatingmarginttm: f32,
  pub returnonassetsttm: f32,
  pub returnonequityttm: f32,
  pub revenuettm: i64,
  pub grossprofitttm: i64,
  pub dilutedepsttm: f32,
  pub quarterlyearningsgrowthyoy: f32,
  pub quarterlyrevenuegrowthyoy: f32,
  pub analysttargetprice: f32,
  pub trailingpe: f32,
  pub forwardpe: f32,
  pub pricetosalesratiottm: f32,
  pub pricetobookratio: f32,
  pub evtorevenue: f32,
  pub evtoebitda: f32,
  pub beta: f64,
  pub annweekhigh: f64,
  pub annweeklow: f64,
  pub fiftydaymovingaverage: f64,
  pub twohdaymovingaverage: f64,
  pub sharesoutstanding: f64,
  pub dividenddate: NaiveDate,
  pub exdividenddate: NaiveDate,
  pub c_time: NaiveDateTime,
  pub mod_time: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = overviewext)]
pub struct NewOverviewext <'a> {
  pub sid: &'a i64,
  pub revenuepersharettm:&'a  f32,
  pub profitmargin:&'a  f32,
  pub operatingmarginttm: &'a f32,
  pub returnonassetsttm: &'a f32,
  pub returnonequityttm: &'a f32,
  pub revenuettm: &'a i64,
  pub grossprofitttm: &'a i64,
  pub dilutedepsttm: &'a f32,
  pub quarterlyearningsgrowthyoy: &'a f32,
  pub quarterlyrevenuegrowthyoy: &'a f32,
  pub analysttargetprice: &'a f32,
  pub trailingpe: &'a f32,
  pub forwardpe: &'a f32,
  pub pricetosalesratiottm: &'a f32,
  pub pricetobookratio: &'a f32,
  pub evtorevenue: &'a f32,
  pub evtoebitda: &'a f32,
  pub beta: &'a f64,
  pub annweekhigh: &'a f64,
  pub annweeklow: &'a f64,
  pub fiftydaymovingaverage: f64,
  pub twohdaymovingaverage: f64,
  pub sharesoutstanding: f64,
  pub dividenddate: NaiveDate,
  pub exdividenddate: NaiveDate,
  pub c_time: NaiveDateTime,
  pub mod_time: NaiveDateTime,
}
}
