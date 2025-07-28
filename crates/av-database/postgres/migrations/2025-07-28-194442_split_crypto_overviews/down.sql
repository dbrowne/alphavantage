-- This file should undo anything in `up.sql`
-- Drop the split tables and recreate the original
DROP VIEW IF EXISTS crypto_full_view;
DROP VIEW IF EXISTS crypto_overviews;
DROP TRIGGER IF EXISTS update_crypto_overview_basic_modtime ON crypto_overview_basic;
DROP TRIGGER IF EXISTS update_crypto_overview_metrics_modtime ON crypto_overview_metrics;
DROP TABLE IF EXISTS crypto_overview_metrics;
DROP TABLE IF EXISTS crypto_overview_basic;

-- Recreate the original crypto_overviews table
CREATE TABLE crypto_overviews (
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
                                  circulating_supply NUMERIC(30,8),
                                  total_supply NUMERIC(30,8),
                                  max_supply NUMERIC(30,8),
                                  last_updated TIMESTAMPTZ,
                                  c_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                                  m_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Recreate indexes
CREATE INDEX idx_crypto_overviews_symbol ON crypto_overviews(symbol);
CREATE INDEX idx_crypto_overviews_rank ON crypto_overviews(market_cap_rank);
CREATE INDEX idx_crypto_overviews_market_cap ON crypto_overviews(market_cap DESC);
CREATE INDEX idx_crypto_overviews_volume ON crypto_overviews(volume_24h DESC);