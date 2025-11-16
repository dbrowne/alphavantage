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
use av_core::Config;
use clap::Subcommand;

use crate::commands::update::crypto_update_cli;
use crate::commands::update::crypto_update_functions;

use crate::commands::update::crypto_metadata_etl;
pub use crypto_update_cli::*;
pub use crypto_update_functions::*;

#[derive(Subcommand, Debug)]
pub enum CryptoUpdateCommands {
  /// Update crypto_overview_basic table with descriptions and market cap ranks
  Basic(UpdateCryptoArgs),
  /// Update crypto_social table with social media and scoring data
  Social(UpdateCryptoArgs),
  /// Update crypto_technical table with blockchain and GitHub data
  Technical(UpdateCryptoArgs),
  /// Update all crypto tables at once
  All(UpdateCryptoArgs),
  Metadata,
}

pub async fn handle_crypto_update(cmd: CryptoUpdateCommands, config: Config) -> Result<()> {
  match cmd {
    CryptoUpdateCommands::Basic(mut args) => {
      args.basic_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::Social(mut args) => {
      args.social_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::Technical(mut args) => {
      args.technical_only = true;
      update_crypto_command(args, config).await
    }
    CryptoUpdateCommands::All(args) => update_crypto_command(args, config).await,
    CryptoUpdateCommands::Metadata => {
      // Get database URL from environment
      dotenvy::dotenv().ok();
      let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set in .env file"))?;

      crypto_metadata_etl::execute_metadata_etl(&database_url)?;
      Ok(())
    }
  }
}
