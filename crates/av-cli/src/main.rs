use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

mod commands;
mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  /// Verbose output
  #[arg(short, long, global = true)]
  verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
  /// Load data from various sources
  Load(commands::load::LoadCommand),

  /// Query data from the database
  Query(commands::query::QueryCommand),
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
  }

  Ok(())
}
