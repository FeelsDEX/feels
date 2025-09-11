# Feels Protocol Task Runner
# Run `just` to see all available tasks

# Default task - show help
default:
    @echo "Feels Protocol Development Tasks"
    @echo "================================"
    @echo ""
    @echo "Build & Test:"
    @echo "  just build         - Build the protocol"
    @echo "  just test          - Run all tests"
    @echo "  just test-unit     - Run unit tests only"
    @echo "  just test-integration - Run integration tests only"
    @echo ""
    @echo "Code Quality:"
    @echo "  just format        - Format all code"
    @echo "  just lint          - Run clippy lints"
    @echo "  just check         - Run all quality checks"
    @echo ""
    @echo "Development:"
    @echo "  just clean         - Clean build artifacts"
    @echo "  just local-devnet  - Start local development network"
    @echo "  just deploy        - Deploy to local devnet"
    @echo "  just deploy-devnet - Deploy to Solana devnet"
    @echo ""
    @echo "IDL & Types:"
    @echo "  just idl-build     - Generate IDL files"
    @echo "  just idl-validate  - Validate IDL matches SDK"
    @echo "  just types         - Generate TypeScript types"
    @echo ""
    @echo "Nix:"
    @echo "  just nix-build     - Build with Nix"
    @echo "  just nix-shell     - Enter Nix shell"
    @echo "  just cargo-nix     - Generate Cargo.nix for fast builds"

