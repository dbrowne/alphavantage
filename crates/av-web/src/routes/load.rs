/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Missing-symbol / overview loader route.
//!
//! Flow:
//!  - `GET  /load?q=apple` — calls AlphaVantage `SYMBOL_SEARCH`, looks each
//!    match up in the local `symbols` table, and renders a checkbox list
//!    showing per-row status (not-in-db / no-overview / loaded).
//!  - `POST /load` — applies the user's selections:
//!     - `add_symbol[]=AAPL` → allocate a new SID and insert a `symbols` row
//!     - `load_overview[]=AAPL` → fetch `OVERVIEW` from AlphaVantage and
//!       upsert into `overviews` + `overviewexts`, flipping `symbols.overview`
//!       to `true`.
//!
//! Per the agreed UX, "add symbol" is a *two-step* operation: it only inserts
//! the `symbols` row. The user must then re-submit the form to load the
//! overview for the now-present symbol.

use actix_web::{HttpResponse, web};
use av_client::AlphaVantageClient;
use av_core::types::market::{SecurityIdentifier, SecurityType, normalize_alpha_region};
use av_database_postgres::models::security::{
  NewOverviewOwned, NewOverviewextOwned, NewSymbol, Symbol,
};
use av_database_postgres::{DatabaseContext, OverviewRepository};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tera::Tera;

// ─── Inlined SidGenerator ─────────────────────────────────────────────────
//
// Copied from av-cli/src/commands/load/sid_generator.rs because that crate
// is a binary and not depended on by av-web. The logic is small enough to
// duplicate; if a third caller appears, this should be promoted to a shared
// crate (av-database-postgres or a new av-sid crate).

/// Per-type monotonic SID allocator, seeded from the current `symbols` table.
struct SidGenerator {
  next_raw_ids: HashMap<SecurityType, u32>,
}

impl SidGenerator {
  fn new(conn: &mut diesel::pg::PgConnection) -> diesel::QueryResult<Self> {
    use av_database_postgres::schema::symbols::dsl::*;

    let sids: Vec<i64> = symbols.select(sid).load(conn)?;
    let mut max_raw: HashMap<SecurityType, u32> = HashMap::new();
    for v in sids {
      if let Some(id) = SecurityIdentifier::decode(v) {
        let cur = max_raw.entry(id.security_type).or_insert(0);
        if id.raw_id > *cur {
          *cur = id.raw_id;
        }
      }
    }
    let next_raw_ids = max_raw.into_iter().map(|(k, v)| (k, v + 1)).collect();
    Ok(Self { next_raw_ids })
  }

  fn next_sid(&mut self, st: SecurityType) -> i64 {
    let raw = self.next_raw_ids.entry(st).or_insert(1);
    let sid = SecurityType::encode(st, *raw);
    *raw += 1;
    sid
  }
}

// ─── AlphaVantage string parsers (mirrors av-cli/.../overviews.rs:530) ────

fn clean_string(v: &str) -> String {
  if v.is_empty() || v == "None" || v == "-" { String::new() } else { v.to_string() }
}

fn parse_i64(v: &str) -> Option<i64> {
  if v.is_empty() || v == "None" || v == "-" {
    return None;
  }
  v.parse::<i64>().ok()
}

fn parse_f32(v: &str) -> Option<f32> {
  if v.is_empty() || v == "None" || v == "-" {
    return None;
  }
  v.parse::<f32>().ok()
}

fn parse_date(v: &str) -> Option<NaiveDate> {
  if v.is_empty() || v == "None" || v == "-" {
    return None;
  }
  NaiveDate::parse_from_str(v, "%Y-%m-%d").ok()
}

fn default_date() -> NaiveDate {
  NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()
}

