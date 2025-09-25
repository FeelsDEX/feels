# ╔═══════════════════════════════════════════════════════════════════════════╗
# ║               END-TO-END TESTING & ORCHESTRATION MODULE                   ║
# ╚═══════════════════════════════════════════════════════════════════════════╝
#
# This module orchestrates the complete E2E environment for Feels Protocol,
# including validator, streaming adapter, indexer, and frontend app.
#
# Main Commands:
#   just e2e::run     # Start complete E2E environment
#   just e2e::stop    # Stop all E2E services
#   just e2e::status  # Check status of all services
#   just e2e::logs <service>  # View service logs
#
# Services:
#   - Solana Validator (port 8899)
#   - Streaming Adapter (port 8081) 
#   - Feels Indexer (port 8080)
#   - Frontend App (port 3000)
#
# The E2E environment provides a complete local development setup with:
#   - Local Solana validator with deployed programs
#   - Real-time data streaming via Geyser plugin
#   - PostgreSQL + Redis backed indexer
#   - Full frontend application
#
# Import dependencies
import 'common.just'

# ═══════════════════════════════════════════════════════════════════════════
# Main E2E Commands
# ═══════════════════════════════════════════════════════════════════════════

# Run complete E2E environment with all services
# This is the one-stop command to get a full local dev environment
run:
    @just show-progress "Starting complete E2E environment"
    @just validate-e2e
    @just stop  # Clean stop any existing services
    @just start-validator
    @just setup-metaplex
    @just deploy-e2e
    @just start-streaming-adapter
    @just start-indexer
    @just start-app
    @just show-success "E2E environment ready at http://localhost:{{APP_PORT}}"

# Stop all E2E services gracefully
# Ensures clean shutdown of all components
stop:
    @just show-progress "Stopping all E2E services"
    @pkill -f "solana-test-validator" || true
    @pkill -f "streaming-adapter" || true
    @pkill -f "feels-indexer" || true
    @pkill -f "next-server" || true
    @sleep 1
    @just show-success "All services stopped"

# Show status of all E2E services
# Displays running/stopped state and URLs for each service
status:
    @echo "E2E Service Status:"
    @echo "=================="
    @just check-validator-status
    @just check-streaming-status
    @just check-indexer-status
    @just check-app-status

# View logs from E2E services (internal - use 'just e2e-logs' from main justfile)
# Usage: just e2e::_logs [validator|streaming|indexer|app]
_logs service="all":
    #!/usr/bin/env bash
    case "{{service}}" in
        validator)
            just show-validator-logs
            ;;
        streaming|streaming-adapter)
            just show-streaming-logs
            ;;
        indexer)
            just show-indexer-logs
            ;;
        app|frontend)
            just show-app-logs
            ;;
        all)
            echo "Use 'just logs [service]' where service is: validator, streaming, indexer, or app"
            ;;
        *)
            just show-error "Unknown service: {{service}}"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# Service Management
# ═══════════════════════════════════════════════════════════════════════════

# Start Solana validator with E2E configuration
# Includes Geyser plugin support and optimal settings for local dev
[private]
start-validator:
    #!/usr/bin/env bash
    if just check-service "{{LOCALNET_RPC}}" "Solana validator"; then
        just show-warning "Validator already running"
    else
        just show-progress "Starting Solana validator"
        mkdir -p {{LOGS_PATH}}
        
        if [[ -n "$IN_NIX_SHELL" ]]; then
            solana-test-validator \
                --reset \
                --quiet \
                --rpc-port {{VALIDATOR_RPC_PORT}} \
                --rpc-pubsub-enable-block-subscription \
                --slots-per-epoch 400 \
                > {{LOGS_PATH}}/validator.log 2>&1 &
        else
            # Use --quiet to suppress nix shell output
            nix develop --quiet -c solana-test-validator \
                --reset \
                --quiet \
                --rpc-port {{VALIDATOR_RPC_PORT}} \
                --rpc-pubsub-enable-block-subscription \
                --slots-per-epoch 400 \
                > {{LOGS_PATH}}/validator.log 2>&1 &
        fi
        
        just wait-for-service "{{LOCALNET_RPC}}" "validator" "30"
    fi