# Build the protocol using Nix BPF builder (default)
build:
    @echo "Building Feels Protocol with Nix BPF builder..."
    nix build .#feels-protocol --out-link target/nix-feels
    @mkdir -p target/deploy
    @cp target/nix-feels/deploy/*.so target/deploy/ 2>/dev/null || true
    @echo "Program built with Nix and copied to target/deploy/"

# Build with cargo (fallback)
build-cargo:
    @echo "Building with cargo build-sbf..."
    @mkdir -p target/deploy
    cd programs/feels && cargo build-sbf
    @echo "Program built with cargo"

# Run all tests
test:
    @echo "Running all tests..."
    cargo test

# Run unit tests only
test-unit:
    @echo "Running unit tests..."
    cargo test --lib

# Run integration tests only
test-integration:
    @echo "Running integration tests..."
    @echo "No integration tests configured yet"

# Format all code
format:
    @echo "Formatting code..."
    cargo fmt --all
    @if command -v nixpkgs-fmt >/dev/null 2>&1; then \
        find . -name "*.nix" -exec nixpkgs-fmt {} \; ; \
    else \
        echo "nixpkgs-fmt not available, skipping nix file formatting" ; \
    fi

# Run clippy lints
lint:
    @echo "Running clippy lints..."
    cargo clippy --all-targets --all-features -- -D warnings -A unexpected_cfgs

# Run all quality checks
check: format lint test
    @echo "All quality checks passed!"

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    rm -rf target/
    rm -rf .anchor/

# Start local development network
local-devnet:
    @echo "Starting local development network..."
    nix run .#local-devnet

# Deploy to local devnet
deploy:
    @echo "Deploying to local devnet..."
    anchor deploy --provider.cluster localnet

# Deploy to Solana devnet
deploy-devnet:
    @echo "Deploying to Solana devnet..."
    anchor deploy --provider.cluster devnet

# Generate IDL files
idl-build:
    @echo "Generating IDL files..."
    nix run .#idl-build

# Validate IDL against SDK
idl-validate:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "Validating IDL consistency..."
    
    if [ ! -f "target/idl/feels.json" ]; then
        echo "IDL not found at target/idl/feels.json"
        echo "   Run 'just idl-build' to generate the IDL first"
        exit 1
    fi
    
    echo "Found in IDL:"
    echo "   - $(jq '.instructions | length' target/idl/feels.json) instructions"
    echo "   - $(jq '.accounts | length' target/idl/feels.json) accounts"
    echo "   - $(jq '.types | length' target/idl/feels.json) types"
    echo ""
    
    echo "Checking critical instructions..."
    MISSING=""
    for ix in initialize_market swap open_position close_position collect_fees enter_feelssol exit_feelssol; do
        if ! jq -e ".instructions[] | select(.name == \"$ix\")" target/idl/feels.json >/dev/null 2>&1; then
            MISSING="$MISSING $ix"
        fi
    done
    
    if [ -z "$MISSING" ]; then
        echo "   All critical instructions found"
    else
        echo "   Missing instructions:$MISSING"
        exit 1
    fi
    
    echo ""
    echo "Checking critical account types..."
    MISSING=""
    for acc in Market Position Buffer TickArray; do
        if ! jq -e ".accounts[] | select(.name | endswith(\"::$acc\"))" target/idl/feels.json >/dev/null 2>&1; then
            MISSING="$MISSING $acc"
        fi
    done
    
    if [ -z "$MISSING" ]; then
        echo "   All critical account types found"
    else
        echo "   Missing accounts:$MISSING"
        exit 1
    fi
    
    echo ""
    echo "Checking parameter types..."
    MISSING=""
    for type in SwapParams InitializeMarketParams MintTokenParams ClosePositionParams; do
        if ! jq -e ".types[] | select(.name | endswith(\"::$type\"))" target/idl/feels.json >/dev/null 2>&1; then
            MISSING="$MISSING $type"
        fi
    done
    
    if [ -z "$MISSING" ]; then
        echo "   All critical parameter types found"
    else
        echo "   Missing types:$MISSING"
        exit 1
    fi
    
    echo ""
    echo "============================================================"
    echo "IDL validation passed!"

# Generate TypeScript types
types: idl-build
    @echo "Generating TypeScript types..."
    @echo "Types are generated as part of anchor build"

# Generate Cargo.nix for fast builds
cargo-nix:
    @echo "Generating Cargo.nix..."
    nix run .#generate-cargo-nix

# Enter Nix development shell
nix-shell:
    @echo "Entering Nix development shell..."
    nix develop

# Benchmark the protocol (placeholder)
bench:
    @echo "Running benchmarks..."
    @echo "Benchmarks not yet implemented"

# Security audit
audit:
    @echo "Running security audit..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit ; \
    else \
        echo "cargo-audit not installed. Install with: cargo install cargo-audit" ; \
    fi

# Update dependencies
update:
    @echo "Updating dependencies..."
    cargo update

# Generate documentation
docs:
    @echo "Generating documentation..."
    cargo doc --no-deps --open

# Initialize development environment
init:
    @echo "Initializing development environment..."
    @echo "Installing git hooks..."
    @mkdir -p .git/hooks
    @echo "#!/bin/sh\njust format" > .git/hooks/pre-commit
    @chmod +x .git/hooks/pre-commit
    @echo "Development environment initialized!"

# Quick development cycle (format, lint, test)
dev: format lint test-unit
    @echo "Quick development cycle complete!"

# Full CI pipeline simulation
ci: format lint test build idl-build idl-validate
    @echo "Full CI pipeline simulation complete!"

# Tail logs from local validator
logs:
    @echo "Tailing validator logs..."
    @if [ -f logs/validator.log ]; then \
        tail -f logs/validator.log ; \
    else \
        echo "No validator log found. Start local devnet first with 'just local-devnet'" ; \
    fi

# Show program address
program-id:
    @echo "Feels Protocol Program ID:"
    @if [ -f target/deploy/feels-keypair.json ]; then \
        cat target/deploy/feels-keypair.json | solana address ; \
    else \
        echo "Program keypair not found. Build the program first with 'just build'" ; \
    fi

# Airdrop SOL to development wallet
airdrop AMOUNT="10":
    @echo "Airdropping {{AMOUNT}} SOL..."
    solana airdrop {{AMOUNT}}

# Show account balance
balance:
    @echo "Account balance:"
    solana balance

# Reset local development environment
reset:
    @echo "Resetting local development environment..."
    just clean
    rm -rf logs/ test-ledger/ keypairs/
    @echo "Reset complete!"