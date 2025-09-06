use clap::Args;

#[derive(Args, Debug)]
pub struct UpdateCryptoArgs {
    /// Specific symbols to update (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    pub symbols: Option<String>,

    /// Limit number of symbols to update
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Only update basic crypto data (descriptions, market cap ranks)
    #[arg(long)]
    pub basic_only: bool,

    /// Only update social data (social media metrics)
    #[arg(long)]
    pub social_only: bool,

    /// Only update technical data (blockchain/GitHub data)
    #[arg(long)]
    pub technical_only: bool,

    /// Delay between requests in milliseconds
    #[arg(long, default_value = "2000")]
    pub delay_ms: u64,

    /// CoinGecko API key for enhanced data
    #[arg(long, env = "COINGECKO_API_KEY")]
    pub coingecko_api_key: Option<String>,

    /// GitHub token for GitHub API access
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,

    /// Dry run mode - don't update database
    #[arg(long)]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}