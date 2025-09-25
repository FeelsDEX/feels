# ╔═══════════════════════════════════════════════════════════════════════════╗
# ║                   FRONTEND APPLICATION MODULE                             ║
# ╚═══════════════════════════════════════════════════════════════════════════╝
#
# This module manages the Next.js frontend application for Feels Protocol,
# including development server, production builds, and SDK generation.
#
# Main Commands:
#   just frontend::dev         # Start development server
#   just frontend::build       # Build for production
#   just frontend::install     # Install dependencies
#   just frontend::generate-sdk # Generate TypeScript SDK
#
# Development:
#   just frontend::dev-indexer    # Dev with indexer integration
#   just frontend::lint           # Run ESLint
#   just frontend::type-check     # TypeScript checking
#   just frontend::format         # Format code
#
# Setup:
#   just frontend::setup          # Full setup and start
#   just frontend::setup-metaplex # Deploy Metaplex for NFTs
#   just frontend::clean          # Clean build artifacts
#   just frontend::fresh          # Fresh rebuild
#
# Import dependencies
import 'common.just'

# Frontend app directory
app-dir := FEELS_APP_PATH

# ═══════════════════════════════════════════════════════════════════════════
# Main Frontend Commands
# ═══════════════════════════════════════════════════════════════════════════

# Install Node.js dependencies for Next.js app
install:
    @just show-progress "Installing Next.js app dependencies"
    @cd {{app-dir}} && npm install

# Start Next.js development server (port 3000)
# Hot reload enabled for rapid development (use 'just app' from main justfile)
frontend-dev:
    @just show-progress "Starting Next.js development server with DevBridge"
    @cd {{app-dir}} && npm run dev:all

# Start with indexer integration enabled
# Connects to local indexer on port 8080
dev-indexer:
    @just show-progress "Starting Next.js app with indexer integration"
    @cd {{app-dir}} && npm run dev:indexer

# Build Next.js app for production deployment
# Optimizes bundle size and performance
build-frontend:
    @just show-progress "Building Next.js app for production"
    @cd {{app-dir}} && npm run build
    @just show-success "Production build complete"

# Start production Next.js server
# Run 'build' first to generate production bundle
start:
    @just show-progress "Starting production Next.js server"
    @cd {{app-dir}} && npm start

# ═══════════════════════════════════════════════════════════════════════════
# Development Utilities
# ═══════════════════════════════════════════════════════════════════════════

# Run ESLint to check code quality
frontend-lint:
    @just show-progress "Linting Next.js application"
    @cd {{app-dir}} && npm run lint

# Run TypeScript compiler for type checking
# Ensures type safety without emitting files
type-check:
    @just show-progress "Type checking Next.js application"
    @cd {{app-dir}} && npm run type-check

# Format code with Prettier
# Ensures consistent code style
frontend-format:
    @just show-progress "Formatting Next.js application code"
    @cd {{app-dir}} && npm run format

# Run test suite (when implemented)
frontend-test:
    @just show-progress "Running Next.js app tests"
    @cd {{app-dir}} && npm test

# ═══════════════════════════════════════════════════════════════════════════
# Setup Commands
# ═══════════════════════════════════════════════════════════════════════════

# Full setup: install dependencies and start dev server
setup: install frontend-dev

# Clean Next.js build artifacts and caches
frontend-clean:
    @just show-progress "Cleaning Next.js build artifacts"
    @cd {{app-dir}} && rm -rf .next out node_modules/.cache
    @just show-success "Build artifacts cleaned"

# Fresh rebuild: clean everything, reinstall deps, and build
fresh: frontend-clean install build-frontend

# Full development setup with SDK generation
dev-full: generate-sdk install frontend-dev

# Development with indexer integration and SDK generation
dev-with-indexer: generate-sdk install
    @just show-progress "Starting full development environment with indexer"
    @just dev-indexer

# ═══════════════════════════════════════════════════════════════════════════
# SDK Integration
# ═══════════════════════════════════════════════════════════════════════════

# Generate TypeScript SDK from Anchor IDL
# Includes IDL generation, copying, and fixing
generate-sdk:
    @just show-progress "Generating TypeScript SDK for frontend"
    @just idl-build
    @just copy-idl-to-app
    @just fix-app-idl
    @just show-success "SDK generated and copied to app"

# Copy generated IDL to frontend app directory
[private]
copy-idl-to-app:
    #!/usr/bin/env bash
    if [[ -f "{{IDL_PATH}}" ]]; then
        mkdir -p {{app-dir}}/src/idl
        cp {{IDL_PATH}} {{app-dir}}/src/idl/feels.json
        echo "  IDL copied to app"
    else
        just show-error "IDL not found. Run 'just idl-build' first"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# Fix IDL for browser compatibility
