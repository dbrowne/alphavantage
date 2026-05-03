/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Symbol lookup route handler.

use actix_web::{web, HttpResponse};
use av_api::queries::{
  format_market_cap, get_overview_row, get_sids, get_symbol_row, security_snapshot,
};
use av_database_postgres::DatabaseContext;
use serde::Deserialize;
use tera::Tera;

/// Query parameters for the search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
  pub q: Option<String>,
}

/// GET / — renders the empty search form.
pub async fn index(tmpl: web::Data<Tera>) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "search");
  ctx.insert("query", "");
  ctx.insert("searched", &false);

  render(tmpl.get_ref(), ctx)
}

/// GET /search?q=AAPL — performs the lookup and renders results.
pub async fn search(
  tmpl: web::Data<Tera>,
  db: web::Data<DatabaseContext>,
  query: web::Query<SearchQuery>,
) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "search");

  let ticker = query.q.as_deref().unwrap_or("").trim().to_string();
  ctx.insert("query", &ticker);

  if ticker.is_empty() {
    ctx.insert("searched", &false);
    return render(tmpl.get_ref(), ctx);
  }

  ctx.insert("searched", &true);

  // 1) All SID entries for this ticker (handles crypto duplicates).
  let sid_entries = match get_sids(&db, &ticker).await {
    Ok(entries) => entries,
    Err(e) => {
      tracing::error!("get_sids failed for '{}': {}", ticker, e);
      Vec::new()
    }
  };

  // 2) Snapshot (joined view) for the best match.
  let snapshot = match security_snapshot(&db, &ticker).await {
    Ok(snap) => snap,
    Err(e) => {
      tracing::error!("security_snapshot failed for '{}': {}", ticker, e);
      None
    }
  };

  // 3) Full symbol row + overview row for the best match SID.
  let best_sid = snapshot.as_ref().map(|s| s.sid);

  let symbol_row = if let Some(sid) = best_sid {
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

  let overview_row = if let Some(sid) = best_sid {
    match get_overview_row(&db, sid).await {
      Ok(row) => row,
      Err(e) => {
        tracing::error!("get_overview_row failed for SID {}: {}", sid, e);
        None
      }
    }
  } else {
    None
  };

  // Pre-format market cap values for the template.
  let market_cap_fmt = snapshot
    .as_ref()
    .and_then(|s| s.market_cap)
    .map(format_market_cap)
    .unwrap_or_default();

  let ebitda_fmt = overview_row
    .as_ref()
    .map(|o| format_market_cap(o.ebitda))
    .unwrap_or_default();

  ctx.insert("sid_entries", &sid_entries);
  ctx.insert("snapshot", &snapshot);
  ctx.insert("symbol_row", &symbol_row);
  ctx.insert("overview_row", &overview_row);
  ctx.insert("market_cap_fmt", &market_cap_fmt);
  ctx.insert("ebitda_fmt", &ebitda_fmt);

  render(tmpl.get_ref(), ctx)
}

/// Render the search template, falling back to a plain-text error.
fn render(tmpl: &Tera, ctx: tera::Context) -> HttpResponse {
  match tmpl.render("search.html", &ctx) {
    Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
    Err(e) => {
      tracing::error!("Template render error: {}", e);
      HttpResponse::InternalServerError().body(format!("Template error: {}", e))
    }
  }
}
