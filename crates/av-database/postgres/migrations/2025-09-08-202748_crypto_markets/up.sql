-- Your SQL goes here
-- Add unique constraint for market identification
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