# Setup Metaplex programs (required for NFT functionality)
[private]
setup-metaplex:
    #!/usr/bin/env bash
    # On macOS, we skip Metaplex binary deployment
    if [[ "$(uname -s)" == "Darwin" ]]; then
        just show-warning "Metaplex binary deployment not supported on macOS"
        just show-progress "Using localnet Metaplex ID: {{MAINNET_METAPLEX_ID}}"
        just show-success "Mock Metaplex configuration ready"
    else
        # On Linux, delegate to the frontend module's setup
        just show-progress "Setting up Metaplex for Linux"
        cd {{FEELS_APP_PATH}}/scripts
        just frontend::_setup-metaplex || true
    fi

# Deploy Feels Protocol programs to localnet
# Builds programs and deploys with proper configuration
[private]
deploy-e2e:
    #!/usr/bin/env bash
    # Check if programs are already built
    if [[ -f "{{DEPLOY_PATH}}/feels.so" ]]; then
        just show-progress "Using existing program binary"
    else
        just show-progress "Building programs"
        # Use nix BPF builder to avoid stack size issues
        if ! just nix-build; then
            just show-warning "Nix BPF build failed, trying regular build"
            if ! just build; then
                just exit-with-error {{EXIT_BUILD_FAILURE}} "Build failed"
            fi
        fi
    fi
    
    just show-progress "Deploying to localnet"
    if [[ -n "$IN_NIX_SHELL" ]]; then
        if ! anchor deploy --provider.cluster localnet; then
            just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
        fi
    else
        if ! nix develop --quiet -c anchor deploy --provider.cluster localnet; then
            just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
        fi
    fi
    
    # Save program ID using the get-program-id from common.just
    PROGRAM_ID=$(just get-program-id)
    just save-program-id "$PROGRAM_ID"
    echo "Deployed program: $PROGRAM_ID"
    
    # Wait a moment for program to be available on-chain
    sleep 2
    
    # Initialize protocol (skip for now due to IDL version issues)
    # just initialize-protocol
    echo "Skipping protocol initialization for now (IDL version mismatch)"

# Initialize Feels Protocol after deployment
# Sets up oracle, fee parameters, and initial state
[private]
initialize-protocol:
    #!/usr/bin/env bash
    just show-progress "Initializing protocol"
    
    cd {{FEELS_APP_PATH}}/scripts
    
    # Export program ID for the initialization script
    export FEELS_PROGRAM_ID=$(just get-program-id)
    
    # Run initialization
    if node initialize-protocol.js; then
        just show-success "Protocol initialized"
    else
        echo -e "{{YELLOW}}[WARNING] Protocol initialization failed (may already be initialized){{NC}}"
    fi

# Start streaming adapter for real-time data
# Connects to validator's Geyser plugin and streams updates
[private]
start-streaming-adapter:
    #!/usr/bin/env bash
    if pgrep -f "streaming-adapter" > /dev/null; then
        just show-warning "Streaming adapter already running"
    else
        just show-progress "Starting streaming adapter"
        mkdir -p {{LOGS_PATH}}
        
        # Skip streaming adapter if directory doesn't exist
        if [[ ! -d "{{E2E_PATH}}/streaming-adapter" ]]; then
            just show-warning "Streaming adapter directory not found, skipping"
            return 0
        fi
        
        cd {{E2E_PATH}}/streaming-adapter
        
        # Build if needed
        if [[ ! -f target/debug/streaming-adapter ]]; then
            just show-progress "Building streaming adapter"
            if [[ -n "$IN_NIX_SHELL" ]]; then
                cargo build
            else
                nix develop --quiet -c cargo build
            fi
        fi
        
        export PROGRAM_ID=$(just get-program-id)
        ./target/debug/streaming-adapter \
            --rpc-url http://localhost:{{VALIDATOR_RPC_PORT}} \
            --port {{STREAMING_PORT}} \
            --program-id "$PROGRAM_ID" \
            > {{LOGS_PATH}}/streaming-adapter.log 2>&1 &
        
        just show-success "Streaming adapter started on port {{STREAMING_PORT}}"
    fi

