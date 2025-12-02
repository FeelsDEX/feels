#!/usr/bin/env bash
# Test runner that ensures all required services are running
# This script is designed to work within the Nix development environment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get project root (parent of feels-indexer)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     FEELS INDEXER - INTEGRATION TEST WITH SERVICES           ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to check if a service is running
check_service() {
    local service=$1
    local check_cmd=$2
    
    if eval "$check_cmd" > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} $service is running"
        return 0
    else
        echo -e "  ${RED}✗${NC} $service is not running"
        return 1
    fi
}

# Function to start services
start_services() {
    echo -e "${YELLOW}Starting required services...${NC}"
    echo ""
    
    # Start PostgreSQL
    if ! check_service "PostgreSQL" "pg_isready -h localhost -p 5432"; then
        echo "  Starting PostgreSQL..."
        just services pg-start
    fi
    
    # Start Redis
    if ! check_service "Redis" "redis-cli ping"; then
        echo "  Starting Redis..."
        just services redis-start
    fi
    
    # Initialize RocksDB directory
    if [ ! -d "$PROJECT_ROOT/localnet/indexer-storage/rocksdb" ]; then
        echo "  Initializing RocksDB..."
        just services rocksdb-init
    fi
    check_service "RocksDB" "test -d '$PROJECT_ROOT/localnet/indexer-storage/rocksdb'"
    
    echo ""
    echo -e "${GREEN}✓ All services are ready${NC}"
    echo ""
}

# Function to setup test database
setup_test_db() {
    echo -e "${YELLOW}Setting up test database...${NC}"
    
    # Drop existing test database
    dropdb feels_indexer_test 2>/dev/null || true
    
    # Create test database
    createdb feels_indexer_test
    psql feels_indexer_test -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";" > /dev/null
    
    # Run migrations
    cd "$PROJECT_ROOT/feels-indexer"
    DATABASE_URL="postgresql://localhost/feels_indexer_test" sqlx migrate run
    cd "$PROJECT_ROOT"
    
    echo -e "${GREEN}✓ Test database ready${NC}"
    echo ""
}

# Function to run tests
run_tests() {
    echo -e "${YELLOW}Running integration tests...${NC}"
    echo ""
    
    cd "$PROJECT_ROOT/feels-indexer"
    
    # Set environment variables for tests
    export DATABASE_URL="postgresql://localhost/feels_indexer_test"
    export REDIS_URL="redis://localhost:6379/1"
    export ROCKSDB_DATA_PATH="$PROJECT_ROOT/localnet/indexer-storage/rocksdb"
    export RUST_LOG="${RUST_LOG:-info,feels_indexer=debug}"
    
    # Run tests
    cargo test --test basic_tests "$@" -- --test-threads=1
    
    TEST_RESULT=$?
    
    cd "$PROJECT_ROOT"
    return $TEST_RESULT
}

# Function to cleanup
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up test data...${NC}"
    dropdb feels_indexer_test 2>/dev/null || true
    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Main execution
main() {
    # Start services
    start_services
    
    # Setup test database
    setup_test_db
    
    # Run tests
    if run_tests "$@"; then
        echo ""
        echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║                   ALL TESTS PASSED ✓                          ║${NC}"
        echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
        EXIT_CODE=0
    else
        echo ""
        echo -e "${RED}╔═══════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║                   TESTS FAILED ✗                              ║${NC}"
        echo -e "${RED}╚═══════════════════════════════════════════════════════════════╝${NC}"
        EXIT_CODE=1
    fi
    
    # Cleanup
    cleanup
    
    exit $EXIT_CODE
}

# Handle Ctrl+C gracefully
trap cleanup EXIT INT TERM

# Run main function with all arguments
main "$@"