// ─── Symbol lookup helpers (sec_type-aware) ───────────────────────────────
//
// IMPORTANT: SymbolRepository::find_by_symbol(symbol) is *non-deterministic*
// for tickers that exist as more than one SecurityType (e.g. ticker "C"
// exists both as Citigroup Equity and as Chainbase Cryptocurrency). It does
// `.first()` with no ORDER BY, so it picks an arbitrary row. We MUST
// disambiguate by the AlphaVantage match's `stock_type` to avoid loading
// one security's overview against another security's SID — that's an
// integrity bug, not a UX issue.

/// Canonical `symbols.sec_type` string for a given AlphaVantage stock_type
/// (e.g. "Equity" → "Equity", "Mutual Fund" → "MutualFund"). Matches the
/// CLI's `format!("{:?}", SecurityType::...)` convention used at write-time.
fn av_sec_type_str(av_stock_type: &str) -> String {
  format!("{:?}", SecurityType::from_alpha_vantage(av_stock_type))
}

type DbPool = av_database_postgres::repository::DbPool;

/// Find the `symbols` row matching BOTH (symbol, sec_type). Returns `None`
/// when no such row exists — including the case where rows exist for the
/// same ticker under a different `sec_type`.
async fn find_symbol_by_type(
  pool: DbPool,
  symbol: String,
  sec_type: String,
) -> Result<Option<Symbol>, String> {
  tokio::task::spawn_blocking(move || -> Result<Option<Symbol>, String> {
    use av_database_postgres::schema::symbols;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let row: Option<Symbol> = symbols::table
      .filter(symbols::symbol.eq(&symbol))
      .filter(symbols::sec_type.eq(&sec_type))
      .first::<Symbol>(&mut conn)
      .optional()
      .map_err(|e| e.to_string())?;
    Ok(row)
  })
  .await
  .map_err(|e| format!("task join: {e}"))?
}

// ─── View model ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoadQuery {
  pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MatchRow {
  symbol: String,
  name: String,
  stock_type: String,
  region: String,
  currency: String,
  match_score: String,
  /// "not_in_db" | "in_db_no_overview" | "loaded"
  status: String,
  existing_sid: Option<i64>,
  raw_json: String,
}

/// Build the per-row view model for a `SymbolMatch`, including DB status.
///
/// Disambiguates by `(symbol, sec_type)` — looking up by symbol alone is
/// non-deterministic when multiple security types share the ticker.
async fn build_match_row(m: &av_models::SymbolMatch, pool: &DbPool) -> MatchRow {
  let raw_json = serde_json::to_string_pretty(m).unwrap_or_default();
  let sec_type_str = av_sec_type_str(&m.stock_type);

  let (status, existing_sid) =
    match find_symbol_by_type(pool.clone(), m.symbol.clone(), sec_type_str).await {
      Ok(Some(s)) => {
        if s.overview {
          ("loaded".to_string(), Some(s.sid))
        } else {
          ("in_db_no_overview".to_string(), Some(s.sid))
        }
      }
      Ok(None) => ("not_in_db".to_string(), None),
      Err(e) => {
        tracing::error!("find_symbol_by_type({},{}) failed: {}", m.symbol, m.stock_type, e);
        ("not_in_db".to_string(), None)
      }
    };

  MatchRow {
    symbol: m.symbol.clone(),
    name: m.name.clone(),
    stock_type: m.stock_type.clone(),
    region: m.region.clone(),
    currency: m.currency.clone(),
    match_score: m.match_score.clone(),
    status,
    existing_sid,
    raw_json,
  }
}

// ─── GET /load ────────────────────────────────────────────────────────────

