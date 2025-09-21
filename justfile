# Feels Protocol Task Runner
# Run `just` to see all available tasks

# =============================================================================
# Variables
# =============================================================================

IDL_PATH := "target/idl/feels.json"
DEPLOY_PATH := "target/deploy"
LOGS_PATH := "logs"
KEYPAIRS_PATH := "keypairs"
GENERATED_SDK_PATH := "generated-sdk"

# =============================================================================
# Imports
# =============================================================================

# Import modular justfiles
import 'justfiles/common.just'
import 'justfiles/validation.just'
import 'justfiles/solana-tools.just'

# Import test justfile
import? 'programs/feels/tests/justfile'

# =============================================================================
# Default Help
# =============================================================================

# Default task - show help
default:
    @echo "Feels Protocol Development Commands"
    @echo "==================================="
    @echo ""
    @echo "Build & Deploy:"
    @echo "  just build         - Build the protocol with Anchor"
    @echo "  just nix-build     - Build with Nix BPF builder"
    @echo "  just check-env     - Check environment configuration"
    @echo "  just validate [OP] - Validate environment for operation"
    @echo "  just deploy        - Deploy to local devnet"
    @echo "  just deploy-devnet - Deploy to Solana devnet"
    @echo ""
    @echo "Development:"
    @echo "  just clean         - Clean build artifacts"
    @echo "  just local-devnet  - Start local development network"
    @echo "  just logs          - Tail validator logs"
    @echo "  just program-id    - Show program address"
    @echo "  just reset         - Reset local development environment"
    @echo ""
    @echo "IDL & Client Generation:"
    @echo "  just idl-build [PROGRAM] - Generate IDL + clients (default: all)"
    @echo "  just idl-validate  - Validate IDL consistency"
    @echo "  just generate-clients - Generate TypeScript & Rust clients"
    @echo "  just generate-sdk [PROGRAM_ID] - Generate SDK (optionally with custom ID)"
    @echo ""
    @echo "Testing:"
    @echo "  just test          - Run all in-memory tests"
    @echo "  just test-all      - Run ALL tests (in-memory + devnet + localnet)"
    @echo "  just test-unit     - Run unit tests only"
    @echo "  just test-integration - Run integration tests only"
    @echo "  just test-e2e      - Run end-to-end tests only"
    @echo "  just test-e2e-pipeline - Run E2E pipeline tests (indexer + frontend)"
    @echo "  just test-property - Run property-based tests only"
    @echo "  just test-devnet   - Run devnet tests only"
    @echo "  just test-localnet - Run all localnet tests with full setup"
    @echo ""
    @echo "Frontend Application:"
    @echo "  just app-install   - Install Next.js app dependencies"
    @echo "  just app-dev       - Start Next.js development server"
    @echo "  just app-dev-indexer - Start Next.js app with indexer"
    @echo "  just app-build     - Build Next.js app for production"
    @echo "  just app-clean     - Clean Next.js build artifacts"
    @echo ""
    @echo "Complete E2E Development:"
    @echo "  just dev-e2e       - Start complete environment (node+streaming+indexer+app)"
    @echo "  just dev-e2e-status - Check status of all E2E services"
    @echo "  just dev-e2e-stop  - Stop all E2E services"
    @echo "  just dev-e2e-logs [SERVICE] - View logs (validator|streaming-adapter|indexer|app)"
    @echo ""
    @echo "Solana Tools:"
    @echo "  just airdrop [AMT] - Airdrop SOL to wallet (default: 10)"
    @echo "  just balance       - Show account balance"
    @echo ""
    @echo "Documentation:"
    @echo "  See JUSTFILE.md for architecture and detailed command reference"

# =============================================================================
# Build & Deploy
# =============================================================================

# Build the protocol using Anchor
build:
    #!/usr/bin/env bash
    # Pre-flight checks
    just run-preflight-checks "anchor anchor.toml"
    
    just show-building
    if nix develop --command "anchor build --no-idl --program-name feels"; then
        just show-success "Programs built and available in {{DEPLOY_PATH}}/"
        exit {{EXIT_SUCCESS}}
    else
        just exit-with-error {{EXIT_BUILD_FAILURE}} "Anchor build failed"
    fi

