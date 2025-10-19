# End-to-end environment orchestration

# Import dependencies
import 'common.just'

# ===== Main E2E Commands =====

# Start complete E2E environment
run:
    @just show-progress "Starting complete E2E environment"
    @just stop
    @just _e2e-start-validator
    @just setup-metaplex
    @just _deploy-and-init
    @just _start-indexer
    @just _start-app
    @just show-success "E2E environment ready at http://localhost:{{APP_PORT}}"

# Stop all E2E services
stop:
    @just show-progress "Stopping all E2E services"
    @pkill -f "solana-test-validator" || true
    @pkill -f "feels-indexer" || true
    @pkill -f "next-server" || true
    @sleep 1
    @just show-success "All services stopped"

# Show service status
status:
    @echo "E2E Service Status:"
    @echo "=================="
    @just _check-validator-status
    @just _check-indexer-status
    @just _check-app-status

# View service logs
logs service="all":
    #!/usr/bin/env bash
    case "{{service}}" in
        validator)
            just _show-validator-logs
            ;;
        indexer)
            just _show-indexer-logs
            ;;
        app|frontend)
            just _show-app-logs
            ;;
        all)
            echo "Use 'just logs [service]' where service is: validator, indexer, or app"
            ;;
        *)
            just show-error "Unknown service: {{service}}"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# Verify protocol setup
verify:
    #!/usr/bin/env bash
    PROGRAM_ID=$(just get-program-id)
    just show-progress "Verifying protocol setup"
    echo "Program ID: $PROGRAM_ID"
    
    # Check protocol config (simplified check)
    if solana account $PROGRAM_ID --url {{LOCALNET_RPC}} >/dev/null 2>&1; then
        echo "[SUCCESS] Program account exists"
    else
        echo "[ERROR] Program account not found"
    fi
    
    # Check indexer
    if curl -s http://localhost:{{INDEXER_PORT}}/health >/dev/null 2>&1; then
        echo "[SUCCESS] Indexer health check passed"
    else
        echo "[ERROR] Indexer health check failed"
    fi

# ===== Service Management =====

# Start Solana validator
[private]
_e2e-start-validator:
    #!/usr/bin/env bash
    if just check-service "{{LOCALNET_RPC}}" "Solana validator"; then
        just show-warning "Validator already running"
    else
        just show-progress "Starting Solana validator"
        mkdir -p {{LOGS_PATH}}
        just nix-cmd solana-test-validator \
            --reset \
            --quiet \
            --rpc-port {{VALIDATOR_RPC_PORT}} \
            > {{LOGS_PATH}}/validator.log 2>&1 &
        just wait-for-service "{{LOCALNET_RPC}}" "validator" "30"
    fi

# Deploy and initialize protocol
[private]
_deploy-and-init:
    #!/usr/bin/env bash
    just show-progress "Deploying to localnet"
    if ! just nix-cmd anchor deploy --provider.cluster localnet; then
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
    fi
    
    PROGRAM_ID=$(just get-program-id)
    just save-program-id "$PROGRAM_ID"
    echo "Deployed program: $PROGRAM_ID"
    
    # Initialize protocol
    just show-progress "Initializing protocol"
    feels init protocol \
        --treasury "$PROGRAM_ID" \
        --base-fee-bps 30 \
        --program-id "$PROGRAM_ID" \
        --rpc-url "{{LOCALNET_RPC}}" || true

