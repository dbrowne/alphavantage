-- Create maintenance jobs using pg_cron
\c sec_master

-- Enable pg_cron
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Update the cron.job table to log jobs in our database
UPDATE cron.database_name SET database_name = 'sec_master';

-- Job to refresh continuous aggregates more frequently during market hours
SELECT cron.schedule(
    'refresh-intraday-aggregates',
    '*/15 9-16 * * 1-5',  -- Every 15 minutes during market hours
    $$CALL refresh_continuous_aggregate('prices_hourly', NULL, NULL);$$
);

-- Job to compress old chunks daily
SELECT cron.schedule(
    'compress-old-chunks',
    '0 2 * * *',  -- 2 AM daily
    $$SELECT compress_chunk(c.chunk_name)
      FROM timescaledb_information.chunks c
      WHERE c.hypertable_name IN ('intradayprices', 'summaryprices', 'topstats')
        AND c.range_end < NOW() - INTERVAL '7 days'
        AND NOT c.is_compressed;$$
);

-- Job to update statistics
SELECT cron.schedule(
    'update-statistics',
    '0 3 * * *',  -- 3 AM daily
    $$ANALYZE intradayprices; ANALYZE summaryprices; ANALYZE topstats;$$
);

-- Job to clean up old news data (optional)
-- SELECT cron.schedule(
--     'cleanup-old-news',
--     '0 4 * * 0',  -- 4 AM every Sunday
--     $$DELETE FROM newsoverviews WHERE creation < NOW() - INTERVAL '1 year';$$
-- );
