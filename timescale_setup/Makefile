# Makefile
DOCKER_ENV = ./docker-env.sh

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
	$(DOCKER_ENV) docker-compose up -d
	@echo "=== Services started ==="
	@echo "PostgreSQL: localhost:$$($(DOCKER_ENV) sh -c 'echo $$DB_PORT')"
	@echo "PgAdmin: http://localhost:$$($(DOCKER_ENV) sh -c 'echo $$PGADMIN_PORT')"

# Stop services
down:
	$(DOCKER_ENV) docker-compose down

# View logs
logs:
	$(DOCKER_ENV) docker-compose logs -f av_timescaledb

# Connect to database
psql:
	docker exec -it ts_pg psql -U av_user -d ts_pg

# Clean up everything (including volumes)
clean:
	$(DOCKER_ENV) docker-compose down -v
	@echo "✅ All containers, networks, and volumes removed"

delete:
	docker volume rm av_archive_dev av_pgadmin_dev av_pg_data_dev
	docker network rm -f av_network_dev


