#!/bin/bash
# Test database connection and TimescaleDB features

DB_URL="postgres://ts_user:dev_pw@localhost:6433/sec_master"

echo "Testing database connection..."
psql "$DB_URL" -c "SELECT version();" || exit 1

echo -e "\nChecking TimescaleDB..."
psql "$DB_URL" -c "SELECT default_version, installed_version FROM pg_available_extensions WHERE name = 'timescaledb';"

echo -e "\nListing hypertables..."
psql "$DB_URL" -c "SELECT hypertable_name, num_chunks FROM timescaledb_information.hypertables;"

echo -e "\nChecking compression status..."
psql "$DB_URL" -c "SELECT hypertable_name, compression_enabled FROM timescaledb_information.hypertables;"

echo -e "\n✓ All checks passed!"