# Removes module paths that cause issues in browser
[private]
fix-app-idl:
    #!/usr/bin/env bash
    if [[ -f "{{app-dir}}/src/lib/fix-idl.js" ]]; then
        node {{app-dir}}/src/lib/fix-idl.js \
            {{app-dir}}/src/idl/feels.json \
            {{app-dir}}/src/idl/feels.json
        echo "  IDL fixed for app usage"
    else
        just show-warning "fix-idl.js not found, skipping IDL fix"
    fi

# ═══════════════════════════════════════════════════════════════════════════
# Metaplex Setup
# ═══════════════════════════════════════════════════════════════════════════

# Download and deploy Metaplex Token Metadata program
# Required for NFT functionality in Feels Protocol
_setup-metaplex:
    #!/usr/bin/env bash
    just show-progress "Setting up Metaplex"
    
    # Configuration
    METAPLEX_PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    METAPLEX_SO_URL="https://github.com/metaplex-foundation/mpl-token-metadata/releases/download/v1.13.3/mpl_token_metadata.so"
    WORK_DIR="{{app-dir}}/scripts"
    
    # Ensure validator is running
    if ! just check-service "{{LOCALNET_RPC}}" "validator"; then
        just show-error "Solana validator not running. Start with 'just validator' first"
        exit {{EXIT_SERVICE_START_FAILURE}}
    fi
    
    cd "$WORK_DIR"
    
    # Download the .so file if not present
    if [[ ! -f "mpl_token_metadata.so" ]]; then
        just show-progress "Downloading Metaplex program"
        if command -v wget &> /dev/null; then
            wget -q "$METAPLEX_SO_URL" -O mpl_token_metadata.so
        elif command -v curl &> /dev/null; then
            curl -sL "$METAPLEX_SO_URL" -o mpl_token_metadata.so
        else
            just show-error "Neither wget nor curl is installed"
            exit {{EXIT_MISSING_DEPS}}
        fi
    fi
    
    # Check if already deployed
    if [[ -n "$IN_NIX_SHELL" ]]; then
        DEPLOYED=$(solana program show "$METAPLEX_PROGRAM_ID" --url "{{LOCALNET_RPC}}" 2>/dev/null | grep -q "Program Id:" && echo "yes" || echo "no")
    else
        DEPLOYED=$(nix develop --quiet -c solana program show "$METAPLEX_PROGRAM_ID" --url "{{LOCALNET_RPC}}" 2>/dev/null | grep -q "Program Id:" && echo "yes" || echo "no")
    fi
    
    if [[ "$DEPLOYED" == "yes" ]]; then
        just show-success "Metaplex already deployed at: $METAPLEX_PROGRAM_ID"
    else
        # Check if we're on macOS
        if [[ "$(uname -s)" == "Darwin" ]]; then
            just show-warning "Metaplex binary deployment not supported on macOS"
            just show-progress "Using mock Metaplex program ID for testing"
            
            # For macOS, we'll just save the configuration without deploying
            # The tests will use a mock implementation
            echo "{" > metaplex-localnet.json
            echo "  \"programId\": \"$METAPLEX_PROGRAM_ID\"," >> metaplex-localnet.json
            echo "  \"deployedAt\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"," >> metaplex-localnet.json
            echo "  \"mock\": true" >> metaplex-localnet.json
            echo "}" >> metaplex-localnet.json
            
            just show-success "Mock Metaplex configuration saved"
        else
            just show-progress "Deploying Metaplex to localnet"
            if [[ -n "$IN_NIX_SHELL" ]]; then
                DEPLOY_CMD="solana program deploy"
            else
                DEPLOY_CMD="nix develop --quiet -c solana program deploy"
            fi
            
            if $DEPLOY_CMD \
                --url "{{LOCALNET_RPC}}" \
                --program-id "$METAPLEX_PROGRAM_ID" \
                mpl_token_metadata.so; then
                just show-success "Metaplex deployed successfully"
                
                # Save configuration
                echo "{" > metaplex-localnet.json
                echo "  \"programId\": \"$METAPLEX_PROGRAM_ID\"," >> metaplex-localnet.json
                echo "  \"deployedAt\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"" >> metaplex-localnet.json
                echo "}" >> metaplex-localnet.json
            else
                just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Failed to deploy Metaplex"
            fi
        fi
    fi