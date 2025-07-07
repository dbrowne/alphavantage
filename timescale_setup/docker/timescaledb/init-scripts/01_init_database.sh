#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create application user
    CREATE USER ts_user WITH PASSWORD '${AV_USER_PASSWORD:-av_password}';
    
    -- Create database
    CREATE DATABASE sec_master OWNER ts_user;
    
    -- Grant permissions
    GRANT ALL PRIVILEGES ON DATABASE sec_master TO ts_user;
    
    -- Connect to sec_master database
    \c sec_master
    
    -- Create extensions
    CREATE EXTENSION IF NOT EXISTS "timescaledb";
    -- Skip pg_cron for now
    
    -- Grant extension usage
    GRANT USAGE ON SCHEMA public TO ts_user;
    GRANT CREATE ON SCHEMA public TO ts_user;
EOSQL
