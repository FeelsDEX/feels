# ╔═══════════════════════════════════════════════════════════════════════════╗
# ║                      TEST EXECUTION MODULE                                ║
# ╚═══════════════════════════════════════════════════════════════════════════╝
#
# This module handles all test execution for the Feels Protocol, including:
# - Unit tests (isolated component testing)
# - Integration tests (multi-component testing)
# - Property-based tests (invariant testing)
# - End-to-end tests (full protocol flows)
# - Network tests (localnet/devnet testing)
#
# Test Categories:
#   just testing::unit          # Fast, isolated unit tests
#   just testing::integration   # Component integration tests
#   just testing::property      # Property-based invariant tests
#   just testing::e2e          # Full end-to-end protocol tests
#   just testing::localnet     # Tests against local validator
#   just testing::devnet       # Tests against Solana devnet
#
# Utilities:
#   just testing::filter <pattern>   # Run tests matching pattern
#   just testing::verbose [category] # Run with verbose output
#   just testing::parallel <n>       # Run with n threads
#
# Import dependencies
import 'common.just'

# ═══════════════════════════════════════════════════════════════════════════
# Main Test Commands
# ═══════════════════════════════════════════════════════════════════════════

# Run all in-memory tests (excludes network tests)
# This is the default test suite for development
_test:
    @just show-progress "Running all in-memory tests"
    @just _test-unit
    @just _test-integration  
    @just _test-property
    @just _test-e2e
    @just show-success "All in-memory tests passed!"

# Run ALL tests including network tests
# WARNING: This requires running validators and takes significant time
_test-all:
    @just show-progress "Running ALL tests (in-memory + network tests)"
    @just _test
    @just show-progress "Setting up and running localnet tests"
    @just _test-localnet
    @just show-progress "Running devnet tests"
    @just _test-devnet  
    @just show-success "All comprehensive tests passed!"

# Run all tests with automatic environment detection
# Tests will run in in-memory mode unless RUN_LOCALNET_TESTS or RUN_DEVNET_TESTS is set
test-auto:
    @just show-progress "Running tests with automatic environment detection"
    @if [[ -n "${RUN_DEVNET_TESTS:-}" ]]; then \
        just show-progress "Devnet mode detected"; \
        DISABLE_AIRDROP_RATE_LIMIT=1 cargo test -p feels -- --test-threads={{TEST_THREADS}}; \
    elif [[ -n "${RUN_LOCALNET_TESTS:-}" ]]; then \
        just show-progress "Localnet mode detected"; \
        just test-localnet; \
    else \
        just show-progress "Running in-memory tests (set RUN_DEVNET_TESTS=1 or RUN_LOCALNET_TESTS=1 for network tests)"; \
        cargo test -p feels -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; \
    fi

# ═══════════════════════════════════════════════════════════════════════════
# Test Categories
# ═══════════════════════════════════════════════════════════════════════════

# Run unit tests only
# These are fast, isolated tests that don't require external dependencies
_test-unit:
    @just show-progress "Running unit tests"
    @if cargo test -p feels --test mod unit:: -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; then \
        just show-success "Unit tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "Unit tests failed"; \
    fi

# Run integration tests only
# These test interactions between multiple components
_test-integration:
    @just show-progress "Running integration tests"
    @if cargo test -p feels --test mod integration:: -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; then \
        just show-success "Integration tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "Integration tests failed"; \
    fi

# Run property-based tests only
# These test protocol invariants with random inputs
_test-property:
    @just show-progress "Running property-based tests"
    @if cargo test -p feels --test mod property:: -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; then \
        just show-success "Property tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "Property tests failed"; \
    fi

# Run end-to-end tests only
# These test complete protocol flows from user perspective
_test-e2e:
    @just show-progress "Running e2e tests"
    @if cargo test -p feels --test mod e2e:: -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; then \
        just show-success "E2E tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "E2E tests failed"; \
    fi

# Run devnet tests only
# These require connection to Solana devnet
# Set RUN_DEVNET_TESTS=1 to enable
_test-devnet:
    @just show-progress "Running devnet tests"
    @if DISABLE_AIRDROP_RATE_LIMIT=1 RUN_DEVNET_TESTS=1 cargo test -p feels -- --test-threads={{TEST_THREADS}} devnet; then \
        just show-success "Devnet tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "Devnet tests failed"; \
    fi

# Run all localnet tests (with automatic setup)
# This starts a local validator, deploys programs, and runs tests
_test-localnet: setup-localnet build-localnet deploy-localnet
    @just show-progress "Running localnet tests"
    @just _test-localnet-only
    @just show-success "Localnet tests complete!"

# ═══════════════════════════════════════════════════════════════════════════
# Test Utilities
# ═══════════════════════════════════════════════════════════════════════════

# Run specific test by name or pattern
# Example: just testing::filter test_swap
test-filter pattern:
    @just show-progress "Running tests matching '{{pattern}}'"
    @cargo test -p feels {{pattern}} -- --nocapture

