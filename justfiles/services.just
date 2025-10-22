# Services & Infrastructure management

# Import dependencies
import 'common.just'

# Service data directories
postgres-data := PROJECT_ROOT + "/localnet/data/postgres"
redis-data := PROJECT_ROOT + "/localnet/data/redis"
# Service log directory
logs-dir := PROJECT_ROOT + "/localnet/logs"
rocksdb-data := PROJECT_ROOT + "/localnet/indexer-storage/rocksdb"
indexer-dir := PROJECT_ROOT + "/feels-indexer"

# ===== All Services Control =====

# Show available services commands (default)
_default:
    @echo ""
    @echo "╔═══════════════════════════════════════════════════════════════╗"
    @echo "║         FEELS PROTOCOL - SERVICES & INFRASTRUCTURE            ║"
    @echo "╚═══════════════════════════════════════════════════════════════╝"
    @echo ""
    @echo "All Services:"
    @echo "  just services services-start    Start all services"
    @echo "  just services services-stop     Stop all services"
    @echo "  just services services-status   Check status of all services"
    @echo ""
    @echo "PostgreSQL:"
    @echo "  just services pg-start          Start PostgreSQL"
    @echo "  just services pg-stop           Stop PostgreSQL"
    @echo "  just services pg-status         Check PostgreSQL status"
    @echo "  just services pg-reset          Reset PostgreSQL database"
    @echo "  just services pg-shell          Open PostgreSQL shell"
    @echo ""
    @echo "Redis:"
    @echo "  just services redis-start       Start Redis"
    @echo "  just services redis-stop        Stop Redis"
    @echo "  just services redis-status      Check Redis status"
    @echo "  just services redis-reset       Reset Redis data"
    @echo "  just services redis-shell       Open Redis shell"
    @echo ""
    @echo "RocksDB:"
    @echo "  just services rocksdb-status    Check RocksDB status"
    @echo "  just services rocksdb-reset     Reset RocksDB data"
    @echo ""
    @echo "Indexer:"
    @echo "  just services indexer-start     Start indexer"
    @echo "  just services indexer-stop      Stop indexer"
    @echo "  just services indexer-logs      Show indexer logs"
    @echo ""
    @echo "For more details: just --list services"
    @echo ""

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
    mkdir -p {{logs-dir}}
    if just nix-cmd pg_isready -h localhost -p 5432 > /dev/null 2>&1; then
        just show-success "PostgreSQL already running"
    else
        just show-progress "Starting PostgreSQL"
        just nix-cmd initdb -D {{postgres-data}} > /dev/null 2>&1 || true
        just nix-cmd pg_ctl -D {{postgres-data}} -l {{logs-dir}}/postgres.log start
        
        # Wait for PostgreSQL to be ready
        timeout=10
        while ! just nix-cmd pg_isready -h localhost -p 5432 > /dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "PostgreSQL failed to start"
        else
            just show-success "PostgreSQL started"
        fi
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
    if just nix-cmd redis-cli ping > /dev/null 2>&1; then
        just show-success "Redis already running"
    else
        just show-progress "Starting Redis"
        just nix-cmd redis-server --dir {{redis-data}} --daemonize yes --logfile {{logs-dir}}/redis.log
        
        # Wait for Redis to be ready
        timeout=10
        while ! just nix-cmd redis-cli ping > /dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "Redis failed to start"
        else
            just show-success "Redis started"
        fi
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

# ===== Indexer Management =====

# Build indexer
indexer-build:
    @just show-progress "Building feels-indexer"
    @cd {{indexer-dir}} && cargo build --release
    @just show-success "Indexer built successfully"

# Run indexer
indexer-run:
    @just show-progress "Starting feels-indexer"
    @cd {{indexer-dir}} && ./target/release/feels-indexer

# Test indexer
indexer-test:
    @just show-progress "Running indexer tests"
    @cd {{indexer-dir}} && cargo test --lib --bins
    @just show-success "Indexer tests passed"

# Run database migrations
indexer-migrate:
    @just show-progress "Running database migrations"
    @cd {{indexer-dir}} && sqlx migrate run
    @just show-success "Migrations completed"

# Advanced indexer testing (specialized indexer test development)
indexer-test-advanced:
    @echo "Advanced Indexer Testing Commands"
    @echo "================================="
    @echo ""
    @echo "Location: feels-indexer/tests/"
    @echo "Usage:    cd feels-indexer/tests && just <command>"
    @echo ""
    @echo "Key Features:"
    @echo "  test-integration-geyser  - Integration tests with Geyser-enabled devnet"
    @echo "  test-e2e-flow           - Full E2E indexer flow testing"
    @echo "  test-performance        - Performance and load testing"
    @echo "  test-data-integrity     - Data consistency validation"
    @echo "  setup-test-db           - Set up isolated test database"
    @echo ""
    @echo "Example workflow:"
    @echo "  cd feels-indexer/tests"
    @echo "  just setup-test-db      # Set up test environment"
    @echo "  just test-integration-geyser  # Run integration tests"
    @echo ""
    @echo "Run 'cd feels-indexer/tests && just --list' to see all available commands."

# ===== Clean Everything =====

# Clean all service data
clean-services: pg-clean redis-clean rocksdb-clean
    @just show-success "All service data cleaned"