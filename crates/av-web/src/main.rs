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
use actix_web::{App, HttpServer, web};
use av_client::AlphaVantageClient;
use av_core::Config;
use av_database_postgres::DatabaseContext;
use std::env;
use std::sync::Arc;
use tera::Tera;
use tracing::{info, warn};

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

  // ── AlphaVantage client ──────────────────────────────────────────────
  // Optional: if ALPHA_VANTAGE_API_KEY is not set, /load will return a
  // friendly error but the rest of the site still works.
  let client: Option<Arc<AlphaVantageClient>> = match Config::from_env() {
    Ok(cfg) => match AlphaVantageClient::new(cfg) {
      Ok(c) => {
        info!("AlphaVantage client initialised");
        Some(Arc::new(c))
      }
      Err(e) => {
        warn!("AlphaVantageClient init failed ({}); /load disabled", e);
        None
      }
    },
    Err(e) => {
      warn!("Config::from_env failed ({}); /load disabled", e);
      None
    }
  };
  let client = web::Data::new(client);

  // ── Templates ────────────────────────────────────────────────────────
  let template_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
  let tera = Tera::new(template_dir).unwrap_or_else(|e| {
    eprintln!("Template parsing error: {}", e);
    std::process::exit(1);
  });
  let tera = web::Data::new(tera);

  // ── Server ───────────────────────────────────────────────────────────
  let port: u16 = env::var("AV_WEB_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

  info!("Starting server on 0.0.0.0:{}", port);

  HttpServer::new(move || {
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

    App::new()
      .app_data(db.clone())
      .app_data(tera.clone())
      .app_data(client.clone())
      .service(fs::Files::new("/static", static_dir).show_files_listing())
      .route("/", web::get().to(routes::search::index))
      .route("/search", web::get().to(routes::search::search))
      .route("/news", web::get().to(routes::news::news))
      .route("/load", web::get().to(routes::load::load))
      .route("/load", web::post().to(routes::load::apply))
  })
  .bind(("0.0.0.0", port))?
  .run()
  .await
}
