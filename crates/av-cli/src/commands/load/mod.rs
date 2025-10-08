use crate::config::Config;
use anyhow::Result;
use clap::{Args, Subcommand};

pub mod crypto;
pub mod crypto_mapping;
mod crypto_markets;
pub mod crypto_metadata;
pub mod crypto_news;
pub mod crypto_overview;
pub mod crypto_social;
pub mod news;
pub mod news_utils;
pub mod overviews;
pub mod securities;
pub mod top_movers;

use tracing::info;

#[derive(Args, Debug)]
pub struct LoadCommand {
  #[command(subcommand)]
  command: LoadSubcommands,
}

#[derive(Subcommand, Debug)]
enum LoadSubcommands {
  /// Load securities from CSV files and fetch data from AlphaVantage
  Securities(securities::SecuritiesArgs),

  /// Load company overviews for existing securities
  Overviews(overviews::OverviewsArgs),

  Crypto(crypto::CryptoArgs),

  /// Load cryptocurrency overviews (market data)
  CryptoOverview(crypto_overview::CryptoOverviewArgs),

  /// Load cryptocurrency social data from CoinGecko and GitHub
  CryptoSocial(crypto_social::CryptoSocialArgs),

  /// Update GitHub data for cryptocurrencies
  UpdateGithub(crypto_overview::UpdateGitHubArgs),

  /// Load cryptocurrency market data from exchanges
  CryptoMarkets(crypto_markets::CryptoMarketsArgs),

  /// Manage cryptocurrency symbol mappings (discover, stats)
  CryptoMapping(crypto_mapping::MappingArgs),

  // Manage crypto Meta data
  CryptoMetadata(crypto_metadata::CryptoMetadataArgs),
  CryptoNews(crypto_news::CryptoNewsArgs),

  /// Load intraday price data
  Intraday {
    /// Symbol to load (if not specified, loads all active symbols)
    #[arg(short, long)]
    symbol: Option<String>,

    /// Time interval (1min, 5min, 15min, 30min, 60min)
    #[arg(short, long, default_value = "5min")]
    interval: String,
  },

  /// Load daily price data
  Daily {
    /// Symbol to load (if not specified, loads all active symbols)
    #[arg(short, long)]
    symbol: Option<String>,
  },

  /// Load news and sentiment data
  News(news::NewsArgs),
  
  TopMovers(top_movers::TopMoversArgs),
}

// And add to the execute match:
pub async fn execute(cmd: LoadCommand, config: Config) -> Result<()> {
  match cmd.command {
    LoadSubcommands::Securities(args) => securities::execute(args, config).await,
    LoadSubcommands::Overviews(args) => overviews::execute(args, config).await,
    LoadSubcommands::Crypto(args) => crypto::execute(args, config).await,
    LoadSubcommands::CryptoOverview(args) => crypto_overview::execute(args, config).await,
    LoadSubcommands::CryptoMarkets(args) => crypto_markets::execute(args, &config).await,
    LoadSubcommands::CryptoSocial(args) => crypto_social::execute(args, config).await,
    LoadSubcommands::CryptoMapping(args) => crypto_mapping::execute(args, &config).await,
    LoadSubcommands::CryptoMetadata(args) => crypto_metadata::execute(args, &config).await,
    LoadSubcommands::CryptoNews(args) => crypto_news::execute(args, config).await,
    LoadSubcommands::UpdateGithub(args) => {
      info!("Updating GitHub data for cryptocurrencies");
      crypto_overview::update_github_data(args, config).await
    }
    LoadSubcommands::Intraday { symbol: _, interval: _ } => {
      todo!("Implement intraday loading")
    }
    LoadSubcommands::Daily { symbol: _ } => {
      todo!("Implement daily loading")
    }
    LoadSubcommands::News(args) => news::execute(args, config).await,
    LoadSubcommands::TopMovers(args) => top_movers::execute(args, config).await,
  }
}
