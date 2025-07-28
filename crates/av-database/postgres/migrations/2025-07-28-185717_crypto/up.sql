-- Your SQL goes here

-- Main cryptocurrency overview table
CREATE TABLE crypto_overviews (
                                  sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                                  symbol                  VARCHAR(20) NOT NULL,
                                  name                    TEXT NOT NULL,
                                  slug                    VARCHAR(100),         -- URL-friendly name (e.g., 'bitcoin')
                                  description             TEXT,

    -- Market data
                                  market_cap_rank         INTEGER,
                                  market_cap              BIGINT,
                                  fully_diluted_valuation BIGINT,
                                  volume_24h              BIGINT,
                                  volume_change_24h       NUMERIC(10,2),

    -- Price data
                                  current_price           NUMERIC(20,8),
                                  price_change_24h        NUMERIC(20,8),
                                  price_change_pct_24h    NUMERIC(10,2),
                                  price_change_pct_7d     NUMERIC(10,2),
                                  price_change_pct_14d    NUMERIC(10,2),
                                  price_change_pct_30d    NUMERIC(10,2),
                                  price_change_pct_60d    NUMERIC(10,2),
                                  price_change_pct_200d   NUMERIC(10,2),
                                  price_change_pct_1y     NUMERIC(10,2),

    -- Historical data
                                  ath                     NUMERIC(20,8),       -- All-time high
                                  ath_date                TIMESTAMPTZ,
                                  ath_change_percentage   NUMERIC(10,2),
                                  atl                     NUMERIC(20,8),       -- All-time low
                                  atl_date                TIMESTAMPTZ,
                                  atl_change_percentage   NUMERIC(10,2),

    -- ROI (Return on Investment)
                                  roi_times               NUMERIC(20,2),       -- e.g., 100x since launch
                                  roi_currency            VARCHAR(10),
                                  roi_percentage          NUMERIC(20,2),

    -- Supply data
                                  circulating_supply      NUMERIC(30,8),
                                  total_supply            NUMERIC(30,8),
                                  max_supply              NUMERIC(30,8),

    -- Timestamps
                                  last_updated            TIMESTAMPTZ,
                                  c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                  m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Technical information table
CREATE TABLE crypto_technical (
                                  sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,

    -- Blockchain info
                                  blockchain_platform     VARCHAR(100),        -- e.g., 'Ethereum', 'Binance Smart Chain'
                                  token_standard          VARCHAR(50),         -- e.g., 'ERC-20', 'BEP-20'
                                  consensus_mechanism     VARCHAR(100),        -- e.g., 'Proof of Work', 'Proof of Stake'
                                  hashing_algorithm       VARCHAR(100),        -- e.g., 'SHA-256', 'Ethash'

    -- Network stats
                                  block_time_minutes      NUMERIC(10,2),
                                  block_reward            NUMERIC(20,8),
                                  block_height            BIGINT,
                                  hash_rate               NUMERIC(30,2),
                                  difficulty              NUMERIC(30,2),

    -- Development
                                  github_forks            INTEGER,
                                  github_stars            INTEGER,
                                  github_subscribers      INTEGER,
                                  github_total_issues     INTEGER,
                                  github_closed_issues    INTEGER,
                                  github_pull_requests    INTEGER,
                                  github_contributors     INTEGER,
                                  github_commits_4_weeks  INTEGER,

    -- Categories
                                  is_defi                 BOOLEAN DEFAULT FALSE,
                                  is_stablecoin           BOOLEAN DEFAULT FALSE,
                                  is_nft_platform         BOOLEAN DEFAULT FALSE,
                                  is_exchange_token       BOOLEAN DEFAULT FALSE,
                                  is_gaming               BOOLEAN DEFAULT FALSE,
                                  is_metaverse            BOOLEAN DEFAULT FALSE,
                                  is_privacy_coin         BOOLEAN DEFAULT FALSE,
                                  is_layer2               BOOLEAN DEFAULT FALSE,
                                  is_wrapped              BOOLEAN DEFAULT FALSE,

    -- Other info
                                  genesis_date            DATE,
                                  ico_price               NUMERIC(20,8),
                                  ico_date                DATE,

                                  c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                  m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Social and community data
CREATE TABLE crypto_social (
                               sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,

    -- Official links
                               website_url             TEXT,
                               whitepaper_url          TEXT,
                               github_url              TEXT,

    -- Social media
                               twitter_handle          VARCHAR(100),
                               twitter_followers       INTEGER,
                               telegram_url            TEXT,
                               telegram_members        INTEGER,
                               discord_url             TEXT,
                               discord_members         INTEGER,
                               reddit_url              TEXT,
                               reddit_subscribers      INTEGER,
                               facebook_url            TEXT,
                               facebook_likes          INTEGER,

    -- Community scores
                               coingecko_score         NUMERIC(5,2),
                               developer_score         NUMERIC(5,2),
                               community_score         NUMERIC(5,2),
                               liquidity_score         NUMERIC(5,2),
                               public_interest_score   NUMERIC(5,2),

    -- Sentiment
                               sentiment_votes_up_pct  NUMERIC(5,2),
                               sentiment_votes_down_pct NUMERIC(5,2),

                               c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trading pairs and exchanges
CREATE TABLE crypto_markets (
                                id                      SERIAL PRIMARY KEY,
                                sid                     BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                                exchange                VARCHAR(100) NOT NULL,
                                base                    VARCHAR(20) NOT NULL,
                                target                  VARCHAR(20) NOT NULL,
                                market_type             VARCHAR(20),         -- 'spot', 'futures', 'perpetual'

    -- Volume and liquidity
                                volume_24h              NUMERIC(30,2),
                                volume_percentage       NUMERIC(10,2),       -- % of total volume
                                bid_ask_spread_pct      NUMERIC(10,4),
                                liquidity_score         VARCHAR(20),         -- e.g., 'green', 'yellow', 'red'

    -- Trading info
                                is_active               BOOLEAN DEFAULT TRUE,
                                is_anomaly              BOOLEAN DEFAULT FALSE,
                                is_stale                BOOLEAN DEFAULT FALSE,
                                trust_score             VARCHAR(20),         -- 'green', 'yellow', 'red', 'grey'

                                last_traded_at          TIMESTAMPTZ,
                                last_fetch_at           TIMESTAMPTZ,

                                c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

                                UNIQUE(sid, exchange, base, target)
);

-- Create indexes for performance
CREATE INDEX idx_crypto_overviews_symbol ON crypto_overviews(symbol);
CREATE INDEX idx_crypto_overviews_rank ON crypto_overviews(market_cap_rank);
CREATE INDEX idx_crypto_overviews_market_cap ON crypto_overviews(market_cap DESC);
CREATE INDEX idx_crypto_overviews_volume ON crypto_overviews(volume_24h DESC);

CREATE INDEX idx_crypto_technical_blockchain ON crypto_technical(blockchain_platform);
CREATE INDEX idx_crypto_technical_categories ON crypto_technical(is_defi, is_stablecoin, is_nft_platform);

CREATE INDEX idx_crypto_social_scores ON crypto_social(coingecko_score DESC);

CREATE INDEX idx_crypto_markets_sid ON crypto_markets(sid);
CREATE INDEX idx_crypto_markets_exchange ON crypto_markets(exchange);
CREATE INDEX idx_crypto_markets_volume ON crypto_markets(volume_24h DESC);

-- Create a comprehensive view
CREATE VIEW crypto_full_view AS
SELECT
    s.sid,
    s.symbol,
    s.name,
    co.slug,
    co.description,
    co.market_cap_rank,
    co.market_cap,
    co.fully_diluted_valuation,
    co.current_price,
    co.volume_24h,
    co.price_change_pct_24h,
    co.price_change_pct_7d,
    co.price_change_pct_30d,
    co.circulating_supply,
    co.total_supply,
    co.max_supply,
    co.ath,
    co.ath_date,
    co.atl,
    co.atl_date,
    ct.blockchain_platform,
    ct.consensus_mechanism,
    ct.is_defi,
    ct.is_stablecoin,
    ct.genesis_date,
    cs.website_url,
    cs.twitter_handle,
    cs.github_url,
    cs.coingecko_score,
    cs.developer_score,
    cs.community_score
FROM symbols s
         INNER JOIN crypto_overviews co ON s.sid = co.sid
         LEFT JOIN crypto_technical ct ON s.sid = ct.sid
         LEFT JOIN crypto_social cs ON s.sid = cs.sid
WHERE s.sec_type = 'Cryptocurrency'
ORDER BY co.market_cap_rank;