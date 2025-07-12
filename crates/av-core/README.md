# av-core

Core types and traits for the AlphaVantage Rust client ecosystem.

This crate provides:
- Common error types used across all av-* crates
- Configuration management
- Shared types and traits
- API function type definitions

## Usage

```rust
use av_core::{Error, Config, FuncType};
use av_core::types::{SecurityType, TopType};

// Load configuration
let config = Config::from_env()?;

// Use common types
let security_type = SecurityType::CommonStock;
```

## Features

- Unified error handling with `thiserror`
- Environment-based configuration
- Type-safe API function definitions
- Common market data types# av-core

Core types and traits for the AlphaVantage Rust client ecosystem.

This crate provides:
- Common error types used across all av-* crates
- Configuration management
- Shared types and traits
- API function type definitions

## Usage

```rust
use av_core::{Error, Config, FuncType};
use av_core::types::{SecurityType, TopType};

// Load configuration
let config = Config::from_env()?;

// Use common types
let security_type = SecurityType::CommonStock;
```

## Features

- Unified error handling with `thiserror`
- Environment-based configuration
- Type-safe API function definitions
- Common market data types
