-- Optimized for TimescaleDB
-- Drop existing tables

-- Core symbols table (not time-series)
CREATE TABLE symbols (
    sid         BIGINT PRIMARY KEY NOT NULL,
    symbol      VARCHAR(20) NOT NULL,
    name        TEXT NOT NULL,
    sec_type    VARCHAR(50) NOT NULL,
    region      VARCHAR(10) NOT NULL,
    market_open TIME NOT NULL,
    market_close TIME NOT NULL,
    timezone    VARCHAR(50) NOT NULL,
    currency    VARCHAR(10) NOT NULL,
    overview    BOOLEAN NOT NULL DEFAULT FALSE,
    intraday    BOOLEAN NOT NULL DEFAULT FALSE,
    summary     BOOLEAN NOT NULL DEFAULT FALSE,
    c_time      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    m_time      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for symbols
CREATE INDEX idx_symbols_symbol ON symbols(symbol);
CREATE INDEX idx_symbols_active ON symbols(symbol) WHERE overview = TRUE OR intraday = TRUE OR summary = TRUE;

-- Company overview data (not time-series)
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
    m_time             TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
    m_time                    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- Intraday prices - HYPERTABLE with sequence number
CREATE TABLE intradayprices (
    eventid BIGSERIAL NOT NULL,  -- Sequence number for unique identification
    tstamp  TIMESTAMPTZ NOT NULL,
    sid     BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
    symbol  VARCHAR(20) NOT NULL,
    open    REAL NOT NULL,
    high    REAL NOT NULL,
    low     REAL NOT NULL,
    close   REAL NOT NULL,
    volume  BIGINT NOT NULL,
    PRIMARY KEY (tstamp, sid, eventid)  -- Include eventid in PK for uniqueness
);

-- Convert to hypertable with 1 day chunks
SELECT create_hypertable('intradayprices', 'tstamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes for intraday prices
CREATE INDEX idx_intradayprices_sid_time ON intradayprices (sid, tstamp DESC);
CREATE INDEX idx_intradayprices_symbol_time ON intradayprices (symbol, tstamp DESC);
CREATE INDEX idx_intradayprices_eventid ON intradayprices (eventid);  -- Index on sequence


-- Summary prices - HYPERTABLE with sequence number
CREATE TABLE summaryprices (
    eventid BIGSERIAL NOT NULL,  -- Sequence number
    tstamp  TIMESTAMPTZ NOT NULL,  -- For hypertable
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

-- Convert to hypertable with 1 month chunks
SELECT create_hypertable('summaryprices', 'tstamp', chunk_time_interval => INTERVAL '1 month');

-- Create indexes
CREATE INDEX idx_summaryprices_sid_date ON summaryprices (sid, date DESC);
CREATE INDEX idx_summaryprices_symbol_date ON summaryprices (symbol, date DESC);
CREATE INDEX idx_summaryprices_eventid ON summaryprices (eventid);
-- Top stats (gainers/losers) - HYPERTABLE
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

-- Convert to hypertable with 1 week chunks
SELECT create_hypertable('topstats', 'date', chunk_time_interval => INTERVAL '1 week');

-- Create indexes
CREATE INDEX idx_topstats_event_type ON topstats (event_type, date DESC);
CREATE INDEX idx_topstats_change_pct ON topstats (change_pct DESC, date DESC);

-- News overviews - HYPERTABLE
CREATE TABLE newsoverviews (
    id       SERIAL,
    creation TIMESTAMPTZ NOT NULL,
    sid      BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
    items    INTEGER NOT NULL,
    hashid   TEXT NOT NULL,
    PRIMARY KEY (creation, id),
    -- Include creation in the unique constraint for TimescaleDB
    CONSTRAINT unique_hashid_sid_creation UNIQUE (hashid, sid, creation)
);

-- Convert to hypertable with 1 month chunks
SELECT create_hypertable('newsoverviews', 'creation', chunk_time_interval => INTERVAL '1 month');

-- Create indexes
CREATE INDEX idx_newsoverviews_sid_creation ON newsoverviews (sid, creation DESC);
CREATE INDEX idx_newsoverviews_hashid ON newsoverviews (hashid);

-- Feeds table (references external articles and sources)
CREATE TABLE feeds (
    id             SERIAL PRIMARY KEY,
    sid            BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
    newsoverviewid INTEGER NOT NULL, -- Will add FK after hypertable creation
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
    authorid INTEGER NOT NULL, -- Assuming authors table exists
    UNIQUE (feedid, authorid)
);

-- Topic references
CREATE TABLE topicrefs (
    id   SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE
);

-- Populate predefined topics
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

-- Add compression policies for hypertables
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

-- Add compression policies (compress data older than specified interval)
SELECT add_compression_policy('intradayprices', INTERVAL '7 days');
SELECT add_compression_policy('summaryprices', INTERVAL '30 days');
SELECT add_compression_policy('topstats', INTERVAL '30 days');
SELECT add_compression_policy('newsoverviews', INTERVAL '30 days');

-- Create continuous aggregate for hourly OHLC from intraday
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

-- Add refresh policy for continuous aggregate
SELECT add_continuous_aggregate_policy('intradayprices_hourly',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '10 minutes',
    schedule_interval => INTERVAL '30 minutes');

-- Add trigger to update m_time on symbols
CREATE OR REPLACE FUNCTION update_modified_time()
RETURNS TRIGGER AS $$
BEGIN
    NEW.m_time = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_symbols_modtime
    BEFORE UPDATE ON symbols
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

-- Add comment for documentation
COMMENT ON TABLE intradayprices IS 'Intraday price data - TimescaleDB hypertable with 1-day chunks';
COMMENT ON TABLE summaryprices IS 'Daily summary price data - TimescaleDB hypertable with 1-month chunks';
COMMENT ON TABLE topstats IS 'Top gainers/losers - TimescaleDB hypertable with 1-week chunks';
COMMENT ON TABLE newsoverviews IS 'News overview data - TimescaleDB hypertable with 1-month chunks';-- Your SQL goes here
CREATE TABLE authors
(
    id          SERIAL PRIMARY KEY,
    author_name text unique not NULL
);

insert into authors(author_name)
VALUES ('NONE');


create table sources
(
    id          serial primary key,
    source_name text not null,
    domain      text not null
);

create table articles
(
    hashid   Text primary key not null,
    sourceid int references sources (id) not null,
    category text      not null,
    title    text      not null,
    url      text      not null,
    summary  text      not null,
    banner   text      not null,
    author   int references authors (id) not null,
    ct       timestamp not null
);

-- Process types table
CREATE TABLE IF NOT EXISTS proctypes (
                                         id   SERIAL PRIMARY KEY,
                                         name TEXT NOT NULL UNIQUE
);

-- Insert process types, ignoring conflicts
INSERT INTO proctypes (name) VALUES
                                 ('load_symbols'),
                                 ('load_overviews'),
                                 ('load_intraday'),
                                 ('load_summary'),
                                 ('load_topstats'),
                                 ('load_news'),
                                 ('calculate_sentiment')
    ON CONFLICT (name) DO NOTHING;

-- States table
CREATE TABLE IF NOT EXISTS states (
                                      id   SERIAL PRIMARY KEY,
                                      name TEXT NOT NULL UNIQUE
);

-- Insert states, ignoring conflicts
INSERT INTO states (name) VALUES
                              ('started'),
                              ('completed'),
                              ('failed'),
                              ('cancelled'),
                              ('retrying')
    ON CONFLICT (name) DO NOTHING;

-- Process states table
CREATE TABLE IF NOT EXISTS procstates (
                                          spid       SERIAL PRIMARY KEY,
                                          proc_id    INTEGER REFERENCES proctypes(id),
    start_time TIMESTAMP NOT NULL DEFAULT NOW(),
    end_state  INTEGER REFERENCES states(id),
    end_time   TIMESTAMP,
    error_msg  TEXT,
    records_processed INTEGER DEFAULT 0
    );

-- Create indexes if they don't exist
CREATE INDEX IF NOT EXISTS idx_procstates_proc_id ON procstates(proc_id);
CREATE INDEX IF NOT EXISTS idx_procstates_start_time ON procstates(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_procstates_end_state ON procstates(end_state);

-- Drop trigger if exists before recreating
DROP TRIGGER IF EXISTS prevent_procstate_update ON procstates;

-- Function with CREATE OR REPLACE handles existing function
CREATE OR REPLACE FUNCTION prevent_completed_update()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.end_state = 2 AND OLD.end_state IS NOT NULL THEN
        RAISE EXCEPTION 'Cannot update completed process %', OLD.spid;
END IF;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Recreate trigger
CREATE TRIGGER prevent_procstate_update
    BEFORE UPDATE ON procstates
    FOR EACH ROW
    EXECUTE FUNCTION prevent_completed_update();
