# Crypto Crate Separation Refactoring Plan

## Overview

Separate non-AlphaVantage crypto code into a new `crypto-loaders` crate. Code that doesn't call AlphaVantage APIs should not live in `av-*` crates.

## Analysis Summary

| File | AV Dependency | Recommendation |
|------|---------------|----------------|
| `sources/coingecko.rs` | None | **MOVE** |
| `sources/coinmarketcap.rs` | None | **MOVE** |
| `sources/coinpaprika.rs` | None | **MOVE** |
| `sources/coincap.rs` | None | **MOVE** |
| `sources/sosovalue.rs` | None | **MOVE** |
| `coingecko_details_loader.rs` | None | **MOVE** |
| `loader.rs` | None | **MOVE** |
| `types.rs` | None | **MOVE** |
| `social_loader.rs` | None | **MOVE** |
| `mapping_service.rs` | DB only | **MOVE** (with interface) |
| `metadata_types.rs` | None | **MOVE** |
| `metadata_providers.rs` | Partial | **SPLIT** |
| `metadata_loader.rs` | Partial | **SPLIT** |
| `markets_loader.rs` | DB caching | **NEEDS ANALYSIS** |
| `intraday_loader.rs` | **Yes** (AV API) | STAY |
| `crypto_news_loader.rs` | **Yes** (AV format) | STAY |
| `database.rs` | Orchestrator | STAY |

---

## New Crate Structure

```
crates/
├── crypto-loaders/           # NEW CRATE
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs          # CryptoSymbol, CryptoDataSource, etc.
│       ├── error.rs          # CryptoLoaderError
│       ├── traits.rs         # CryptoDataProvider trait
│       ├── providers/
│       │   ├── mod.rs
│       │   ├── coingecko.rs
│       │   ├── coinmarketcap.rs
│       │   ├── coinpaprika.rs
│       │   ├── coincap.rs
│       │   └── sosovalue.rs
│       ├── loaders/
│       │   ├── mod.rs
│       │   ├── symbol_loader.rs      # CryptoSymbolLoader
│       │   └── details_loader.rs     # CoinGeckoDetailsLoader
│       ├── metadata/
│       │   ├── mod.rs
│       │   ├── types.rs              # CryptoMetadataConfig, etc.
│       │   └── coingecko_provider.rs
│       ├── mapping/
│       │   ├── mod.rs
│       │   └── service.rs            # CryptoMappingService
│       └── social/
│           ├── mod.rs
│           └── loader.rs             # SocialLoader
│
├── av-loaders/               # EXISTING (simplified)
│   └── src/
│       ├── crypto/
│       │   ├── mod.rs
│       │   ├── intraday_loader.rs    # STAYS (uses AV API)
│       │   ├── crypto_news_loader.rs # STAYS (uses AV format)
│       │   ├── database.rs           # STAYS (orchestrator)
│       │   └── metadata/
│       │       ├── mod.rs
│       │       ├── loader.rs         # Orchestrates both providers
│       │       └── alphavantage_provider.rs  # AV-specific
│       └── ...
```

---

## Phase 1: Create `crypto-loaders` Crate

### 1.1 Initialize Crate

```bash
mkdir -p crates/crypto-loaders/src
```

**Cargo.toml:**
```toml
[package]
name = "crypto-loaders"
version = "0.1.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
indicatif = "0.17"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tokio = { version = "1", features = ["time"] }
tracing = "0.1"

# Optional database support (feature-gated)
[features]
default = []
caching = ["dep:av-database-postgres"]

[dependencies.av-database-postgres]
path = "../av-database/postgres"
optional = true
```

### 1.2 Move Core Types

**From:** `av-loaders/src/crypto/types.rs`
**To:** `crypto-loaders/src/types.rs`

```rust
// crypto-loaders/src/types.rs
pub struct CryptoSymbol { ... }
pub enum CryptoDataSource { ... }
pub struct CryptoLoaderConfig { ... }
pub struct SourceResult { ... }
```

### 1.3 Move Error Types

**From:** `av-loaders/src/crypto/mod.rs` (CryptoLoaderError)
**To:** `crypto-loaders/src/error.rs`

---

## Phase 2: Move Providers

### 2.1 Provider Trait

**Create:** `crypto-loaders/src/traits.rs`

```rust
use async_trait::async_trait;

/// Trait for crypto data providers
#[async_trait]
pub trait CryptoDataProvider: Send + Sync {
    async fn fetch_symbols(
        &self,
        client: &reqwest::Client,
        cache_repo: Option<&dyn CacheRepository>,
    ) -> Result<Vec<CryptoSymbol>, CryptoLoaderError>;

    fn name(&self) -> &'static str;
    fn requires_api_key(&self) -> bool;
}
```

### 2.2 Move Provider Implementations

| Source | Destination |
|--------|-------------|
| `av-loaders/src/crypto/sources/coingecko.rs` | `crypto-loaders/src/providers/coingecko.rs` |
| `av-loaders/src/crypto/sources/coinmarketcap.rs` | `crypto-loaders/src/providers/coinmarketcap.rs` |
| `av-loaders/src/crypto/sources/coinpaprika.rs` | `crypto-loaders/src/providers/coinpaprika.rs` |
| `av-loaders/src/crypto/sources/coincap.rs` | `crypto-loaders/src/providers/coincap.rs` |
| `av-loaders/src/crypto/sources/sosovalue.rs` | `crypto-loaders/src/providers/sosovalue.rs` |

