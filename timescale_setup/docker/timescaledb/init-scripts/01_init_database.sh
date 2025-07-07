#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create application user
    CREATE USER ts_user WITH PASSWORD '${ts_user_PASSWORD:-av_password}';
    
    -- Create database
    CREATE DATABASE sec_master OWNER ts_user;
    
    -- Grant permissions
    GRANT ALL PRIVILEGES ON DATABASE sec_master TO ts_user;
    
    -- Connect to sec_master database
    \c sec_master
    
    -- Create extensions (no need for uuid-ossp anymore)
    CREATE EXTENSION IF NOT EXISTS "timescaledb";
    CREATE EXTENSION IF NOT EXISTS "pg_cron";
    
    -- Grant extension usage
    GRANT USAGE ON SCHEMA public TO ts_user;
    GRANT CREATE ON SCHEMA public TO ts_user;
    GRANT ALL ON ALL TABLES IN SCHEMA public TO ts_user;
    GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO ts_user;
    GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO ts_user;
    
    -- Grant sequences permissions for SERIAL columns
    GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ts_user;
    ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO ts_user;
EOSQL
