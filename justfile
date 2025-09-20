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
    @echo "  just check-env     - Check environment configuration"
    @echo "  just deploy        - Deploy to local devnet (uses .env PROGRAM_AUTHORITY)"
    @echo "  just deploy-devnet - Deploy to Solana devnet (uses .env PROGRAM_AUTHORITY)"
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
    @echo "  just test          - Run all in-memory tests"
    @echo "  just test-all      - Run ALL tests (in-memory + devnet + localnet)"
    @echo "  just test-unit     - Run unit tests only"
    @echo "  just test-integration - Run integration tests only"
    @echo "  just test-e2e      - Run end-to-end tests only"
    @echo "  just test-property - Run property-based tests only"
    @echo "  just test-devnet   - Run devnet tests only"
    @echo "  just test-localnet - Run all localnet tests with full setup"
    @echo "  just test-localnet-only - Run localnet tests (validator must be running)"
    @echo "  just setup-localnet - Setup localnet environment for testing"
    @echo "  just stop-localnet - Stop the localnet validator"
    @echo "  just localnet-logs - View localnet validator logs"
    @echo ""
    @echo "SDK Generation:"
    @echo "  just generate-sdk  - Generate client SDK (TypeScript & Rust)"
    @echo "  just generate-sdk-with-id ID - Generate SDK with custom program ID"
    @echo ""
    @echo "Frontend Application:"
    @echo "  just app-install   - Install Next.js app dependencies"
    @echo "  just app-dev       - Start Next.js development server"
    @echo "  just app-dev-indexer - Start Next.js app with indexer integration"
    @echo "  just app-dev-with-indexer - Full setup with indexer integration"
    @echo "  just app-build     - Build Next.js app for production"
    @echo "  just app-start     - Start production Next.js server"
    @echo "  just app-lint      - Lint the Next.js application"
    @echo "  just app-test      - Run Next.js app tests"
    @echo "  just app-clean     - Clean Next.js build artifacts"
    @echo ""
    @echo "Complete E2E Development:"
    @echo "  just dev-e2e       - Start complete environment (node+streaming+indexer+app)"
    @echo "  just dev-e2e-simple - Quick E2E startup without full monitoring"
    @echo "  just dev-e2e-status - Check status of all E2E services"
    @echo "  just dev-e2e-stop  - Stop all E2E services"
    @echo "  just dev-e2e-logs [SERVICE] - View logs (validator|streaming-adapter|indexer|app)"
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