/// GET /load and GET /load?q=keyword
pub async fn load(
  tmpl: web::Data<Tera>,
  db: web::Data<DatabaseContext>,
  client: web::Data<Option<Arc<AlphaVantageClient>>>,
  query: web::Query<LoadQuery>,
) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "load");

  let q = query.q.as_deref().unwrap_or("").trim().to_string();
  ctx.insert("query", &q);

  if q.is_empty() {
    ctx.insert("searched", &false);
    return render(tmpl.get_ref(), ctx);
  }
  ctx.insert("searched", &true);

  // API key not configured: render an explanatory empty state.
  let av = match client.as_ref().as_ref() {
    Some(c) => c.clone(),
    None => {
      ctx.insert("api_unavailable", &true);
      ctx.insert("matches", &Vec::<MatchRow>::new());
      return render(tmpl.get_ref(), ctx);
    }
  };

  let matches = run_search_and_status(&av, &db, &q).await;
  match matches {
    Ok(rows) => ctx.insert("matches", &rows),
    Err(e) => {
      tracing::error!("symbol_search for '{}' failed: {}", q, e);
      ctx.insert("search_error", &e);
      ctx.insert("matches", &Vec::<MatchRow>::new());
    }
  }
  render(tmpl.get_ref(), ctx)
}

async fn run_search_and_status(
  av: &AlphaVantageClient,
  db: &DatabaseContext,
  q: &str,
) -> Result<Vec<MatchRow>, String> {
  let search = av.time_series().symbol_search(q).await.map_err(|e| e.to_string())?;

  let pool = db.pool().clone();

  let mut rows = Vec::with_capacity(search.best_matches.len());
  for m in &search.best_matches {
    rows.push(build_match_row(m, &pool).await);
  }
  Ok(rows)
}

// ─── POST /load ───────────────────────────────────────────────────────────

/// POST /load — body is application/x-www-form-urlencoded with repeated
/// `add_symbol` and `load_overview` keys. We parse pairs manually so duplicate
/// keys collect into a Vec (serde_urlencoded into a struct can't do that).
pub async fn apply(
  tmpl: web::Data<Tera>,
  db: web::Data<DatabaseContext>,
  client: web::Data<Option<Arc<AlphaVantageClient>>>,
  body: web::Bytes,
) -> HttpResponse {
  let pairs: Vec<(String, String)> = match serde_urlencoded::from_bytes(body.as_ref()) {
    Ok(p) => p,
    Err(e) => return HttpResponse::BadRequest().body(format!("Form parse error: {}", e)),
  };

  let q = pairs.iter().find(|(k, _)| k == "q").map(|(_, v)| v.clone()).unwrap_or_default();

  // Form values are encoded as `SYMBOL|AV_STOCK_TYPE` (e.g. "C|Equity")
  // so the POST handler can disambiguate between same-ticker rows of
  // different security types. Anything not matching that shape is dropped
  // with a warning rather than silently coerced.
  let parse_pair = |raw: &str| -> Option<(String, String)> {
    let mut it = raw.splitn(2, '|');
    let sym = it.next()?.trim().to_string();
    let typ = it.next()?.trim().to_string();
    if sym.is_empty() || typ.is_empty() { None } else { Some((sym, typ)) }
  };
  let add_symbols: Vec<(String, String)> = pairs
    .iter()
    .filter(|(k, _)| k == "add_symbol")
    .filter_map(|(_, v)| {
      let p = parse_pair(v);
      if p.is_none() {
        tracing::warn!("Dropping malformed add_symbol value: {:?}", v);
      }
      p
    })
    .collect();
  let load_overviews: Vec<(String, String)> = pairs
    .iter()
    .filter(|(k, _)| k == "load_overview")
    .filter_map(|(_, v)| {
      let p = parse_pair(v);
      if p.is_none() {
        tracing::warn!("Dropping malformed load_overview value: {:?}", v);
      }
      p
    })
    .collect();

  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "load");
  ctx.insert("query", &q);
  ctx.insert("searched", &true);

  let mut flashes: Vec<String> = Vec::new();
  let mut errors: Vec<String> = Vec::new();
  let mut raw_overviews: HashMap<String, String> = HashMap::new();

  let av = match client.as_ref().as_ref() {
    Some(c) => c.clone(),
    None => {
      ctx.insert("api_unavailable", &true);
      ctx.insert("matches", &Vec::<MatchRow>::new());
      ctx.insert("flashes", &flashes);
      ctx.insert(
        "errors",
        &vec!["AlphaVantage client not configured (missing ALPHA_VANTAGE_API_KEY)."],
      );
      ctx.insert("raw_overviews", &raw_overviews);
      return render(tmpl.get_ref(), ctx);
    }
  };

  // Re-run the search so we can recover stock_type/region/currency for
  // each selected "add" symbol without trusting hidden form fields.
  let search_matches = if !add_symbols.is_empty() {
    match av.time_series().symbol_search(&q).await {
      Ok(s) => s.best_matches,
      Err(e) => {
        errors.push(format!("symbol_search failed: {e}"));
        Vec::new()
      }
    }
  } else {
    Vec::new()
  };

  // ── 1) Insert new symbols ──────────────────────────────────────────────
  if !add_symbols.is_empty() {
    match insert_new_symbols(&db, &search_matches, &add_symbols).await {
      Ok((added, skipped)) => {
        for s in &added {
          flashes.push(format!("Added symbol {s} to the symbols table."));
        }
        for (s, why) in &skipped {
          flashes.push(format!("Skipped {s}: {why}"));
        }
      }
      Err(e) => errors.push(format!("Symbol insert failed: {e}")),
    }
  }

  // ── 2) Load overviews ──────────────────────────────────────────────────
  for (sym, av_type) in &load_overviews {
    match load_one_overview(&av, &db, sym, av_type).await {
      Ok(raw) => {
        flashes.push(format!("Loaded overview for {sym} ({av_type})."));
        raw_overviews.insert(sym.clone(), raw);
      }
      Err(e) => errors.push(format!("{sym} ({av_type}) overview load failed: {e}")),
    }
  }

  // ── 3) Re-render with fresh status ─────────────────────────────────────
  match run_search_and_status(&av, &db, &q).await {
    Ok(rows) => ctx.insert("matches", &rows),
    Err(e) => {
      errors.push(format!("re-fetching results failed: {e}"));
      ctx.insert("matches", &Vec::<MatchRow>::new());
    }
  }
  ctx.insert("flashes", &flashes);
  ctx.insert("errors", &errors);
  ctx.insert("raw_overviews", &raw_overviews);
  render(tmpl.get_ref(), ctx)
}

