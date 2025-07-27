
-- Drop continuous aggregates first
DROP MATERIALIZED VIEW IF EXISTS intradayprices_hourly;
-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS tickersentiments CASCADE;
DROP TABLE IF EXISTS topicmaps CASCADE;
DROP TABLE IF EXISTS topicrefs CASCADE;
DROP TABLE IF EXISTS authormaps CASCADE;
DROP TABLE IF EXISTS feeds CASCADE;
DROP TABLE IF EXISTS newsoverviews CASCADE;
DROP TABLE IF EXISTS topstats CASCADE;
DROP TABLE IF EXISTS summaryprices CASCADE;
DROP TABLE IF EXISTS intradayprices CASCADE;
DROP TABLE IF EXISTS overviewexts CASCADE;
DROP TABLE IF EXISTS overviews CASCADE;
DROP TABLE IF EXISTS symbols CASCADE;
drop table if exists authors;
drop table if exists sources;
drop table if exists articles;
DROP TRIGGER IF EXISTS prevent_procstate_update ON procstates;
DROP FUNCTION IF EXISTS prevent_completed_update();
DROP TABLE IF EXISTS procstates;
DROP TABLE IF EXISTS states;
DROP TABLE IF EXISTS proctypes;
-- Drop functions
DROP FUNCTION IF EXISTS update_modified_time() CASCADE;