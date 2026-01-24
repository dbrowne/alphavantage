# AlphaVantage Rust Client

[![Build Status](https://github.com/dbrowne/alphavantage/actions/workflows/rust.yaml/badge.svg)](https://github.com/dbrowne/alphavantage/actions/workflows/rust.yml)

A high-performance, async Rust client library and comprehensive data pipeline for financial market data. Built with a modular workspace architecture, it provides robust integration with AlphaVantage API, CoinGecko, CoinMarketCap, CoinPaprika, CoinCap, SosoValue, and other data sources, featuring advanced caching, TimescaleDB support, and comprehensive cryptocurrency coverage.

## Table of Contents

- [Features](#features)
- [Architecture Overview](#architecture-overview)
- [Project Structure](#project-structure)
- [Crate Documentation](#crate-documentation)
- [CLI Commands](#cli-commands)
- [Caching System](#caching-system)
- [Database Schema](#database-schema)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [Development](#development)
- [Security Advisories](#security-advisories)
- [License](#license)

---

## Features

### Core Capabilities

| Feature | Description |
|---------|-------------|
| **Multi-Source Data Integration** | Unified access to AlphaVantage, CoinGecko, CoinMarketCap, CoinPaprika, CoinCap, and SosoValue APIs |
| **Async/Await Architecture** | Built on Tokio runtime for maximum concurrency and throughput |
| **Unified Caching System** | CacheRepository pattern with configurable TTLs per data type, reducing API calls and costs |
| **TimescaleDB Integration** | Optimized time-series data storage with hypertables for efficient querying of historical data |
| **Comprehensive Crypto Support** | Enhanced metadata, social metrics, market data, and technical indicators from multiple providers |
| **Process Tracking** | ETL monitoring with automatic retry mechanisms and state persistence |
| **Rate Limiting** | Intelligent rate limiting based on API tier to avoid throttling |
| **Connection Pooling** | R2D2 connection pooling for efficient database access |

### Data Coverage

#### Equities
- **Stocks & ETFs**: Full symbol search and metadata loading
- **Company Fundamentals**: Revenue, earnings, balance sheet, cash flow data
- **Company Overviews**: Sector, industry, market cap, P/E ratios, dividend information
- **Price Data**: Daily OHLCV and intraday prices at various intervals (1min, 5min, 15min, 30min, 60min)

#### Cryptocurrency
- **10,000+ Coins**: Comprehensive coverage from multiple data providers
- **Cross-Provider Mapping**: Unified symbol mapping across CoinGecko, CoinMarketCap, CoinPaprika, CoinCap
- **Enhanced Metadata**: Descriptions, categories, platforms, contract addresses
- **Social Metrics**: Twitter followers, Reddit subscribers, GitHub activity, community scores
- **Market Data**: Exchange listings, trading pairs, volume, liquidity metrics
- **Technical Indicators**: Blockchain metrics, network statistics

#### News & Sentiment
- **NLP-Powered Analysis**: Sentiment scores for articles and individual ticker mentions
- **Topic Categorization**: Automatic categorization by topic (technology, finance, crypto, etc.)
- **Multi-Source Aggregation**: News from various financial news providers
- **Ticker Extraction**: Automatic identification of mentioned securities

#### Market Analytics
- **Top Movers**: Daily gainers, losers, and most actively traded securities
- **Missing Symbol Tracking**: Automatic logging of symbols not found in database for later resolution

### Implementation Status

#### Completed
- Symbol loading and persistence for equities, bonds, mutual funds, and cryptocurrencies
- Database schema with TimescaleDB hypertables for time-series data
- AlphaVantage API client with comprehensive endpoint coverage
- Unified cache implementation with `CacheHelper` and `CacheConfigProvider` trait
- Separate `crypto-loaders` crate for non-AlphaVantage cryptocurrency data sources
- All data loaders with caching support
- CLI commands for all data loading operations
- Response caching with configurable TTLs
- Process state tracking and recovery
- Database backup and maintenance scripts

#### In Development
- Enhanced batch processing with parallel execution optimization
- Additional cryptocurrency analytics and technical indicators
- Corporate actions support (splits, dividends, mergers)
- Real-time streaming data support

---

## Architecture Overview

The project follows clean architecture principles with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────┐
│                          av-cli                                  │
│                    (Command Line Interface)                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         av-loaders                               │
│              (Data Loading & ETL Orchestration)                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
│  │ Security    │ │ Overview    │ │ Price       │ │ News       │ │
│  │ Loader      │ │ Loader      │ │ Loaders     │ │ Loader     │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └────────────┘ │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Unified Cache Layer                       ││
│  │  CacheHelper │ CacheConfig │ CacheConfigProvider trait       ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  av-client    │     │ crypto-loaders  │     │  av-database    │
│  (AV API)     │     │ (Multi-Provider)│     │  (PostgreSQL)   │
└───────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  AlphaVantage │     │ CoinGecko       │     │  TimescaleDB    │
│  API          │     │ CoinMarketCap   │     │  PostgreSQL     │
│               │     │ CoinPaprika     │     │                 │
│               │     │ CoinCap         │     │                 │
│               │     │ SosoValue       │     │                 │
└───────────────┘     └─────────────────┘     └─────────────────┘
```

### Key Design Patterns

1. **Repository Pattern**: Database access is abstracted through repository traits (`CacheRepository`, `NewsRepository`, etc.)
2. **DataLoader Trait**: All loaders implement a common `DataLoader` trait for consistent behavior
3. **CacheConfigProvider Trait**: Unified cache configuration across all loaders
4. **Builder Pattern**: Loaders use builder pattern for flexible configuration

---

## Project Structure

```
alphavantage/
├── crates/
│   ├── av-core/                    # Core types, traits, and configuration
│   │   ├── src/
│   │   │   ├── types/
│   │   │   │   └── market/         # Market type definitions
│   │   │   │       ├── classifications.rs  # Asset classifications
│   │   │   │       ├── exchange.rs         # Exchange definitions
│   │   │   │       └── security_type.rs    # Security type enums
│   │   │   ├── config.rs           # Configuration structures
│   │   │   └── error.rs            # Error types
│   │   └── Cargo.toml
│   │
│   ├── av-client/                  # AlphaVantage API client
│   │   ├── src/
│   │   │   ├── client.rs           # Main client implementation
│   │   │   ├── transport.rs        # HTTP transport layer
│   │   │   ├── time_series.rs      # Time series endpoints
│   │   │   ├── fundamentals.rs     # Fundamental data endpoints
│   │   │   ├── news.rs             # News & sentiment endpoints
│   │   │   └── crypto.rs           # Crypto endpoints (AV)
│   │   ├── examples/               # Usage examples
│   │   │   ├── news_analysis.rs
│   │   │   └── portfolio_tracker.rs
│   │   └── Cargo.toml
│   │
│   ├── av-models/                  # Data models for API responses
│   │   ├── src/
│   │   │   ├── time_series.rs      # Price data models
│   │   │   ├── fundamentals.rs     # Company data models
│   │   │   ├── news.rs             # News article models
│   │   │   └── common.rs           # Shared models
│   │   └── Cargo.toml
│   │
│   ├── av-database/                # Database integration layer
│   │   └── postgres/               # PostgreSQL/TimescaleDB implementation
│   │       ├── src/
│   │       │   ├── repository.rs   # Repository traits and implementations
│   │       │   ├── models/         # Diesel ORM models
│   │       │   ├── schema.rs       # Database schema (auto-generated)
│   │       │   └── lib.rs          # Database connection management
│   │       ├── scripts/            # Maintenance scripts
│   │       │   ├── backup_postgres.sh
│   │       │   └── populate_intraday.sh
│   │       ├── migrations/         # Diesel migrations
│   │       └── Cargo.toml
│   │
│   ├── av-loaders/                 # AlphaVantage data loading functionality
│   │   ├── src/
│   │   │   ├── lib.rs              # Module exports and DataLoader trait
│   │   │   ├── loader.rs           # LoaderContext and configuration
│   │   │   ├── cache.rs            # Unified cache implementation
│   │   │   │                       # - CacheConfig struct
│   │   │   │                       # - CacheHelper utility
│   │   │   │                       # - CacheConfigProvider trait
│   │   │   │                       # - TTL constants (cache::ttl)
│   │   │   │                       # - Key prefixes (cache::keys)
│   │   │   ├── security_loader.rs  # Symbol search and loading
│   │   │   ├── overview_loader.rs  # Company overview loader
│   │   │   ├── news_loader.rs      # News with sentiment analysis
│   │   │   ├── top_movers_loader.rs    # Market gainers/losers
│   │   │   ├── intraday_price_loader.rs # Intraday OHLCV data
│   │   │   ├── summary_price_loader.rs  # Daily OHLCV data
│   │   │   ├── batch_processor.rs  # Batch processing utilities
│   │   │   ├── process_tracker.rs  # ETL state tracking
│   │   │   ├── csv_processor.rs    # CSV file parsing
│   │   │   ├── error.rs            # Loader error types
│   │   │   └── crypto/             # AlphaVantage crypto loaders
│   │   │       ├── mod.rs
│   │   │       ├── loader.rs       # Crypto symbol loader
│   │   │       ├── markets_loader.rs
│   │   │       ├── metadata_loader.rs
│   │   │       ├── social_loader.rs
│   │   │       └── mapping_service.rs
│   │   └── Cargo.toml
│   │
│   ├── crypto-loaders/             # Non-AlphaVantage crypto data sources
│   │   ├── src/
│   │   │   ├── lib.rs              # Module exports
│   │   │   ├── error.rs            # Crypto loader errors
│   │   │   ├── traits.rs           # Provider traits
│   │   │   ├── types.rs            # Shared crypto types
│   │   │   ├── providers/          # Data provider implementations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── coingecko.rs    # CoinGecko API client
│   │   │   │   ├── coinmarketcap.rs # CoinMarketCap API client
│   │   │   │   ├── coinpaprika.rs  # CoinPaprika API client
│   │   │   │   ├── coincap.rs      # CoinCap API client
│   │   │   │   └── sosovalue.rs    # SosoValue API client
│   │   │   ├── loaders/            # Data loaders
│   │   │   │   ├── mod.rs
│   │   │   │   ├── symbol_loader.rs    # Multi-provider symbol loading
│   │   │   │   └── details_loader.rs   # Detailed crypto information
│   │   │   ├── mapping/            # Cross-provider symbol mapping
│   │   │   │   ├── mod.rs
│   │   │   │   ├── service.rs      # Mapping service implementation
│   │   │   │   └── discovery.rs    # Symbol discovery utilities
│   │   │   ├── metadata/           # Metadata handling
│   │   │   │   ├── mod.rs
│   │   │   │   ├── types.rs        # Metadata types
│   │   │   │   └── coingecko_provider.rs
│   │   │   └── social/             # Social metrics
│   │   │       ├── mod.rs
│   │   │       └── loader.rs       # Social data loader
│   │   └── Cargo.toml
│   │
│   └── av-cli/                     # Command-line interface
│       ├── src/
│       │   ├── main.rs             # CLI entry point
│       │   └── commands/
│       │       ├── mod.rs
│       │       └── load/           # Data loading commands
│       │           ├── mod.rs
│       │           ├── securities.rs       # Load securities from CSV
│       │           ├── overviews.rs        # Load company overviews
│       │           ├── daily.rs            # Load daily prices
│       │           ├── intraday.rs         # Load intraday prices
│       │           ├── news.rs             # Load news articles
│       │           ├── top_movers.rs       # Load market movers
│       │           ├── crypto.rs           # Load crypto symbols
│       │           ├── crypto_details.rs   # Load crypto details
│       │           ├── crypto_intraday.rs  # Load crypto intraday
│       │           ├── crypto_markets.rs   # Load crypto markets
│       │           ├── crypto_metadata.rs  # Load crypto metadata
│       │           ├── crypto_news.rs      # Load crypto news
│       │           ├── crypto_overview.rs  # Load crypto overviews
│       │           ├── crypto_prices.rs    # Load crypto prices
│       │           ├── crypto_mapping.rs   # Manage symbol mappings
│       │           ├── missing_symbols.rs  # Process missing symbols
│       │           └── missing_symbol_logger.rs
│       └── Cargo.toml
│
├── timescale_setup/                # TimescaleDB Docker configuration
│   ├── Makefile                    # Docker management commands
│   └── docker-compose.yml
│
├── data/                           # CSV data files for symbol imports
│   ├── nasdaq_symbols.csv
│   ├── nyse_symbols.csv
│   └── crypto_symbols.csv
│
├── tests/                          # Integration tests
├── CLAUDE.md                       # AI assistant context file
├── Cargo.toml                      # Workspace configuration
└── README.md                       # This file
```

---

## Crate Documentation

### av-core

The foundation crate providing core types, traits, and configuration used across the workspace.

**Key Components:**
- `Market` types: Classifications, exchanges, security types
- Configuration structures for API clients and database connections
- Shared error types

### av-client

Async HTTP client for the AlphaVantage API with full endpoint coverage.

**Supported Endpoints:**
- Time Series: Daily, weekly, monthly, intraday prices
- Fundamentals: Company overview, income statement, balance sheet, cash flow
- News: Market news with sentiment analysis
- Crypto: Digital currency exchange rates (AlphaVantage)

**Example Usage:**
```rust
use av_client::AlphaVantageClient;

let client = AlphaVantageClient::new("YOUR_API_KEY");

// Get daily prices
let daily = client.time_series().daily("AAPL").await?;

// Get company overview
let overview = client.fundamentals().company_overview("AAPL").await?;

// Search for symbols
let results = client.time_series().symbol_search("Apple").await?;
```

### av-models

Data models representing API responses, designed for serialization/deserialization with serde.

**Model Categories:**
- `time_series`: OHLCV price data, symbol search results
- `fundamentals`: Company overview, financial statements, top movers
- `news`: Articles, sentiment scores, topics
- `common`: Shared types like `SymbolMatch`

### av-database/postgres

PostgreSQL/TimescaleDB integration with Diesel ORM.

**Features:**
- Connection pooling with r2d2
- Repository pattern for data access
- TimescaleDB hypertables for time-series data
- Automated schema migrations

**Repository Traits:**
- `CacheRepository`: API response caching with TTL
- `NewsRepository`: News article and sentiment storage
- `SymbolRepository`: Security symbol management

### av-loaders

Data loading and ETL orchestration for AlphaVantage data.

**Loaders:**
| Loader | Description | Cache TTL |
|--------|-------------|-----------|
| `SecurityLoader` | Symbol search and metadata | 7 days |
| `OverviewLoader` | Company fundamentals | 30 days |
| `NewsLoader` | News with sentiment | 1 hour |
| `TopMoversLoader` | Market gainers/losers | 4 hours |
| `IntradayPriceLoader` | Intraday OHLCV | 15 minutes |
| `SummaryPriceLoader` | Daily OHLCV | 24 hours |

**Cache Module (`cache.rs`):**
```rust
// TTL constants
pub mod ttl {
    pub const SYMBOL_SEARCH: i64 = 168;  // 7 days
    pub const OVERVIEW: i64 = 720;       // 30 days
    pub const NEWS: i64 = 1;             // 1 hour
    pub const TOP_MOVERS: i64 = 4;       // 4 hours
    pub const INTRADAY: i64 = 0;         // 15 minutes (0.25 hours)
    pub const DAILY: i64 = 24;           // 24 hours
}

// Implement CacheConfigProvider for your config
impl CacheConfigProvider for MyLoaderConfig {
    fn cache_enabled(&self) -> bool { self.enable_cache }
    fn cache_ttl_hours(&self) -> i64 { self.cache_ttl_hours }
    fn force_refresh(&self) -> bool { self.force_refresh }
}
```

### crypto-loaders

Standalone crate for cryptocurrency data from non-AlphaVantage sources.

**Providers:**
| Provider | Data Types | Rate Limits |
|----------|------------|-------------|
| CoinGecko | Prices, metadata, social, markets | 10-50 req/min |
| CoinMarketCap | Prices, metadata, rankings | Tier-based |
| CoinPaprika | Prices, metadata, events | 10 req/sec |
| CoinCap | Real-time prices, history | 200 req/min |
| SosoValue | ETF flows, institutional data | Varies |

**Features:**
- `CryptoSymbolLoader`: Load symbols from multiple providers
- `CryptoDetailsLoader`: Comprehensive coin information
- `CryptoMappingService`: Cross-provider symbol resolution
- `SocialLoader`: Twitter, Reddit, GitHub metrics

### av-cli

Command-line interface for all data loading operations.

---

## CLI Commands

### Global Options
```bash
av-cli [OPTIONS] <COMMAND>

Options:
  -v, --verbose    Enable verbose logging
  -q, --quiet      Suppress non-error output
  --config <FILE>  Path to configuration file
  -h, --help       Print help information
  -V, --version    Print version information
```

### Equity Data Commands

#### Load Securities
Load security symbols from a CSV file into the database.
```bash
av-cli load securities --file <PATH> --exchange <EXCHANGE>

Options:
  --file <PATH>       Path to CSV file containing symbols
  --exchange <NAME>   Exchange name (NYSE, NASDAQ, etc.)
  --match-mode <MODE> Symbol matching: exact, all, top-n (default: all)
  --force-refresh     Bypass cache and fetch fresh data
  --dry-run           Preview without saving to database

Example:
  av-cli load securities --file data/nasdaq_symbols.csv --exchange NASDAQ
```

#### Load Company Overviews
Fetch and store company fundamental data.
```bash
av-cli load overviews [OPTIONS]

Options:
  --symbol <SYMBOL>   Load overview for specific symbol
  --limit <N>         Limit number of symbols to process
  --offset <N>        Skip first N symbols
  --force-refresh     Bypass cache
  --concurrent <N>    Max concurrent API requests (default: 5)

Example:
  av-cli load overviews --limit 100 --concurrent 3
```

#### Load Daily Prices
Fetch daily OHLCV price data.
```bash
av-cli load daily [OPTIONS]

Options:
  --symbol <SYMBOL>   Symbol to load (or all if not specified)
  --outputsize <SIZE> compact (100 days) or full (20+ years)
  --from <DATE>       Start date (YYYY-MM-DD)
  --to <DATE>         End date (YYYY-MM-DD)
  --force-refresh     Bypass cache

Example:
  av-cli load daily --symbol AAPL --outputsize full
```

#### Load Intraday Prices
Fetch intraday OHLCV price data.
```bash
av-cli load intraday [OPTIONS]

Options:
  --symbol <SYMBOL>   Symbol to load
  --interval <INT>    1min, 5min, 15min, 30min, 60min
  --outputsize <SIZE> compact or full
  --month <YYYY-MM>   Specific month for historical data

Example:
  av-cli load intraday --symbol AAPL --interval 5min --outputsize compact
```

### Cryptocurrency Commands

#### Load Crypto Symbols
Load cryptocurrency symbols from various providers.
```bash
av-cli load crypto [OPTIONS]

Options:
  --source <SOURCE>   Provider: coingecko, coinmarketcap, coinpaprika, coincap
  --limit <N>         Maximum symbols to load
  --active-only       Only load actively traded coins

Example:
  av-cli load crypto --source coingecko --limit 1000
```

#### Load Crypto Metadata
Fetch detailed cryptocurrency metadata.
```bash
av-cli load crypto-metadata [OPTIONS]

Options:
  --provider <NAME>   Data provider to use
  --symbol <SYMBOL>   Specific symbol or all
  --force-refresh     Bypass cache

Example:
  av-cli load crypto-metadata --provider coingecko --symbol BTC
```

#### Load Crypto Markets
Fetch exchange and market pair information.
```bash
av-cli load crypto-markets [OPTIONS]

Options:
  --symbol <SYMBOL>   Cryptocurrency symbol
  --exchange <NAME>   Filter by exchange

Example:
  av-cli load crypto-markets --symbol ETH
```

#### Load Crypto Details
Fetch comprehensive coin details including social metrics.
```bash
av-cli load crypto-details [OPTIONS]

Options:
  --symbol <SYMBOL>   Cryptocurrency symbol
  --include-social    Include social media metrics

Example:
  av-cli load crypto-details --symbol BTC --include-social
```

#### Load Crypto Intraday
Fetch intraday cryptocurrency prices.
```bash
av-cli load crypto-intraday [OPTIONS]

Options:
  --symbol <SYMBOL>   Crypto symbol (e.g., BTC)
  --market <MARKET>   Quote currency (e.g., USD)
  --interval <INT>    Time interval

Example:
  av-cli load crypto-intraday --symbol BTC --market USD --interval 5min
```

### News & Analytics Commands

#### Load News
Fetch news articles with sentiment analysis.
```bash
av-cli load news [OPTIONS]

Options:
  --topics <TOPICS>   Comma-separated topics
  --tickers <TICKERS> Comma-separated stock symbols
  --limit <N>         Maximum articles to fetch
  --from <DATETIME>   Start datetime
  --to <DATETIME>     End datetime
  --sort <ORDER>      latest, earliest, relevance

Example:
  av-cli load news --topics technology,finance --limit 50
```

#### Load Top Movers
Fetch daily market gainers, losers, and most active.
```bash
av-cli load top-movers [OPTIONS]

Options:
  --date <DATE>       Specific date (default: today)
  --force-refresh     Bypass cache

Example:
  av-cli load top-movers
```

### Utility Commands

#### Process Missing Symbols
Attempt to resolve previously unmatched symbols.
```bash
av-cli load missing-symbols [OPTIONS]

Options:
  --source <SOURCE>   Filter by original source
  --limit <N>         Maximum to process

Example:
  av-cli load missing-symbols --limit 100
```

---

## Caching System

The project implements a unified caching layer to minimize API calls and reduce costs. All loaders implement the `CacheConfigProvider` trait for consistent behavior.

### Cache Configuration

```rust
pub struct CacheConfig {
    pub enable_cache: bool,      // Enable/disable caching
    pub cache_ttl_hours: i64,    // Time-to-live in hours
    pub force_refresh: bool,     // Bypass cache and fetch fresh
    pub api_source: String,      // Source identifier (e.g., "alphavantage")
}
```

### Default TTL Values

| Data Type | TTL | Rationale |
|-----------|-----|-----------|
| Symbol Search | 7 days (168 hours) | Symbol metadata rarely changes |
| Company Overview | 30 days (720 hours) | Fundamental data updates quarterly |
| News Articles | 1 hour | News is time-sensitive |
| Top Movers | 4 hours | Updated throughout trading day |
| Intraday Prices | 15 minutes | High-frequency data |
| Daily Prices | 24 hours | Updates after market close |
| Crypto Metadata | 24 hours | Blockchain data changes slowly |
| Crypto Prices | 5 minutes | Crypto markets are 24/7 |

### Cache Storage

Cached responses are stored in the `api_response_cache` table:

```sql
CREATE TABLE api_response_cache (
    id BIGSERIAL PRIMARY KEY,
    cache_key VARCHAR(512) NOT NULL,
    api_source VARCHAR(64) NOT NULL,
    endpoint_url TEXT,
    response_data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    hit_count INTEGER DEFAULT 0,
    UNIQUE(cache_key, api_source)
);

CREATE INDEX idx_cache_expires ON api_response_cache(expires_at);
CREATE INDEX idx_cache_key ON api_response_cache(cache_key, api_source);
```

### Cache Operations

```rust
// Get from cache
let result = cache_repo.get::<SymbolSearch>(&cache_key, "alphavantage").await?;

// Store in cache
cache_repo.set(&cache_key, "alphavantage", &endpoint_url, &data, ttl_hours).await?;

// Cleanup expired entries
let deleted = cache_repo.cleanup_expired("alphavantage").await?;
```

---

## Database Schema

The project includes a comprehensive PostgreSQL schema with TimescaleDB extensions for optimal time-series performance.

### Core Tables

#### symbols
Master table for all security types.
```sql
CREATE TABLE symbols (
    sid BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL UNIQUE,
    name VARCHAR(255),
    sec_type VARCHAR(50),        -- Equity, ETF, Crypto, Bond, etc.
    exchange VARCHAR(50),
    currency VARCHAR(10),
    region VARCHAR(100),
    market_open TIME,
    market_close TIME,
    timezone VARCHAR(50),
    match_score FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### overviews
Company fundamental data.
```sql
CREATE TABLE overviews (
    sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
    asset_type VARCHAR(50),
    description TEXT,
    cik VARCHAR(20),
    exchange VARCHAR(50),
    currency VARCHAR(10),
    country VARCHAR(100),
    sector VARCHAR(100),
    industry VARCHAR(200),
    address TEXT,
    fiscal_year_end VARCHAR(20),
    latest_quarter DATE,
    market_capitalization BIGINT,
    ebitda BIGINT,
    pe_ratio FLOAT,
    peg_ratio FLOAT,
    book_value FLOAT,
    dividend_per_share FLOAT,
    dividend_yield FLOAT,
    eps FLOAT,
    -- ... additional fields
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### summaryprices (TimescaleDB Hypertable)
Daily OHLCV data.
```sql
CREATE TABLE summaryprices (
    sid BIGINT REFERENCES symbols(sid),
    trade_date TIMESTAMPTZ NOT NULL,
    open FLOAT,
    high FLOAT,
    low FLOAT,
    close FLOAT,
    adjusted_close FLOAT,
    volume BIGINT,
    dividend_amount FLOAT,
    split_coefficient FLOAT,
    PRIMARY KEY (sid, trade_date)
);

SELECT create_hypertable('summaryprices', 'trade_date');
```

#### intradayprices (TimescaleDB Hypertable)
Intraday OHLCV data.
```sql
CREATE TABLE intradayprices (
    sid BIGINT REFERENCES symbols(sid),
    trade_time TIMESTAMPTZ NOT NULL,
    interval VARCHAR(10),        -- 1min, 5min, 15min, 30min, 60min
    open FLOAT,
    high FLOAT,
    low FLOAT,
    close FLOAT,
    volume BIGINT,
    PRIMARY KEY (sid, trade_time, interval)
);

SELECT create_hypertable('intradayprices', 'trade_time');
```

### Cryptocurrency Tables

#### crypto_api_map
Cross-provider symbol mapping.
```sql
CREATE TABLE crypto_api_map (
    id BIGSERIAL PRIMARY KEY,
    sid BIGINT REFERENCES symbols(sid),
    provider VARCHAR(50) NOT NULL,      -- coingecko, coinmarketcap, etc.
    provider_id VARCHAR(100) NOT NULL,  -- Provider's internal ID
    provider_symbol VARCHAR(50),
    provider_name VARCHAR(255),
    rank INTEGER,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(sid, provider)
);
```

#### crypto_metadata
Core cryptocurrency information.
```sql
CREATE TABLE crypto_metadata (
    sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
    description TEXT,
    category VARCHAR(100),
    platform VARCHAR(100),
    contract_address VARCHAR(100),
    decimals INTEGER,
    genesis_date DATE,
    homepage_url TEXT,
    whitepaper_url TEXT,
    github_url TEXT,
    reddit_url TEXT,
    twitter_handle VARCHAR(100),
    telegram_url TEXT,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### crypto_social
Social media and community metrics.
```sql
CREATE TABLE crypto_social (
    sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
    twitter_followers INTEGER,
    reddit_subscribers INTEGER,
    reddit_active_users INTEGER,
    telegram_members INTEGER,
    facebook_likes INTEGER,
    github_stars INTEGER,
    github_forks INTEGER,
    github_contributors INTEGER,
    github_commits_30d INTEGER,
    coingecko_score FLOAT,
    developer_score FLOAT,
    community_score FLOAT,
    liquidity_score FLOAT,
    sentiment_votes_up INTEGER,
    sentiment_votes_down INTEGER,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### crypto_markets
Exchange and trading pair information.
```sql
CREATE TABLE crypto_markets (
    id BIGSERIAL PRIMARY KEY,
    sid BIGINT REFERENCES symbols(sid),
    exchange VARCHAR(100) NOT NULL,
    pair VARCHAR(50) NOT NULL,
    base_currency VARCHAR(20),
    quote_currency VARCHAR(20),
    price_usd FLOAT,
    volume_24h FLOAT,
    spread FLOAT,
    trust_score VARCHAR(20),
    is_anomaly BOOLEAN DEFAULT false,
    last_traded_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(sid, exchange, pair)
);
```

### News & Sentiment Tables

#### newsoverviews
Article metadata with overall sentiment.
```sql
CREATE TABLE newsoverviews (
    news_id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    url TEXT UNIQUE NOT NULL,
    time_published TIMESTAMPTZ,
    summary TEXT,
    banner_image TEXT,
    source VARCHAR(255),
    category_within_source VARCHAR(100),
    source_domain VARCHAR(255),
    overall_sentiment_score FLOAT,
    overall_sentiment_label VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### tickersentiments
Per-ticker sentiment analysis.
```sql
CREATE TABLE tickersentiments (
    id BIGSERIAL PRIMARY KEY,
    news_id BIGINT REFERENCES newsoverviews(news_id),
    ticker VARCHAR(20),
    relevance_score FLOAT,
    ticker_sentiment_score FLOAT,
    ticker_sentiment_label VARCHAR(50)
);
```

### System Tables

#### api_response_cache
Response caching for API efficiency.

#### procstates
ETL process state tracking.
```sql
CREATE TABLE procstates (
    id BIGSERIAL PRIMARY KEY,
    process_name VARCHAR(100) NOT NULL,
    state VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### missing_symbols
Tracking symbols not found for later resolution.
```sql
CREATE TABLE missing_symbols (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    source VARCHAR(100) NOT NULL,
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    attempt_count INTEGER DEFAULT 1,
    resolved BOOLEAN DEFAULT false,
    resolved_sid BIGINT REFERENCES symbols(sid),
    UNIQUE(symbol, source)
);
```

---

## Getting Started

### Prerequisites

- **Rust**: Version 1.70 or higher
- **Docker & Docker Compose**: For TimescaleDB container
- **PostgreSQL Client Tools**: `psql`, `pg_dump` for database operations
- **Diesel CLI**: For database migrations (`cargo install diesel_cli --no-default-features --features postgres`)
- **API Keys**:
  - [AlphaVantage API Key](https://www.alphavantage.co/support/#api-key) (required)
  - [CoinGecko API Key](https://www.coingecko.com/en/api) (optional, for higher rate limits)
  - [CoinMarketCap API Key](https://coinmarketcap.com/api/) (optional)

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/dbrowne/alphavantage.git
   cd alphavantage
   ```

2. **Setup TimescaleDB with Docker:**
   ```bash
   cd timescale_setup
   make up

   # Verify container is running
   docker ps | grep timescale
   ```

3. **Setup the database:**
   ```bash
   cd ../crates/av-database/postgres

   # Initialize Diesel
   diesel setup

   # Generate and run migrations
   diesel migration generate base_tables
   cp base_migration/* migrations/20*base_tables/
   diesel migration run
   ```

4. **Verify database setup:**
   ```bash
   PGPASSWORD=dev_pw psql -U ts_user -h localhost -p 6433 -d sec_master -c "\dt"
   ```

   You should see 30+ tables listed.

5. **Set environment variables:**
   ```bash
   # Required
   export ALPHAVANTAGE_API_KEY=your_alphavantage_key
   export DATABASE_URL=postgres://ts_user:dev_pw@localhost:6433/sec_master

   # Optional - for additional crypto providers
   export COINGECKO_API_KEY=your_coingecko_key
   export COINMARKETCAP_API_KEY=your_coinmarketcap_key
   ```

6. **Build the project:**
   ```bash
   cargo build --release
   ```

7. **Run the CLI:**
   ```bash
   ./target/release/av-cli --help
   ```

### Quick Start Example

Load some initial data:

```bash
# Load NASDAQ symbols
./target/release/av-cli load securities \
    --file data/nasdaq_symbols.csv \
    --exchange NASDAQ

# Load company overviews for first 10 symbols
./target/release/av-cli load overviews --limit 10

# Load daily prices for Apple
./target/release/av-cli load daily --symbol AAPL --outputsize compact

# Load recent news
./target/release/av-cli load news --topics technology --limit 20

# Load top cryptocurrency symbols
./target/release/av-cli load crypto --source coingecko --limit 100
```

---

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ALPHAVANTAGE_API_KEY` | Yes | Your AlphaVantage API key |
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `COINGECKO_API_KEY` | No | CoinGecko Pro API key |
| `COINMARKETCAP_API_KEY` | No | CoinMarketCap API key |
| `RUST_LOG` | No | Logging level (debug, info, warn, error) |

### Rate Limiting

AlphaVantage API has different rate limits based on your subscription:

| Plan | Requests/Minute | Requests/Day |
|------|-----------------|--------------|
| Free | 5 | 500 |
| Premium | 75 | 7,500 |
| Premium+ | 150 | Unlimited |

The client automatically handles rate limiting based on your configured tier.

---

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific crate
cargo build -p av-client
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p av-loaders

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for security vulnerabilities
cargo audit
```

### Database Operations

```bash
# Backup database
./crates/av-database/postgres/scripts/backup_postgres.sh

# Run new migration
cd crates/av-database/postgres
diesel migration generate <migration_name>
diesel migration run

# Revert last migration
diesel migration revert
```

### Generate Schema Documentation

```bash
java -jar schemaspy.jar \
  -t pgsql11 \
  -dp postgresql-42.x.x.jar \
  -db sec_master \
  -host localhost \
  -port 6433 \
  -u ts_user \
  -p dev_pw \
  -o db_relations

# View in browser
open db_relations/index.html
```

---

## Security Advisories

Dependencies as of January 2026:

| Advisory | Package       | Severity | Status                                                                       |
|----------|---------------|----------|------------------------------------------------------------------------------|
| [RUSTSEC-2025-0047](https://rustsec.org/advisories/RUSTSEC-2025-0047) | slab          | Medium | Out-of-bounds access in get_disjoint_mut                                     |
| [RUSTSEC-2024-0375](https://rustsec.org/advisories/RUSTSEC-2024-0375) | atty          | Low | Unmaintained                                                                 |
| [RUSTSEC-2021-0141](https://rustsec.org/advisories/RUSTSEC-2021-0141) | dotenv        | Low | Unmaintained                                                                 |
| [RUSTSEC-2025-0119](https://rustsec.org/advisories/RUSTSEC-2025-0119) | number_prefix | Low | Unmaintained                                                                 |
| [RUSTSEC-2026-0001](https://rustsec.org/advisories/RUSTSEC-2026-0001) | rust_decimal  | Low | Potential Undefined Behaviors in Arc\<T\>/Rc\<T\> impls of from_value on OOM |

Run `cargo audit` to check for the latest security advisories.

---

## License

MIT License - See [LICENSE](LICENSE) file for details.

---

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## Acknowledgments

- [AlphaVantage](https://www.alphavantage.co/) for financial market data API
- [CoinGecko](https://www.coingecko.com/) for cryptocurrency data
- [TimescaleDB](https://www.timescale.com/) for time-series database extensions
- [Diesel](https://diesel.rs/) for Rust ORM
- [Tokio](https://tokio.rs/) for async runtime

---

### DB Schema
![Database Schema](db_schema.png)