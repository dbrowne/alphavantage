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

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::schema::{intradayprices, summaryprices, topstats};

// Corrected to match schema: Primary key is (tstamp, sid, eventid)
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = intradayprices)]
#[diesel(primary_key(tstamp, sid, eventid))]
pub struct IntradayPrice {
  pub eventid: i64,
  pub tstamp: chrono::DateTime<chrono::Utc>,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
  pub price_source_id: i32,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = intradayprices)]
pub struct NewIntradayPrice<'a> {
  pub eventid: &'a i64,
  pub tstamp: &'a chrono::DateTime<chrono::Utc>,
  pub sid: &'a i64,
  pub symbol: &'a str,
  pub open: &'a f32,
  pub high: &'a f32,
  pub low: &'a f32,
  pub close: &'a f32,
  pub volume: &'a i64,
  pub price_source_id: &'a i32,
}

impl<'a> NewIntradayPrice<'a> {
  // Batch insert for TimescaleDB optimization
  pub async fn bulk_insert(
    conn: &mut diesel_async::AsyncPgConnection,
    records: Vec<Self>,
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 1000;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(intradayprices::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

// Query result structures
#[derive(QueryableByName, Debug, Serialize)]
pub struct OhlcBucket {
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  pub bucket: chrono::DateTime<chrono::Utc>,
  #[diesel(sql_type = diesel::sql_types::Text)]
  pub symbol: String,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub open: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub high: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub low: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub close: f32,
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  pub volume: i64,
}

impl IntradayPrice {
  /// Get OHLC data aggregated by time bucket
  pub async fn time_bucket_ohlc(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
    bucket_size: &str,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
  ) -> Result<Vec<OhlcBucket>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{Text, Timestamptz};

    sql_query(format!(
      r#"
            SELECT 
                time_bucket('{}', tstamp) AS bucket,
                symbol,
                first(open, tstamp) AS open,
                max(high) AS high,
                min(low) AS low,
                last(close, tstamp) AS close,
                sum(volume) AS volume
            FROM intradayprices
            WHERE symbol = $1
                AND tstamp >= $2
                AND tstamp <= $3
            GROUP BY bucket, symbol
            ORDER BY bucket DESC
            "#,
      bucket_size
    ))
    .bind::<Text, _>(symbol)
    .bind::<Timestamptz, _>(start)
    .bind::<Timestamptz, _>(end)
    .load::<OhlcBucket>(conn)
    .await
  }
}

// Corrected to match schema: Primary key is (tstamp, sid, eventid)
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = summaryprices)]
#[diesel(primary_key(tstamp, sid, eventid))]
pub struct SummaryPrice {
  pub eventid: i64,
  pub tstamp: chrono::DateTime<chrono::Utc>,
  pub date: NaiveDate,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
  pub price_source_id: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = summaryprices)]
pub struct NewSummaryPrice<'a> {
  pub eventid: &'a i64,
  pub tstamp: &'a chrono::DateTime<chrono::Utc>,
  pub date: &'a NaiveDate,
  pub sid: &'a i64,
  pub symbol: &'a str,
  pub open: &'a f32,
  pub high: &'a f32,
  pub low: &'a f32,
  pub close: &'a f32,
  pub volume: &'a i64,
  pub price_source_id: &'a i32,
}

// Add owned variant for API responses
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = summaryprices)]
pub struct NewSummaryPriceOwned {
  pub eventid: i64,
  pub tstamp: chrono::DateTime<chrono::Utc>,
  pub date: NaiveDate,
  pub sid: i64,
  pub symbol: String,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: i64,
  pub price_source_id: i32,
}

#[derive(QueryableByName, Debug, Serialize)]
pub struct PriceWithMA {
  #[diesel(sql_type = diesel::sql_types::Date)]
  pub date: NaiveDate,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub close: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub ma: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub volume_ma: f32,
}

