# Feels Protocol Task Runner
# Run `just` to see all available tasks

# Import test justfile
import? 'programs/feels/tests/justfile'

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
    @echo "IDL & Client Generation:"
    @echo "  just idl-build [PROGRAM] - Generate IDL + TypeScript/Rust clients (default: all)"
    @echo "  just idl-validate  - Validate IDL consistency"
    @echo "  just generate-clients - Generate TypeScript & Rust clients from existing IDL"
    @echo ""
    @echo "Testing:"
    @echo "  just test          - Run all tests"
    @echo "  just test-unit     - Run unit tests only"
    @echo "  just test-integration - Run integration tests only"
    @echo "  just test-e2e      - Run end-to-end tests only"
    @echo "  just test-property - Run property-based tests only"
    @echo ""
    @echo "SDK Generation:"
    @echo "  just generate-sdk  - Generate client SDK (TypeScript & Rust)"
    @echo "  just generate-sdk-with-id ID - Generate SDK with custom program ID"
    @echo ""
    @echo "Solana Tools:"
    @echo "  just airdrop [AMT] - Airdrop SOL to wallet (default: 10)"
    @echo "  just balance       - Show account balance"

# Build the protocol using Anchor (default)
build:
    @echo "Building Feels Protocol..."
    nix develop --command anchor build --no-idl --program-name feels
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
    nix develop --command cargo clean
    rm -rf target/
    rm -rf .anchor/

# Start local development network
local-devnet:
    @echo "Starting local development network..."
    nix run .#devnet

# Deploy to local devnet
deploy:
    @echo "Deploying to local devnet..."
    nix develop --command anchor deploy --provider.cluster localnet

# Deploy to Solana devnet
deploy-devnet:
    @echo "Deploying to Solana devnet..."
    nix develop --command anchor deploy --provider.cluster devnet

# Generate IDL files
idl-build PROGRAM="":
    #!/usr/bin/env bash
    if [ -z "{{PROGRAM}}" ]; then
        echo "Generating IDL files for all on-chain programs..."
        nix run .#idl-build -- feels
        echo "Note: feels-jupiter-adapter is a library, not an on-chain program, so it doesn't have an IDL"
        echo ""
        echo "Generating TypeScript and Rust clients from IDL..."
        just generate-clients
    else
        echo "Generating IDL for {{PROGRAM}}..."
        nix run .#idl-build -- {{PROGRAM}}
        if [ "{{PROGRAM}}" = "feels" ]; then
            echo ""
            echo "Generating TypeScript and Rust clients from IDL..."
            just generate-clients
        fi
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
        cat target/deploy/{{PROGRAM}}-keypair.json | nix develop --command solana address ; \
    else \
        echo "Program keypair not found. Build the program first with 'just build'" ; \
    fi

# Airdrop SOL to development wallet
airdrop AMOUNT="10":
    @echo "Airdropping {{AMOUNT}} SOL..."
    nix develop --command solana airdrop {{AMOUNT}}

# Show account balance
balance:
    @echo "Account balance:"
    nix develop --command solana balance

# Run all tests (alias to imported 'all' from test justfile)
test:
    @just all

# Backward compatibility aliases
test-unit: unit
test-integration: integration
test-property: property
test-e2e: e2e

# Reset local development environment
reset:
    @echo "Resetting local development environment..."
    just clean
    rm -rf logs/ test-ledger/ keypairs/
    @echo "Reset complete!"

# Generate TypeScript and Rust clients from IDL
generate-clients:
    @echo "Generating TypeScript and Rust clients from IDL..."
    @bash scripts/generate-clients.sh

