use clap::{Args, Subcommand};
use anyhow::Result;
use crate::config::Config;

pub mod securities;
pub mod overviews;
 pub  mod crypto;
pub  mod crypto_overview;
use tracing::info;

#[derive(Args)]
pub struct LoadCommand {
  #[command(subcommand)]
  command: LoadSubcommands,
}

#[derive(Subcommand)]
enum LoadSubcommands {
  /// Load securities from CSV files and fetch data from AlphaVantage
  Securities(securities::SecuritiesArgs),

  /// Load company overviews for existing securities
  Overviews(overviews::OverviewsArgs),

  Crypto(crypto::CryptoArgs),
  /// Load cryptocurrency overviews (market data)
  CryptoOverview(crypto_overview::CryptoOverviewArgs),

  /// Update GitHub data for cryptocurrencies
  UpdateGithub(crypto_overview::UpdateGitHubArgs),

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
  News {
    /// Limit the number of articles per symbol
    #[arg(short, long, default_value = "50")]
    limit: usize,
  },
}

// Changed back to async
pub async fn execute(cmd: LoadCommand, config: Config) -> Result<()> {
  match cmd.command {
    LoadSubcommands::Securities(args) => securities::execute(args, config).await,
    LoadSubcommands::Overviews(args) => overviews::execute(args, config).await,
    LoadSubcommands::Crypto(args) => crypto::execute(args, config).await,
    LoadSubcommands::CryptoOverview(args) => crypto_overview::execute(args, config).await,
    LoadSubcommands::UpdateGithub(args) => {
      info!("Updating GitHub data for cryptocurrencies");
      crypto_overview::update_github_data( args,config).await
    }
    LoadSubcommands::Intraday { symbol: _, interval: _ } => {
      todo!("Implement intraday loading")
    }
    LoadSubcommands::Daily { symbol: _ } => {
      todo!("Implement daily loading")
    }
    LoadSubcommands::News { limit: _ } => {
      todo!("Implement news loading")
    }
  }
}