# Delegate to main TimescaleDB setup
TIMESCALE_DIR = ../../../timescale_setup

.PHONY: help db-up db-down db-shell migrate rollback test
# Database connection settings
DB_PORT = 6433
DB_USER = ts_user
DB_PASS = dev_pw
DB_NAME = sec_master
DB_HOST = localhost


help:
	@echo "Database Commands (delegates to timescale_setup):"
	@echo "  make db-up       - Start TimescaleDB"
	@echo "  make db-down     - Stop TimescaleDB"
	@echo "  make db-shell    - Connect to database"
	@echo "  make migrate     - Run Diesel migrations"
	@echo "  make rollback    - Rollback last migration"
	@echo "  make test        - Run database tests"
	@echo ""
	@echo "TimescaleDB Commands:"
	@echo "  make timescale-stats   - Show hypertable statistics"
	@echo "  make compress-old-data - Compress chunks older than 7 days"
	@echo "  make compression-stats - Compression statistics"
	@echo "  make show-continuous-aggregates "
	@echo "  make vacuum-analyze "

db-up:
	@$(MAKE) -C $(TIMESCALE_DIR) up

db-down:
	@$(MAKE) -C $(TIMESCALE_DIR) down

db-shell:
	@cd $(TIMESCALE_DIR) && PGPASSWORD=${DB_PASS} psql -U ${DB_USER} -h ${DB_HOST} -p ${DB_PORT} -d ${DB_NAME}

migrate:
	@echo "Running Diesel migrations..."
	@export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}" && \
		diesel migration run

rollback:
	@echo "Rolling back last migration..."
	@export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}" && \
		diesel migration revert

test:
	@echo "Running database tests..."
	@export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}" && \
		cargo test -- --test-threads=1


# TimescaleDB specific commands
timescale-stats:
	@echo "TimescaleDB Statistics:"
	@PGPASSWORD=$(DB_PASS) psql -h $(DB_HOST) -p $(DB_PORT) -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT hypertable_name, \
		       num_chunks, \
		       compression_enabled, \
		       pg_size_pretty(hypertable_size(hypertable_name)) as size \
		FROM timescaledb_information.hypertables \
		ORDER BY hypertable_name;"

compress-old-data:
	@echo "Compressing old data..."
	@PGPASSWORD=$(DB_PASS) psql -h $(DB_HOST) -p $(DB_PORT) -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT compress_chunk(c) \
		FROM show_chunks('intradayprices', older_than => INTERVAL '7 days') c \
		WHERE NOT is_compressed(c);"

# Additional useful TimescaleDB commands
compression-stats:
	@echo "Compression Statistics:"
	@PGPASSWORD=$(DB_PASS) psql -h $(DB_HOST) -p $(DB_PORT) -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT \
			hypertable_name, \
			chunk_name, \
			pg_size_pretty(before_compression_total_bytes) as uncompressed, \
			pg_size_pretty(after_compression_total_bytes) as compressed, \
			ROUND(compression_ratio::numeric, 2) as ratio \
		FROM chunk_compression_stats('intradayprices') \
		ORDER BY chunk_name DESC \
		LIMIT 10;"

show-continuous-aggregates:
	@echo "Continuous Aggregates:"
	@PGPASSWORD=$(DB_PASS) psql -h $(DB_HOST) -p $(DB_PORT) -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT view_name, \
		       refresh_lag, \
		       max_interval_per_job, \
		       materialization_hypertable_name \
		FROM timescaledb_information.continuous_aggregates;"

# Maintenance commands
vacuum-analyze:
	@echo "Running VACUUM ANALYZE on hypertables..."
	@PGPASSWORD=$(DB_PASS) psql -h $(DB_HOST) -p $(DB_PORT) -U $(DB_USER) -d $(DB_NAME) -c "\
		VACUUM ANALYZE intradayprices; \
		VACUUM ANALYZE summaryprices; \
		VACUUM ANALYZE topstats; \
		VACUUM ANALYZE newsoverviews;"