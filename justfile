# Feels Protocol Task Runner
# Run `just` to see all available tasks

# =============================================================================
# Variables
# =============================================================================

# All paths are now defined in nix-env.just via constants.just
# IDL_PATH, DEPLOY_PATH, LOGS_PATH, etc. come from the Nix environment

# =============================================================================
# Imports
# =============================================================================

# Import modular justfiles
import 'justfiles/common.just'
import 'justfiles/validation.just'
import 'justfiles/build.just'
import 'justfiles/testing.just'
import 'justfiles/solana-tools.just'
import 'justfiles/development.just'
import 'justfiles/frontend.just'
import 'justfiles/services.just'
import 'justfiles/indexer.just'
import 'justfiles/e2e.just'

# Note: test justfile import removed to avoid conflicts with modular system

# =============================================================================
# Default Help
# =============================================================================

# Default task - show help
default:
    @echo "Feels Protocol Development Commands"
    @echo "==================================="
    @echo ""
    @echo "Build & Deploy:"
    @echo "  just build         - Build the protocol (Nix BPF preferred)"
    @echo "  just nix-build     - Build with Nix BPF builder"
    @echo "  just check-env     - Check environment configuration"
    @echo "  just validate [OP] - Validate environment for operation"
    @echo "  just deploy        - Deploy to local devnet"
    @echo "  just deploy-devnet - Deploy to Solana devnet"
    @echo ""
    @echo "Development:"
    @echo "  just clean         - Clean build artifacts"
    @echo "  just local-devnet  - Start local development network"
    @echo "  just validator-logs - Tail validator logs"
    @echo "  just program-id    - Show program address"
    @echo "  just reset         - Reset local development environment"
    @echo ""
    @echo "IDL & Client Generation:"
    @echo "  just idl-build [PROGRAM] - Generate IDL + clients (default: all)"
    @echo "  just idl-validate  - Validate IDL consistency"
    @echo "  just generate-clients - Generate TypeScript & Rust clients"
    @echo "  just frontend::generate-sdk [PROGRAM_ID] - Generate SDK (optionally with custom ID)"
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
    @echo "  just run          - Start complete E2E environment"
    @echo "  just stop         - Stop all E2E services" 
    @echo "  just status       - Check status of all services"
    @echo "  just logs [service] - View service logs"
    @echo ""
    @echo "Indexer Testing:"
    @echo "  cd feels-indexer/tests && just <command>  - Run indexer test commands"
    @echo "  Common: test-unit, test-integration, services-start, services-stop"
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
    # Ensure validator is running
    just ensure-validator
    
    # Pre-flight checks
    just run-preflight-checks "anchor .env"
    
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

# Note: SDK generation is now handled by frontend::generate-sdk

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

# Airdrop SOL to development wallet
airdrop AMOUNT="10":
    @just ensure-validator
    @echo "Airdropping {{AMOUNT}} SOL..."
    nix develop -c solana airdrop {{AMOUNT}}

# Show account balance
balance:
    @just ensure-validator
    @echo "Account balance:"
    nix develop -c solana balance

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
# E2E commands are imported from justfiles/e2e.just and available directly:
#   just run     - Start complete E2E environment
#   just stop    - Stop all E2E services
#   just status  - Check status of all services
#   just logs [service] - View service logs