# Run all tests in their in-memory environment (excludes network variants)
test-in-memory:
    @just show-progress "Running all in-memory tests (excluding network variants)"
    @if cargo test -p feels -- --test-threads={{TEST_THREADS}} --skip devnet --skip localnet; then \
        just show-success "All in-memory tests passed"; \
    else \
        just exit-with-error {{EXIT_TEST_FAILURE}} "In-memory tests failed"; \
    fi

# Run tests with verbose output and debug logging
# Example: just testing::verbose unit
test-verbose category="":
    #!/usr/bin/env bash
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
    if [[ -z "{{category}}" ]]; then
        just show-progress "Running all tests with verbose output"
        cargo test -p feels -- --nocapture
    else
        just show-progress "Running {{category}} tests with verbose output"
        just test-{{category}}
    fi

# Run tests with specific thread count
# Default is 1 for deterministic test execution
# Example: just testing::parallel 4
test-parallel threads="1":
    @just show-progress "Running tests with {{threads}} threads"
    @TEST_THREADS={{threads}} just test

# ═══════════════════════════════════════════════════════════════════════════
# Localnet Testing Infrastructure
# ═══════════════════════════════════════════════════════════════════════════

# Setup localnet environment for testing
# Creates directories, starts validator, funds test accounts
[private]
setup-localnet:
    @just show-progress "Setting up localnet for tests"
    @mkdir -p {{LOGS_PATH}} {{KEYPAIRS_PATH}} test-ledger
    @just check-or-start-validator

# Check if validator is running, start if not
# Ensures we have a clean test environment
[private]
check-or-start-validator:
    #!/usr/bin/env bash
    if solana cluster-version 2>/dev/null | grep -q "Feature Set"; then
        just show-success "Localnet validator is already running"
    else
        just show-progress "Starting localnet validator"
        solana-test-validator \
            --ledger test-ledger \
            --rpc-port 8899 \
            --faucet-port 9900 \
            --quiet \
            --reset \
            > {{LOGS_PATH}}/validator.log 2>&1 &
        
        # Wait for validator
        timeout=30
        while ! solana cluster-version >/dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "Validator failed to start"
        else
            just show-success "Validator started"
        fi
    fi
    
    # Setup test authority
    just setup-test-authority

# Setup test authority keypair and fund accounts
# Creates deterministic test wallets for reproducible testing
[private]
setup-test-authority:
    #!/usr/bin/env bash
    if [[ ! -f {{KEYPAIRS_PATH}}/test-authority.json ]]; then
        solana-keygen new -o {{KEYPAIRS_PATH}}/test-authority.json --no-bip39-passphrase --force
    fi
    if [[ ! -f {{KEYPAIRS_PATH}}/payer.json ]]; then
        solana-keygen new -o {{KEYPAIRS_PATH}}/payer.json --no-bip39-passphrase --force
    fi
    
    # Fund accounts
    export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json
    authority_addr=$(solana address -k {{KEYPAIRS_PATH}}/test-authority.json)
    payer_addr=$(solana address -k {{KEYPAIRS_PATH}}/payer.json)
    
    echo "Test authority: $authority_addr"
    solana airdrop 100 -k {{KEYPAIRS_PATH}}/test-authority.json --url {{LOCALNET_RPC}} >/dev/null 2>&1 || true
    
    echo "Funding payer: $payer_addr"
    solana airdrop 10 -k {{KEYPAIRS_PATH}}/payer.json --url {{LOCALNET_RPC}} >/dev/null 2>&1 || true

# Build programs for localnet deployment
[private]
build-localnet:
    @just build

# Deploy programs to localnet validator
[private]
deploy-localnet:
    #!/usr/bin/env bash
    export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json
    export ANCHOR_PROVIDER_URL={{LOCALNET_RPC}}
    just show-progress "Deploying to localnet"
    if anchor deploy --provider.cluster localnet; then
        just show-success "Deployed to localnet"
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Localnet deployment failed"
    fi

# Run localnet tests without setup (assumes validator running)
# Use this when validator is already configured
_test-localnet-only:
    #!/usr/bin/env bash
    export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json
    export ANCHOR_PROVIDER_URL={{LOCALNET_RPC}}
    export DISABLE_AIRDROP_RATE_LIMIT=1
    export RUN_LOCALNET_TESTS=1
    
    just show-progress "Running localnet integration tests"
    cargo test -p feels -- --test-threads=1 --nocapture localnet
    
    just show-progress "Running SDK localnet tests"
    cargo test -p feels-sdk -- --test-threads=1

# Stop localnet validator and clean up processes
stop-localnet:
    @just show-progress "Stopping localnet validator"
    @pkill -f solana-test-validator || true
    @just show-success "Localnet stopped"

# View localnet validator logs (tail -f)
localnet-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/validator.log ]]; then
        tail -f {{LOGS_PATH}}/validator.log
    else
        just show-error "No validator logs found. Start localnet first with 'just test-localnet'"
    fi