# Start Feels indexer service
# Processes blockchain data and provides REST API
[private]
start-indexer:
    #!/usr/bin/env bash
    if just check-service "http://localhost:{{INDEXER_PORT}}/health" "indexer"; then
        just show-warning "Indexer already running"
    else
        # Find the indexer directory - it might be at PROJECT_ROOT or one level up
        INDEXER_DIR=""
        if [[ -d "{{PROJECT_ROOT}}/feels-indexer" ]]; then
            INDEXER_DIR="{{PROJECT_ROOT}}/feels-indexer"
        elif [[ -d "{{PROJECT_ROOT}}/../feels-indexer" ]]; then
            INDEXER_DIR="{{PROJECT_ROOT}}/../feels-indexer"
        elif [[ -d "./feels-indexer" ]]; then
            INDEXER_DIR="./feels-indexer"
        elif [[ -d "../feels-indexer" ]]; then
            INDEXER_DIR="../feels-indexer"
        fi
        
        if [[ -z "$INDEXER_DIR" ]]; then
            just show-warning "Indexer directory not found, skipping"
            return 0
        fi
        
        just show-progress "Found indexer at: $INDEXER_DIR"
        
        just show-progress "Building and starting indexer"
        mkdir -p {{LOGS_PATH}}
        
        cd "$INDEXER_DIR"
        
        # Check if indexer binary exists first
        if [[ -f ./target/release/feels-indexer ]]; then
            just show-progress "Using existing indexer binary"
        else
            # Build indexer in its own isolated nix environment
            just show-progress "Building indexer in isolated environment"
            if [[ -n "$IN_NIX_SHELL" ]]; then
                # We're already in a nix shell, need to exit and enter the indexer one
                if ! nix develop --quiet ..#indexer -c cargo build --release 2>&1; then
                    just show-warning "Indexer build failed - checking for binary"
                    if [[ ! -f ./target/release/feels-indexer ]]; then
                        just show-warning "No indexer binary found - indexer will be skipped"
                        return 0
                    fi
                fi
            else
                # Not in nix shell, can directly use the indexer environment
                if ! nix develop --quiet {{PROJECT_ROOT}}#indexer -c cargo build --release 2>&1; then
                    just show-warning "Indexer build failed - checking for binary"
                    if [[ ! -f ./target/release/feels-indexer ]]; then
                        just show-warning "No indexer binary found - indexer will be skipped"
                        return 0
                    fi
                fi
            fi
        fi
        
        # Get the actual program ID
        PROGRAM_ID=$(just get-program-id)
        just show-progress "Using program ID: $PROGRAM_ID"
        
        # Create a temporary config file with the correct program ID
        if [[ -f indexer-e2e.toml ]]; then
            # Update the program ID in the e2e config
            sed "s/program_id = .*/program_id = \"$PROGRAM_ID\"/" indexer-e2e.toml > indexer-e2e-temp.toml
            CONFIG_FILE="indexer-e2e-temp.toml"
        else
            # Use default config
            CONFIG_FILE="indexer.toml"
        fi
        
        # Ensure database services are running
        just show-progress "Checking database services"
        if ! just check-service "localhost:5432" "PostgreSQL"; then
            just show-progress "Starting database services"
            just services-start || true
        fi
        
        # Clean up existing data if present to avoid "Index already exists" error
        if [[ -d "./data" ]]; then
            just show-progress "Cleaning existing indexer data"
            rm -rf ./data/rocksdb ./data/tantivy
        fi
        
        # Run indexer in its isolated environment
        if [[ -n "$IN_NIX_SHELL" ]]; then
            nix develop --quiet ..#indexer -c ./target/release/feels-indexer --config "$CONFIG_FILE" > {{LOGS_PATH}}/indexer.log 2>&1 &
        else
            nix develop --quiet {{PROJECT_ROOT}}#indexer -c ./target/release/feels-indexer --config "$CONFIG_FILE" > {{LOGS_PATH}}/indexer.log 2>&1 &
        fi
        
        just wait-for-service "http://localhost:{{INDEXER_PORT}}/health" "indexer" "30"
        
        # Clean up temp file after indexer starts
        sleep 1
        rm -f indexer-e2e-temp.toml
    fi

