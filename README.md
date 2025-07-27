This is a re write of the original AlphaVantage rust api.  Daily updates
markdown# AlphaVantage Rust Client

A Rust implementation and complete re write of my  Rust [Alphavantage API](https://github.com/dbrowne/AlphaVantage_Rust)   client with PostgreSQL/TimescaleDB support for financial market data storage and analysis.

## ⚠️ Project Status

**This project is currently under active development.** Only the symbol persistence functionality is fully implemented end-to-end. Other features are in various stages of development.

### Currently Implemented
- ✅ Symbol loading and persistence for equities, bonds, and mutual funds
- ✅ Database schema with TimescaleDB support
- ✅ Basic project structure and workspace organization

### In Development
- 🚧 AlphaVantage API client endpoints
- 🚧 Data loaders for price data, fundamentals, and news
- 🚧 CLI commands for data fetching and analysis
- 🚧 Full integration between API client and database

## Overview

This project aims to provide a complete solution for fetching, storing, and analyzing financial market data from AlphaVantage. Built with Rust's async ecosystem, it will offer high-performance data loading capabilities with proper rate limiting, concurrent processing, and comprehensive error handling.

## Project Structure
```alphavantage/
├── crates/
│   ├── av-core/              # Core types, traits, and configuration
│   ├── av-client/            # API client (in development)
│   ├── av-models/            # Data models for API responses
│   ├── av-database/          # Database integration layer
│   │   └── postgres/         # PostgreSQL/TimescaleDB implementation
│   ├── av-loaders/           # Data loading functionality (in development)
│   └── av-cli/               # Command-line interface (in development)
├── timescale_setup/          # TimescaleDB Docker setup
├── migrations/               # Database migrations
└── data/                     # CSV data files
```


## Database Schema

The project includes a comprehensive PostgreSQL schema with TimescaleDB extensions:

### Implemented Tables
- `symbols` - Security master data (fully implemented)
- `overviews` - Company fundamentals (schema only)
- `intradayprices` - High-frequency price data as hypertable (schema only)
- `summaryprices` - Daily OHLCV data (schema only)
- `newsoverviews` - News articles with sentiment (schema only)
- `topstats` - Market movers tracking (schema only)
- Process management tables for ETL tracking

## Tentative Development Roadmap  

### Phase 1: Core Infrastructure ✅
- Workspace structure
- Database schema
- Symbol loading


### Phase 2: Data Loaders
- Company overview loader
- Price data loaders
- News loader with sentiment analysis
- Batch processing with progress tracking
 
### Phase 3: API Client (In Progress)
- Time series endpoints
- Fundamental data endpoints
- News sentiment endpoints
- Rate limiting implementation

### Phase 4: CLI Enhancement
- Complete command structure
- Query capabilities
- Analytics commands
- Process management

### Phase 5: Production Features
- Comprehensive error handling
- Retry logic
- Caching layer
- Performance optimizationsRetryClaude can make mistakes. Please double-check responses.



## Getting Started

### Prerequisites

- Rust 1.70+
- Docker & Docker Compose
- PostgreSQL client tools
- AlphaVantage API key ([get one here](https://www.alphavantage.co/support/#api-key))

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/dbrowne/alphavantage.git
   cd alphavantage
   
2. **setup docker**
    ```bash
   cd timescale_setup
   make up
   
   3. **setup database**
      ```bash
      cd ../crates/av-database/postgres
      diesel setup
      diesel migration generate base_tables
      cp ../base_migrations/* migrations/20*base_tables
      diesel migration run
      PGPASSWORD=dev_pw psql -U ts_user -h localhost -p 6433 -d sec_master
      
   4. **check database**
```
   psql (16.8 (Ubuntu 16.8-0ubuntu0.24.04.1), server 15.13)
   Type "help" for help.

sec_master=> \dt
List of relations
Schema |            Name            | Type  |  Owner  
--------+----------------------------+-------+---------
public | __diesel_schema_migrations | table | ts_user
public | articles                   | table | ts_user
public | authormaps                 | table | ts_user
public | authors                    | table | ts_user
public | feeds                      | table | ts_user
public | intradayprices             | table | ts_user
public | newsoverviews              | table | ts_user
public | overviewexts               | table | ts_user
public | overviews                  | table | ts_user
public | procstates                 | table | ts_user
public | proctypes                  | table | ts_user
public | sources                    | table | ts_user
public | states                     | table | ts_user
public | summaryprices              | table | ts_user
public | symbols                    | table | ts_user
public | tickersentiments           | table | ts_user
public | topicmaps                  | table | ts_user
public | topicrefs                  | table | ts_user
public | topstats                   | table | ts_user
(19 rows)
```
   

   