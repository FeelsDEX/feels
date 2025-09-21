# Indexer development and testing recipes

# Import dependencies
import 'common.just'

# Indexer paths
indexer-dir := PROJECT_ROOT + "/feels-indexer"
indexer-test-dir := indexer-dir + "/tests"

# ===== Main Indexer Commands =====

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
indexer-test: indexer-test-unit indexer-test-integration
    @just show-success "All indexer tests passed"

# ===== Indexer Testing =====

# Run unit tests
indexer-test-unit:
    @just show-progress "Running indexer unit tests"
    @cd {{indexer-dir}} && cargo test --lib --bins

# Run integration tests
indexer-test-integration:
    #!/usr/bin/env bash
    cd {{indexer-dir}}/tests
    just show-progress "Starting indexer integration tests"
    
    # Start services using services module
    just services-start
    
    # Setup database
    just db-create feels_indexer_test
    cd {{indexer-dir}}
    DATABASE_URL=postgresql://localhost/feels_indexer_test sqlx migrate run
    
    # Clean test data
    cd {{indexer-dir}}/tests
    rm -rf ./test-data
    
    # Run tests
    RUST_LOG=info,feels_indexer=debug cargo test integration_devnet -- --ignored --nocapture
    TEST_RESULT=$?
    
    # Cleanup
    just show-progress "Cleaning up test services"
    just services-stop
    
    if [[ $TEST_RESULT -eq 0 ]]; then
        just show-success "Integration tests passed"
    else
        just exit-with-error {{EXIT_TEST_FAILURE}} "Integration tests failed"
    fi

# Test with Geyser
indexer-test-geyser:
    #!/usr/bin/env bash
    just show-progress "Testing Geyser integration"
    
    # Check services
    echo "Checking services..."
    just check-port {{VALIDATOR_RPC_PORT}} "Solana RPC" || just show-warning "Solana RPC not running"
    just check-port 10000 "Geyser gRPC" || just show-warning "Geyser gRPC not running"
    just check-port 8999 "Geyser Prometheus" || just show-warning "Geyser metrics not running"
    
    # Test gRPC connection if grpcurl available
    if command -v grpcurl >/dev/null 2>&1; then
        echo "Testing Geyser gRPC..."
        grpcurl -plaintext localhost:10000 list || echo "Failed to list services"
        grpcurl -plaintext localhost:10000 geyser.Geyser/GetVersion || echo "Failed to get version"
    else
        just show-warning "grpcurl not found, skipping gRPC tests"
    fi
    
    # Test Solana RPC
    echo "Testing Solana RPC..."
    curl -s -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}' \
        http://localhost:{{VALIDATOR_RPC_PORT}} | jq . || echo "Failed to get Solana version"
    
    # Check Prometheus metrics
    echo "Checking Prometheus metrics..."
    curl -s http://localhost:8999/metrics | head -10 || echo "Failed to get metrics"
    
    just show-success "Geyser test complete"

# ===== Indexer Development =====

# Setup test environment
indexer-setup-env:
    @just show-progress "Setting up indexer test environment"
    @just services-start
    @just db-create feels_indexer_test
    @mkdir -p {{indexer-test-dir}}/test-data/{rocksdb,tantivy}
    @just show-success "Test environment ready"

# Clean test environment
indexer-clean-env:
    @just show-progress "Cleaning indexer test environment"
    @just db-drop feels_indexer_test
    @rm -rf {{indexer-test-dir}}/test-data
    @rm -rf {{indexer-test-dir}}/test-ledger
    @just services-stop
    @just show-success "Test environment cleaned"

# Run specific test with logging
indexer-test-specific name:
    @cd {{indexer-dir}} && RUST_LOG=debug cargo test {{name}} -- --nocapture

# Run tests with coverage
indexer-coverage:
    @just show-progress "Generating test coverage"
    @cd {{indexer-dir}} && cargo tarpaulin --out Html --output-dir ./coverage
    @just show-success "Coverage report generated"

# Run benchmarks
indexer-bench:
    @just show-progress "Running indexer benchmarks"
    @cd {{indexer-dir}} && cargo bench

# Watch for changes
indexer-watch:
    @cd {{indexer-dir}} && cargo watch -x "test --lib" -i "test-data/*"

# Generate test data
indexer-generate-data:
    @just show-progress "Generating test data"
    @cd {{indexer-dir}} && cargo run --bin generate-test-data --release

# Profile memory usage
indexer-profile-memory:
    @cd {{indexer-dir}} && RUST_LOG=info valgrind --leak-check=full --show-leak-kinds=all \
        cargo test integration_devnet -- --ignored

# Run stress test
indexer-stress-test:
    @just show-progress "Running indexer stress test"
    @cd {{indexer-dir}} && cargo test stress_test_indexer -- --ignored --release

# Lint indexer code
indexer-lint:
    @just show-progress "Linting indexer code"
    @cd {{indexer-dir}} && cargo clippy --all-targets -- -D warnings

# Format indexer code
indexer-format:
    @cd {{indexer-dir}} && cargo fmt --all

# Run all checks
indexer-check: indexer-format indexer-lint indexer-test-unit
    @just show-success "All indexer checks passed"

# ===== Database Management =====

# Run migrations
indexer-migrate:
    @just show-progress "Running database migrations"
    @cd {{indexer-dir}} && sqlx migrate run
    @just show-success "Migrations completed"

# Create migration
indexer-migration name:
    @cd {{indexer-dir}} && sqlx migrate add {{name}}

# Revert last migration
indexer-migrate-revert:
    @cd {{indexer-dir}} && sqlx migrate revert

# ===== Docker Support =====

# Build indexer Docker image
indexer-docker-build:
    @just show-progress "Building indexer Docker image"
    @cd {{indexer-dir}} && docker build -t feels-indexer:latest .
    @just show-success "Docker image built"

# Run indexer in Docker
indexer-docker-run:
    @cd {{indexer-dir}} && docker-compose up -d

# Stop Docker services
indexer-docker-stop:
    @cd {{indexer-dir}} && docker-compose down