---

## Phase 3: Move Loaders

### 3.1 Symbol Loader

**From:** `av-loaders/src/crypto/loader.rs`
**To:** `crypto-loaders/src/loaders/symbol_loader.rs`

No changes needed - already provider-agnostic.

### 3.2 Details Loader

**From:** `av-loaders/src/crypto/coingecko_details_loader.rs`
**To:** `crypto-loaders/src/loaders/details_loader.rs`

---

## Phase 4: Split Metadata Code

### 4.1 Types (Move Entirely)

**From:** `av-loaders/src/crypto/metadata_types.rs`
**To:** `crypto-loaders/src/metadata/types.rs`

### 4.2 Providers (Split)

**CoinGecko Provider:**
- **From:** `av-loaders/src/crypto/metadata_providers.rs` (CoinGeckoMetadataProvider)
- **To:** `crypto-loaders/src/metadata/coingecko_provider.rs`

**AlphaVantage Provider:**
- **Stays:** `av-loaders/src/crypto/metadata/alphavantage_provider.rs`
- Depends on `av_models::crypto::CryptoDaily`

### 4.3 Loader (Stays in av-loaders)

**Stays:** `av-loaders/src/crypto/metadata/loader.rs`
- Orchestrates both providers
- Imports CoinGecko provider from `crypto-loaders`
- Imports AlphaVantage provider locally

---

## Phase 5: Move Supporting Code

### 5.1 Mapping Service

**From:** `av-loaders/src/crypto/mapping_service.rs`
**To:** `crypto-loaders/src/mapping/service.rs`

**Changes needed:**
- Abstract database operations behind a trait
- Feature-gate database implementation

### 5.2 Social Loader

**From:** `av-loaders/src/crypto/social_loader.rs`
**To:** `crypto-loaders/src/social/loader.rs`

---

## Phase 6: Markets Loader Analysis

**File:** `av-loaders/src/crypto/markets_loader.rs`

**Current state:** Uses raw SQL for caching, CoinGecko API for data.

**Options:**
1. **Split:** Extract CoinGecko fetching → `crypto-loaders`, keep DB caching → `av-loaders`
2. **Move entirely:** With feature-gated caching
3. **Keep in av-loaders:** If caching is essential

**Recommendation:** Option 1 - Split the file

---

## Phase 7: Update av-loaders

### 7.1 Add Dependency

**av-loaders/Cargo.toml:**
```toml
[dependencies]
crypto-loaders = { path = "../crypto-loaders" }
```

### 7.2 Re-export for Backward Compatibility

**av-loaders/src/crypto/mod.rs:**
```rust
// Re-export from crypto-loaders for backward compatibility
pub use crypto_loaders::{
    CryptoSymbol, CryptoDataSource, CryptoLoaderConfig,
    CryptoSymbolLoader, CryptoLoaderError,
    providers::{
        CoinGeckoProvider, CoinMarketCapProvider,
        CoinPaprikaProvider, CoinCapProvider, SosoValueProvider,
    },
};

// Local AV-specific code
pub mod intraday_loader;
pub mod crypto_news_loader;
pub mod database;
pub mod metadata;
```

---

## Files That Stay in av-loaders

| File | Reason |
|------|--------|
| `intraday_loader.rs` | Calls AlphaVantage CRYPTO_INTRADAY API |
| `crypto_news_loader.rs` | Uses AV-specific CRYPTO:XXX symbol format |
| `database.rs` | Orchestrator for av-loaders context |
| `metadata/alphavantage_provider.rs` | Uses av_models::CryptoDaily |
| `metadata/loader.rs` | Orchestrates AV + non-AV providers |

---

## Migration Steps

1. **Create `crypto-loaders` crate structure**
2. **Move types.rs and error types**
3. **Move provider implementations** (coingecko, coinmarketcap, etc.)
4. **Move symbol_loader.rs**
5. **Move details_loader.rs**
6. **Split metadata_providers.rs** (CoinGecko → crypto-loaders, AV → av-loaders)
7. **Move metadata_types.rs**
8. **Update av-loaders to depend on crypto-loaders**
9. **Add re-exports for backward compatibility**
10. **Update all imports across the workspace**
11. **Run tests**

---

## Caching Strategy

The `CacheRepository` interface is currently in `av-database-postgres`. Options:

1. **Feature-gate in crypto-loaders:** `caching` feature enables av-database-postgres dependency
2. **Extract trait to shared crate:** Create `crypto-cache` trait crate
3. **Pass closures:** Use callback-based caching instead of trait

**Recommendation:** Option 1 - Feature-gate, simplest migration path.

---

## Testing Checklist

- [ ] `crypto-loaders` compiles standalone
- [ ] `crypto-loaders` tests pass
- [ ] `av-loaders` compiles with `crypto-loaders` dependency
- [ ] All workspace tests pass
- [ ] `av-cli` works with refactored code
- [ ] Backward compatibility maintained via re-exports