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
        
        solana-test-validator \
            --reset \
            --quiet \
            --rpc-port {{VALIDATOR_RPC_PORT}} \
            --rpc-pubsub-enable-block-subscription \
            --slots-per-epoch 400 \
            > {{LOGS_PATH}}/validator.log 2>&1 &
        
        just wait-for-service "{{LOCALNET_RPC}}" "validator" "30"
    fi

# Setup Metaplex programs (required for NFT functionality)
[private]
setup-metaplex:
    @just frontend::setup-metaplex

# Deploy Feels Protocol programs to localnet
# Builds programs and deploys with proper configuration
[private]
deploy-e2e:
    #!/usr/bin/env bash
    just show-progress "Building programs"
    if ! just build; then
        just exit-with-error {{EXIT_BUILD_FAILURE}} "Build failed"
    fi
    
    just show-progress "Deploying to localnet"
    if ! anchor deploy --provider.cluster localnet; then
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
    fi
    
    # Save program ID
    PROGRAM_ID=$(just get-program-id)
    just save-program-id "$PROGRAM_ID"
    echo "Deployed program: $PROGRAM_ID"
    
    # Initialize protocol
    just initialize-protocol

# Initialize Feels Protocol after deployment
# Sets up oracle, fee parameters, and initial state
[private]
initialize-protocol:
    #!/usr/bin/env bash
    just show-progress "Initializing protocol"
    
    cd {{E2E_PATH}}/streaming-adapter
    export PROGRAM_ID=$(just get-program-id)
    
    # Generate keypair for oracle if needed
    if [[ ! -f oracle-keypair.json ]]; then
        solana-keygen new -o oracle-keypair.json --no-bip39-passphrase --force
    fi
    
    # Run initialization
    if npm run init-protocol; then
        just show-success "Protocol initialized"
    else
        just show-warning "Protocol initialization failed (may already be initialized)"
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
        
        cd {{E2E_PATH}}/streaming-adapter
        
        # Build if needed
        if [[ ! -f target/debug/streaming-adapter ]]; then
            just show-progress "Building streaming adapter"
            cargo build
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
        just show-progress "Building and starting indexer"
        mkdir -p {{LOGS_PATH}}
        
        cd {{PROJECT_ROOT}}/feels-indexer
        
        # Build indexer
        if ! cargo build --release; then
            just exit-with-error {{EXIT_BUILD_FAILURE}} "Indexer build failed"
        fi
        
        export PROGRAM_ID=$(just get-program-id)
        ./target/release/feels-indexer > {{LOGS_PATH}}/indexer.log 2>&1 &
        
        just wait-for-service "http://localhost:{{INDEXER_PORT}}/health" "indexer" "30"
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
            npm install --silent
        fi
        
        # Fix IDL
        if [[ -f "{{IDL_PATH}}" ]]; then
            cp {{IDL_PATH}} src/idl/feels.json
            node src/lib/fix-idl.js src/idl/feels.json src/idl/feels.json
        fi
        
        export NEXT_PUBLIC_PROGRAM_ID=$(just get-program-id)
        npm run dev > {{LOGS_PATH}}/app.log 2>&1 &
        
        just wait-for-service "http://localhost:{{APP_PORT}}" "frontend app" "60"
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