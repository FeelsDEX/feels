# Feels Protocol Task Runner
# Run `just` to see all available tasks

# Default task - show help
default:
    @echo "Feels Protocol Development Tasks"
    @echo "================================"
    @echo ""
    @echo "Build & Deploy:"
    @echo "  just build         - Build the protocol with Anchor"
    @echo "  just nix-build     - Build with Nix BPF builder"
    @echo "  just deploy        - Deploy to local devnet"
    @echo "  just deploy-devnet - Deploy to Solana devnet"
    @echo ""
    @echo "Development:"
    @echo "  just clean         - Clean build artifacts"
    @echo "  just local-devnet  - Start local development network"
    @echo "  just logs          - Tail validator logs"
    @echo "  just program-id    - Show program address"
    @echo ""
    @echo "IDL Generation:"
    @echo "  just idl-build [PROGRAM] - Generate IDL files (default: all)"
    @echo "  just idl-validate  - Validate IDL consistency"
    @echo ""
    @echo "Solana Tools:"
    @echo "  just airdrop [AMT] - Airdrop SOL to wallet (default: 10)"
    @echo "  just balance       - Show account balance"

# Build the protocol using Anchor (default)
build:
    @echo "Building Feels Protocol..."
    anchor build
    @echo "Programs built and available in target/deploy/"

# Build with Nix BPF builder
nix-build:
    @echo "Building with Nix BPF builder..."
    nix build .#feels --out-link target/nix-feels
    @mkdir -p target/deploy
    @cp target/nix-feels/deploy/*.so target/deploy/ 2>/dev/null || true
    @echo "Feels program built with Nix and copied to target/deploy/"
    @echo "Note: feels-jupiter-adapter is a library, not deployed on-chain"


# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    rm -rf target/
    rm -rf .anchor/

# Start local development network
local-devnet:
    @echo "Starting local development network..."
    nix run .#devnet

# Deploy to local devnet
deploy:
    @echo "Deploying to local devnet..."
    anchor deploy --provider.cluster localnet

# Deploy to Solana devnet
deploy-devnet:
    @echo "Deploying to Solana devnet..."
    anchor deploy --provider.cluster devnet

# Generate IDL files
idl-build PROGRAM="":
    #!/usr/bin/env bash
    if [ -z "{{PROGRAM}}" ]; then
        echo "Generating IDL files for all on-chain programs..."
        nix run .#idl-build -- feels
        echo "Note: feels-jupiter-adapter is a library, not an on-chain program, so it doesn't have an IDL"
    else
        echo "Generating IDL for {{PROGRAM}}..."
        nix run .#idl-build -- {{PROGRAM}}
    fi

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

# Tail logs from local validator
logs:
    @echo "Tailing validator logs..."
    @if [ -f logs/validator.log ]; then \
        tail -f logs/validator.log ; \
    else \
        echo "No validator log found. Start local devnet first with 'just local-devnet'" ; \
    fi

# Show program address
program-id PROGRAM="feels":
    @echo "{{PROGRAM}} Program ID:"
    @if [ -f target/deploy/{{PROGRAM}}-keypair.json ]; then \
        cat target/deploy/{{PROGRAM}}-keypair.json | solana address ; \
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