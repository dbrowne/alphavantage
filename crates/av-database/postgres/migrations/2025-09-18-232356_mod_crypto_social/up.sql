-- Your SQL goes here
-- Add blockchain_sites column to crypto_social table as JSONB
ALTER TABLE crypto_social
    ADD COLUMN if not exists blockchain_sites JSONB;

-- Add GIN index for efficient JSONB queries
CREATE INDEX idx_crypto_social_blockchain_sites ON crypto_social USING GIN(blockchain_sites);