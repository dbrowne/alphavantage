# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **av-cli**: New `coins-market` loader for CoinGecko `/coins/markets` endpoint
  - Creates new cryptocurrency symbols with proper SID generation
  - Links existing symbols by matching (symbol, name) to avoid duplicates
  - Updates `crypto_api_map` with CoinGecko ID mappings
  - Populates `crypto_overview_basic` and `crypto_overview_metrics` tables
  - Supports response caching with configurable TTL
  - CLI options: `--pages`, `--per-page`, `--start-page`, `--update-only`, `--force-refresh`, `--dry-run`

- **av-database-postgres**: Migration to expand NUMERIC precision for crypto tables
  - Changed from `NUMERIC(20,8)` to `NUMERIC(38,8)` to prevent overflow with large values
  - Affected tables: `crypto_overview_basic`, `crypto_overview_metrics`
  - Recreated dependent views: `crypto_overviews`, `crypto_full_view`

- **av-database-postgres**: Migration to add missing CoinGecko market fields
  - `crypto_overview_basic`: `image_url`, `market_cap_rank_rehyp`
  - `crypto_overview_metrics`: `high_24h`, `low_24h`, `market_cap_change_24h`, `market_cap_change_pct_24h`

### Changed
- **av-cli**: Refactored `coins_market.rs` to use typed `LoaderError` instead of `anyhow`
  - All database operations use explicit `map_err` with contextual messages
  - API errors include provider name and status codes
  - Failed coins are logged and skipped instead of aborting the entire batch

- **av-loaders**: Standardized `api_source` to 'CoinGecko' (mixed case) across codebase
  - Updated `markets_loader.rs` SQL queries
  - Updated `cache.rs` default api_source

### Fixed
- **av-cli**: Fixed N+1 query performance issue in intraday price loader
  - `get_latest_timestamps` now uses batched `GROUP BY` queries instead of per-symbol queries
  - Reduced ~5400 sequential queries to ~11 batched queries (500 SIDs per batch)
  - Timestamp retrieval time reduced from ~55 seconds to ~4 seconds

- **av-cli**: Fixed `crypto_api_map` update bug in `coins_market` loader
  - `update_market_data` now filters by `api_id` in addition to `sid`
  - Previously could update wrong entry when multiple api_ids existed for same SID
  - Prevents rank from being incorrectly assigned to wrong CoinGecko ID

- **av-cli**: Fixed duplicate symbol creation in `coins_market` loader
  - Now checks existing symbols by (symbol, name) before creating new ones
  - Links to existing symbol instead of creating duplicate
  - Adds `crypto_api_map` entry for the CoinGecko ID

## [0.1.1] - 2025-02-03

### Changed
- **crypto-loaders**: Consolidated error types with structured context
  - `RateLimitExceeded` now includes `provider` and optional `retry_after_secs` fields
  - `ApiError`, `InvalidResponse`, `ServerError`, `AccessDenied` now include `provider` field for source attribution
  - Removed duplicate variants (`MissingAPIKey`, `InvalidAPIKey`, `CoinGeckoEndpoint`)
- **av-loaders**: Improved error conversion from `CryptoLoaderError` to `LoaderError`
  - Rate limit retry timing now preserved (previously hardcoded to 60s)
  - All error variants handled explicitly (removed catch-all conversion)
  - Provider context included in error messages
- **av-database-postgres**: Fixed async trait warnings in `CacheRepositoryExt`
  - Added `#[allow(async_fn_in_trait)]` with documentation explaining the rationale
  - Extension trait is internal-only with default implementations

### Added
- **loader-base**: New shared crate with loader abstractions (extracted from av-loaders)
  - `CacheableConfig` trait for consistent cache configuration across loaders
  - `ConcurrentLoader` for semaphore-based concurrency management
  - `LoaderStatistics` for thread-safe statistics tracking (cache hits, API calls, errors)
  - `ProgressManager` and `ProgressStyle` for consistent progress bar creation
  - Can now be used by both `av-loaders` and `crypto-loaders`

### Refactored
- **av-loaders**: Migrated all loaders to use `loader-base` abstractions
  - `OverviewLoader`: Uses `ConcurrentLoader`, `CacheableConfig`, `LoaderStatistics`, `ProgressManager`
  - `SecurityLoader`: Uses `ConcurrentLoader`, `CacheableConfig`, `LoaderStatistics`, `ProgressManager`
  - `TopMoversLoader`: Implements `CacheableConfig` for consistent cache behavior
  - `SummaryPriceLoader`: Uses `ConcurrentLoader`, `CacheableConfig`, `ProgressManager`
  - `IntradayPriceLoader`: Uses `ConcurrentLoader`, `CacheableConfig`, `ProgressManager`
  - `NewsLoader`: Implements `CacheableConfig` for consistent cache behavior
- **crypto-loaders**: Migrated `CoinGeckoDetailsLoader` to use `loader-base` abstractions
  - Uses `ConcurrentLoader` instead of manual semaphore management
  - Uses `LoaderStatistics` for atomic statistics tracking
  - Uses `ProgressManager` for progress bar creation



## [0.1.0] - 2025-01-15

### Added
- **av-core**: test cases:  error.rs, lib.rs  
- **av-loaders**: test cases: bath_processor, csv_processor, error, loader, process_tracker 
- **av-models**:  test cases: crypto
- minor cleanup: added CHANGELOG.md, LICENSE file corrected dependencies in cargo.tomls and format fixes
- **av-core**: Core types, configuration, and error handling for AlphaVantage API
- **av-models**: Data models for API responses with serde serialization
- **av-client**: Async HTTP client with rate limiting (75/min free, 600/min premium)
- **av-database-postgres**: TimescaleDB integration via Diesel ORM with async support
- **av-loaders**: ETL data loaders for securities, prices, news, and crypto
- **av-cli**: Command-line interface for data ingestion and queries
- Time series endpoints (intraday, daily, weekly, monthly)
- Fundamentals endpoints (overview, income statement, balance sheet, cash flow)
- Cryptocurrency and forex endpoints
- News sentiment analysis endpoint
- Top gainers/losers endpoint
- Batch processing with progress indicators
- Connection pooling with BB8