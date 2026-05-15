/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! News-sources route — lists the top publishers by article count with
//! canonical (domain-based) links. Pure DB read; no AlphaVantage calls.

use actix_web::{web, HttpResponse};
use av_api::queries::top_news_sources;
use av_database_postgres::DatabaseContext;
use tera::Tera;

/// Maximum sources displayed. AlphaVantage ingests from a few dozen
/// publishers regularly; 200 is well above that ceiling.
const SOURCES_LIMIT: i64 = 200;

/// GET /sources — top publishers by total article count.
pub async fn sources(tmpl: web::Data<Tera>, db: web::Data<DatabaseContext>) -> HttpResponse {
  let mut ctx = tera::Context::new();
  ctx.insert("active_page", "sources");

  let rows = match top_news_sources(&db, SOURCES_LIMIT).await {
    Ok(r) => r,
    Err(e) => {
      tracing::error!("top_news_sources failed: {}", e);
      Vec::new()
    }
  };

  ctx.insert("sources_count", &rows.len());
  ctx.insert("sources", &rows);
  render(tmpl.get_ref(), ctx)
}

fn render(tmpl: &Tera, ctx: tera::Context) -> HttpResponse {
  match tmpl.render("sources.html", &ctx) {
    Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
    Err(e) => {
      tracing::error!("Template render error: {}", e);
      HttpResponse::InternalServerError().body(format!("Template error: {}", e))
    }
  }
}