use clap::{Args, Subcommand};
use anyhow::Result;
use crate::config::Config;

pub mod securities;

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
  Overviews {
    /// Limit the number of securities to process
    #[arg(short, long)]
    limit: Option<usize>,
  },

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

pub async fn execute(cmd: LoadCommand, config: Config) -> Result<()> {
  match cmd.command {
    LoadSubcommands::Securities(args) => securities::execute(args, config).await,
    LoadSubcommands::Overviews { limit: _ } => {
      todo!("Implement overview loading")
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