/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

mod commands;
use crate::commands::update::stats::{StatsCommands, handle_stats};
use commands::{
  load::LoadCommand,
  query::QueryCommand,
  sync::{SyncCommands, handle_sync},
  update::crypto::{CryptoUpdateCommands, handle_crypto_update},
};

mod config;

#[derive(Parser, Debug)]
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

#[derive(Subcommand, Debug)]
enum Commands {
  Load(LoadCommand),
  Query(QueryCommand),
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
  Stats {
    #[command(subcommand)]
    cmd: StatsCommands,
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
    Commands::Sync { cmd } => handle_sync(cmd, config).await?,
    Commands::Update { cmd } => handle_update(cmd, config).await?,
  }

  Ok(())
}

async fn handle_update(cmd: UpdateCommands, config: config::Config) -> Result<()> {
  match cmd {
    UpdateCommands::Crypto { cmd } => {
      // Convert config::Config to av_core::Config for the crypto handler
      let core_config = av_core::Config {
        api_key: config.api_config.api_key,
        base_url: config.api_config.base_url,
        rate_limit: config.api_config.rate_limit,
        timeout_secs: config.api_config.timeout_secs,
        max_retries: config.api_config.max_retries,
      };
      handle_crypto_update(cmd, core_config).await
    }
    UpdateCommands::Stats { cmd } => handle_stats(cmd, config).await,
  }
}
