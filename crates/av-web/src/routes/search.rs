/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Symbol lookup route handler.

use actix_web::{web, HttpResponse};
use av_api::queries::{get_sids, security_snapshot};
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

  // Fetch all SID entries for this ticker (handles duplicates).
  let sid_entries = match get_sids(&db, &ticker).await {
    Ok(entries) => entries,
    Err(e) => {
      tracing::error!("get_sids failed for '{}': {}", ticker, e);
      Vec::new()
    }
  };

  // Fetch the full snapshot for the best (first) match.
  let snapshot = match security_snapshot(&db, &ticker).await {
    Ok(snap) => snap,
    Err(e) => {
      tracing::error!("security_snapshot failed for '{}': {}", ticker, e);
      None
    }
  };

  ctx.insert("sid_entries", &sid_entries);
  ctx.insert("snapshot", &snapshot);

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
