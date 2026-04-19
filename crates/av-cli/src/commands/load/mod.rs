/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

//! Initial data ingestion commands for `av-cli load`.
//!
//! This module provides the `av-cli load` command group, which fetches data from
//! external APIs (AlphaVantage, CoinGecko, CoinMarketCap, SosoValue) and
//! persists it to the PostgreSQL database. Unlike [`update`](super::update)
//! (enrichment / analytics) or [`sync`](super::sync) (refresh), `load` is the
//! **primary ingestion path** — it creates new records and populates tables for
//! the first time.
//!
//! ## Subcommand Overview
//!
//! ```text
//! av-cli load
//! ├── securities         Load NASDAQ/NYSE securities from CSV files
//! ├── overviews          Fetch company overview data for equities
//! ├── daily              Load daily price history for equities
//! ├── intraday           Load intraday price data for equities
//! ├── news               Fetch equity news articles with sentiment
//! ├── top-movers         Fetch market top gainers/losers
//! ├── missing-symbols    Resolve pending missing symbols from news feeds
//! ├── crypto             Load crypto symbols from CoinGecko/CMC/SosoValue
//! ├── crypto-overview    Fetch crypto overview data (prices, supply, market cap)
//! ├── crypto-markets     Load crypto trading markets/exchange pairs
//! ├── crypto-mapping     Discover/initialize API source mappings for crypto
//! ├── crypto-metadata    Load enhanced crypto metadata from multiple sources
//! ├── crypto-news        Fetch crypto news and sentiment from AlphaVantage
//! ├── crypto-intraday    Load crypto intraday price data at specified intervals
//! ├── crypto-prices      Fetch current crypto prices from multiple sources
//! ├── crypto-details     Load social and technical data for crypto from CoinGecko
//! └── update-github      Update GitHub repository data for cryptocurrencies
//! ```
//!
//! ## Module Organization
//!
//! The 20 submodules are organized into three functional groups:
//!
//! ### Equity Modules
//!
//! - [`securities`] — Loads NASDAQ/NYSE securities from CSV files. Validates
//!   region names, deduplicates, and generates Security IDs (SIDs).
//! - [`overviews`] — Fetches company overview data (financials, ratios, market
//!   info) for equities from AlphaVantage.
//! - [`daily`] — Loads daily price history (`compact` = 100 days, `full` = 20+
//!   years) into the `summaryprices` table.
//! - [`intraday`] — Loads intraday price data for equities with multi-interval
//!   support (1min, 5min, 15min, 30min, 60min).
//! - [`news`] — Fetches equity news articles from AlphaVantage with sentiment
//!   scoring, topic filtering, and hash-based deduplication.
//! - [`top_movers`] — Fetches market top gainers/losers for a given date and
//!   tracks missing symbols found in the response.
//! - [`missing_symbols`] — Resolves pending missing symbols (logged during news
//!   or top-movers ingestion) via AlphaVantage symbol search.
//!
//! ### Cryptocurrency Modules
//!
//! - [`crypto`] — Loads cryptocurrency symbols from CoinGecko, CoinMarketCap,
//!   and SosoValue APIs. Generates crypto SIDs and populates `symbols`,
//!   `crypto_api_map`, and `symbol_mappings` tables.
//! - [`crypto_overview`] — Fetches crypto overview data (prices, supply, market
//!   cap, launch dates) from CoinMarketCap/CoinGecko with optional GitHub
//!   scraping. Also provides the `update-github` subcommand.
//! - [`crypto_markets`] — Loads trading market/exchange pairs from CoinGecko
//!   with dynamic mapping and volume validation.
//! - [`crypto_mapping`] — Discovers missing CoinGecko mappings for crypto
//!   symbols and displays mapping statistics.
//! - [`crypto_metadata`] — Loads enhanced metadata from CoinGecko and
//!   AlphaVantage with multi-source support and caching.
//! - [`crypto_news`] — Fetches crypto-specific news and sentiment from
//!   AlphaVantage with pagination and caching.
//! - [`crypto_intraday`] — Loads crypto intraday price data at specified
//!   intervals, deduplicating based on timestamps.
//! - [`crypto_prices`] — Fetches current crypto prices from multiple sources
//!   with parallel fetching and caching.
//! - [`crypto_details`] — Loads social (Twitter, Reddit, community scores) and
//!   technical (blockchain, GitHub) data from CoinGecko.
//!
//! ### Utility Modules
//!
//! These have no CLI-facing `Args` struct or `execute` function:
//!
//! - [`missing_symbol_logger`] — `log_missing_symbol()` and
//!   `log_missing_symbols_batch()` for recording unrecognized symbols
//!   encountered during news or top-movers ingestion.
//! - [`news_utils`] — `save_news_to_database()` for transactional persistence
//!   of parsed news data with hash-based deduplication.
//! - [`numeric_helpers`] — Safe `f64` to `BigDecimal` conversion with precision
//!   clamping for database `NUMERIC` columns (`f64_to_price_bigdecimal`,
//!   `f64_to_supply_bigdecimal`).
//! - [`sid_generator`] — `SidGenerator` struct for encoding/decoding Security
//!   IDs by reading max existing SIDs from the database and generating the
//!   next ID per `SecurityType`.
//!
//! ## Configuration
//!
//! All subcommands receive the full CLI [`Config`](crate::config::Config) which
//! includes both `database_url` and API credentials. Some subcommands pass
//! `config` by value, others by reference — this is a historical inconsistency,
//! not a design choice.
//!
//! ## Common Argument Patterns
//!
//! Most subcommands share a common set of flags (though not via a shared struct):
//!
//! | Flag                  | Typical default | Description                              |
//! |-----------------------|-----------------|------------------------------------------|
//! | `--dry-run`           | `false`         | Print actions without writing to database |
//! | `--continue-on-error` | varies          | Skip failures instead of aborting         |
//! | `--limit`             | `None`          | Cap number of records to process          |
//! | `--concurrent`        | `5`             | Number of concurrent API requests         |
//! | `--api-delay`         | `800` ms        | Delay between API calls for rate limiting |
//! | `--enable-cache`      | `true`          | Use local response cache                  |
//! | `--force-refresh`     | `false`         | Ignore cache and fetch fresh data         |
//! | `--verbose`           | `false`         | Enable detailed output                    |

