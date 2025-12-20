# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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