/// Insert selected symbols as a single transaction. Each entry in
/// `add_symbols` is `(ticker, AlphaVantage stock_type)` — both are
/// authoritative for the (symbol, sec_type) identity, so we can correctly
/// add a new ticker even when other rows already share its symbol under a
/// different security type.
///
/// Returns `(added, skipped)` where `added` lists the tickers that landed
/// in the DB and `skipped` carries a per-row reason.
async fn insert_new_symbols(
  db: &DatabaseContext,
  search_matches: &[av_models::SymbolMatch],
  add_symbols: &[(String, String)],
) -> Result<(Vec<String>, Vec<(String, String)>), String> {
  // For each (symbol, av_type), find the matching AV result. Skip if AV's
  // current response no longer carries that pair — guards against the user
  // pressing back/forward then submitting a stale form.
  let selected: Vec<av_models::SymbolMatch> = add_symbols
    .iter()
    .filter_map(|(sym, av_type)| {
      search_matches.iter().find(|m| &m.symbol == sym && &m.stock_type == av_type).cloned()
    })
    .collect();

  if selected.is_empty() {
    return Ok((vec![], vec![]));
  }

  let pool_outer = db.pool().clone();

  // One spawn_blocking covers: build SidGenerator from current DB state,
  // then insert all selected rows in a single transaction.
  tokio::task::spawn_blocking(move || -> Result<(Vec<String>, Vec<(String, String)>), String> {
    use av_database_postgres::schema::symbols;

    let mut conn = pool_outer.get().map_err(|e| e.to_string())?;
    let mut gen = SidGenerator::new(&mut conn).map_err(|e| e.to_string())?;

    let mut added: Vec<String> = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();
    let now: NaiveDateTime = Utc::now().naive_utc();

    conn
      .transaction::<_, diesel::result::Error, _>(|conn| {
        for m in &selected {
          let st = SecurityType::from_alpha_vantage(&m.stock_type);
          let sec_type_str = format!("{:?}", st);

          // Existence check disambiguated by (symbol, sec_type). A row for
          // the same ticker under a different type does NOT block this insert.
          let existing: Option<Symbol> = symbols::table
            .filter(symbols::symbol.eq(&m.symbol))
            .filter(symbols::sec_type.eq(&sec_type_str))
            .first::<Symbol>(conn)
            .optional()?;
          if existing.is_some() {
            skipped.push((m.symbol.clone(), format!("already present as {sec_type_str}")));
            continue;
          }

          let sid = gen.next_sid(st);
          let priority: i32 = 1;
          let overview = false;
          let intraday = false;
          let summary = false;

          // VARCHAR(10) constraints — map AlphaVantage's long-form region
          // ("United States" → "USA" etc.) before insert.
          let region = normalize_alpha_region(&m.region);

          let new = NewSymbol {
            sid: &sid,
            symbol: &m.symbol,
            priority: &priority,
            name: &m.name,
            sec_type: &sec_type_str,
            region: &region,
            currency: &m.currency,
            overview: &overview,
            intraday: &intraday,
            summary: &summary,
            c_time: &now,
            m_time: &now,
          };

          diesel::insert_into(symbols::table).values(&new).execute(conn)?;
          added.push(m.symbol.clone());
        }
        Ok(())
      })
      .map_err(|e| e.to_string())?;

    Ok((added, skipped))
  })
  .await
  .map_err(|e| format!("Task join error: {e}"))?
}

