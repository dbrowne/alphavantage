/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! News article lookup route handler.
//!
//! Mirrors the symbol-lookup flow in [`search`](crate::routes::search):
//! the user submits a ticker (or a direct SID), we resolve it to a single
//! security, then fetch recent news via [`news_by_sid_recent`].

use actix_web::{HttpResponse, web};
use av_api::queries::{get_sids, get_symbol_row, news_by_sid_recent, security_snapshot_by_sid};
use av_database_postgres::DatabaseContext;
use serde::Deserialize;
use tera::Tera;

/// Default lookback window when `?days=` is omitted.
const DEFAULT_DAYS: i32 = 30;
/// Default article cap when `?limit=` is omitted.
const DEFAULT_LIMIT: i64 = 50;

/// Query parameters for the news endpoint.
#[derive(Debug, Deserialize)]
pub struct NewsQuery {
  /// Ticker symbol to search for (optional — `sid` alone also works).
  pub q: Option<String>,
  /// Direct SID lookup or disambiguation override.
  pub sid: Option<i64>,
  /// Lookback window in days. Defaults to [`DEFAULT_DAYS`].
  pub days: Option<i32>,
  /// Maximum articles returned. Defaults to [`DEFAULT_LIMIT`].
  pub limit: Option<i64>,
}

/// GET /news, /news?q=AAPL, /news?q=AAPL&sid=12345, /news?sid=12345
pub async fn news(
  tmpl: web::Data<Tera>,
  db: web::Data<DatabaseContext>,
  query: web::Query<NewsQuery>,
) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "news");

  let ticker = query.q.as_deref().unwrap_or("").trim().to_string();
  ctx.insert("query", &ticker);

  let days = query.days.filter(|&d| d > 0).unwrap_or(DEFAULT_DAYS);
  let limit = query.limit.filter(|&l| l > 0).unwrap_or(DEFAULT_LIMIT);
  ctx.insert("days", &days);
  ctx.insert("limit", &limit);

  // Empty form when neither q nor sid is supplied.
  if ticker.is_empty() && query.sid.is_none() {
    ctx.insert("searched", &false);
    ctx.insert("sid_entries", &Vec::<()>::new());
    return render(tmpl.get_ref(), ctx);
  }

  ctx.insert("searched", &true);

  // 1) When a ticker is supplied, look up its SID candidates so the
  //    disambiguation table can render. SID-only requests skip this.
  let sid_entries = if !ticker.is_empty() {
    match get_sids(&db, &ticker).await {
      Ok(entries) => entries,
      Err(e) => {
        tracing::error!("get_sids failed for '{}': {}", ticker, e);
        Vec::new()
      }
    }
  } else {
    Vec::new()
  };

  // 2) Pick the active SID: explicit ?sid= wins; otherwise top-priority
  //    match from the ticker lookup.
  let selected_sid: Option<i64> = query.sid.or_else(|| sid_entries.first().map(|e| e.sid));
  ctx.insert("selected_sid", &selected_sid);

  // 3) Snapshot + symbol row for the security header (best-effort —
  //    a SID-only deep-link with no symbols-table row still shows news).
  let snapshot = if let Some(sid) = selected_sid {
    match security_snapshot_by_sid(&db, sid).await {
      Ok(snap) => snap,
      Err(e) => {
        tracing::error!("security_snapshot_by_sid failed for SID {}: {}", sid, e);
        None
      }
    }
  } else {
    None
  };

  let symbol_row = if let Some(sid) = selected_sid {
    match get_symbol_row(&db, sid).await {
      Ok(row) => row,
      Err(e) => {
        tracing::error!("get_symbol_row failed for SID {}: {}", sid, e);
        None
      }
    }
  } else {
    None
  };

  // 4) Articles. We always paginate via days+limit so the page stays
  //    responsive even for popular SIDs.
  let articles = if let Some(sid) = selected_sid {
    match news_by_sid_recent(&db, sid, days, limit).await {
      Ok(rows) => rows,
      Err(e) => {
        tracing::error!("news_by_sid_recent failed for SID {}: {}", sid, e);
        Vec::new()
      }
    }
  } else {
    Vec::new()
  };

  ctx.insert("sid_entries", &sid_entries);
  ctx.insert("snapshot", &snapshot);
  ctx.insert("symbol_row", &symbol_row);
  ctx.insert("articles", &articles);
  ctx.insert("article_count", &articles.len());

  render(tmpl.get_ref(), ctx)
}

/// Render the news template, falling back to a plain-text error.
fn render(tmpl: &Tera, ctx: tera::Context) -> HttpResponse {
  match tmpl.render("news.html", &ctx) {
    Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
    Err(e) => {
      tracing::error!("Template render error: {}", e);
      HttpResponse::InternalServerError().body(format!("Template error: {}", e))
    }
  }
}
