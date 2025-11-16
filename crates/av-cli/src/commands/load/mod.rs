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
pub mod crypto_prices;
pub mod crypto_social;
pub mod crypto_symbols;
pub mod daily;
pub mod intraday;
pub mod missing_symbol_logger;
pub mod missing_symbols;
pub mod news;
pub mod news_utils;
pub mod numeric_helpers;
pub mod overviews;
pub mod securities;
pub mod sid_generator;
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
  CryptoPrices(crypto_prices::CryptoPricesArgs),
  CryptoSymbols(crypto_symbols::CryptoSymbolsArgs),

  MissingSymbols(missing_symbols::MissingSymbolsArgs),

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
    LoadSubcommands::CryptoPrices(args) => crypto_prices::execute(args, config).await,
    LoadSubcommands::CryptoSymbols(args) => crypto_symbols::execute(args, config).await,
    LoadSubcommands::MissingSymbols(args) => missing_symbols::execute(args, config).await,
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
