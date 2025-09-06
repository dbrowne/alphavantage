use clap::{Args, Subcommand};
use anyhow::Result;
use crate::config::Config;

#[derive(Args, Debug)]
pub struct QueryCommand {
  #[command(subcommand)]
  command: QuerySubcommands,
}

#[derive(Subcommand, Debug)]
enum QuerySubcommands {
  /// Query symbol information
  Symbol {
    /// Symbol to query
    symbol: String,
  },

  /// List all symbols
  ListSymbols {
    /// Filter by exchange
    #[arg(short, long)]
    exchange: Option<String>,

    /// Limit results
    #[arg(short, long, default_value = "100")]
    limit: usize,
  },
}

pub async fn execute(cmd: QueryCommand, _config: Config) -> Result<()> {
  match cmd.command {
    QuerySubcommands::Symbol { symbol: _ } => {
      todo!("Implement symbol query")
    }
    QuerySubcommands::ListSymbols { exchange: _, limit: _ } => {
      todo!("Implement list symbols")
    }
  }
}