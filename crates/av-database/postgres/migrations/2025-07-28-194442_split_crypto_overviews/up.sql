-- Your SQL goes here
DROP VIEW IF EXISTS crypto_full_view;
DROP TABLE IF EXISTS crypto_overviews CASCADE;

-- Create crypto_overview_basic with essential columns (20 columns)
CREATE TABLE crypto_overview_basic (
                                       sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
                                       symbol VARCHAR(20) NOT NULL,
                                       name TEXT NOT NULL,
                                       slug VARCHAR(100),
                                       description TEXT,
                                       market_cap_rank INT,
                                       market_cap BIGINT,
                                       fully_diluted_valuation BIGINT,
                                       volume_24h BIGINT,
                                       volume_change_24h NUMERIC(20,8),
                                       current_price NUMERIC(20,8),
                                       circulating_supply NUMERIC(30,8),
                                       total_supply NUMERIC(30,8),
                                       max_supply NUMERIC(30,8),
                                       last_updated TIMESTAMPTZ,
                                       c_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                                       m_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create crypto_overview_metrics for price change and ROI data (19 columns)
CREATE TABLE crypto_overview_metrics (
                                         sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
                                         price_change_24h NUMERIC(20,8),
                                         price_change_pct_24h NUMERIC(20,8),
                                         price_change_pct_7d NUMERIC(20,8),
                                         price_change_pct_14d NUMERIC(20,8),
                                         price_change_pct_30d NUMERIC(20,8),
                                         price_change_pct_60d NUMERIC(20,8),
                                         price_change_pct_200d NUMERIC(20,8),
                                         price_change_pct_1y NUMERIC(20,8),
                                         ath NUMERIC(20,8),
                                         ath_date TIMESTAMPTZ,
                                         ath_change_percentage NUMERIC(20,8),
                                         atl NUMERIC(20,8),
                                         atl_date TIMESTAMPTZ,
                                         atl_change_percentage NUMERIC(20,8),
                                         roi_times NUMERIC(20,8),
                                         roi_currency VARCHAR(10),
                                         roi_percentage NUMERIC(20,8),
                                         c_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                                         m_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for crypto_overview_basic
CREATE INDEX idx_crypto_overview_basic_symbol ON crypto_overview_basic(symbol);
CREATE INDEX idx_crypto_overview_basic_rank ON crypto_overview_basic(market_cap_rank);
CREATE INDEX idx_crypto_overview_basic_market_cap ON crypto_overview_basic(market_cap DESC);
CREATE INDEX idx_crypto_overview_basic_volume ON crypto_overview_basic(volume_24h DESC);

-- Create indexes for crypto_overview_metrics
CREATE INDEX idx_crypto_overview_metrics_24h ON crypto_overview_metrics(price_change_pct_24h);
CREATE INDEX idx_crypto_overview_metrics_7d ON crypto_overview_metrics(price_change_pct_7d);
CREATE INDEX idx_crypto_overview_metrics_30d ON crypto_overview_metrics(price_change_pct_30d);

-- Create a view to maintain compatibility with the original table structure
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

-- Recreate the comprehensive view with all crypto data
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
WHERE s.sec_type = 'CRYPTO';

-- Add triggers to update m_time
CREATE OR REPLACE FUNCTION update_modified_time()
RETURNS TRIGGER AS $$
BEGIN
    NEW.m_time = CURRENT_TIMESTAMP;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_crypto_overview_basic_modtime
    BEFORE UPDATE ON crypto_overview_basic
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();

CREATE TRIGGER update_crypto_overview_metrics_modtime
    BEFORE UPDATE ON crypto_overview_metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_time();