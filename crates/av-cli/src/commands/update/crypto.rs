use anyhow::Result;
use clap::Subcommand;
use av_core::Config;

use crate::commands::update::crypto_update_functions;
use crate::commands::update::crypto_update_cli;

pub use crypto_update_functions::*;
pub use crypto_update_cli::*;

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
        CryptoUpdateCommands::All(args) => {
            update_crypto_command(args, config).await
        }
    }
}