
-- Drop continuous aggregates first
-- Drop views first
DROP VIEW IF EXISTS crypto_full_view CASCADE;
DROP VIEW IF EXISTS crypto_overviews CASCADE;

-- Drop continuous aggregate (materialized view) and its policy
SELECT remove_continuous_aggregate_policy('intradayprices_hourly', if_exists => true);
DROP MATERIALIZED VIEW IF EXISTS intradayprices_hourly CASCADE;

-- Drop compression policies
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

-- Drop tables in reverse dependency order

-- Drop crypto tables
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

-- Drop price data tables (hypertables)
DROP TABLE IF EXISTS topstats CASCADE;
DROP TABLE IF EXISTS summaryprices CASCADE;
DROP TABLE IF EXISTS intradayprices CASCADE;

-- Drop company data tables
DROP TABLE IF EXISTS overviewexts CASCADE;
DROP TABLE IF EXISTS overviews CASCADE;

-- Drop equity details
DROP TABLE IF EXISTS equity_details CASCADE;

-- Drop core symbols table
DROP TABLE IF EXISTS symbols CASCADE;