# Generate client SDK using Anchor
generate-sdk:
    @echo "Generating client SDK..."
    @echo "Step 1: Building program (without IDL to avoid stack issues)..."
    just build
    @echo ""
    @echo "Step 2: Generating IDL using custom builder..."
    just idl-build
    @echo ""
    @echo "Step 3: Generating TypeScript SDK..."
    @mkdir -p generated-sdk
    @if [ -f target/idl/feels.json ]; then \
        echo "Found IDL at target/idl/feels.json"; \
        echo "Converting IDL to TypeScript..."; \
        nix develop --command node -e " \
            const fs = require('fs'); \
            const idl = JSON.parse(fs.readFileSync('target/idl/feels.json', 'utf8')); \
            const ts = 'export type Feels = ' + JSON.stringify(idl, null, 2) + ';\\n\\nexport const IDL: Feels = ' + JSON.stringify(idl, null, 2) + ';'; \
            fs.writeFileSync('generated-sdk/feels.ts', ts); \
            console.log('TypeScript IDL generated successfully'); \
        " || echo "Note: Direct TypeScript generation failed, using raw IDL"; \
    else \
        echo "Error: IDL not found at target/idl/feels.json"; \
        exit 1; \
    fi
    @echo ""
    @echo "Step 4: Generating TypeScript types using anchor idl type..."
    nix develop --command anchor idl type -o generated-sdk/feels_types.ts target/idl/feels.json || echo "Note: Type generation failed"
    @echo ""
    @echo "Step 5: Creating Rust client bindings..."
    @echo "Note: Anchor 0.31.1 doesn't include client-gen. For Rust clients, use anchor-client crate with the IDL."
    @echo ""
    @echo "Client SDK generated in generated-sdk/"
    @echo "  - IDL: target/idl/feels.json"
    @echo "  - TypeScript: generated-sdk/feels.ts (complete IDL module)"
    @echo "  - TypeScript Types: generated-sdk/feels_types.ts (if generated)"
    @echo ""
    @echo "To use the TypeScript SDK:"
    @echo "  import { IDL } from './generated-sdk/feels';"
    @echo "  const program = new anchor.Program(IDL, programId, provider);"
    @echo ""
    @echo "For Rust clients, add to Cargo.toml:"
    @echo "  anchor-client = \"0.31.1\""
    @echo "  feels-sdk = { path = \"../sdk\" }"

# Generate SDK with custom program ID
generate-sdk-with-id PROGRAM_ID:
    @echo "Generating client SDK with custom program ID: {{PROGRAM_ID}}..."
    @echo "Step 1: Building program (without IDL to avoid stack issues)..."
    just build
    @echo ""
    @echo "Step 2: Generating IDL using custom builder..."
    just idl-build
    @echo ""
    @echo "Step 3: Generating TypeScript SDK..."
    @mkdir -p generated-sdk
    @if [ -f target/idl/feels.json ]; then \
        echo "Found IDL at target/idl/feels.json"; \
        echo "Converting IDL to TypeScript with custom program ID..."; \
        nix develop --command node -e " \
            const fs = require('fs'); \
            const idl = JSON.parse(fs.readFileSync('target/idl/feels.json', 'utf8')); \
            const ts = 'export type Feels = ' + JSON.stringify(idl, null, 2) + ';\\n\\nexport const IDL: Feels = ' + JSON.stringify(idl, null, 2) + ';\\n\\nexport const PROGRAM_ID = \"{{PROGRAM_ID}}\";'; \
            fs.writeFileSync('generated-sdk/feels.ts', ts); \
            console.log('TypeScript IDL generated successfully with program ID: {{PROGRAM_ID}}'); \
        " || echo "Note: Direct TypeScript generation failed, using raw IDL"; \
    else \
        echo "Error: IDL not found at target/idl/feels.json"; \
        exit 1; \
    fi
    @echo ""
    @echo "Step 4: Generating TypeScript types using anchor idl type..."
    nix develop --command anchor idl type -o generated-sdk/feels_types.ts target/idl/feels.json || echo "Note: Type generation failed"
    @echo ""
    @echo "Step 5: Creating Rust client bindings..."
    @echo "Note: Anchor 0.31.1 doesn't include client-gen. For Rust clients, use anchor-client crate with the IDL."
    @echo ""
    @echo "Client SDK generated in generated-sdk/"
    @echo "  - IDL: target/idl/feels.json"
    @echo "  - TypeScript: generated-sdk/feels.ts (with PROGRAM_ID export)"
    @echo "  - TypeScript Types: generated-sdk/feels_types.ts (if generated)"
    @echo ""
    @echo "To use the TypeScript SDK:"
    @echo "  import { IDL, PROGRAM_ID } from './generated-sdk/feels';"
    @echo "  const program = new anchor.Program(IDL, PROGRAM_ID, provider);"
    @echo ""
    @echo "For Rust clients, add to Cargo.toml:"
    @echo "  anchor-client = \"0.31.1\""
    @echo "  feels-sdk = { path = \"../sdk\" }"