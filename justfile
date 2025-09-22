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
import 'justfiles/build.just'
import 'justfiles/testing.just'
import 'justfiles/solana-tools.just'

# Import test justfile (for backward compatibility)
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
    @echo "  cd feels-app && just <command>  - Run frontend commands"
    @echo "  Common: install, dev, build, test, clean"
    @echo ""
    @echo "Complete E2E Development:"
    @echo "  just -f e2e/justfile <command>  - Run E2E commands"
    @echo "  Common: run, stop, status, logs"
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

# Build the protocol using Anchor (delegates to build.just)
build:
    @just _build

# Build with Nix BPF builder (delegates to build.just)
nix-build:
    @just _nix-build

# Clean build artifacts (delegates to build.just)
clean:
    @just _clean

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

# Reset local development environment (delegates to build.just)
reset:
    @just _reset

# =============================================================================
# IDL & Client Generation
# =============================================================================

# Generate IDL files and clients (delegates to build.just)
idl-build PROGRAM="":
    @just _idl-build {{PROGRAM}}

# Validate IDL against SDK (delegates to build.just)
idl-validate:
    @just _idl-validate

# Generate TypeScript and Rust clients from IDL (delegates to build.just)
generate-clients:
    @just _generate-clients

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

# Run all tests (delegates to testing.just)
test:
    @just _test

# Run all tests including devnet and localnet (delegates to testing.just)
test-all-full:
    @just _test-all

# Run devnet tests only (delegates to testing.just)
test-devnet:
    @just _test-devnet

# Backward compatibility aliases (delegate to testing.just)
test-unit:
    @just _test-unit
test-integration:
    @just _test-integration
test-property:
    @just _test-property
test-e2e:
    @just _test-e2e

# Run E2E pipeline tests (indexer + frontend)
test-e2e-pipeline:
    @just e2e-pipeline

# Setup localnet for testing (delegates to testing.just)
test-localnet:
    @just _test-localnet

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
# Frontend Application
# =============================================================================
# Frontend commands have been moved to feels-app/justfile for better modularity.
# Usage: cd feels-app && just <command>
# 
# Common commands:
#   just install      - Install dependencies
#   just dev          - Start development server
#   just build        - Build for production
#   just test         - Run tests
#
# Or for SDK generation + frontend:
#   just generate-sdk && cd feels-app && just dev

# =============================================================================
# Complete E2E Local Development Environment  
# =============================================================================
# E2E commands have been moved to e2e/justfile for better modularity.
# Usage: just -f e2e/justfile <command>
#
# Common commands:
#   just -f e2e/justfile run     - Start complete environment
#   just -f e2e/justfile stop    - Stop all services
#   just -f e2e/justfile status  - Check service status
#   just -f e2e/justfile logs    - View service logs