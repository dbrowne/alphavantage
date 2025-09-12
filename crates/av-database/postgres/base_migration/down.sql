-- Drop views first (they depend on tables)
DROP VIEW IF EXISTS crypto_full_view CASCADE;
DROP VIEW IF EXISTS crypto_overviews CASCADE;

-- Drop continuous aggregate (materialized view) and its policy
SELECT remove_continuous_aggregate_policy('intradayprices_hourly', if_exists => true);
DROP MATERIALIZED VIEW IF EXISTS intradayprices_hourly CASCADE;

-- Drop compression policies before dropping hypertables
SELECT remove_compression_policy('newsoverviews', if_exists => true);
SELECT remove_compression_policy('topstats', if_exists => true);
SELECT remove_compression_policy('summaryprices', if_exists => true);
SELECT remove_compression_policy('intradayprices', if_exists => true);

-- Drop triggers
DROP TRIGGER IF EXISTS prevent_procstate_update ON procstates;
DROP TRIGGER IF EXISTS update_crypto_api_map_modtime ON crypto_api_map;
DROP TRIGGER IF EXISTS update_crypto_overview_metrics_modtime ON crypto_overview_metrics;
DROP TRIGGER IF EXISTS update_crypto_overview_basic_modtime ON crypto_overview_basic;
DROP TRIGGER IF EXISTS update_overviewexts_modtime ON overviewexts;
DROP TRIGGER IF EXISTS update_overviews_modtime ON overviews;
DROP TRIGGER IF EXISTS update_equity_details_modtime ON equity_details;
DROP TRIGGER IF EXISTS update_symbols_modtime ON symbols;

-- Drop functions
DROP FUNCTION IF EXISTS prevent_completed_update();
DROP FUNCTION IF EXISTS update_modified_time();

-- Drop indexes explicitly (before dropping tables)
DROP INDEX IF EXISTS idx_crypto_markets_last_fetch;
DROP INDEX IF EXISTS idx_crypto_markets_active;
DROP INDEX IF EXISTS idx_crypto_markets_volume;
DROP INDEX IF EXISTS idx_crypto_markets_exchange;
DROP INDEX IF EXISTS idx_crypto_markets_sid;

-- Drop constraints from crypto_markets before dropping it
ALTER TABLE IF EXISTS crypto_markets DROP CONSTRAINT IF EXISTS crypto_markets_spread_valid;
ALTER TABLE IF EXISTS crypto_markets DROP CONSTRAINT IF EXISTS crypto_markets_volume_positive;
ALTER TABLE IF EXISTS crypto_markets DROP CONSTRAINT IF EXISTS crypto_markets_unique_market;

-- Drop tables in reverse dependency order
-- Drop article extension tables first
DROP TABLE IF EXISTS article_quotes CASCADE;
DROP TABLE IF EXISTS article_symbols CASCADE;
DROP TABLE IF EXISTS article_tags CASCADE;
DROP TABLE IF EXISTS article_media CASCADE;
DROP TABLE IF EXISTS article_translations CASCADE;

-- Drop crypto extension tables
DROP TABLE IF EXISTS crypto_metadata CASCADE;
DROP TABLE IF EXISTS crypto_markets CASCADE;
DROP TABLE IF EXISTS crypto_social CASCADE;
DROP TABLE IF EXISTS crypto_technical CASCADE;
DROP TABLE IF EXISTS crypto_overview_metrics CASCADE;
DROP TABLE IF EXISTS crypto_overview_basic CASCADE;
DROP TABLE IF EXISTS crypto_api_map CASCADE;

-- Drop process tracking tables
DROP TABLE IF EXISTS procstates CASCADE;
DROP TABLE IF EXISTS states CASCADE;
DROP TABLE IF EXISTS proctypes CASCADE;

-- Drop news and sentiment tables
DROP TABLE IF EXISTS tickersentiments CASCADE;
DROP TABLE IF EXISTS topicmaps CASCADE;
DROP TABLE IF EXISTS topicrefs CASCADE;
DROP TABLE IF EXISTS authormaps CASCADE;
DROP TABLE IF EXISTS feeds CASCADE;
DROP TABLE IF EXISTS newsoverviews CASCADE;
DROP TABLE IF EXISTS articles CASCADE;
DROP TABLE IF EXISTS sources CASCADE;
DROP TABLE IF EXISTS authors CASCADE;

-- Drop price data tables (hypertables) - TimescaleDB will handle chunks automatically
DROP TABLE IF EXISTS topstats CASCADE;
DROP TABLE IF EXISTS summaryprices CASCADE;
DROP TABLE IF EXISTS intradayprices CASCADE;

-- Drop company data tables
DROP TABLE IF EXISTS overviewexts CASCADE;
DROP TABLE IF EXISTS overviews CASCADE;

-- Drop equity details
DROP TABLE IF EXISTS equity_details CASCADE;

-- Drop core symbols table (this should be last)
DROP TABLE IF EXISTS symbols CASCADE;

-- Clean up any remaining sequences (optional)
DROP SEQUENCE IF EXISTS authors_id_seq CASCADE;
DROP SEQUENCE IF EXISTS sources_id_seq CASCADE;
DROP SEQUENCE IF EXISTS feeds_id_seq CASCADE;
DROP SEQUENCE IF EXISTS authormaps_id_seq CASCADE;
DROP SEQUENCE IF EXISTS topicrefs_id_seq CASCADE;
DROP SEQUENCE IF EXISTS topicmaps_id_seq CASCADE;
DROP SEQUENCE IF EXISTS tickersentiments_id_seq CASCADE;
DROP SEQUENCE IF EXISTS proctypes_id_seq CASCADE;
DROP SEQUENCE IF EXISTS states_id_seq CASCADE;
DROP SEQUENCE IF EXISTS procstates_spid_seq CASCADE;
DROP SEQUENCE IF EXISTS crypto_markets_id_seq CASCADE;
DROP SEQUENCE IF EXISTS crypto_metadata_sid_seq CASCADE;
DROP SEQUENCE IF EXISTS article_translations_id_seq CASCADE;
DROP SEQUENCE IF EXISTS article_media_id_seq CASCADE;
DROP SEQUENCE IF EXISTS article_quotes_id_seq CASCADE;

drop index if exists idx_api_cache_source_expires;
drop index if exists idx_api_cache_expires;
drop table if exists api_response_cache CASCADE;
-- Final cleanup message
DO $$
BEGIN
    RAISE NOTICE 'Database cleanup completed successfully';
END $$;