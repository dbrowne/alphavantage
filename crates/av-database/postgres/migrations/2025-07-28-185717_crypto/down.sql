-- This file should undo anything in `up.sql`

-- Drop view first
DROP VIEW IF EXISTS crypto_full_view;

-- Drop indexes
DROP INDEX IF EXISTS idx_crypto_markets_volume;
DROP INDEX IF EXISTS idx_crypto_markets_exchange;
DROP INDEX IF EXISTS idx_crypto_markets_sid;

DROP INDEX IF EXISTS idx_crypto_social_scores;

DROP INDEX IF EXISTS idx_crypto_technical_categories;
DROP INDEX IF EXISTS idx_crypto_technical_blockchain;

DROP INDEX IF EXISTS idx_crypto_overviews_volume;
DROP INDEX IF EXISTS idx_crypto_overviews_market_cap;
DROP INDEX IF EXISTS idx_crypto_overviews_rank;
DROP INDEX IF EXISTS idx_crypto_overviews_symbol;

-- Drop tables in reverse order of dependencies
DROP TABLE IF EXISTS crypto_markets;
DROP TABLE IF EXISTS crypto_social;
DROP TABLE IF EXISTS crypto_technical;
DROP TABLE IF EXISTS crypto_overviews;