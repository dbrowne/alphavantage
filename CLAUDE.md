# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AlphaVantage Rust Client - A high-performance async Rust implementation for fetching, storing, and analyzing financial market data from AlphaVantage, CoinGecko, and other sources. Features PostgreSQL/TimescaleDB integration, comprehensive crypto coverage, and ETL pipelines.

## Build Commands

```bash
# Build
cargo build                      # Debug build
cargo build --release            # Optimized release build
cargo build -p av-core           # Build specific crate

# Test
cargo test --verbose             # All tests
cargo test -p av-core            # Specific crate tests
cargo test -p av-client          # Client tests
cargo test -p av-loaders         # Loader tests
RUST_LOG=debug cargo test -- --nocapture  # With logging output

# Lint & Format
cargo fmt                        # Format code
cargo fmt -- --check             # Check formatting (CI)
cargo clippy                     # Run lints
cargo clippy --fix               # Auto-fix lint issues

# Database Migrations (from crates/av-database/postgres/)
diesel migration run             # Run pending migrations
diesel migration generate NAME   # Create new migration

# CLI binary
cargo run --bin av -- <command>  # Run CLI
cargo run --bin av -- load --help
```

## Architecture

**Workspace with 6 crates:**

```
av-core          → Foundation: types, config, traits, errors
    ↓
av-models        → Serializable API response types (serde)
    ↓
av-client        → Async HTTP client with rate limiting (reqwest, governor)
    ↓
├── av-database-postgres  → TimescaleDB/PostgreSQL via Diesel ORM (diesel-async, bb8)
└── av-loaders           → ETL data loaders for securities, prices, news, crypto
        ↓
    av-cli               → Command-line interface (clap)
```

**Key patterns:**
- All async via Tokio runtime
- Rate limiting with Governor crate (75 req/min free tier, 600 req/min premium)
- Connection pooling with BB8
- Repository pattern for database access
- `FuncType` enum defines all 27+ supported AlphaVantage API endpoints

## Configuration

**Required environment variables:**
- `ALPHA_VANTAGE_API_KEY` - AlphaVantage API key
- `DATABASE_URL` - PostgreSQL connection string (for DB operations)

**Optional:**
- `AV_RATE_LIMIT` - Requests/minute (default: 75)
- `AV_TIMEOUT_SECS` - Request timeout (default: 30)
- `COINGECKO_API_KEY`, `CMC_API_KEY` - Additional crypto data sources

See `DOT_env_EXAMPLE` for full configuration template.

## Crate Locations

| Crate | Path | Purpose |
|-------|------|---------|
| av-core | `crates/av-core/` | Core types, config, errors |
| av-models | `crates/av-models/` | API response models |
| av-client | `crates/av-client/` | HTTP client, endpoints |
| av-database-postgres | `crates/av-database/postgres/` | ORM, repositories, migrations |
| av-loaders | `crates/av-loaders/` | ETL loaders |
| av-cli | `crates/av-cli/` | CLI commands |

## Database

- PostgreSQL 15+ / TimescaleDB
- Diesel ORM with async support
- Migrations in `crates/av-database/postgres/migrations/`
- Schema defined in `crates/av-database/postgres/src/schema.rs`
- 33+ tables covering securities, prices, crypto, news/sentiment

**Local database setup:**
```bash
cd timescale_setup && make up    # Start TimescaleDB container
```

## CLI Commands

```bash
av load securities|overviews|intraday|daily|news|top_movers
av load crypto-mapping|crypto-metadata|crypto-markets|crypto-prices
av update crypto metadata-etl|intraday|details
av query ...
av sync ...
```

## Workspace Lints

Configured in root `Cargo.toml`:
- `unused_imports = "deny"`
- `unused_variables = "warn"`
- `dead_code = "warn"`

## Code Style

- Edition 2021, rustfmt with 2-space indentation
- Unix line endings
- Doc comments using `///` with examples