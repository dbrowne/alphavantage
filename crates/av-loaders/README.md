# av-loaders

Data loading functionality for AlphaVantage market data.

## Overview

The `av-loaders` crate provides a comprehensive set of data loaders for fetching and storing market data from the AlphaVantage API. It includes:

- **Security Loader**: Read symbols from CSV files and fetch company data from API
## Features

- Concurrent data loading with configurable rate limiting
- Batch processing for efficient database operations
- Process tracking for monitoring ETL jobs
- Progress indicators for long-running operations
- Comprehensive error handling and retry logic
- Cache support for frequently accessed data