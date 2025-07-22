-- This runs after 01_init_database.sh
\c sec_master

-- Ensure TimescaleDB extension is created (if not already done in 01_init_database.sh)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Grant TimescaleDB permissions to application user
GRANT USAGE ON SCHEMA timescaledb_information TO ts_user;
GRANT USAGE ON SCHEMA timescaledb_experimental TO ts_user;

-- Create a dedicated schema for continuous aggregates
CREATE SCHEMA IF NOT EXISTS aggregates AUTHORIZATION ts_user;
GRANT ALL ON SCHEMA aggregates TO ts_user;

-- Set default timescaledb settings
ALTER DATABASE sec_master SET timescaledb.max_background_workers = 8;
ALTER DATABASE sec_master SET timescaledb.enable_optimizations = 'on';
ALTER DATABASE sec_master SET timescaledb.enable_constraint_aware_append = 'on';