/// Fetch overview from AlphaVantage and upsert into `overviews` + `overviewexts`.
/// Returns the pretty-printed JSON of the raw response on success.
///
/// `av_stock_type` is AlphaVantage's `stock_type` for the match the user
/// selected. This MUST be used (not just the symbol) to disambiguate
/// `symbols` rows that share the ticker across security types — otherwise
/// we risk writing the fetched overview against the wrong SID.
async fn load_one_overview(
  av: &AlphaVantageClient,
  db: &DatabaseContext,
  symbol: &str,
  av_stock_type: &str,
) -> Result<String, String> {
  let sec_type_str = av_sec_type_str(av_stock_type);
  let pool = db.pool().clone();
  let row = find_symbol_by_type(pool, symbol.to_string(), sec_type_str.clone()).await?.ok_or_else(
    || format!("no symbols row matches ({symbol}, sec_type={sec_type_str}) — add it first"),
  )?;
  let sid = row.sid;

  let overview = av.fundamentals().company_overview(symbol).await.map_err(|e| e.to_string())?;
  let raw_json = serde_json::to_string_pretty(&overview).unwrap_or_default();

  let (new_ov, new_ext) = build_overview_records(sid, &overview);
  db.overview_repository()
    .batch_save_overviews(&[(new_ov, new_ext)])
    .await
    .map_err(|e| e.to_string())?;
  Ok(raw_json)
}