use crate::config::Config;
use anyhow::Result;

use clap::{Args, Subcommand};

/// Load NASDAQ/NYSE securities from CSV files via AlphaVantage API.
pub mod crypto;
/// Load social and technical crypto data from CoinGecko.
pub mod crypto_details;
/// Load crypto intraday price data at specified intervals.
pub mod crypto_intraday;
/// Discover and initialize API source mappings for crypto symbols.
pub mod crypto_mapping;
/// Load crypto trading markets/exchange pairs from CoinGecko.
mod crypto_markets;
/// Load enhanced crypto metadata from CoinGecko and AlphaVantage.
pub mod crypto_metadata;
/// Fetch crypto news and sentiment from AlphaVantage.
pub mod crypto_news;
/// Fetch crypto overview data (prices, supply, market cap) from CMC/CoinGecko.
pub mod crypto_overview;
/// Fetch current crypto prices from multiple sources with caching.
pub mod crypto_prices;
/// Load daily price history for equities into the `summaryprices` table.
pub mod daily;
/// Load intraday price data for equities with multi-interval support.
pub mod intraday;
/// Utility: log unrecognized symbols encountered during news/top-movers ingestion.
pub mod missing_symbol_logger;
/// Resolve pending missing symbols via AlphaVantage symbol search.
pub mod missing_symbols;
/// Fetch equity news articles from AlphaVantage with sentiment and deduplication.
pub mod news;
/// Utility: transactional persistence of parsed news data with hash deduplication.
pub mod news_utils;
/// Utility: safe `f64` to `BigDecimal` conversion with precision clamping.
pub mod numeric_helpers;
/// Fetch company overview data (financials, ratios) for equities from AlphaVantage.
pub mod overviews;
/// Load NASDAQ/NYSE securities from CSV files with SID generation.
pub mod securities;
/// Utility: `SidGenerator` for encoding/decoding Security IDs per `SecurityType`.
pub mod sid_generator;
/// Fetch market top gainers/losers for a given date from AlphaVantage.
pub mod top_movers;

use tracing::info;

/// Top-level argument wrapper for `av-cli load`.
///
/// Contains a single `#[command(subcommand)]` field that delegates to
/// [`LoadSubcommands`]. This two-level nesting is required by clap to attach
/// the `load` keyword as a subcommand of the root CLI while still allowing
/// `load` itself to have subcommands.
#[derive(Args, Debug)]
pub struct LoadCommand {
  #[command(subcommand)]
  command: LoadSubcommands,
}

/// All subcommands available under `av-cli load`.
///
/// Each variant wraps the module-specific `Args` struct from its corresponding
/// submodule. The variant name determines the CLI subcommand name (converted to
/// kebab-case by clap, e.g., `CryptoOverview` → `crypto-overview`), except
/// `Intraday` which is explicitly named via `#[clap(name = "intraday")]`.
///
/// ## Equity Subcommands
///
/// - `Securities` — Load NASDAQ/NYSE securities from CSV files
/// - `Overviews` — Fetch company overview data for equities
/// - `Daily` — Load daily price history
/// - `Intraday` — Load intraday price data
/// - `News` — Fetch equity news with sentiment
/// - `TopMovers` — Fetch market top gainers/losers
/// - `MissingSymbols` — Resolve unrecognized symbols from news/top-movers
///
/// ## Cryptocurrency Subcommands
///
/// - `Crypto` — Load crypto symbols from CoinGecko/CMC/SosoValue
/// - `CryptoOverview` — Fetch crypto overview data
/// - `UpdateGithub` — Update GitHub repo data for crypto (via [`crypto_overview`])
/// - `CryptoMarkets` — Load trading markets/exchange pairs
/// - `CryptoMapping` — Discover API source mappings
/// - `CryptoMetadata` — Load enhanced metadata
/// - `CryptoNews` — Fetch crypto news and sentiment
/// - `CryptoIntraday` — Load crypto intraday price data
/// - `CryptoPrices` — Fetch current crypto prices
/// - `CryptoDetails` — Load social and technical data
#[derive(Subcommand, Debug)]
enum LoadSubcommands {
  /// Load NASDAQ/NYSE securities from CSV files with SID generation.
  Securities(securities::SecuritiesArgs),

