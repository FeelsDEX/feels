# Testing recipes

# Import dependencies
import 'common.just'

# ===== Main Test Commands =====

# Run tests with subcommands
test *args:
    #!/usr/bin/env bash
    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="all"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi
    
    case "$subcommand" in
        unit)
            just show-progress "Running unit tests"
            just nix-cmd cargo test -p feels unit:: --no-default-features
            ;;
        integration)
            just show-progress "Running integration tests"
            just nix-cmd cargo test -p feels integration:: --no-default-features
            ;;
        e2e)
            just show-progress "Running E2E tests"
            just nix-cmd cargo test -p feels e2e:: --no-default-features
            ;;
        property)
            just show-progress "Running property tests"
            just nix-cmd cargo test -p feels property:: --no-default-features
            ;;
        localnet)
            just _test-localnet
            ;;
        all)
            just show-progress "Running all tests"
            just nix-cmd cargo test -p feels --no-default-features -- --test-threads=1
            ;;
        filter)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                echo "Error: Pattern required for test filter"
                echo "Usage: just test filter <pattern>"
                exit {{EXIT_GENERAL_ERROR}}
            fi
            pattern=${remaining_args_array[0]}
            just show-progress "Running tests matching: $pattern"
            just nix-cmd cargo test -p feels $pattern --no-default-features -- --test-threads=1
            ;;
        advanced)
            just test-advanced
            ;;
        *)
            echo "Error: Unknown test subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  all         - Run all in-memory tests (default)"
            echo "  unit        - Run unit tests only"
            echo "  integration - Run integration tests only"
            echo "  e2e         - Run end-to-end tests only"
            echo "  property    - Run property-based tests only"
            echo "  localnet    - Run localnet tests (requires validator)"
            echo "  filter      - Run tests matching pattern"
            echo "  advanced    - Show advanced testing capabilities"
            echo ""
            echo "Usage:"
            echo "  just test [subcommand]"
            echo "  just test filter <pattern>"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ===== Localnet Testing =====

# Run localnet tests (complete setup + test + cleanup)
_test-localnet:
    #!/usr/bin/env bash
    just show-progress "Setting up localnet testing environment"
    
    # Start validator if needed
    just _start-validator
    
    # Deploy and setup
    just _deploy-localnet
    just _setup-test-wallets
    
    # Run localnet tests
    just show-progress "Running localnet tests"
    just nix-cmd cargo test --test "*localnet*" -- --test-threads=1 --nocapture
    
    just show-success "Localnet tests completed"

# Start validator if not running
_start-validator:
    #!/usr/bin/env bash
    if ! just check-service "{{LOCALNET_RPC}}" "validator"; then
        just show-progress "Starting test validator"
        just validator &
        sleep 3
        
        # Wait for validator to be ready
        for i in {1..30}; do
            if just check-service "{{LOCALNET_RPC}}" "validator"; then
                break
            fi
            sleep 1
        done
    fi

# Setup and fund test wallets  
_setup-test-wallets:
    @just show-progress "Setting up test wallets"
    @just airdrop 100 >/dev/null 2>&1 || true

# Deploy programs to localnet
_deploy-localnet:
    @just show-progress "Deploying to localnet"
    @just deploy localnet

# Stop localnet validator (use 'just localnet stop' instead)
[private]
stop-localnet:
    @just show-progress "Stopping localnet validator"
    @pkill -f solana-test-validator || true
    @rm -rf {{TEST_LEDGER_PATH}} || true
    @just show-success "Localnet stopped"

# Advanced testing capabilities (specialized test development) - use 'just test advanced' instead
[private]
test-advanced:
    @echo "Advanced Testing Commands"
    @echo "========================"
    @echo ""
    @echo "Location: programs/feels/tests/"
    @echo "Usage:    cd programs/feels/tests && just <command>"
    @echo ""
    @echo "Key Features:"
    @echo "  coverage          - Generate test coverage reports with cargo-tarpaulin"
    @echo "  watch             - Continuous testing with cargo-watch"
    @echo "  bench             - Run performance benchmarks"
    @echo "  localnet          - Run tests requiring local validator"
    @echo "  devnet            - Run tests against Solana devnet"
    @echo "  e2e-pipeline      - Full E2E pipeline testing (indexer + frontend)"
    @echo "  parallel <n>      - Run tests with multiple threads"
    @echo "  verbose           - Run tests with full output capture"
    @echo "  filter <pattern>  - Run tests matching specific pattern"
    @echo ""
    @echo "Example workflow:"
    @echo "  cd programs/feels/tests"
    @echo "  just coverage     # Generate coverage report"
    @echo "  just watch unit   # Watch unit tests continuously"
    @echo ""
    @echo "Run 'cd programs/feels/tests && just --list' to see all available commands."

# ===== Indexer Integration Tests =====

# Run feels-indexer integration tests with services
test-indexer-integration:
    @echo "╔═══════════════════════════════════════════════════════════════╗"
    @echo "║     FEELS INDEXER - INTEGRATION TESTS WITH SERVICES          ║"
    @echo "╚═══════════════════════════════════════════════════════════════╝"
    @echo ""
    @cd feels-indexer && nix develop ..#indexer -c tests/test_with_services.sh

# Run indexer unit tests only (no services required)
test-indexer-unit:
    @just show-progress "Running feels-indexer unit tests"
    @cd feels-indexer && cargo test --lib
    @just show-success "Indexer unit tests passed"