# Start indexer service
[private]
_start-indexer:
    #!/usr/bin/env bash
    if just check-service "http://localhost:{{INDEXER_PORT}}/health" "indexer"; then
        just show-warning "Indexer already running"
    else
        INDEXER_DIR="{{PROJECT_ROOT}}/feels-indexer"
        if [[ ! -d "$INDEXER_DIR" ]]; then
            just show-warning "Indexer directory not found, skipping"
            return 0
        fi
        
        just show-progress "Starting indexer"
        mkdir -p {{LOGS_PATH}}
        cd "$INDEXER_DIR"
        
        # Build if needed
        if [[ ! -f ./target/release/feels-indexer ]]; then
            just show-progress "Building indexer"
            nix develop --quiet {{PROJECT_ROOT}}#indexer -c cargo build --release || return 0
        fi
        
        # Start with basic config
        nix develop --quiet {{PROJECT_ROOT}}#indexer -c ./target/release/feels-indexer \
            > {{LOGS_PATH}}/indexer.log 2>&1 &
        
        just wait-for-service "http://localhost:{{INDEXER_PORT}}/health" "indexer" "30"
    fi

# Start frontend application
[private]
_start-app:
    #!/usr/bin/env bash
    if just check-service "http://localhost:{{APP_PORT}}" "frontend app"; then
        just show-warning "Frontend app already running"
    else
        just show-progress "Starting frontend app"
        mkdir -p {{LOGS_PATH}}
        cd {{FEELS_APP_PATH}}
        
        # Install deps if needed
        if [[ ! -d "node_modules" ]]; then
            just nix-shell-cmd frontend npm install --silent
        fi
        
        # Copy IDL
        if [[ -f "{{IDL_PATH}}" ]]; then
            cp {{IDL_PATH}} src/idl/feels.json
        fi
        
        # Start app
        just nix-shell-cmd frontend npm run dev > {{LOGS_PATH}}/app.log 2>&1 &
        
        # Wait for startup
        for i in {1..30}; do
            if curl -s http://localhost:{{APP_PORT}} > /dev/null; then
                just show-success "Frontend app started on port {{APP_PORT}}"
                break
            fi
            sleep 1
        done
    fi

# ===== Status Checks =====

[private]
_check-validator-status:
    #!/usr/bin/env bash
    echo -n "  Validator: "
    if just check-service "{{LOCALNET_RPC}}" "validator" 2>/dev/null; then
        echo "[RUNNING] http://localhost:{{VALIDATOR_RPC_PORT}}"
    else
        echo "[STOPPED]"
    fi

[private]
_check-indexer-status:
    #!/usr/bin/env bash
    echo -n "  Indexer: "
    if just check-service "http://localhost:{{INDEXER_PORT}}/health" "indexer" 2>/dev/null; then
        echo "[RUNNING] http://localhost:{{INDEXER_PORT}}"
    else
        echo "[STOPPED]"
    fi

[private]
_check-app-status:
    #!/usr/bin/env bash
    echo -n "  Frontend App: "
    if just check-service "http://localhost:{{APP_PORT}}" "app" 2>/dev/null; then
        echo "[RUNNING] http://localhost:{{APP_PORT}}"
    else
        echo "[STOPPED]"
    fi

# ===== Log Viewers =====

[private]
_show-validator-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/validator.log ]]; then
        echo "Validator logs (press Ctrl+C to exit):"
        echo "======================================"
        tail -f {{LOGS_PATH}}/validator.log 2>/dev/null
    else
        just show-error "No validator logs found at {{LOGS_PATH}}/validator.log"
        echo "Start the validator first with: just validator"
    fi

[private]
_show-indexer-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/indexer.log ]]; then
        echo "Indexer logs (press Ctrl+C to exit):"
        echo "====================================="
        tail -f {{LOGS_PATH}}/indexer.log 2>/dev/null
    else
        just show-error "No indexer logs found at {{LOGS_PATH}}/indexer.log"
        echo "Start the indexer first with: just run"
    fi

[private]
_show-app-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/app.log ]]; then
        echo "Frontend app logs (press Ctrl+C to exit):"
        echo "=========================================="
        tail -f {{LOGS_PATH}}/app.log 2>/dev/null
    else
        just show-error "No app logs found at {{LOGS_PATH}}/app.log"
        echo "Start the frontend first with: just run"
    fi
