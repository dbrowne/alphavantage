/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Symbol lookup route handler.

use actix_web::{HttpResponse, web};
use av_api::queries::{
  format_market_cap, get_crypto_overview_row, get_overview_row, get_sids, get_symbol_row,
  security_snapshot_by_sid,
};
use av_database_postgres::DatabaseContext;
use serde::Deserialize;
use tera::Tera;

/// Query parameters for the search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
  /// Ticker symbol to search for.
  pub q: Option<String>,
  /// Optional SID override — when present, display this specific security
  /// instead of the highest-priority match.
  pub sid: Option<i64>,
}

/// GET / — renders the empty search form.
pub async fn index(tmpl: web::Data<Tera>) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "search");
  ctx.insert("query", "");
  ctx.insert("searched", &false);

  render(tmpl.get_ref(), ctx)
}

/// GET /search?q=BTC or /search?q=BTC&sid=12345
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

  // 1) All SID entries for this ticker, priority-ordered.
  let sid_entries = match get_sids(&db, &ticker).await {
    Ok(entries) => entries,
    Err(e) => {
      tracing::error!("get_sids failed for '{}': {}", ticker, e);
      Vec::new()
    }
  };

  // 2) Determine which SID to show details for.
  let selected_sid: Option<i64> = query.sid.or_else(|| sid_entries.first().map(|e| e.sid));
  ctx.insert("selected_sid", &selected_sid);

  // 3) Fetch snapshot and symbol row.
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

  // 4) Determine if this is a cryptocurrency.
  let is_crypto = symbol_row.as_ref().map(|s| s.sec_type == "Cryptocurrency").unwrap_or(false);
  ctx.insert("is_crypto", &is_crypto);

  // 5) Fetch the appropriate overview: crypto_overview_basic for crypto,
  //    overviews for everything else.
  let overview_row = if !is_crypto {
    if let Some(sid) = selected_sid {
      match get_overview_row(&db, sid).await {
        Ok(row) => row,
        Err(e) => {
          tracing::error!("get_overview_row failed for SID {}: {}", sid, e);
          None
        }
      }
    } else {
      None
    }
  } else {
    None
  };

  let crypto_overview = if is_crypto {
    if let Some(sid) = selected_sid {
      match get_crypto_overview_row(&db, sid).await {
        Ok(row) => row,
        Err(e) => {
          tracing::error!("get_crypto_overview_row failed for SID {}: {}", sid, e);
          None
        }
      }
    } else {
      None
    }
  } else {
    None
  };

  // Pre-format market cap values.
  let market_cap_fmt = if is_crypto {
    crypto_overview.as_ref().and_then(|c| c.market_cap).map(format_market_cap).unwrap_or_default()
  } else {
    snapshot.as_ref().and_then(|s| s.market_cap).map(format_market_cap).unwrap_or_default()
  };

  let ebitda_fmt = overview_row.as_ref().map(|o| format_market_cap(o.ebitda)).unwrap_or_default();

  ctx.insert("sid_entries", &sid_entries);
  ctx.insert("snapshot", &snapshot);
  ctx.insert("symbol_row", &symbol_row);
  ctx.insert("overview_row", &overview_row);
  ctx.insert("crypto_overview", &crypto_overview);
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
