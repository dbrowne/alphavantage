use clap::{Args, Subcommand};
use anyhow::Result;
use crate::config::Config;

#[derive(Args, Debug)]
pub struct SyncCommand {
    #[command(subcommand)]
    command: SyncCommands,
}

#[derive(Subcommand, Debug)]
pub enum SyncCommands {
    /// Sync market data from AlphaVantage
    Market {
        /// Symbol to sync
        #[arg(short, long)]
        symbol: Option<String>,
    },

    /// Sync cryptocurrency data
    Crypto {
        /// Limit number of cryptocurrencies to sync
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

pub async fn handle_sync(cmd: SyncCommands, _config: Config) -> Result<()> {
    match cmd {
        SyncCommands::Market { symbol: _ } => {
            todo!("Implement market sync")
        }
        SyncCommands::Crypto { limit: _ } => {
            todo!("Implement crypto sync")
        }
    }
}