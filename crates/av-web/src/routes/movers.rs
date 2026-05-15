/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Top-movers route — gainers, losers, most-actively traded.
//!
//! Pure DB read against the `topstats` hypertable; freshness depends on
//! how recently `av-cli load top-movers` was run. The page also renders
//! a three-month calendar highlighting dates where data exists, with
//! each highlighted cell linking to `/movers?date=YYYY-MM-DD` to view
//! that day's batch.

use actix_web::{web, HttpResponse};
use av_api::queries::{top_movers_available_dates, top_movers_by_type, TopMoverRow};
use av_database_postgres::DatabaseContext;
use chrono::{Datelike, Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tera::Tera;

/// Per-section row cap. AlphaVantage TOP_GAINERS_LOSERS returns 20 of each.
const MOVERS_LIMIT: i64 = 20;
/// Number of months rendered in the stacked calendar (current first).
const CALENDAR_MONTHS: u32 = 3;

#[derive(Debug, Deserialize)]
pub struct MoversQuery {
  /// Optional `YYYY-MM-DD` filter. Missing or unparseable → most recent batch.
  pub date: Option<String>,
}

/// One cell in the calendar grid. Leading/trailing blanks (for the days
/// before the 1st / after the last day of the month) have `day = None`.
#[derive(Debug, Clone, Serialize)]
struct CalendarCell {
  day: Option<u32>,
  /// ISO date string for the link href; empty for blank cells.
  date_str: String,
  has_data: bool,
  is_selected: bool,
  is_today: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CalendarMonth {
  year: i32,
  month_name: String,
  /// Rows of 7 cells (Sunday-start).
  weeks: Vec<Vec<CalendarCell>>,
}

/// GET /movers and GET /movers?date=YYYY-MM-DD
pub async fn movers(
  tmpl: web::Data<Tera>,
  db: web::Data<DatabaseContext>,
  query: web::Query<MoversQuery>,
) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "movers");

  // Parse ?date= → Option<NaiveDate>. Bad strings are logged and ignored
  // (caller falls back to "most recent batch" behaviour).
  let selected_date: Option<NaiveDate> = query.date.as_deref().and_then(|s| {
    match NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
      Ok(d) => Some(d),
      Err(e) => {
        tracing::warn!("Ignoring invalid ?date={:?}: {}", s, e);
        None
      }
    }
  });

  let gainers = fetch_section(&db, "gainers", selected_date).await;
  let losers = fetch_section(&db, "losers", selected_date).await;
  let active = fetch_section(&db, "most_active", selected_date).await;

  // `date` is identical across rows in one batch, so any non-empty section
  // tells us when the data was last loaded. Pick the first one we find.
  let last_updated = gainers
    .first()
    .or_else(|| losers.first())
    .or_else(|| active.first())
    .map(|r| r.date);

  let has_data = !gainers.is_empty() || !losers.is_empty() || !active.is_empty();

  // ── Calendar ──────────────────────────────────────────────────────
  let available_dates = match top_movers_available_dates(&db).await {
    Ok(v) => v,
    Err(e) => {
      tracing::error!("top_movers_available_dates failed: {}", e);
      Vec::new()
    }
  };
  let available_set: HashSet<NaiveDate> = available_dates.iter().copied().collect();
  let today = Utc::now().date_naive();
  let months = build_calendar_months(today, &available_set, selected_date);

  ctx.insert("gainers", &gainers);
  ctx.insert("losers", &losers);
  ctx.insert("most_active", &active);
  ctx.insert("last_updated", &last_updated);
  ctx.insert("has_data", &has_data);
  ctx.insert("selected_date", &selected_date);
  ctx.insert("calendar_months", &months);
  ctx.insert("available_count", &available_dates.len());

  render(tmpl.get_ref(), ctx)
}

async fn fetch_section(
  db: &DatabaseContext,
  event_type: &str,
  on_date: Option<NaiveDate>,
) -> Vec<TopMoverRow> {
  match top_movers_by_type(db, event_type, MOVERS_LIMIT, on_date).await {
    Ok(rows) => rows,
    Err(e) => {
      tracing::error!("top_movers_by_type({}) failed: {}", event_type, e);
      Vec::new()
    }
  }
}

// ─── Calendar construction ─────────────────────────────────────────

/// Build the most recent N calendar months (current month first, oldest last),
/// each as a weeks×days grid with highlight flags pre-computed.
fn build_calendar_months(
  today: NaiveDate,
  available: &HashSet<NaiveDate>,
  selected: Option<NaiveDate>,
) -> Vec<CalendarMonth> {
  // First-of-month for the current month, then walk back N-1 months.
  let cur_first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
    .expect("today is a real date");

  (0..CALENDAR_MONTHS)
    .map(|offset| {
      let m = cur_first
        .checked_sub_months(Months::new(offset))
        .expect("calendar offset always within chrono range");
      build_one_month(m, today, available, selected)
    })
    .collect()
}

fn build_one_month(
  first_of_month: NaiveDate,
  today: NaiveDate,
  available: &HashSet<NaiveDate>,
  selected: Option<NaiveDate>,
) -> CalendarMonth {
  let year = first_of_month.year();
  let month = first_of_month.month();

  // Last day of month = (first of next month) - 1 day
  let first_of_next = first_of_month
    .checked_add_months(Months::new(1))
    .expect("next month exists");
  let last_day_of_month: u32 = (first_of_next - first_of_month).num_days() as u32;

  // Sunday-start: num_days_from_sunday returns 0..=6
  let leading_blanks = first_of_month.weekday().num_days_from_sunday() as usize;

  let mut cells: Vec<CalendarCell> = Vec::with_capacity(42);
  for _ in 0..leading_blanks {
    cells.push(blank_cell());
  }
  for day in 1..=last_day_of_month {
    let d = NaiveDate::from_ymd_opt(year, month, day).expect("valid day");
    cells.push(CalendarCell {
      day: Some(day),
      date_str: d.format("%Y-%m-%d").to_string(),
      has_data: available.contains(&d),
      is_selected: selected == Some(d),
      is_today: d == today,
    });
  }
  while cells.len() % 7 != 0 {
    cells.push(blank_cell());
  }

  let weeks: Vec<Vec<CalendarCell>> = cells.chunks(7).map(|c| c.to_vec()).collect();

  CalendarMonth {
    year,
    month_name: month_name(month),
    weeks,
  }
}

fn blank_cell() -> CalendarCell {
  CalendarCell {
    day: None,
    date_str: String::new(),
    has_data: false,
    is_selected: false,
    is_today: false,
  }
}

fn month_name(m: u32) -> String {
  match m {
    1 => "January",
    2 => "February",
    3 => "March",
    4 => "April",
    5 => "May",
    6 => "June",
    7 => "July",
    8 => "August",
    9 => "September",
    10 => "October",
    11 => "November",
    12 => "December",
    _ => "",
  }
  .to_string()
}

fn render(tmpl: &Tera, ctx: tera::Context) -> HttpResponse {
  match tmpl.render("movers.html", &ctx) {
    Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
    Err(e) => {
      tracing::error!("Template render error: {}", e);
      HttpResponse::InternalServerError().body(format!("Template error: {}", e))
    }
  }
}