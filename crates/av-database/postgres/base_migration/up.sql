
-- Optimized for TimescaleDB
-- =====================================================
-- CORE SYMBOLS TABLE (Modified to remove trading hours)
-- =====================================================
CREATE TABLE symbols (
                         sid             BIGINT PRIMARY KEY NOT NULL,
                         symbol          VARCHAR(20) NOT NULL,
                         name            TEXT NOT NULL,
                         sec_type        VARCHAR(50) NOT NULL,
                         region          VARCHAR(10) NOT NULL,
                         currency        VARCHAR(10) NOT NULL,
    -- Flags for data availability
                         overview        BOOLEAN NOT NULL DEFAULT FALSE,
                         intraday        BOOLEAN NOT NULL DEFAULT FALSE,
                         summary         BOOLEAN NOT NULL DEFAULT FALSE,
    -- Timestamps
                         c_time          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         m_time          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for symbols table
CREATE INDEX idx_symbols_symbol ON symbols(symbol);
CREATE INDEX idx_symbols_sec_type ON symbols(sec_type);
CREATE INDEX idx_symbols_active ON symbols(symbol)
    WHERE overview = TRUE OR intraday = TRUE OR summary = TRUE;

-- =====================================================
-- EQUITY-SPECIFIC DETAILS (Trading hours and exchange)
-- =====================================================
CREATE TABLE equity_details (
                                sid                 BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                                exchange            VARCHAR(20) NOT NULL,
                                market_open         TIME NOT NULL,
                                market_close        TIME NOT NULL,
                                timezone            VARCHAR(50) NOT NULL,
                                c_time              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                m_time              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for equity_details
CREATE INDEX idx_equity_details_exchange ON equity_details(exchange);

-- =====================================================
-- COMPANY OVERVIEWS (Fundamentals - Unchanged)
-- =====================================================
CREATE TABLE overviews (
                           sid                  BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                           symbol               VARCHAR(20) NOT NULL,
                           name                 TEXT NOT NULL,
                           description          TEXT NOT NULL,
                           cik                  VARCHAR(20) NOT NULL,
                           exchange             VARCHAR(20) NOT NULL,
                           currency             VARCHAR(10) NOT NULL,
                           country              VARCHAR(50) NOT NULL,
                           sector               VARCHAR(100) NOT NULL,
                           industry             VARCHAR(100) NOT NULL,
                           address              TEXT NOT NULL,
                           fiscal_year_end      VARCHAR(20) NOT NULL,
                           latest_quarter       DATE NOT NULL,
                           market_capitalization BIGINT NOT NULL,
                           ebitda               BIGINT NOT NULL,
                           pe_ratio             REAL NOT NULL,
                           peg_ratio            REAL NOT NULL,
                           book_value           REAL NOT NULL,
                           dividend_per_share   REAL NOT NULL,
                           dividend_yield       REAL NOT NULL,
                           eps                  REAL NOT NULL,
                           c_time               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                           m_time               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_overviews_sector ON overviews(sector);
CREATE INDEX idx_overviews_industry ON overviews(industry);

-- Extended overview data
CREATE TABLE overviewexts (
                              sid                         BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                              revenue_per_share_ttm       REAL NOT NULL,
                              profit_margin               REAL NOT NULL,
                              operating_margin_ttm        REAL NOT NULL,
                              return_on_assets_ttm        REAL NOT NULL,
                              return_on_equity_ttm        REAL NOT NULL,
                              revenue_ttm                 BIGINT NOT NULL,
                              gross_profit_ttm            BIGINT NOT NULL,
                              diluted_eps_ttm             REAL NOT NULL,
                              quarterly_earnings_growth_yoy REAL NOT NULL,
                              quarterly_revenue_growth_yoy  REAL NOT NULL,
                              analyst_target_price        REAL NOT NULL,
                              trailing_pe                 REAL NOT NULL,
                              forward_pe                  REAL NOT NULL,
                              price_to_sales_ratio_ttm    REAL NOT NULL,
                              price_to_book_ratio         REAL NOT NULL,
                              ev_to_revenue               REAL NOT NULL,
                              ev_to_ebitda                REAL NOT NULL,
                              beta                        REAL NOT NULL,
                              week_high_52                REAL NOT NULL,
                              week_low_52                 REAL NOT NULL,
                              day_moving_average_50       REAL NOT NULL,
                              day_moving_average_200      REAL NOT NULL,
                              shares_outstanding          BIGINT NOT NULL,
                              dividend_date               DATE,
                              ex_dividend_date            DATE,
                              c_time                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                              m_time                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =====================================================
-- PRICE DATA TABLES (TimescaleDB Hypertables)
-- =====================================================

-- Intraday prices
CREATE TABLE intradayprices (
                                eventid BIGSERIAL NOT NULL,
                                tstamp  TIMESTAMPTZ NOT NULL,
                                sid     BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                                symbol  VARCHAR(20) NOT NULL,
                                open    REAL NOT NULL,
                                high    REAL NOT NULL,
                                low     REAL NOT NULL,
                                close   REAL NOT NULL,
                                volume  BIGINT NOT NULL,
                                PRIMARY KEY (tstamp, sid, eventid)
);

SELECT create_hypertable('intradayprices', 'tstamp', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_intradayprices_sid_time ON intradayprices (sid, tstamp DESC);
CREATE INDEX idx_intradayprices_symbol_time ON intradayprices (symbol, tstamp DESC);
CREATE INDEX idx_intradayprices_eventid ON intradayprices (eventid);

-- Summary prices
CREATE TABLE summaryprices (
                               eventid BIGSERIAL NOT NULL,
                               tstamp  TIMESTAMPTZ NOT NULL,
                               date    DATE NOT NULL,
                               sid     BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                               symbol  VARCHAR(20) NOT NULL,
                               open    REAL NOT NULL,
                               high    REAL NOT NULL,
                               low     REAL NOT NULL,
                               close   REAL NOT NULL,
                               volume  BIGINT NOT NULL,
                               PRIMARY KEY (tstamp, sid, eventid)
);

SELECT create_hypertable('summaryprices', 'tstamp', chunk_time_interval => INTERVAL '1 month');

CREATE INDEX idx_summaryprices_sid_date ON summaryprices (sid, date DESC);
CREATE INDEX idx_summaryprices_symbol_date ON summaryprices (symbol, date DESC);
CREATE INDEX idx_summaryprices_eventid ON summaryprices (eventid);

-- Top stats
CREATE TABLE topstats (
                          date        TIMESTAMPTZ NOT NULL,
                          event_type  VARCHAR(50) NOT NULL,
                          sid         BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                          symbol      VARCHAR(20) NOT NULL,
                          price       REAL NOT NULL,
                          change_val  REAL NOT NULL,
                          change_pct  REAL NOT NULL,
                          volume      BIGINT NOT NULL,
                          PRIMARY KEY (date, event_type, sid)
);

SELECT create_hypertable('topstats', 'date', chunk_time_interval => INTERVAL '1 week');

CREATE INDEX idx_topstats_event_type ON topstats (event_type, date DESC);
CREATE INDEX idx_topstats_change_pct ON topstats (change_pct DESC, date DESC);

-- =====================================================
-- NEWS AND SENTIMENT TABLES
-- =====================================================

-- Authors table (must come before articles)
CREATE TABLE authors (
                         id          SERIAL PRIMARY KEY,
                         author_name TEXT UNIQUE NOT NULL
);

INSERT INTO authors(author_name) VALUES ('NONE');

-- Sources table
CREATE TABLE sources (
                         id          SERIAL PRIMARY KEY,
                         source_name TEXT NOT NULL,
                         domain      TEXT NOT NULL
);

-- Articles table
CREATE TABLE articles (
                          hashid   TEXT PRIMARY KEY NOT NULL,
                          sourceid INTEGER REFERENCES sources(id) NOT NULL,
                          category TEXT NOT NULL,
                          title    TEXT NOT NULL,
                          url      TEXT NOT NULL,
                          summary  TEXT NOT NULL,
                          banner   TEXT NOT NULL,
                          author   INTEGER REFERENCES authors(id) NOT NULL,
                          ct       TIMESTAMP NOT NULL
);

-- News overviews
CREATE TABLE newsoverviews (
                               id       SERIAL,
                               creation TIMESTAMPTZ NOT NULL,
                               sid      BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                               items    INTEGER NOT NULL,
                               hashid   TEXT NOT NULL,
                               PRIMARY KEY (creation, id),
                               CONSTRAINT unique_hashid_sid_creation UNIQUE (hashid, sid, creation)
);

SELECT create_hypertable('newsoverviews', 'creation', chunk_time_interval => INTERVAL '1 month');

CREATE INDEX idx_newsoverviews_sid_creation ON newsoverviews (sid, creation DESC);
CREATE INDEX idx_newsoverviews_hashid ON newsoverviews (hashid);

-- Feeds table
CREATE TABLE feeds (
                       id             SERIAL PRIMARY KEY,
                       sid            BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                       newsoverviewid INTEGER NOT NULL,
                       articleid      TEXT NOT NULL,
                       sourceid       INTEGER NOT NULL,
                       osentiment     REAL NOT NULL,
                       sentlabel      VARCHAR(20) NOT NULL,
                       created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_feeds_sid ON feeds(sid);
CREATE INDEX idx_feeds_newsoverviewid ON feeds(newsoverviewid);
CREATE INDEX idx_feeds_sentiment ON feeds(osentiment);

-- Author mappings
CREATE TABLE authormaps (
                            id       SERIAL PRIMARY KEY,
                            feedid   INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                            authorid INTEGER NOT NULL REFERENCES authors(id),
                            UNIQUE (feedid, authorid)
);

-- Topic references
CREATE TABLE topicrefs (
                           id   SERIAL PRIMARY KEY,
                           name VARCHAR(100) NOT NULL UNIQUE
);

INSERT INTO topicrefs (name) VALUES
                                 ('Blockchain'),
                                 ('Earnings'),
                                 ('Economy - Fiscal'),
                                 ('Economy - Macro'),
                                 ('Economy - Monetary'),
                                 ('Energy & Transportation'),
                                 ('Finance'),
                                 ('Financial Markets'),
                                 ('IPO'),
                                 ('Life Sciences'),
                                 ('Manufacturing'),
                                 ('Real Estate & Construction'),
                                 ('Retail & Wholesale'),
                                 ('Technology');

-- Topic mappings
CREATE TABLE topicmaps (
                           id       SERIAL PRIMARY KEY,
                           sid      BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                           feedid   INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                           topicid  INTEGER NOT NULL REFERENCES topicrefs(id),
                           relscore REAL NOT NULL
);

CREATE INDEX idx_topicmaps_sid ON topicmaps(sid);
CREATE INDEX idx_topicmaps_topicid ON topicmaps(topicid);

-- Ticker sentiments
CREATE TABLE tickersentiments (
                                  id              SERIAL PRIMARY KEY,
                                  feedid          INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                                  sid             BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                                  relevance       REAL NOT NULL,
                                  tsentiment      REAL NOT NULL,
                                  sentiment_label VARCHAR(20) NOT NULL
);

CREATE INDEX idx_tickersentiments_sid ON tickersentiments(sid);
CREATE INDEX idx_tickersentiments_feedid ON tickersentiments(feedid);

-- =====================================================
-- PROCESS TRACKING TABLES
-- =====================================================

-- Process types
CREATE TABLE proctypes (
                           id   SERIAL PRIMARY KEY,
                           name TEXT NOT NULL UNIQUE
);

INSERT INTO proctypes (name) VALUES
                                 ('load_symbols'),
                                 ('load_overviews'),
                                 ('load_intraday'),
                                 ('load_summary'),
                                 ('load_topstats'),
                                 ('load_news'),
                                 ('calculate_sentiment')
    ON CONFLICT (name) DO NOTHING;

-- States
CREATE TABLE states (
                        id   SERIAL PRIMARY KEY,
                        name TEXT NOT NULL UNIQUE
);

INSERT INTO states (name) VALUES
                              ('started'),
                              ('completed'),
                              ('failed'),
                              ('cancelled'),
                              ('retrying')
    ON CONFLICT (name) DO NOTHING;

-- Process states
CREATE TABLE procstates (
                            spid              SERIAL PRIMARY KEY,
                            proc_id           INTEGER REFERENCES proctypes(id),
                            start_time        TIMESTAMP NOT NULL DEFAULT NOW(),
                            end_state         INTEGER REFERENCES states(id),
                            end_time          TIMESTAMP,
                            error_msg         TEXT,
                            records_processed INTEGER DEFAULT 0
);

CREATE INDEX idx_procstates_proc_id ON procstates(proc_id);
CREATE INDEX idx_procstates_start_time ON procstates(start_time DESC);
CREATE INDEX idx_procstates_end_state ON procstates(end_state);

-- =====================================================
-- CRYPTO TABLES
-- =====================================================

-- Crypto API mappings
CREATE TABLE crypto_api_map (
                                sid                 BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                                api_source          VARCHAR(50) NOT NULL,
                                api_id              VARCHAR(100) NOT NULL,
                                api_slug            VARCHAR(100),
                                api_symbol          VARCHAR(20),
                                rank                INTEGER,
                                is_active           BOOLEAN DEFAULT TRUE,
                                last_verified       TIMESTAMPTZ,
                                c_time              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                m_time              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                PRIMARY KEY (sid, api_source)
);

CREATE INDEX idx_crypto_api_map_source ON crypto_api_map(api_source);
CREATE INDEX idx_crypto_api_map_api_id ON crypto_api_map(api_source, api_id);
CREATE INDEX idx_crypto_api_map_active ON crypto_api_map(api_source, is_active);

-- Crypto overview basic (20 columns)
CREATE TABLE crypto_overview_basic (
                                       sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid),
                                       symbol                  VARCHAR(20) NOT NULL,
                                       name                    TEXT NOT NULL,
                                       slug                    VARCHAR(100),
                                       description             TEXT,
                                       market_cap_rank         INTEGER,
                                       market_cap              BIGINT,
                                       fully_diluted_valuation BIGINT,
                                       volume_24h              BIGINT,
                                       volume_change_24h       NUMERIC(20,8),
                                       current_price           NUMERIC(20,8),
                                       circulating_supply      NUMERIC(30,8),
                                       total_supply            NUMERIC(30,8),
                                       max_supply              NUMERIC(30,8),
                                       last_updated            TIMESTAMPTZ,
                                       c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                       m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_crypto_overview_basic_symbol ON crypto_overview_basic(symbol);
CREATE INDEX idx_crypto_overview_basic_rank ON crypto_overview_basic(market_cap_rank);
CREATE INDEX idx_crypto_overview_basic_market_cap ON crypto_overview_basic(market_cap DESC);
CREATE INDEX idx_crypto_overview_basic_volume ON crypto_overview_basic(volume_24h DESC);

-- Crypto overview metrics (19 columns)
CREATE TABLE crypto_overview_metrics (
                                         sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid),
                                         price_change_24h        NUMERIC(20,8),
                                         price_change_pct_24h    NUMERIC(20,8),
                                         price_change_pct_7d     NUMERIC(20,8),
                                         price_change_pct_14d    NUMERIC(20,8),
                                         price_change_pct_30d    NUMERIC(20,8),
                                         price_change_pct_60d    NUMERIC(20,8),
                                         price_change_pct_200d   NUMERIC(20,8),
                                         price_change_pct_1y     NUMERIC(20,8),
                                         ath                     NUMERIC(20,8),
                                         ath_date                TIMESTAMPTZ,
                                         ath_change_percentage   NUMERIC(20,8),
                                         atl                     NUMERIC(20,8),
                                         atl_date                TIMESTAMPTZ,
                                         atl_change_percentage   NUMERIC(20,8),
                                         roi_times               NUMERIC(20,8),
                                         roi_currency            VARCHAR(10),
                                         roi_percentage          NUMERIC(20,8),
                                         c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                         m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_crypto_overview_metrics_24h ON crypto_overview_metrics(price_change_pct_24h);
CREATE INDEX idx_crypto_overview_metrics_7d ON crypto_overview_metrics(price_change_pct_7d);
CREATE INDEX idx_crypto_overview_metrics_30d ON crypto_overview_metrics(price_change_pct_30d);

-- Crypto technical
CREATE TABLE crypto_technical (
                                  sid                     BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                                  blockchain_platform     VARCHAR(100),
                                  token_standard          VARCHAR(50),
                                  consensus_mechanism     VARCHAR(100),
                                  hashing_algorithm       VARCHAR(100),
                                  block_time_minutes      NUMERIC(10,2),
                                  block_reward            NUMERIC(20,8),
                                  block_height            BIGINT,
                                  hash_rate               NUMERIC(30,2),
                                  difficulty              NUMERIC(30,2),
                                  github_forks            INTEGER,
                                  github_stars            INTEGER,
                                  github_subscribers      INTEGER,
                                  github_total_issues     INTEGER,
                                  github_closed_issues    INTEGER,
                                  github_pull_requests    INTEGER,
                                  github_contributors     INTEGER,
                                  github_commits_4_weeks  INTEGER,
                                  is_defi                 BOOLEAN DEFAULT FALSE,
                                  is_stablecoin           BOOLEAN DEFAULT FALSE,
                                  is_nft_platform         BOOLEAN DEFAULT FALSE,
                                  is_exchange_token       BOOLEAN DEFAULT FALSE,
                                  is_gaming               BOOLEAN DEFAULT FALSE,
                                  is_metaverse            BOOLEAN DEFAULT FALSE,
                                  is_privacy_coin         BOOLEAN DEFAULT FALSE,
                                  is_layer2               BOOLEAN DEFAULT FALSE,
                                  is_wrapped              BOOLEAN DEFAULT FALSE,
                                  genesis_date            DATE,
                                  ico_price               NUMERIC(20,8),
                                  ico_date                DATE,
                                  c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                  m_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Crypto social
CREATE TABLE crypto_social (
                               sid                      BIGINT PRIMARY KEY REFERENCES symbols(sid) ON DELETE CASCADE,
                               website_url              TEXT,
                               whitepaper_url           TEXT,
                               github_url               TEXT,
                               twitter_handle           VARCHAR(100),
                               twitter_followers        INTEGER,
                               telegram_url             TEXT,
                               telegram_members         INTEGER,
                               discord_url              TEXT,
                               discord_members          INTEGER,
                               reddit_url               TEXT,
                               reddit_subscribers       INTEGER,
                               facebook_url             TEXT,
                               facebook_likes           INTEGER,
                               coingecko_score          NUMERIC(5,2),
                               developer_score          NUMERIC(5,2),
                               community_score          NUMERIC(5,2),
                               liquidity_score          NUMERIC(5,2),
                               public_interest_score    NUMERIC(5,2),
                               sentiment_votes_up_pct   NUMERIC(5,2),
                               sentiment_votes_down_pct NUMERIC(5,2),
                               c_time                   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               m_time                   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Crypto markets
CREATE TABLE crypto_markets (
                                id                      SERIAL PRIMARY KEY,
                                sid                     BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
                                exchange                VARCHAR(250) NOT NULL,
                                base                    VARCHAR(120) NOT NULL,
                                target                  VARCHAR(100) NOT NULL,
                                market_type             VARCHAR(20),
                                volume_24h              NUMERIC(30,2),
                                volume_percentage       NUMERIC(11,2),
                                bid_ask_spread_pct      NUMERIC(10,4),
                                liquidity_score         VARCHAR(100),
                                is_active               BOOLEAN DEFAULT TRUE,
                                is_anomaly              BOOLEAN DEFAULT FALSE,
                                is_stale                BOOLEAN DEFAULT FALSE,
                                trust_score             VARCHAR(100),
                                last_traded_at          TIMESTAMPTZ,
                                last_fetch_at           TIMESTAMPTZ,
                                c_time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                UNIQUE(sid, exchange, base, target)
);

-- =====================================================
-- VIEWS
-- =====================================================

-- Crypto overview compatibility view
CREATE VIEW crypto_overviews AS
SELECT
    b.sid,
    b.symbol,
    b.name,
    b.slug,
    b.description,
    b.market_cap_rank,
    b.market_cap,
    b.fully_diluted_valuation,
    b.volume_24h,
    b.volume_change_24h,
    b.current_price,
    m.price_change_24h,
    m.price_change_pct_24h,
    m.price_change_pct_7d,
    m.price_change_pct_14d,
    m.price_change_pct_30d,
    m.price_change_pct_60d,
    m.price_change_pct_200d,
    m.price_change_pct_1y,
    m.ath,
    m.ath_date,
    m.ath_change_percentage,
    m.atl,
    m.atl_date,
    m.atl_change_percentage,
    m.roi_times,
    m.roi_currency,
    m.roi_percentage,
    b.circulating_supply,
    b.total_supply,
    b.max_supply,
    b.last_updated,
    b.c_time,
    b.m_time
FROM crypto_overview_basic b
         LEFT JOIN crypto_overview_metrics m ON b.sid = m.sid;

-- Crypto full view
CREATE VIEW crypto_full_view AS
SELECT
    s.sid,
    s.symbol,
    s.name,
    s.sec_type,
    cob.market_cap_rank,
    cob.market_cap,
    cob.current_price,
    cob.volume_24h,
    com.price_change_pct_24h,
    com.price_change_pct_7d,
    com.price_change_pct_30d,
    cs.coingecko_score,
    cs.developer_score,
    cs.community_score,
    ct.is_defi,
    ct.is_stablecoin,
    ct.blockchain_platform,
    cm.exchange,
    cm.base,
    cm.target
FROM symbols s
         LEFT JOIN crypto_overview_basic cob ON s.sid = cob.sid
         LEFT JOIN crypto_overview_metrics com ON s.sid = com.sid
         LEFT JOIN crypto_social cs ON s.sid = cs.sid
         LEFT JOIN crypto_technical ct ON s.sid = ct.sid
         LEFT JOIN crypto_markets cm ON s.sid = cm.sid
WHERE s.sec_type = 'Cryptocurrency';

-- =====================================================
-- FUNCTIONS AND TRIGGERS
-- =====================================================

-- Update modified time function
CREATE OR REPLACE FUNCTION update_modified_time()
RETURNS TRIGGER AS $$
BEGIN
    NEW.m_time = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Prevent completed process update
CREATE OR REPLACE FUNCTION prevent_completed_update()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.end_state = 2 AND OLD.end_state IS NOT NULL THEN
        RAISE EXCEPTION 'Cannot update completed process %', OLD.spid;
END IF;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create triggers
CREATE TRIGGER update_symbols_modtime
    BEFORE UPDATE ON symbols
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_equity_details_modtime
    BEFORE UPDATE ON equity_details
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_overviews_modtime
    BEFORE UPDATE ON overviews
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_overviewexts_modtime
    BEFORE UPDATE ON overviewexts
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_crypto_overview_basic_modtime
    BEFORE UPDATE ON crypto_overview_basic
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_crypto_overview_metrics_modtime
    BEFORE UPDATE ON crypto_overview_metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_crypto_api_map_modtime
    BEFORE UPDATE ON crypto_api_map
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER prevent_procstate_update
    BEFORE UPDATE ON procstates
    FOR EACH ROW
    EXECUTE FUNCTION prevent_completed_update();

-- =====================================================
-- COMPRESSION POLICIES
-- =====================================================

-- Add compression settings
ALTER TABLE intradayprices SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'sid',
    timescaledb.compress_orderby = 'tstamp DESC'
    );

ALTER TABLE summaryprices SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'sid',
    timescaledb.compress_orderby = 'tstamp DESC'
    );

ALTER TABLE topstats SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'event_type,sid',
    timescaledb.compress_orderby = 'date DESC'
    );

ALTER TABLE newsoverviews SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'sid',
    timescaledb.compress_orderby = 'creation DESC'
    );

-- Add compression policies
SELECT add_compression_policy('intradayprices', INTERVAL '7 days');
SELECT add_compression_policy('summaryprices', INTERVAL '30 days');
SELECT add_compression_policy('topstats', INTERVAL '30 days');
SELECT add_compression_policy('newsoverviews', INTERVAL '30 days');

-- =====================================================
-- CONTINUOUS AGGREGATES
-- =====================================================

-- Hourly OHLC from intraday
CREATE MATERIALIZED VIEW intradayprices_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', tstamp) AS hour,
    sid,
    symbol,
    first(open, tstamp) AS open,
    max(high) AS high,
    min(low) AS low,
    last(close, tstamp) AS close,
    sum(volume) AS volume,
    count(*) AS tick_count
FROM intradayprices
GROUP BY hour, sid, symbol
WITH NO DATA;

-- Add refresh policy
SELECT add_continuous_aggregate_policy('intradayprices_hourly',
                                       start_offset => INTERVAL '3 hours',
                                       end_offset => INTERVAL '10 minutes',
                                       schedule_interval => INTERVAL '30 minutes');

-- =====================================================
-- DOCUMENTATION
-- =====================================================

COMMENT ON TABLE symbols IS 'Core symbol registry for all security types';
COMMENT ON TABLE equity_details IS 'Trading hours and exchange info for equity securities';
COMMENT ON TABLE intradayprices IS 'Intraday price data - TimescaleDB hypertable with 1-day chunks';
COMMENT ON TABLE summaryprices IS 'Daily summary price data - TimescaleDB hypertable with 1-month chunks';
COMMENT ON TABLE topstats IS 'Top gainers/losers - TimescaleDB hypertable with 1-week chunks';
COMMENT ON TABLE newsoverviews IS 'News overview data - TimescaleDB hypertable with 1-month chunks';
COMMENT ON TABLE crypto_api_map IS 'Maps cryptocurrencies to various API providers';
COMMENT ON VIEW crypto_overviews IS 'Compatibility view combining crypto_overview_basic and crypto_overview_metrics';

CREATE TABLE article_translations (
                                      id SERIAL PRIMARY KEY,
                                      articleid TEXT REFERENCES articles(hashid),
                                      language VARCHAR(10) NOT NULL,
                                      title TEXT NOT NULL,
                                      content TEXT NOT NULL,
                                      UNIQUE(articleid, language)
);
-- Media attachments
CREATE TABLE article_media (
                               id SERIAL PRIMARY KEY,
                               articleid TEXT REFERENCES articles(hashid),
                               soso_url TEXT,
                               original_url TEXT,
                               short_url TEXT,
                               media_type VARCHAR(20),
                               media_order INTEGER DEFAULT 0
);
-- Tags
CREATE TABLE article_tags (
                              articleid TEXT REFERENCES articles(hashid),
                              tag VARCHAR(100),
                              PRIMARY KEY (articleid, tag)
);
-- Multiple symbol associations
CREATE TABLE article_symbols (
                                 articleid TEXT REFERENCES articles(hashid),
                                 sid BIGINT REFERENCES symbols(sid),
                                 full_name TEXT,
                                 PRIMARY KEY (articleid, sid)
);
-- Quote/Retweet information
CREATE TABLE article_quotes (
                                id SERIAL PRIMARY KEY,
                                articleid TEXT REFERENCES articles(hashid),
                                original_url TEXT,
                                author VARCHAR(255),
                                author_avatar_url TEXT,
                                nick_name VARCHAR(255),
                                impression_count BIGINT,
                                like_count INTEGER,
                                reply_count INTEGER,
                                retweet_count INTEGER,
                                twitter_created_at TIMESTAMPTZ
);
-- Extend articles table
ALTER TABLE articles
    ADD COLUMN source_link TEXT,
ADD COLUMN release_time BIGINT,
ADD COLUMN author_description TEXT,
ADD COLUMN author_avatar_url TEXT,
ADD COLUMN feature_image TEXT,
ADD COLUMN author_nick_name VARCHAR(255);

CREATE TABLE crypto_metadata (
                                 sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
                                 source VARCHAR(50) NOT NULL,
                                 source_id TEXT NOT NULL,
                                 market_cap_rank INTEGER,
                                 base_currency VARCHAR(10),
                                 quote_currency VARCHAR(10),
                                 is_active BOOLEAN NOT NULL DEFAULT true,
                                 additional_data JSONB,
                                 last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                 UNIQUE(source, source_id)
);

ALTER TABLE crypto_markets
    ADD CONSTRAINT crypto_markets_unique_market
        UNIQUE (sid, exchange, base, target);

-- Add indexes for performance
CREATE INDEX idx_crypto_markets_sid ON crypto_markets(sid);
CREATE INDEX idx_crypto_markets_exchange ON crypto_markets(exchange);
CREATE INDEX idx_crypto_markets_volume ON crypto_markets(volume_24h DESC) WHERE volume_24h IS NOT NULL;
CREATE INDEX idx_crypto_markets_active ON crypto_markets(is_active) WHERE is_active = true;
CREATE INDEX idx_crypto_markets_last_fetch ON crypto_markets(last_fetch_at DESC);

-- Add check constraints
ALTER TABLE crypto_markets
    ADD CONSTRAINT crypto_markets_volume_positive
        CHECK (volume_24h IS NULL OR volume_24h >= 0);

ALTER TABLE crypto_markets
    ADD CONSTRAINT crypto_markets_spread_valid
        CHECK (bid_ask_spread_pct IS NULL OR (bid_ask_spread_pct >= 0 AND bid_ask_spread_pct <= 100));

-- Add comment
COMMENT ON TABLE crypto_markets IS 'Cryptocurrency market data from various exchanges';
