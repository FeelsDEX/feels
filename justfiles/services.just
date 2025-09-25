# Service management recipes

# Import dependencies
import 'common.just'

# Service data directories
postgres-data := PROJECT_ROOT + "/test-data/postgres"
redis-data := PROJECT_ROOT + "/test-data/redis"
rocksdb-data := PROJECT_ROOT + "/data/rocksdb"

# ===== All Services Control =====

# Start all services
services-start: pg-start redis-start
    @just show-success "All services started"

# Stop all services
services-stop: pg-stop redis-stop
    @just show-success "All services stopped"

# Check status of all services
services-status:
    @echo "Service Status:"
    @echo "=============="
    @just pg-status
    @just redis-status
    @just rocksdb-status

# ===== PostgreSQL Management =====

# Start PostgreSQL
pg-start:
    #!/usr/bin/env bash
    mkdir -p {{postgres-data}}
    if [[ -n "$IN_NIX_SHELL" ]]; then
        pg_isready_cmd="pg_isready"
    else
        pg_isready_cmd="nix develop --quiet -c pg_isready"
    fi
    if ! $pg_isready_cmd -h localhost -p 5432 > /dev/null 2>&1; then
        just show-progress "Starting PostgreSQL"
        if [[ -n "$IN_NIX_SHELL" ]]; then
            initdb -D {{postgres-data}} > /dev/null 2>&1 || true
        else
            nix develop --quiet -c initdb -D {{postgres-data}} > /dev/null 2>&1 || true
        fi
        if [[ -n "$IN_NIX_SHELL" ]]; then
            pg_ctl -D {{postgres-data}} -l {{postgres-data}}.log start
        else
            nix develop --quiet -c pg_ctl -D {{postgres-data}} -l {{postgres-data}}.log start
        fi
        
        # Wait for PostgreSQL to be ready
        timeout=10
        while ! $pg_isready_cmd -h localhost -p 5432 > /dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "PostgreSQL failed to start"
        else
            just show-success "PostgreSQL started"
        fi
    else
        just show-success "PostgreSQL already running"
    fi

# Stop PostgreSQL
pg-stop:
    @just show-progress "Stopping PostgreSQL"
    @pg_ctl -D {{postgres-data}} stop 2>/dev/null || true
    @just show-success "PostgreSQL stopped"

# Check PostgreSQL status
pg-status:
    @if pg_isready -h localhost -p 5432 > /dev/null 2>&1; then \
        echo "  PostgreSQL: [RUNNING]"; \
    else \
        echo "  PostgreSQL: [STOPPED]"; \
    fi

# Clean PostgreSQL data
pg-clean:
    @just show-progress "Cleaning PostgreSQL data"
    @just pg-stop
    @rm -rf {{postgres-data}}
    @just show-success "PostgreSQL data cleaned"

# ===== Redis Management =====

# Start Redis
redis-start:
    #!/usr/bin/env bash
    mkdir -p {{redis-data}}
    if [[ -n "$IN_NIX_SHELL" ]]; then
        redis_cli_cmd="redis-cli"
        redis_server_cmd="redis-server"
    else
        redis_cli_cmd="nix develop --quiet -c redis-cli"
        redis_server_cmd="nix develop --quiet -c redis-server"
    fi
    if ! $redis_cli_cmd ping > /dev/null 2>&1; then
        just show-progress "Starting Redis"
        $redis_server_cmd --dir {{redis-data}} --daemonize yes --logfile {{redis-data}}.log
        
        # Wait for Redis to be ready
        timeout=10
        while ! $redis_cli_cmd ping > /dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "Redis failed to start"
        else
            just show-success "Redis started"
        fi
    else
        just show-success "Redis already running"
    fi

# Stop Redis
redis-stop:
    @just show-progress "Stopping Redis"
    @redis-cli shutdown 2>/dev/null || true
    @just show-success "Redis stopped"

# Check Redis status
redis-status:
    @if redis-cli ping > /dev/null 2>&1; then \
        echo "  Redis: [RUNNING]"; \
    else \
        echo "  Redis: [STOPPED]"; \
    fi

# Clean Redis data
redis-clean:
    @just show-progress "Cleaning Redis data"
    @just redis-stop
    @rm -rf {{redis-data}}*
    @just show-success "Redis data cleaned"

# ===== RocksDB Management =====

# Initialize RocksDB
rocksdb-init:
    #!/usr/bin/env bash
    just show-progress "Initializing RocksDB data directory"
    
    mkdir -p "{{rocksdb-data}}"
    chmod 755 "{{rocksdb-data}}"
    
    # Create environment file
    echo "export ROCKSDB_DATA_PATH=\"{{rocksdb-data}}\"" > {{PROJECT_ROOT}}/.rocksdb_env
    
    just show-success "RocksDB initialized at {{rocksdb-data}}"
    echo "Run 'source .rocksdb_env' to load environment variables"

# Clean RocksDB data
rocksdb-clean:
    @just show-progress "Cleaning RocksDB data"
    @rm -rf {{rocksdb-data}}
    @rm -f {{PROJECT_ROOT}}/.rocksdb_env
    @just show-success "RocksDB data cleaned"

# Check RocksDB status
rocksdb-status:
    @if [[ -d "{{rocksdb-data}}" ]]; then \
        size=$(du -sh {{rocksdb-data}} 2>/dev/null | cut -f1 || echo "0"); \
        echo "  RocksDB: [DATA EXISTS] Size: $size"; \
    else \
        echo "  RocksDB: [NOT INITIALIZED]"; \
    fi

# ===== Database Operations =====

# Create database
db-create name="feels_indexer_test":
    @just show-progress "Creating database {{name}}"
    @createdb {{name}} || true
    @psql {{name}} -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";" || true
    @just show-success "Database {{name}} created"

# Drop database
db-drop name="feels_indexer_test":
    @just show-progress "Dropping database {{name}}"
    @dropdb {{name}} || true
    @just show-success "Database {{name}} dropped"

# ===== Clean Everything =====

# Clean all service data
clean-services: pg-clean redis-clean rocksdb-clean
    @just show-success "All service data cleaned"