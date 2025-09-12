-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS IDX_API_CACHE_SOURCE_EXPIRES;
DROP INDEX IF EXISTS idx_api_cache_expires;
drop table if exists api_response_cache cascade;