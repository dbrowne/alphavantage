-- This file should undo anything in `up.sql`
-- Drop check constraints
ALTER TABLE crypto_markets
DROP CONSTRAINT IF EXISTS crypto_markets_spread_valid;

ALTER TABLE crypto_markets
DROP CONSTRAINT IF EXISTS crypto_markets_volume_positive;

-- Drop indexes
DROP INDEX IF EXISTS idx_crypto_markets_last_fetch;
DROP INDEX IF EXISTS idx_crypto_markets_active;
DROP INDEX IF EXISTS idx_crypto_markets_volume;
DROP INDEX IF EXISTS idx_crypto_markets_exchange;
DROP INDEX IF EXISTS idx_crypto_markets_sid;

-- Drop unique constraint
ALTER TABLE crypto_markets
DROP CONSTRAINT IF EXISTS crypto_markets_unique_market;

-- Remove table comment
COMMENT ON TABLE crypto_markets IS NULL;