# Build with Nix BPF builder
nix-build:
    @echo "Building with Nix BPF builder..."
    nix build .#feels --out-link target/nix-feels
    @mkdir -p {{DEPLOY_PATH}}
    @cp target/nix-feels/deploy/*.so {{DEPLOY_PATH}}/ 2>/dev/null || true
    @echo "Feels program built with Nix and copied to {{DEPLOY_PATH}}/"
    @echo "Note: feels-jupiter-adapter is a library, not deployed on-chain"

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    nix develop --command "cargo clean"
    rm -rf target/ .anchor/

# Check environment configuration
check-env:
    @just validate all

# Validate environment for specific operation
validate OPERATION="all":
    @just _validate {{OPERATION}}

# Deploy to local devnet
deploy:
    #!/usr/bin/env bash
    # Pre-flight checks
    just run-preflight-checks "anchor validator .env"
    
    just show-deploying
    if nix develop --command anchor deploy --provider.cluster localnet; then
        just show-success "Deployed to localnet successfully"
        exit {{EXIT_SUCCESS}}
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment to localnet failed"
    fi

# Deploy to Solana devnet
deploy-devnet:
    #!/usr/bin/env bash
    # Pre-flight checks
    just run-preflight-checks "anchor .env"
    
    just show-deploying
    echo "Deploying to Solana devnet..."
    if nix develop --command anchor deploy --provider.cluster devnet; then
        just show-success "Deployed to devnet successfully"
        exit {{EXIT_SUCCESS}}
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment to devnet failed"
    fi

# Start local development network
local-devnet:
    @echo "Starting local development network..."
    nix run .#devnet

# Reset local development environment
reset:
    @echo "Resetting local development environment..."
    just clean
    rm -rf {{LOGS_PATH}}/ test-ledger/ {{KEYPAIRS_PATH}}/
    @echo "Reset complete!"

# =============================================================================
# IDL & Client Generation
# =============================================================================

# Generate IDL files and clients
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
    
    if [ ! -f "{{IDL_PATH}}" ]; then
        echo "IDL not found at {{IDL_PATH}}"
        echo "   Run 'just idl-build' to generate the IDL first"
        exit 1
    fi
    
    echo "Found in IDL:"
    echo "   - $(jq '.instructions | length' {{IDL_PATH}}) instructions"
    echo "   - $(jq '.accounts | length' {{IDL_PATH}}) accounts"
    echo "   - $(jq '.types | length' {{IDL_PATH}}) types"
    echo ""
    
    echo "Checking critical instructions..."
    MISSING=""
    for ix in initialize_market swap open_position close_position collect_fees enter_feelssol exit_feelssol; do
        if ! jq -e ".instructions[] | select(.name == \"$ix\")" {{IDL_PATH}} >/dev/null 2>&1; then
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
        if ! jq -e ".accounts[] | select(.name | endswith(\"::$acc\"))" {{IDL_PATH}} >/dev/null 2>&1; then
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
        if ! jq -e ".types[] | select(.name | endswith(\"::$type\"))" {{IDL_PATH}} >/dev/null 2>&1; then
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

# Generate TypeScript and Rust clients from IDL
generate-clients:
    #!/usr/bin/env bash
    set -euo pipefail
    
    if [ ! -f "{{IDL_PATH}}" ]; then
        echo "Error: IDL not found at {{IDL_PATH}}"
        echo "Run 'just idl-build' first to generate the IDL"
        exit 1
    fi
    
    echo "=== Generating TypeScript Client ==="
    mkdir -p generated-sdk/typescript
    
    # Generate TypeScript types using Anchor
    echo "Generating TypeScript types..."
    if command -v anchor &> /dev/null; then
        anchor idl type -o generated-sdk/typescript/types.ts {{IDL_PATH}} || {
            echo "Warning: Anchor type generation failed, generating manually..."
        }
    fi
    
    # Generate TypeScript IDL module
    echo "Creating TypeScript IDL module..."
    cat > generated-sdk/typescript/index.ts << 'TYPESCRIPT_EOF'
    // Auto-generated TypeScript client for Feels Protocol
    import { PublicKey } from '@solana/web3.js';
    import { Program, AnchorProvider } from '@project-serum/anchor';
    
    TYPESCRIPT_EOF
    
    # Append the IDL
    echo "export const IDL = " >> generated-sdk/typescript/index.ts
    cat {{IDL_PATH}} >> generated-sdk/typescript/index.ts
    echo ";" >> generated-sdk/typescript/index.ts
    
    # Add program ID and helper
    PROGRAM_ID=$(jq -r '.address // empty' {{IDL_PATH}} || echo "11111111111111111111111111111112")
    cat >> generated-sdk/typescript/index.ts << EOF
    
    export const PROGRAM_ID = new PublicKey('${PROGRAM_ID}');
    
    export type Feels = typeof IDL;
    
    // Helper function to get the program
    export function getProgram(provider: AnchorProvider): Program<Feels> {
      return new Program(IDL as Feels, PROGRAM_ID, provider);
    }
    EOF
    
    echo "✓ TypeScript client generated at generated-sdk/typescript/"
    
    echo ""
    echo "=== Generating Rust Client ==="
    mkdir -p generated-sdk/rust/src
    
    # Generate Rust client bindings
    echo "Creating Rust client module..."
    cat > generated-sdk/rust/Cargo.toml << 'EOF'
    [package]
    name = "feels-client"
    version = "0.1.0"
    edition = "2021"
    
    [dependencies]
    anchor-client = "0.31.1"
    anchor-lang = "0.31.1"
    solana-sdk = "2.3.0"
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    
    [features]
    cpi = ["anchor-lang/cpi"]
    EOF
    
    # Create Rust client lib.rs
    echo "Generating Rust client code..."
    
    # Create the Rust client file
    cat > generated-sdk/rust/src/lib.rs << 'RUST_EOF'
    //! Auto-generated Rust client for Feels Protocol
    #![allow(dead_code)]
    
    use anchor_client::{
        solana_sdk::{
            instruction::Instruction,
            pubkey::Pubkey,
            signature::{Keypair, Signer},
            system_program,
        },
        Client, ClientError, Cluster, Program,
    };
    use std::rc::Rc;
    use std::str::FromStr;
    
    pub mod types {
        use super::*;
        use anchor_lang::prelude::*;
        
        // Re-export instruction parameter types
        // These would be generated from the IDL in a full implementation
        
        #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
        pub struct SwapParams {
            pub amount_in: u64,
            pub min_amount_out: u64,
            pub sqrt_price_limit: Option<u128>,
            pub is_token_0_in: bool,
            pub is_exact_in: bool,
        }
        
        #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
        pub struct InitializeMarketParams {
            pub fee_tier: u16,
            pub tick_spacing: u16,
            pub initial_sqrt_price: u128,
            pub initial_buy_feelssol_amount: u64,
        }
    }
    
    RUST_EOF
    
    # Add program ID
    echo "pub const PROGRAM_ID: &str = \"${PROGRAM_ID}\";" >> generated-sdk/rust/src/lib.rs
    
    # Add client struct and basic implementation
    cat >> generated-sdk/rust/src/lib.rs << 'RUST_EOF'
    
    pub struct FeelsClient {
        program: Program<Rc<Keypair>>,
    }
    
    type ClientResult<T> = Result<T, ClientError>;
    
    impl FeelsClient {
        pub fn new(
            cluster: Cluster,
            payer: Rc<Keypair>,
        ) -> ClientResult<Self> {
            let client = Client::new(cluster, payer.clone());
            let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
            let program = client.program(program_id)?;
            
            Ok(Self { program })
        }
        
        pub fn new_with_program_id(
            cluster: Cluster,
            payer: Rc<Keypair>,
            program_id: Pubkey,
        ) -> ClientResult<Self> {
            let client = Client::new(cluster, payer.clone());
            let program = client.program(program_id)?;
            
            Ok(Self { program })
        }
        
        pub fn program(&self) -> &Program<Rc<Keypair>> {
            &self.program
        }
    
        // Example instruction builders
        pub fn initialize_market(
            &self,
            deployer: Pubkey,
            token_0: Pubkey,
            token_1: Pubkey,
            feelssol_mint: Pubkey,
            params: types::InitializeMarketParams,
        ) -> ClientResult<Instruction> {
            // In a full implementation, this would use the IDL to build the instruction
            // For now, return a placeholder
            todo!("Implement based on IDL")
        }
        
        pub fn swap(
            &self,
            user: Pubkey,
            market: Pubkey,
            user_token_in: Pubkey,
            user_token_out: Pubkey,
            params: types::SwapParams,
        ) -> ClientResult<Instruction> {
            // In a full implementation, this would use the IDL to build the instruction
            todo!("Implement based on IDL")
        }
    }
    
    // Include the IDL as a constant
    pub const IDL_JSON: &str = include_str!("../../../target/idl/feels.json");
    
    #[cfg(test)]
    mod tests {
        use super::*;
        
        #[test]
        fn test_parse_idl() {
            let idl: serde_json::Value = serde_json::from_str(IDL_JSON).unwrap();
            assert!(idl.is_object());
        }
        
        #[test]
        fn test_program_id() {
            let program_id = Pubkey::from_str(PROGRAM_ID);
            assert!(program_id.is_ok());
        }
    }
    RUST_EOF
    
    echo "✓ Rust client generated at generated-sdk/rust/"
    
    echo ""
    echo "=== Client Generation Complete ==="
    echo ""
    echo "Generated clients:"
    echo "  TypeScript: generated-sdk/typescript/"
    echo "    - index.ts: Complete IDL and types"
    echo "    - types.ts: TypeScript type definitions (if generated)"
    echo ""
    echo "  Rust: generated-sdk/rust/"
    echo "    - Cargo.toml: Package manifest"
    echo "    - src/lib.rs: Client implementation with instruction builders"
    echo ""
    echo "To use the TypeScript client:"
    echo "  import { IDL, PROGRAM_ID } from './generated-sdk/typescript';"
    echo "  const program = new anchor.Program(IDL, PROGRAM_ID, provider);"
    echo ""
    echo "To use the Rust client:"
    echo "  Add to your Cargo.toml:"
    echo "    feels-client = { path = \"./generated-sdk/rust\" }"
    echo ""
    echo "  Then in your code:"
    echo "    use feels_client::{FeelsClient, PROGRAM_ID};"
    echo "    let client = FeelsClient::new(cluster, payer)?;"

# Generate client SDK (with optional custom program ID)
generate-sdk PROGRAM_ID="":
    @echo "Generating client SDK..."
    @echo "Step 1: Building program (without IDL to avoid stack issues)..."
    just build
    @echo ""
    @echo "Step 2: Generating IDL using custom builder..."
    just idl-build
    @echo ""
    @echo "Step 3: Generating TypeScript SDK..."
    @mkdir -p {{GENERATED_SDK_PATH}}
    @if [ -f "{{IDL_PATH}}" ]; then \
        echo "Found IDL at {{IDL_PATH}}"; \
        echo "Converting IDL to TypeScript..."; \
        if [ -n "{{PROGRAM_ID}}" ]; then \
            echo "Using custom program ID: {{PROGRAM_ID}}"; \
            nix develop --command "node -e \" \
                const fs = require('fs'); \
                const idl = JSON.parse(fs.readFileSync('{{IDL_PATH}}', 'utf8')); \
                const ts = 'export type Feels = ' + JSON.stringify(idl, null, 2) + ';\\\\n\\\\nexport const IDL: Feels = ' + JSON.stringify(idl, null, 2) + ';\\\\n\\\\nexport const PROGRAM_ID = \\\"{{PROGRAM_ID}}\\\";'; \
                fs.writeFileSync('{{GENERATED_SDK_PATH}}/feels.ts', ts); \
                console.log('TypeScript IDL generated successfully with program ID: {{PROGRAM_ID}}'); \
            \""; \
        else \
            nix develop --command "node -e \" \
                const fs = require('fs'); \
                const idl = JSON.parse(fs.readFileSync('{{IDL_PATH}}', 'utf8')); \
                const ts = 'export type Feels = ' + JSON.stringify(idl, null, 2) + ';\\\\n\\\\nexport const IDL: Feels = ' + JSON.stringify(idl, null, 2) + ';'; \
                fs.writeFileSync('{{GENERATED_SDK_PATH}}/feels.ts', ts); \
                console.log('TypeScript IDL generated successfully'); \
            \""; \
        fi; \
    else \
        echo "Error: IDL not found at {{IDL_PATH}}"; \
        exit 1; \
    fi
    @echo ""
    @echo "Step 4: Generating TypeScript types using anchor idl type..."
    nix develop --command "anchor idl type -o {{GENERATED_SDK_PATH}}/feels_types.ts {{IDL_PATH}}" || echo "Note: Type generation failed"
    @echo ""
    @echo "Step 5: Creating Rust client bindings..."
    @echo "Note: Anchor 0.31.1 doesn't include client-gen. For Rust clients, use anchor-client crate with the IDL."
    @echo ""
    @echo "Client SDK generated in {{GENERATED_SDK_PATH}}/"
    @echo "  - IDL: {{IDL_PATH}}"
    @echo "  - TypeScript: {{GENERATED_SDK_PATH}}/feels.ts"
    @echo "  - TypeScript Types: {{GENERATED_SDK_PATH}}/feels_types.ts (if generated)"
    @echo ""
    @if [ -n "{{PROGRAM_ID}}" ]; then \
        echo "To use the TypeScript SDK:"; \
        echo "  import { IDL, PROGRAM_ID } from './{{GENERATED_SDK_PATH}}/feels';"; \
        echo "  const program = new anchor.Program(IDL, PROGRAM_ID, provider);"; \
    else \
        echo "To use the TypeScript SDK:"; \
        echo "  import { IDL } from './{{GENERATED_SDK_PATH}}/feels';"; \
        echo "  const program = new anchor.Program(IDL, programId, provider);"; \
    fi
    @echo ""
    @echo "For Rust clients, add to Cargo.toml:"
    @echo "  anchor-client = \"0.31.1\""
    @echo "  feels-sdk = { path = \"../sdk\" }"

# =============================================================================
# Testing
# =============================================================================

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

# Run E2E pipeline tests (indexer + frontend)
test-e2e-pipeline:
    @just e2e-pipeline

# Setup localnet for testing
setup-localnet:
    @echo "Setting up localnet for tests..."
    @mkdir -p {{LOGS_PATH}} {{KEYPAIRS_PATH}} test-ledger
    @echo ""
    @echo "Checking if validator is running..."
    @if nix develop --command "solana cluster-version" 2>/dev/null | grep -q "Feature Set"; then \
        echo "[OK] Localnet validator is already running"; \
    else \
        echo "Starting localnet validator..."; \
        nix develop --command "solana-test-validator \
            --ledger test-ledger \
            --rpc-port 8899 \
            --faucet-port 9900 \
            --quiet \
            --reset" \
            > {{LOGS_PATH}}/validator.log 2>&1 & \
        echo "Waiting for validator to start..."; \
        sleep 5; \
        while ! nix develop --command "solana cluster-version" >/dev/null 2>&1; do \
            echo -n "."; \
            sleep 1; \
        done; \
        echo ""; \
        echo "[OK] Validator started"; \
    fi
    @echo ""
    @echo "Setting up test authority..."
    @if [ ! -f {{KEYPAIRS_PATH}}/test-authority.json ]; then \
        nix develop --command "solana-keygen new -o {{KEYPAIRS_PATH}}/test-authority.json --no-bip39-passphrase --force"; \
    fi
    @echo "Setting up payer for tests..."
    @if [ ! -f {{KEYPAIRS_PATH}}/payer.json ]; then \
        nix develop --command "solana-keygen new -o {{KEYPAIRS_PATH}}/payer.json --no-bip39-passphrase --force"; \
    fi
    @export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json && \
        echo "Test authority: $$(nix develop --command "solana address -k {{KEYPAIRS_PATH}}/test-authority.json")" && \
        nix develop --command "solana airdrop 100 -k {{KEYPAIRS_PATH}}/test-authority.json --url http://localhost:8899" >/dev/null 2>&1 || true
    @echo "Funding payer: $$(nix develop --command "solana address -k {{KEYPAIRS_PATH}}/payer.json")"
    @nix develop --command "solana airdrop 10 -k {{KEYPAIRS_PATH}}/payer.json --url http://localhost:8899" >/dev/null 2>&1 || true
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
    @export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        export DISABLE_AIRDROP_RATE_LIMIT=1 && \
        RUN_LOCALNET_TESTS=1 nix develop --command "cargo test -p feels -- --test-threads=1 --nocapture --ignored"
    @echo ""
    @echo "Running SDK localnet tests..."
    @export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        RUN_LOCALNET_TESTS=1 nix develop --command "cargo test -p feels-sdk -- --test-threads=1"
    @echo ""
    @echo "Localnet tests complete!"

# Run localnet tests without setup (assumes validator is running)
test-localnet-only:
    @echo "Running localnet tests (assuming validator is already running)..."
    @export ANCHOR_WALLET={{KEYPAIRS_PATH}}/test-authority.json && \
        export ANCHOR_PROVIDER_URL=http://localhost:8899 && \
        RUN_LOCALNET_TESTS=1 nix develop --command "cargo test -- --test-threads=1"

# Stop localnet validator
stop-localnet:
    @echo "Stopping localnet validator..."
    @pkill -f solana-test-validator || true
    @echo "Localnet stopped"
    
# View localnet logs
localnet-logs:
    @if [ -f {{LOGS_PATH}}/validator.log ]; then \
        tail -f {{LOGS_PATH}}/validator.log; \
    else \
        echo "No validator logs found. Start localnet first with 'just setup-localnet' or 'just test-localnet'"; \
    fi

# =============================================================================
# Utilities
# =============================================================================

# Tail logs from local validator
logs:
    @echo "Tailing validator logs..."
    @if [ -f {{LOGS_PATH}}/validator.log ]; then \
        tail -f {{LOGS_PATH}}/validator.log ; \
    else \
        echo "No validator log found. Start local devnet first with 'just local-devnet'" ; \
    fi


# Airdrop SOL to development wallet
airdrop AMOUNT="10":
    @echo "Airdropping {{AMOUNT}} SOL..."
    nix develop --command "solana airdrop {{AMOUNT}}"

# Show account balance
balance:
    @echo "Account balance:"
    nix develop --command "solana balance"

# =============================================================================
# Frontend Application (forwarded to feels-app/justfile)
# =============================================================================

# Install Next.js app dependencies
app-install:
    @echo "Installing Next.js app dependencies..."
    cd feels-app && just install

# Start Next.js development server
app-dev:
    @echo "Starting Next.js development server..."
    cd feels-app && just dev

# Start Next.js app with indexer integration
app-dev-indexer:
    @echo "Starting Next.js app with indexer integration..."
    cd feels-app && just dev-indexer

# Build Next.js app for production
app-build:
    @echo "Building Next.js app for production..."
    cd feels-app && just build

# Start production Next.js server
app-start:
    @echo "Starting production Next.js server..."
    cd feels-app && just start

# Lint the Next.js application
app-lint:
    @echo "Linting Next.js application..."
    cd feels-app && just lint

# Type check the Next.js application
app-type-check:
    @echo "Type checking Next.js application..."
    cd feels-app && just type-check

# Format the Next.js application code
app-format:
    @echo "Formatting Next.js application code..."
    cd feels-app && just format

# Run Next.js app tests
app-test:
    @echo "Running Next.js app tests..."
    cd feels-app && just test

# Clean Next.js build artifacts
app-clean:
    @echo "Cleaning Next.js build artifacts..."
    cd feels-app && just clean

# Full app setup: install dependencies and start development server
app-setup:
    @echo "Setting up Next.js app..."
    cd feels-app && just setup

# Build app with fresh dependencies
app-fresh:
    @echo "Fresh Next.js app build..."
    cd feels-app && just fresh

# Development workflow: generate SDK, install deps, and start dev server
app-dev-full: generate-sdk app-install app-dev
    @echo "Full development environment ready!"

# Full development setup with indexer
app-dev-with-indexer: generate-sdk app-install
    @echo "Starting full development environment with indexer..."
    cd feels-app && just dev-with-indexer

# =============================================================================
# Complete E2E Local Development Environment
# =============================================================================

# Start complete E2E development environment (node + streaming + indexer + app)
dev-e2e:
    @just -f e2e/justfile run

# Stop all E2E services
dev-e2e-stop:
    @just -f e2e/justfile stop

# Show status of E2E services
dev-e2e-status:
    @just -f e2e/justfile status

# View logs from E2E services
dev-e2e-logs SERVICE="all":
    @just -f e2e/justfile logs {{SERVICE}}