# Check environment configuration
check-env:
    #!/usr/bin/env bash
    echo "Checking environment configuration..."
    if [ -f .env ]; then
        echo "[OK] .env file found"
        export $(grep -v '^#' .env | xargs)
        if [ -n "$PROGRAM_AUTHORITY" ]; then
            echo "[OK] PROGRAM_AUTHORITY set to: $PROGRAM_AUTHORITY"
            # Check if we can find the corresponding keypair
            found_keypair=false
            for keypair in ~/.config/solana/id.json keypairs/*.json; do
                if [ -f "$keypair" ]; then
                    pubkey=$(solana-keygen pubkey "$keypair" 2>/dev/null || true)
                    if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
                        echo "[OK] Found matching keypair at: $keypair"
                        found_keypair=true
                        break
                    fi
                fi
            done
            if [ "$found_keypair" = false ]; then
                echo "[WARNING] Could not find keypair for authority $PROGRAM_AUTHORITY"
                echo "   Make sure the keypair is available when deploying"
            fi
        else
            echo "[WARNING] PROGRAM_AUTHORITY not set in .env file"
        fi
    else
        echo "[WARNING] No .env file found"
        echo "   Create a .env file with: PROGRAM_AUTHORITY=<your-authority-pubkey>"
    fi

# Start local development network
local-devnet:
    @echo "Starting local development network..."
    nix run .#devnet

# Deploy to local devnet
deploy:
    #!/usr/bin/env bash
    echo "Deploying to local devnet..."
    # Load environment variables from .env if it exists
    if [ -f .env ]; then
        echo "Loading environment from .env..."
        export $(grep -v '^#' .env | xargs)
    fi
    if [ -n "$PROGRAM_AUTHORITY" ]; then
        echo "Using program authority: $PROGRAM_AUTHORITY"
        # Check for authority keypair
        for keypair in ~/.config/solana/id.json keypairs/*.json; do
            if [ -f "$keypair" ]; then
                pubkey=$(nix develop --command solana-keygen pubkey "$keypair" 2>/dev/null || true)
                if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
                    echo "Found authority keypair at: $keypair"
                    export ANCHOR_WALLET="$keypair"
                    break
                fi
            fi
        done
    fi
    nix develop --command anchor deploy --provider.cluster localnet

# Deploy to Solana devnet
deploy-devnet:
    #!/usr/bin/env bash
    echo "Deploying to Solana devnet..."
    # Load environment variables from .env if it exists
    if [ -f .env ]; then
        echo "Loading environment from .env..."
        export $(grep -v '^#' .env | xargs)
    fi
    if [ -n "$PROGRAM_AUTHORITY" ]; then
        echo "Using program authority: $PROGRAM_AUTHORITY"
        # Check for authority keypair
        for keypair in ~/.config/solana/id.json keypairs/*.json; do
            if [ -f "$keypair" ]; then
                pubkey=$(nix develop --command solana-keygen pubkey "$keypair" 2>/dev/null || true)
                if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
                    echo "Found authority keypair at: $keypair"
                    export ANCHOR_WALLET="$keypair"
                    break
                fi
            fi
        done
    fi
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

# Run all tests including devnet and localnet (alias to imported command)
test-all-full: test-all

# Run devnet tests only
test-devnet:
    @just devnet

# Backward compatibility aliases
test-unit: unit
test-integration: integration
test-property: property
test-e2e: e2e

# Setup localnet for testing
setup-localnet:
    @echo "Setting up localnet for tests..."
    @mkdir -p logs keypairs test-ledger
    @echo ""
    @echo "Checking if validator is running..."
    @if nix develop --command solana cluster-version 2>/dev/null | grep -q "Feature Set"; then \
        echo "[OK] Localnet validator is already running"; \
    else \
        echo "Starting localnet validator..."; \
        nix develop --command solana-test-validator \
            --ledger test-ledger \
            --rpc-port 8899 \
            --faucet-port 9900 \
            --quiet \
            --reset \
            > logs/validator.log 2>&1 & \
        echo "Waiting for validator to start..."; \
        sleep 5; \
        while ! nix develop --command solana cluster-version >/dev/null 2>&1; do \
            echo -n "."; \
            sleep 1; \
        done; \
        echo ""; \
        echo "[OK] Validator started"; \
    fi
    @echo ""
    @echo "Setting up test authority..."
    @if [ ! -f keypairs/test-authority.json ]; then \
        nix develop --command solana-keygen new -o keypairs/test-authority.json --no-bip39-passphrase --force; \
    fi
    @echo "Setting up payer for tests..."
    @if [ ! -f keypairs/payer.json ]; then \
        nix develop --command solana-keygen new -o keypairs/payer.json --no-bip39-passphrase --force; \
    fi
    @export ANCHOR_WALLET=keypairs/test-authority.json && \
        echo "Test authority: $$(nix develop --command solana address -k keypairs/test-authority.json)" && \
        nix develop --command solana airdrop 100 -k keypairs/test-authority.json --url http://localhost:8899 >/dev/null 2>&1 || true
    @echo "Funding payer: $$(nix develop --command solana address -k keypairs/payer.json)"
    @nix develop --command solana airdrop 10 -k keypairs/payer.json --url http://localhost:8899 >/dev/null 2>&1 || true
    @echo "[OK] Test environment ready"

# Run all tests that require localnet (with setup)
test-localnet: setup-localnet build deploy
    @echo ""
    @echo "Running localnet tests..."
    @echo "This will run:"
    @echo "  - Integration tests with localnet (including oracle safety tests)"
    @echo "  - Exact output swap tests" 
    @echo "  - SDK tests against localnet"
    @echo ""
    @echo "Running localnet integration tests (including oracle safety)..."
    @export ANCHOR_WALLET=keypairs/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        export DISABLE_AIRDROP_RATE_LIMIT=1 && \
        RUN_LOCALNET_TESTS=1 nix develop --command cargo test -p feels -- --test-threads=1 --nocapture --ignored
    @echo ""
    @echo "Running SDK localnet tests..."
    @export ANCHOR_WALLET=keypairs/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        RUN_LOCALNET_TESTS=1 nix develop --command cargo test -p feels-sdk -- --test-threads=1
    @echo ""
    @echo "Localnet tests complete!"

# Run localnet tests without setup (assumes validator is running)
test-localnet-only:
    @echo "Running localnet tests (assuming validator is already running)..."
    @export ANCHOR_WALLET=keypairs/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        RUN_LOCALNET_TESTS=1 nix develop --command cargo test -- --test-threads=1

# Stop localnet validator
stop-localnet:
    @echo "Stopping localnet validator..."
    @pkill -f solana-test-validator || true
    @echo "Localnet stopped"
    
# View localnet logs
localnet-logs:
    @if [ -f logs/validator.log ]; then \
        tail -f logs/validator.log; \
    else \
        echo "No validator logs found. Start localnet first with 'just setup-localnet' or 'just test-localnet'"; \
    fi

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
# =============================================================================
# Frontend Application Commands
# =============================================================================

# Install Next.js app dependencies
app-install:
	@echo "Installing Next.js app dependencies..."
	cd feels-app && npm install

# Start Next.js development server
app-dev:
	@echo "Starting Next.js development server..."
	@echo "App will be available at http://localhost:3000"
	cd feels-app && npm run dev

# Build Next.js app for production
app-build:
	@echo "Building Next.js app for production..."
	cd feels-app && npm run build

# Start production Next.js server
app-start: app-build
	@echo "Starting production Next.js server..."
	@echo "App will be available at http://localhost:3000"
	cd feels-app && npm run start

# Lint the Next.js application
app-lint:
	@echo "Linting Next.js application..."
	cd feels-app && npm run lint

# Type check the Next.js application
app-type-check:
	@echo "Type checking Next.js application..."
	cd feels-app && npm run type-check

# Format the Next.js application code
app-format:
	@echo "Formatting Next.js application code..."
	cd feels-app && npm run format

# Run Next.js app tests (placeholder - add when tests are implemented)
app-test:
	@echo "Running Next.js app tests..."
	@echo "Note: Tests not yet implemented. Add test scripts to app/package.json"
	cd feels-app && npm run type-check

# Clean Next.js build artifacts
app-clean:
	@echo "Cleaning Next.js build artifacts..."
	cd feels-app && rm -rf .next node_modules/.cache

# Full app setup: install dependencies and start development server
app-setup: app-install
	@echo "Next.js app setup complete!"
	@echo "Run 'just app-dev' to start the development server"

# Build app with fresh dependencies
app-fresh: app-clean app-install app-build
	@echo "Fresh Next.js app build complete!"

# Development workflow: generate SDK, install deps, and start dev server
app-dev-full: generate-sdk app-install app-dev
	@echo "Full development environment ready!"

# Start Next.js app with indexer integration
app-dev-indexer:
	@echo "Starting Next.js app with indexer integration..."
	@echo "App will connect to indexer at http://localhost:8080"
	cd feels-app && npm run dev:indexer

# Full development setup with indexer
app-dev-with-indexer: generate-sdk app-install
	@echo "Starting full development environment with indexer..."
	@echo "Make sure the indexer is running on http://localhost:8080"
	cd feels-app && npm run dev:indexer

# =============================================================================
# Complete E2E Local Development Environment
# =============================================================================

# Start complete E2E development environment (node + streaming + indexer + app)
dev-e2e:
	@just -f e2e/justfile run

# Start development environment with monitoring
dev-e2e-monitor: dev-e2e

# Quick E2E setup without full monitoring
dev-e2e-simple: dev-e2e

# Stop all E2E services
dev-e2e-stop:
	@just -f e2e/justfile stop

# Show status of E2E services
dev-e2e-status:
	@just -f e2e/justfile status

# View logs from E2E services
dev-e2e-logs SERVICE="all":
	@just -f e2e/justfile logs {{SERVICE}}