  /// Fetch company overview data (financials, ratios) for equities.
  Overviews(overviews::OverviewsArgs),

  /// Load cryptocurrency symbols from CoinGecko, CoinMarketCap, and SosoValue.
  Crypto(crypto::CryptoArgs),

  /// Fetch crypto overview data (prices, supply, market cap, launch dates).
  CryptoOverview(crypto_overview::CryptoOverviewArgs),

  /// Update GitHub repository data for cryptocurrencies.
  UpdateGithub(crypto_overview::UpdateGitHubArgs),

  /// Load crypto trading markets/exchange pairs from CoinGecko.
  CryptoMarkets(crypto_markets::CryptoMarketsArgs),

  /// Discover and initialize API source mappings for crypto symbols.
  CryptoMapping(crypto_mapping::MappingArgs),

  /// Load enhanced crypto metadata from CoinGecko and AlphaVantage.
  CryptoMetadata(crypto_metadata::CryptoMetadataArgs),
  /// Fetch crypto news and sentiment from AlphaVantage.
  CryptoNews(crypto_news::CryptoNewsArgs),
  /// Load crypto intraday price data at specified intervals.
  CryptoIntraday(crypto_intraday::CryptoIntradayArgs),
  /// Fetch current crypto prices from multiple sources.
  CryptoPrices(crypto_prices::CryptoPricesArgs),
  /// Load social and technical crypto data from CoinGecko.
  CryptoDetails(crypto_details::CryptoDetailsArgs),

  /// Resolve pending missing symbols via AlphaVantage symbol search.
  MissingSymbols(missing_symbols::MissingSymbolsArgs),

  /// Fetch equity news articles from AlphaVantage with sentiment scoring.
  News(news::NewsArgs),

  /// Fetch market top gainers/losers for a given date.
  TopMovers(top_movers::TopMoversArgs),
  /// Load daily price history for equities.
  Daily(daily::DailyArgs),
  /// Load intraday price data for equities.
  #[clap(name = "intraday")]
  Intraday(intraday::IntradayArgs),
}

/// Dispatches `av-cli load` subcommands to their module-level `execute` functions.
///
/// Each [`LoadSubcommands`] variant is matched and forwarded to the corresponding
/// module's `execute()` (or `update_github_data()` for `UpdateGithub`). All
/// handlers are async and receive the full CLI [`Config`].
///
/// # Arguments
///
/// * `cmd` — The parsed [`LoadCommand`] containing the selected subcommand and its
///   arguments.
/// * `config` — The full CLI [`Config`](crate::config::Config) with `database_url`
///   and API credentials.
///
/// # Errors
///
/// Returns errors from the individual subcommand handlers — typically API failures,
/// database write errors, CSV parsing errors, or missing configuration.
pub async fn execute(cmd: LoadCommand, config: Config) -> Result<()> {
  match cmd.command {
    LoadSubcommands::Securities(args) => securities::execute(args, config).await,
    LoadSubcommands::Overviews(args) => overviews::execute(args, config).await,
    LoadSubcommands::Crypto(args) => crypto::execute(args, config).await,
    LoadSubcommands::CryptoOverview(args) => crypto_overview::execute(args, config).await,
    LoadSubcommands::CryptoMarkets(args) => crypto_markets::execute(args, &config).await,
    LoadSubcommands::CryptoMapping(args) => crypto_mapping::execute(args, &config).await,
    LoadSubcommands::CryptoMetadata(args) => crypto_metadata::execute(args, &config).await,
    LoadSubcommands::CryptoNews(args) => crypto_news::execute(args, config).await,
    LoadSubcommands::CryptoIntraday(args) => crypto_intraday::execute(args, config).await,
    LoadSubcommands::CryptoPrices(args) => crypto_prices::execute(args, config).await,
    LoadSubcommands::CryptoDetails(args) => crypto_details::execute(args, config).await,
    LoadSubcommands::MissingSymbols(args) => missing_symbols::execute(args, config).await,
    LoadSubcommands::UpdateGithub(args) => {
      info!("Updating GitHub data for cryptocurrencies");
      crypto_overview::update_github_data(args, config).await
    }
    LoadSubcommands::Intraday(args) => intraday::execute(args, config).await,
    LoadSubcommands::Daily(args) => daily::execute(args, config).await,
    LoadSubcommands::News(args) => news::execute(args, config).await,
    LoadSubcommands::TopMovers(args) => top_movers::execute(args, config).await,
  }
}
