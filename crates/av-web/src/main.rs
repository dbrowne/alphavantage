/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Alpha Vantage Web UI server.
//!
//! Starts an Actix-web server on `0.0.0.0:8080` (configurable via `AV_WEB_PORT`)
//! with a TimescaleDB connection pool and Tera template engine.
//!
//! # Environment variables
//!
//! | Variable       | Default                                               | Description           |
//! |----------------|-------------------------------------------------------|-----------------------|
//! | `DATABASE_URL` | `postgresql://ts_user:dev_pw@localhost:6433/sec_master`| PostgreSQL URL        |
//! | `AV_WEB_PORT`  | `8080`                                                | HTTP listen port      |

mod routes;

use actix_files as fs;
use actix_web::{web, App, HttpServer};
use av_database_postgres::DatabaseContext;
use std::env;
use tera::Tera;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  // ── Logging ──────────────────────────────────────────────────────────
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,av_web=debug")),
    )
    .init();

  dotenvy::dotenv().ok();

  // ── Database ─────────────────────────────────────────────────────────
  let database_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "postgresql://ts_user:dev_pw@localhost:6433/sec_master".to_string());

  let db = DatabaseContext::new(&database_url).unwrap_or_else(|e| {
    eprintln!("Failed to connect to database: {}", e);
    eprintln!("  URL: {}", database_url);
    std::process::exit(1);
  });

  info!("Connected to database");
  let db = web::Data::new(db);

  // ── Templates ────────────────────────────────────────────────────────
  let template_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
  let tera = Tera::new(template_dir).unwrap_or_else(|e| {
    eprintln!("Template parsing error: {}", e);
    std::process::exit(1);
  });
  let tera = web::Data::new(tera);

  // ── Server ───────────────────────────────────────────────────────────
  let port: u16 = env::var("AV_WEB_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8080);

  info!("Starting server on 0.0.0.0:{}", port);

  HttpServer::new(move || {
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

    App::new()
      .app_data(db.clone())
      .app_data(tera.clone())
      .service(fs::Files::new("/static", static_dir).show_files_listing())
      .route("/", web::get().to(routes::search::index))
      .route("/search", web::get().to(routes::search::search))
  })
  .bind(("0.0.0.0", port))?
  .run()
  .await
}
