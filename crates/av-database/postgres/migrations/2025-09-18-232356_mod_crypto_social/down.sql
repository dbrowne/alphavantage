-- This file should undo anything in `up.sql`
-- Remove the index
DROP INDEX IF EXISTS idx_crypto_social_blockchain_sites;

-- Remove the column
ALTER TABLE crypto_social
    DROP COLUMN blockchain_sites;