impl<'a> NewSummaryPrice<'a> {
  pub async fn bulk_insert_refs(
    conn: &mut diesel_async::AsyncPgConnection,
    records: &[NewSummaryPrice<'a>],
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 1000;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(summaryprices::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl NewSummaryPriceOwned {
  pub fn as_ref(&self) -> NewSummaryPrice<'_> {
    NewSummaryPrice {
      eventid: &self.eventid,
      tstamp: &self.tstamp,
      date: &self.date,
      sid: &self.sid,
      symbol: &self.symbol,
      open: &self.open,
      high: &self.high,
      low: &self.low,
      close: &self.close,
      volume: &self.volume,
      price_source_id: &self.price_source_id,
    }
  }

  pub async fn bulk_insert(
    conn: &mut diesel_async::AsyncPgConnection,
    records: Vec<Self>,
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 1000;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(summaryprices::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl SummaryPrice {
  /// Get latest price for a symbol
  pub async fn get_latest(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
  ) -> Result<Self, diesel::result::Error> {
    summaryprices::table
      .filter(summaryprices::symbol.eq(symbol))
      .order(summaryprices::date.desc())
      .first(conn)
      .await
  }

  /// Get price history for a symbol within date range
  pub async fn get_history(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    summaryprices::table
      .filter(summaryprices::symbol.eq(symbol))
      .filter(summaryprices::date.ge(start_date))
      .filter(summaryprices::date.le(end_date))
      .order(summaryprices::date.desc())
      .load(conn)
      .await
  }

  /// Get moving average using TimescaleDB window functions
  pub async fn get_with_moving_average(
    conn: &mut diesel_async::AsyncPgConnection,
    symbol: &str,
    days: i32,
    ma_period: i32,
  ) -> Result<Vec<PriceWithMA>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{Integer, Text};

    sql_query(
      r#"
            SELECT 
                date,
                close,
                AVG(close) OVER (
                    ORDER BY date 
                    ROWS BETWEEN $3 PRECEDING AND CURRENT ROW
                ) as ma,
                AVG(volume::float4) OVER (
                    ORDER BY date 
                    ROWS BETWEEN $3 PRECEDING AND CURRENT ROW
                ) as volume_ma
            FROM summaryprices
            WHERE symbol = $1
                AND date >= CURRENT_DATE - INTERVAL '1 day' * $2
            ORDER BY date DESC
            "#,
    )
    .bind::<Text, _>(symbol)
    .bind::<Integer, _>(days)
    .bind::<Integer, _>(ma_period - 1)
    .load::<PriceWithMA>(conn)
    .await
  }
}

// Corrected to match schema: Primary key is (date, event_type, sid)
#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = topstats)]
#[diesel(primary_key(date, event_type, sid))]
pub struct TopStat {
  pub date: chrono::DateTime<chrono::Utc>,
  pub event_type: String,
  pub sid: i64,
  pub symbol: String,
  pub price: f32,
  pub change_val: f32,
  pub change_pct: f32,
  pub volume: i64,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = topstats)]
pub struct NewTopStat<'a> {
  pub date: &'a chrono::DateTime<chrono::Utc>,
  pub event_type: &'a str,
  pub sid: &'a i64,
  pub symbol: &'a str,
  pub price: &'a f32,
  pub change_val: &'a f32,
  pub change_pct: &'a f32,
  pub volume: &'a i64,
}

// Add owned variant for API responses
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = topstats)]
pub struct NewTopStatOwned {
  pub date: chrono::DateTime<chrono::Utc>,
  pub event_type: String,
  pub sid: i64,
  pub symbol: String,
  pub price: f32,
  pub change_val: f32,
  pub change_pct: f32,
  pub volume: i64,
}

#[derive(QueryableByName, Debug, Serialize)]
pub struct HistoricalTopMover {
  #[diesel(sql_type = diesel::sql_types::Timestamptz)]
  pub bucket: chrono::DateTime<chrono::Utc>,
  #[diesel(sql_type = diesel::sql_types::Text)]
  pub event_type: String,
  #[diesel(sql_type = diesel::sql_types::Integer)]
  pub mover_count: i32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub avg_change_pct: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub max_change_pct: f32,
  #[diesel(sql_type = diesel::sql_types::Text)]
  pub top_symbol: String,
}

#[derive(QueryableByName, Debug, Serialize)]
pub struct SectorPerformance {
  #[diesel(sql_type = diesel::sql_types::Text)]
  pub sector: String,
  #[diesel(sql_type = diesel::sql_types::Integer)]
  pub gainer_count: i32,
  #[diesel(sql_type = diesel::sql_types::Integer)]
  pub loser_count: i32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub avg_gain: f32,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub avg_loss: f32,
}

impl<'a> NewTopStat<'a> {
  pub async fn bulk_insert_refs(
    conn: &mut diesel_async::AsyncPgConnection,
    records: &[NewTopStat<'a>],
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 500;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(topstats::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl NewTopStatOwned {
  pub fn as_ref(&self) -> NewTopStat<'_> {
    NewTopStat {
      date: &self.date,
      event_type: &self.event_type,
      sid: &self.sid,
      symbol: &self.symbol,
      price: &self.price,
      change_val: &self.change_val,
      change_pct: &self.change_pct,
      volume: &self.volume,
    }
  }

  pub async fn bulk_insert(
    conn: &mut diesel_async::AsyncPgConnection,
    records: Vec<Self>,
  ) -> Result<usize, diesel::result::Error> {
    use diesel::insert_into;

    const BATCH_SIZE: usize = 500;
    let mut total_inserted = 0;

    for chunk in records.chunks(BATCH_SIZE) {
      let inserted = insert_into(topstats::table).values(chunk).execute(conn).await?;
      total_inserted += inserted;
    }

    Ok(total_inserted)
  }
}

impl TopStat {
  /// Get top movers by type (gainers/losers)
  pub async fn get_by_type(
    conn: &mut diesel_async::AsyncPgConnection,
    event_type: &str,
    limit: i64,
  ) -> Result<Vec<Self>, diesel::result::Error> {
    topstats::table
      .filter(topstats::event_type.eq(event_type))
      .filter(topstats::date.ge(chrono::Utc::now() - chrono::Duration::days(1)))
      .order(topstats::change_pct.desc())
      .limit(limit)
      .load(conn)
      .await
  }

  /// Get latest movers with pagination
  pub async fn get_latest_paginated(
    conn: &mut diesel_async::AsyncPgConnection,
    event_type: &str,
    page: i64,
    per_page: i64,
  ) -> Result<(Vec<Self>, i64), diesel::result::Error> {
    let offset = (page - 1) * per_page;

    // Get total count
    let total = topstats::table
      .filter(topstats::event_type.eq(event_type))
      .filter(topstats::date.ge(chrono::Utc::now() - chrono::Duration::days(1)))
      .count()
      .get_result::<i64>(conn)
      .await?;

    // Get paginated results
    let results = topstats::table
      .filter(topstats::event_type.eq(event_type))
      .filter(topstats::date.ge(chrono::Utc::now() - chrono::Duration::days(1)))
      .order(topstats::change_pct.desc())
      .limit(per_page)
      .offset(offset)
      .load(conn)
      .await?;

    Ok((results, total))
  }

  /// Get historical top movers with TimescaleDB time_bucket
  pub async fn get_historical_top_movers(
    conn: &mut diesel_async::AsyncPgConnection,
    event_type: &str,
    bucket_size: &str,
    days_back: i32,
  ) -> Result<Vec<HistoricalTopMover>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::{Integer, Text};

    sql_query(format!(
      r#"
            WITH bucketed_stats AS (
                SELECT 
                    time_bucket('{}', date) AS bucket,
                    event_type,
                    symbol,
                    change_pct,
                    ROW_NUMBER() OVER (
                        PARTITION BY time_bucket('{}', date), event_type 
                        ORDER BY change_pct DESC
                    ) as rn
                FROM topstats
                WHERE event_type = $1
                    AND date >= NOW() - INTERVAL '1 day' * $2
            )
            SELECT 
                bucket,
                event_type,
                COUNT(*)::integer as mover_count,
                AVG(change_pct) as avg_change_pct,
                MAX(change_pct) as max_change_pct,
                MAX(CASE WHEN rn = 1 THEN symbol END) as top_symbol
            FROM bucketed_stats
            GROUP BY bucket, event_type
            ORDER BY bucket DESC
            "#,
      bucket_size, bucket_size
    ))
    .bind::<Text, _>(event_type)
    .bind::<Integer, _>(days_back)
    .load::<HistoricalTopMover>(conn)
    .await
  }

  /// Get sector performance from top movers
  pub async fn get_sector_performance(
    conn: &mut diesel_async::AsyncPgConnection,
    date: chrono::DateTime<chrono::Utc>,
  ) -> Result<Vec<SectorPerformance>, diesel::result::Error> {
    use diesel::sql_query;
    use diesel::sql_types::Timestamptz;

    sql_query(
      r#"
            SELECT 
                o.sector,
                COUNT(CASE WHEN t.event_type = 'gainers' THEN 1 END)::integer as gainer_count,
                COUNT(CASE WHEN t.event_type = 'losers' THEN 1 END)::integer as loser_count,
                AVG(CASE WHEN t.event_type = 'gainers' THEN t.change_pct END) as avg_gain,
                AVG(CASE WHEN t.event_type = 'losers' THEN t.change_pct END) as avg_loss
            FROM topstats t
            JOIN overviews o ON t.sid = o.sid
            WHERE t.date = $1
            GROUP BY o.sector
            ORDER BY gainer_count DESC
            "#,
    )
    .bind::<Timestamptz, _>(date)
    .load::<SectorPerformance>(conn)
    .await
  }
}
