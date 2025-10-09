use crate::config::Config;
use anyhow::Result;
use clap::{Args, Subcommand};

pub mod crypto;
pub mod crypto_intraday;
pub mod crypto_mapping;
mod crypto_markets;
pub mod crypto_metadata;
pub mod crypto_news;
pub mod crypto_overview;
pub mod crypto_social;
pub mod daily;
pub mod intraday;
pub mod news;
pub mod news_utils;
pub mod overviews;
pub mod securities;
pub mod top_movers;

use tracing::info;

#[derive(Args, Debug)]
pub struct LoadCommand {
  #[command(subcommand)]
  command: LoadSubcommands,
}

#[derive(Subcommand, Debug)]
enum LoadSubcommands {
  Securities(securities::SecuritiesArgs),

  Overviews(overviews::OverviewsArgs),

  Crypto(crypto::CryptoArgs),

  CryptoOverview(crypto_overview::CryptoOverviewArgs),

  CryptoSocial(crypto_social::CryptoSocialArgs),

  UpdateGithub(crypto_overview::UpdateGitHubArgs),

  CryptoMarkets(crypto_markets::CryptoMarketsArgs),

  CryptoMapping(crypto_mapping::MappingArgs),

  CryptoMetadata(crypto_metadata::CryptoMetadataArgs),
  CryptoNews(crypto_news::CryptoNewsArgs),
  CryptoIntraday(crypto_intraday::CryptoIntradayArgs),

  News(news::NewsArgs),

  TopMovers(top_movers::TopMoversArgs),
  Daily(daily::DailyArgs),
  #[clap(name = "intraday")]
  Intraday(intraday::IntradayArgs),
}

// And add to the execute match:
pub async fn execute(cmd: LoadCommand, config: Config) -> Result<()> {
  match cmd.command {
    LoadSubcommands::Securities(args) => securities::execute(args, config).await,
    LoadSubcommands::Overviews(args) => overviews::execute(args, config).await,
    LoadSubcommands::Crypto(args) => crypto::execute(args, config).await,
    LoadSubcommands::CryptoOverview(args) => crypto_overview::execute(args, config).await,
    LoadSubcommands::CryptoMarkets(args) => crypto_markets::execute(args, &config).await,
    LoadSubcommands::CryptoSocial(args) => crypto_social::execute(args, config).await,
    LoadSubcommands::CryptoMapping(args) => crypto_mapping::execute(args, &config).await,
    LoadSubcommands::CryptoMetadata(args) => crypto_metadata::execute(args, &config).await,
    LoadSubcommands::CryptoNews(args) => crypto_news::execute(args, config).await,
    LoadSubcommands::CryptoIntraday(args) => crypto_intraday::execute(args, config).await,
    LoadSubcommands::UpdateGithub(args) => {
      info!("Updating GitHub data for cryptocurrencies");
      crypto_overview::update_github_data(args, config).await
    }
    LoadSubcommands::Intraday(args) => intraday::execute(args, config).await,
    LoadSubcommands::Daily(args) => daily::execute(args, config).await,
    LoadSubcommands::News(args) => news::execute(args, config).await,
    LoadSubcommands::TopMovers(args) => top_movers::execute(args, config).await,
  }
}