# Start Next.js frontend application
# Connects to indexer and provides trading UI
[private]
start-app:
    #!/usr/bin/env bash
    if just check-service "http://localhost:{{APP_PORT}}" "frontend app"; then
        just show-warning "Frontend app already running"
    else
        just show-progress "Starting frontend app"
        mkdir -p {{LOGS_PATH}}
        
        cd {{FEELS_APP_PATH}}
        
        # Ensure dependencies
        if [[ ! -d "node_modules" ]]; then
            just show-progress "Installing app dependencies"
            if [[ -n "$IN_NIX_SHELL" ]]; then
                npm install --silent
            else
                nix develop {{PROJECT_ROOT}}#frontend --quiet -c npm install --silent
            fi
        fi
        
        # Copy IDL
        if [[ -f "{{IDL_PATH}}" ]]; then
            cp {{IDL_PATH}} src/idl/feels.json
        fi
        
        # Get program ID without using just (to avoid context issues)
        if [[ -f "{{DEPLOY_PATH}}/feels-keypair.json" ]]; then
            if [[ -n "$IN_NIX_SHELL" ]]; then
                export NEXT_PUBLIC_PROGRAM_ID=$(solana-keygen pubkey "{{DEPLOY_PATH}}/feels-keypair.json" 2>/dev/null)
            else
                export NEXT_PUBLIC_PROGRAM_ID=$(nix develop --quiet -c solana-keygen pubkey "{{DEPLOY_PATH}}/feels-keypair.json" 2>/dev/null)
            fi
        else
            export NEXT_PUBLIC_PROGRAM_ID={{DEFAULT_PROGRAM_ID}}
        fi
        
        if [[ -n "$IN_NIX_SHELL" ]]; then
            npm run dev > {{LOGS_PATH}}/app.log 2>&1 &
        else
            nix develop {{PROJECT_ROOT}}#frontend --quiet -c npm run dev > {{LOGS_PATH}}/app.log 2>&1 &
        fi
        
        # Wait for frontend to start
        echo "Waiting for frontend app to start..."
        for i in {1..60}; do
            if curl -s http://localhost:{{APP_PORT}} > /dev/null; then
                just show-success "Frontend app started on port {{APP_PORT}}"
                break
            fi
            sleep 1
        done
    fi

# ═══════════════════════════════════════════════════════════════════════════
# Status Checks
# ═══════════════════════════════════════════════════════════════════════════

[private]
check-validator-status:
    #!/usr/bin/env bash
    echo -n "  Validator: "
    if just check-service "{{LOCALNET_RPC}}" "validator"; then
        echo "[RUNNING] http://localhost:{{VALIDATOR_RPC_PORT}}"
    else
        echo "[STOPPED]"
    fi

[private]
check-streaming-status:
    #!/usr/bin/env bash
    echo -n "  Streaming Adapter: "
    if pgrep -f "streaming-adapter" > /dev/null; then
        echo "[RUNNING] gRPC server on port {{STREAMING_PORT}}"
    else
        echo "[STOPPED]"
    fi

[private]
check-indexer-status:
    #!/usr/bin/env bash
    echo -n "  Indexer: "
    if just check-service "http://localhost:{{INDEXER_PORT}}/health" "indexer"; then
        echo "[RUNNING] http://localhost:{{INDEXER_PORT}}"
    else
        echo "[STOPPED]"
    fi

[private]
check-app-status:
    #!/usr/bin/env bash
    echo -n "  Frontend App: "
    if just check-service "http://localhost:{{APP_PORT}}" "app"; then
        echo "[RUNNING] http://localhost:{{APP_PORT}}"
    else
        echo "[STOPPED]"
    fi

# ═══════════════════════════════════════════════════════════════════════════
# Log Viewers
# ═══════════════════════════════════════════════════════════════════════════

[private]
show-validator-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/validator.log ]]; then
        tail -f {{LOGS_PATH}}/validator.log
    else
        just show-error "No validator logs found"
    fi

[private]
show-streaming-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/streaming-adapter.log ]]; then
        tail -f {{LOGS_PATH}}/streaming-adapter.log
    else
        just show-error "No streaming adapter logs found"
    fi

[private]
show-indexer-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/indexer.log ]]; then
        tail -f {{LOGS_PATH}}/indexer.log
    else
        just show-error "No indexer logs found"
    fi

[private]
show-app-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/app.log ]]; then
        tail -f {{LOGS_PATH}}/app.log
    else
        just show-error "No app logs found"
    fi