/// Build the `(NewOverviewOwned, NewOverviewextOwned)` pair from an
/// AlphaVantage `CompanyOverview` response. Mirrors the parsing logic in
/// `av-cli/.../overviews.rs::save_overviews_to_db`.
fn build_overview_records(
  sid: i64,
  ov: &av_models::CompanyOverview,
) -> (NewOverviewOwned, NewOverviewextOwned) {
  let now: NaiveDateTime = Utc::now().naive_utc();
  let latest_quarter = parse_date(&ov.latest_quarter).unwrap_or_else(default_date);

  let new_ov = NewOverviewOwned {
    sid,
    symbol: ov.symbol.clone(),
    name: clean_string(&ov.name),
    description: clean_string(&ov.description),
    cik: clean_string(&ov.cik),
    exchange: clean_string(&ov.exchange),
    currency: clean_string(&ov.currency),
    country: clean_string(&ov.country),
    sector: clean_string(&ov.sector),
    industry: clean_string(&ov.industry),
    address: clean_string(&ov.address),
    fiscal_year_end: clean_string(&ov.fiscal_year_end),
    latest_quarter,
    market_capitalization: parse_i64(&ov.market_capitalization).unwrap_or(0),
    ebitda: parse_i64(&ov.ebitda).unwrap_or(0),
    pe_ratio: parse_f32(&ov.pe_ratio).unwrap_or(0.0),
    peg_ratio: parse_f32(&ov.peg_ratio).unwrap_or(0.0),
    book_value: parse_f32(&ov.book_value).unwrap_or(0.0),
    dividend_per_share: parse_f32(&ov.dividend_per_share).unwrap_or(0.0),
    dividend_yield: parse_f32(&ov.dividend_yield).unwrap_or(0.0),
    eps: parse_f32(&ov.eps).unwrap_or(0.0),
    c_time: now,
    m_time: now,
  };

  let new_ext = NewOverviewextOwned {
    sid,
    revenue_per_share_ttm: parse_f32(&ov.revenue_per_share_ttm).unwrap_or(0.0),
    profit_margin: parse_f32(&ov.profit_margin).unwrap_or(0.0),
    operating_margin_ttm: parse_f32(&ov.operating_margin_ttm).unwrap_or(0.0),
    return_on_assets_ttm: parse_f32(&ov.return_on_assets_ttm).unwrap_or(0.0),
    return_on_equity_ttm: parse_f32(&ov.return_on_equity_ttm).unwrap_or(0.0),
    revenue_ttm: parse_i64(&ov.revenue_ttm).unwrap_or(0),
    gross_profit_ttm: parse_i64(&ov.gross_profit_ttm).unwrap_or(0),
    diluted_eps_ttm: parse_f32(&ov.diluted_eps_ttm).unwrap_or(0.0),
    quarterly_earnings_growth_yoy: parse_f32(&ov.quarterly_earnings_growth_yoy).unwrap_or(0.0),
    quarterly_revenue_growth_yoy: parse_f32(&ov.quarterly_revenue_growth_yoy).unwrap_or(0.0),
    analyst_target_price: parse_f32(&ov.analyst_target_price).unwrap_or(0.0),
    trailing_pe: parse_f32(&ov.trailing_pe).unwrap_or(0.0),
    forward_pe: parse_f32(&ov.forward_pe).unwrap_or(0.0),
    price_to_sales_ratio_ttm: parse_f32(&ov.price_to_sales_ratio_ttm).unwrap_or(0.0),
    price_to_book_ratio: parse_f32(&ov.price_to_book_ratio).unwrap_or(0.0),
    ev_to_revenue: parse_f32(&ov.ev_to_revenue).unwrap_or(0.0),
    ev_to_ebitda: parse_f32(&ov.ev_to_ebitda).unwrap_or(0.0),
    beta: parse_f32(&ov.beta).unwrap_or(0.0),
    week_high_52: parse_f32(&ov.week_52_high).unwrap_or(0.0),
    week_low_52: parse_f32(&ov.week_52_low).unwrap_or(0.0),
    day_moving_average_50: parse_f32(&ov.day_50_moving_average).unwrap_or(0.0),
    day_moving_average_200: parse_f32(&ov.day_200_moving_average).unwrap_or(0.0),
    shares_outstanding: parse_i64(&ov.shares_outstanding).unwrap_or(0),
    dividend_date: parse_date(&ov.dividend_date),
    ex_dividend_date: parse_date(&ov.ex_dividend_date),
    c_time: now,
    m_time: now,
  };

  (new_ov, new_ext)
}

fn render(tmpl: &Tera, ctx: tera::Context) -> HttpResponse {
  match tmpl.render("load.html", &ctx) {
    Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
    Err(e) => {
      tracing::error!("Template render error: {}", e);
      HttpResponse::InternalServerError().body(format!("Template error: {}", e))
    }
  }
}
