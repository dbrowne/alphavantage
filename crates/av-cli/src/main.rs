


use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

use av_core::Config;

mod commands;
use commands::{
  load::{LoadCommands, handle_load},
  query::{QueryCommands, handle_query},
  sync::{SyncCommands, handle_sync},
  update::{CryptoUpdateCommands, handle_crypto_update},
};

mod config;

#[derive(Parser,Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "av-cli")]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  /// Verbose output
  #[arg(short, long, global = true)]
  verbose: bool,
}

#[derive(Subcommand,Debug)]
enum Commands {
  Load(commands::load::LoadCommand),

  Query(commands::query::QueryCommand),
  Sync {
    #[command(subcommand)]
    cmd: SyncCommands,
  },
  Update {
    #[command(subcommand)]
    cmd: UpdateCommands,
  },
}
#[derive(Subcommand, Debug)]
pub enum UpdateCommands {
  Crypto {
    #[command(subcommand)]
    cmd: CryptoUpdateCommands,
  },
}

#[tokio::main]
async fn main() -> Result<()> {
  // Load environment variables
  dotenv().ok();

  // Parse CLI arguments
  let cli = Cli::parse();

  // Initialize logging
  let log_level = if cli.verbose { "debug" } else { "info" };
  tracing_subscriber::fmt().with_env_filter(log_level).init();

  // Load configuration
  let config = config::Config::from_env()?;

  // Execute command
  match cli.command {
    Commands::Load(cmd) => commands::load::execute(cmd, config).await?,
    Commands::Query(cmd) => commands::query::execute(cmd, config).await?,
    Commands::Sync { cmd } => handle_sync(cmd, config).await,
    Commands::Update { cmd } => handle_update(cmd, config).await,
  }

  Ok(())
}

async fn handle_update(cmd: UpdateCommands, config: Config) -> Result<()> {
  match cmd {
    UpdateCommands::Crypto { cmd } => handle_crypto_update(cmd, config).await,
  }
}