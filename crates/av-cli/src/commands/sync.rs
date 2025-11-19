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

use crate::config::Config;
use anyhow::Result;
use clap::{Args, Subcommand};

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
