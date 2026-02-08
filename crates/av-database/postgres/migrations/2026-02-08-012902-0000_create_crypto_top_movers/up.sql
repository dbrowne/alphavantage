CREATE TABLE crypto_top_movers (
    tstamp          TIMESTAMPTZ NOT NULL,
    sid             BIGINT NOT NULL REFERENCES symbols(sid),
    api_source      VARCHAR(50) NOT NULL,
    event_type      VARCHAR(20) NOT NULL,
    price_usd       NUMERIC(38,18),
    volume_24h      NUMERIC(38,8),
    change_pct_1h   DOUBLE PRECISION,
    change_pct_24h  DOUBLE PRECISION,
    change_pct_7d   DOUBLE PRECISION,
    change_pct_14d  DOUBLE PRECISION,
    change_pct_30d  DOUBLE PRECISION,
    change_pct_200d DOUBLE PRECISION,
    change_pct_1y   DOUBLE PRECISION,
    PRIMARY KEY (tstamp, sid, api_source, event_type)
);

SELECT create_hypertable('crypto_top_movers', 'tstamp', if_not_exists => TRUE);