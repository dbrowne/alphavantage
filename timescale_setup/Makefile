# Makefile
DOCKER_ENV = ./docker-env.sh
SCHEMASPY_JAR = ~/local/bin/schemaspy-6.2.4.jar
POSTGRES_DRIVER = ~/local/bin/postgresql-42.7.7.jar
DB_NAME = sec_master
DB_HOST = localhost
DB_USER = ts_user
DB_PORT = 6433
DB_PASS = dev_pw
OUTPUT_DIR = ../db_relations

.PHONY: check-conflicts up down logs psql clean

# Check for conflicts before starting
check-conflicts:
	@$(DOCKER_ENV) sh -c 'echo "=== TS conflict check ===" && \
		echo -n "Port $$DB_PORT: " && \
		(lsof -i :$$DB_PORT >/dev/null 2>&1 && echo "❌ IN USE") || echo "✅ Available" && \
		echo -n "Port $$PGADMIN_PORT: " && \
		(lsof -i :$$PGADMIN_PORT >/dev/null 2>&1 && echo "❌ IN USE") || echo "✅ Available"'
	@echo -n "Containers: " && \
		(docker ps -a --format "{{.Names}}" | grep -q "ts_pg" && echo "❌ Exists") || echo "✅ Clear"
	@echo -n "Networks: " && \
		(docker network ls --format "{{.Name}}" | grep -q "av" && echo "❌ Exists") || echo "✅ Clear"

# Start services
up: check-conflicts
	$(DOCKER_ENV) docker compose up -d
	@echo "=== Services started ==="
	@echo "PostgreSQL: localhost:$$($(DOCKER_ENV) sh -c 'echo $$DB_PORT')"
	@echo "PgAdmin: http://localhost:$$($(DOCKER_ENV) sh -c 'echo $$PGADMIN_PORT')"

# Stop services
down:
	$(DOCKER_ENV) docker compose down

# View logs
logs:
	$(DOCKER_ENV) docker compose logs -f av_timescaledb

# Connect to database
psql:
	docker exec -it ts_pg_av_dev psql -U ts_user -d sec_master

# Clean up everything (including volumes)
clean:
	$(DOCKER_ENV) docker compose down -v
	@echo "✅ All containers, networks, and volumes removed"

delete:
	docker volume rm av_archive_dev av_pgadmin_dev av_pg_data_dev
	docker network rm -f av_network_dev

.PHONY: generate-db-docs
generate-db-docs:
	rm -rf $(OUTPUT_DIR)/*
	java -jar $(SCHEMASPY_JAR) \
		-t pgsql11 \
		-dp $(POSTGRES_DRIVER) \
		-db $(DB_NAME) \
		-host $(DB_HOST) \
		-port $(DB_PORT) \
		-u $(DB_USER) \
		-p $(DB_PASS) \
		-o $(OUTPUT_DIR)

.PHONY: view-db-docs
view-db-docs:
	google-chrome $(OUTPUT_DIR)/index.html