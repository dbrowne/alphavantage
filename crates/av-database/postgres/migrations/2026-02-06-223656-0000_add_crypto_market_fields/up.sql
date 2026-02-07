-- Add missing CoinGecko market data fields

-- High priority: 24h price range
ALTER TABLE crypto_overview_metrics
ADD COLUMN high_24h NUMERIC,
ADD COLUMN low_24h NUMERIC;

-- Medium priority: Market cap changes
ALTER TABLE crypto_overview_metrics
ADD COLUMN market_cap_change_24h NUMERIC,
ADD COLUMN market_cap_change_pct_24h NUMERIC;

-- Medium priority: Coin image
ALTER TABLE crypto_overview_basic
ADD COLUMN image_url TEXT;

-- Low priority: Rehypothecated rank
ALTER TABLE crypto_overview_basic
ADD COLUMN market_cap_rank_rehyp INTEGER;