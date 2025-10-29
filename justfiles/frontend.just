# Frontend application module

# Import dependencies
import 'common.just'

# Frontend app directory
app-dir := FEELS_APP_PATH

# ===== Main Frontend Commands =====

# Show available frontend commands (default)
_default:
    @echo ""
    @echo "╔═══════════════════════════════════════════════════════════════╗"
    @echo "║           FEELS PROTOCOL - FRONTEND COMMANDS                  ║"
    @echo "╚═══════════════════════════════════════════════════════════════╝"
    @echo ""
    @echo "Development:"
    @echo "  just frontend install        Install Node.js dependencies"
    @echo "  just frontend dev            Start dev server with automatic page pre-compilation"
    @echo "  just frontend dev-simple     Start dev server (no pre-compilation)"
    @echo "  just frontend dev-test       Start dev server in test data mode with pre-compilation"
    @echo "  just frontend build          Build for production"
    @echo ""
    @echo "Code Quality:"
    @echo "  just frontend lint           Run linter"
    @echo "  just frontend test           Run tests"
    @echo "  just frontend clean          Clean build artifacts"
    @echo "  just frontend clean-all      Deep clean (including node_modules)"
    @echo ""
    @echo "SDK & Tools:"
    @echo "  just frontend generate-sdk   Generate TypeScript SDK from IDL"
    @echo "  just frontend generate-wasm  Build WASM vanity miner"
    @echo ""
    @echo "Deployment (Feelsbox):"
    @echo "  just frontend deploy         Build and deploy to feelsbox server"
    @echo "  just frontend deploy-sync    Sync built files to feelsbox (no build)"
    @echo "  just frontend deploy-restart Restart Next.js on feelsbox"
    @echo "  just frontend deploy-status  Check deployment status on feelsbox"
    @echo "  just frontend deploy-logs    Tail logs from feelsbox"
    @echo ""
    @echo "For more details: just --list frontend"
    @echo ""

# Install Node.js dependencies
install:
    @just show-progress "Installing Next.js dependencies"
    @cd {{app-dir}} && npm install

# Start development server with pre-compilation
dev:
    #!/usr/bin/env bash
    set -e
    
    # Ensure WASM artifacts are built
    wasm_file="{{app-dir}}/public/wasm/vanity_miner_wasm_bg.wasm"
    if [[ ! -f "$wasm_file" ]]; then
        echo "  [!] WASM artifacts not found, building now..."
        cd vanity-miner-wasm && just build
        cd ..
    else
        echo "  [OK] WASM artifacts already exist"
    fi
    
    echo "[...] Starting Next.js development server with page pre-compilation..."
    echo ""
    cd {{app-dir}}
    
    # Guard to prevent multiple cleanup executions
    CLEANUP_DONE=false
    cleanup() {
        if [ "$CLEANUP_DONE" = "true" ]; then
            return
        fi
        CLEANUP_DONE=true
        echo ""
        echo "  Shutting down dev server..."
        kill 0 2>/dev/null || true
        rm -f .dev-server.pid
        exit 0
    }
    trap cleanup SIGINT SIGTERM EXIT
    
    npm run dev:watch &
    DEV_PID=$!
    echo $DEV_PID > .dev-server.pid
    echo ""
    echo "  Waiting for dev server to start..."
    
    # Wait longer and verify server is fully ready with HTML content
    READY=false
    for i in {1..60}; do
      RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000 2>/dev/null || echo "000")
      if [ "$RESPONSE" = "200" ]; then
        # Server responded, wait a bit more to ensure webpack is ready
        sleep 3
        READY=true
        echo "  ✓ Dev server ready at http://localhost:3000"
        break
      fi
      sleep 1
    done
    
    if [ "$READY" = "false" ]; then
      echo "  ⚠ Dev server took longer than expected to start"
    fi
    
    echo ""
    echo "════════════════════════════════════════════════════════════════"
    echo "  Dev server running. File changes will trigger hot reload."
    echo "  Press Ctrl+C to stop."
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    
    # Pre-compile pages in background after server is stable
    if [ "$READY" = "true" ]; then
      ( 
        sleep 2
        echo ""
        echo "  Starting background page pre-compilation..."
        cd {{app-dir}} && npm run dev:precompile
      ) &
    fi
    
    wait $DEV_PID

# Start development server (original, no pre-compilation)
dev-simple:
    #!/usr/bin/env bash
    # Ensure WASM artifacts are built
    wasm_file="{{app-dir}}/public/wasm/vanity_miner_wasm_bg.wasm"
    if [[ ! -f "$wasm_file" ]]; then
        echo "  [!] WASM artifacts not found, building now..."
        cd vanity-miner-wasm && just build
        cd ..
    else
        echo "  [OK] WASM artifacts already exist"
    fi
    just show-progress "Starting Next.js development server"
    cd {{app-dir}} && npm run dev:watch

# Start development server in test data mode (uses predefined development keypair)
dev-test:
    #!/usr/bin/env bash
    # No need to build WASM artifacts in test mode since we use predefined keypair
    echo "  [INFO] Starting in test data mode - using predefined development keypair"
    echo "  [INFO] Development keypair: tRfecbDu1OqMfcjEaR49esSFbLFEEL"
    echo ""
    
    # Guard to prevent multiple cleanup executions
    CLEANUP_DONE=false
    cleanup() {
        if [ "$CLEANUP_DONE" = "true" ]; then
            return
        fi
        CLEANUP_DONE=true
        echo ""
        echo "  Shutting down test mode dev server..."
        kill 0 2>/dev/null || true
        rm -f {{app-dir}}/.dev-server.pid
        exit 0
    }
    trap cleanup SIGINT SIGTERM EXIT
    
    cd {{app-dir}}
    echo "[...] Starting Next.js development server in TEST DATA MODE..."
    echo ""
    
    # Set environment variables to force test data mode
    NEXT_PUBLIC_USE_INDEXER=false \
    NEXT_PUBLIC_INDEXER_URL="" \
    npm run dev:watch &
    DEV_PID=$!
    echo $DEV_PID > .dev-server.pid
    
    echo ""
    echo "  Waiting for dev server to start..."
    
    # Wait and verify server is ready
    READY=false
    for i in {1..60}; do
      RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000 2>/dev/null || echo "000")
      if [ "$RESPONSE" = "200" ]; then
        sleep 3
        READY=true
        echo "  ✓ Test mode dev server ready at http://localhost:3000"
        break
      fi
      sleep 1
    done
    
    if [ "$READY" = "false" ]; then
      echo "  ⚠ Dev server took longer than expected to start"
    fi
    
    echo ""
    echo "════════════════════════════════════════════════════════════════"
    echo "  🧪 TEST DATA MODE ACTIVE"
    echo "  • Using predefined development keypair: tRfecbDu1OqMfcjEaR49esSFbLFEEL"
    echo "  • No vanity mining - instant keypair availability"
    echo "  • All data sourced from test fixtures"
    echo "  • Press Ctrl+C to stop"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    
    # Pre-compile pages in background after server is stable (same as dev mode)
    if [ "$READY" = "true" ]; then
      ( 
        sleep 2
        echo ""
        echo "  Starting background page pre-compilation..."
        cd {{app-dir}} && npm run dev:precompile
      ) &
    fi
    
    wait $DEV_PID

# Build for production
build:
    #!/usr/bin/env bash
    # Ensure WASM artifacts are built
    wasm_file="{{app-dir}}/public/wasm/vanity_miner_wasm_bg.wasm"
    if [[ ! -f "$wasm_file" ]]; then
        echo "  [!] WASM artifacts not found, building now..."
        cd vanity-miner-wasm && just build
        cd ..
    else
        echo "  [OK] WASM artifacts already exist"
    fi
    just show-progress "Building Next.js app for production"
    cd {{app-dir}} && npm run build
    just show-success "Production build complete"

# Run linter
lint:
    @just show-progress "Linting Next.js application"
    @cd {{app-dir}} && npm run lint

# Run tests
test:
    @just show-progress "Running Next.js app tests"
    @cd {{app-dir}} && npm test

# Clean build artifacts
clean:
    @just show-progress "Cleaning Next.js build artifacts"
    @cd {{app-dir}} && rm -rf .next out node_modules/.cache
    @just show-success "Build artifacts cleaned"

# Clean everything including dependencies (use when having issues)
clean-all:
    @just show-progress "Deep cleaning Next.js (including node_modules)"
    @cd {{app-dir}} && rm -rf .next out node_modules node_modules/.cache .dev-server.pid
    @just show-success "Deep clean complete - run 'just frontend install' to reinstall"

# Generate TypeScript SDK from IDL
generate-sdk:
    @just show-progress "Preparing TypeScript SDK for frontend"
    @just _ensure-idl-generated
    @just _copy-idl-to-app
    @just _fix-app-idl
    @just show-success "SDK ready for frontend use"

# Build WASM vanity miner with parallel features
generate-wasm:
    @just show-progress "Building WASM vanity miner"
    @cd vanity-miner-wasm && just build
    @just show-success "WASM artifacts built and integrated into frontend"

# ===== Deployment Commands =====

# Build and deploy to feelsbox production server
deploy:
    #!/usr/bin/env bash
    set -e
    
    # FEELSBOX variable is automatically loaded from .env via justfile's dotenv-load
    if [[ -z "$FEELSBOX" ]]; then
        echo "Error: FEELSBOX variable not set in .env file"
        echo "Copy .env.example to .env and set FEELSBOX=128.199.58.200"
        exit 1
    fi
    
    just show-progress "Building frontend for production"
    # Build the frontend locally using the build command
    just frontend build
    
    just show-progress "Syncing files to feelsbox ($FEELSBOX)"
    # Sync entire application directory to server, excluding node_modules and cache
    rsync -avz --delete \
      --exclude 'node_modules' \
      --exclude '.next/cache' \
      --exclude '.git' \
      -e 'ssh -i ~/.ssh/id_rsa' \
      {{app-dir}}/ root@$FEELSBOX:/opt/feels-app/
    
    just show-progress "Restarting Next.js on feelsbox"
    
    # Kill existing process and restart (use bash heredoc to avoid set -e issues)
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX bash << 'EOFRESTART'
    pkill -f "next-server" || true
    cd /opt/feels-app
    nohup npm start > /var/log/feels-app.log 2>&1 &
    echo $! > /opt/feels-app/feels-app.pid
    EOFRESTART
    
    # Wait a moment for the server to start
    sleep 3
    
    just show-success "Deployment complete! Check status with: just frontend deploy-status"

# Sync built files to feelsbox (without rebuilding)
deploy-sync:
    #!/usr/bin/env bash
    set -e
    
    # FEELSBOX variable is automatically loaded from .env via justfile's dotenv-load
    if [[ -z "$FEELSBOX" ]]; then
        echo "Error: FEELSBOX variable not set in .env file"
        echo "Copy .env.example to .env and set FEELSBOX=128.199.58.200"
        exit 1
    fi
    
    just show-progress "Syncing files to feelsbox ($FEELSBOX)"
    # Sync entire application directory to server
    rsync -avz --delete \
      --exclude 'node_modules' \
      --exclude '.next/cache' \
      --exclude '.git' \
      -e 'ssh -i ~/.ssh/id_rsa' \
      {{app-dir}}/ root@$FEELSBOX:/opt/feels-app/
    
    just show-success "Sync complete!"

# Restart Next.js service on feelsbox
deploy-restart:
    #!/usr/bin/env bash
    set -e
    
    # FEELSBOX variable is automatically loaded from .env via justfile's dotenv-load
    if [[ -z "$FEELSBOX" ]]; then
        echo "Error: FEELSBOX variable not set in .env file"
        echo "Copy .env.example to .env and set FEELSBOX=128.199.58.200"
        exit 1
    fi
    
    just show-progress "Restarting Next.js on feelsbox ($FEELSBOX)"
    
    # Kill existing Next.js process (use bash heredoc to avoid set -e issues)
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX bash << 'EOFKILL'
    pkill -f "next-server" || true
    EOFKILL
    
    # Wait for process to fully stop
    sleep 2
    
    # Start new process in background with logging
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX bash << 'EOFSTART'
    cd /opt/feels-app
    nohup npm start > /var/log/feels-app.log 2>&1 &
    echo $! > /opt/feels-app/feels-app.pid
    EOFSTART
    
    # Wait for server to start
    sleep 3
    
    just show-success "Next.js restarted! Check logs with: just frontend deploy-logs"

# Check deployment status
deploy-status:
    #!/usr/bin/env bash
    # FEELSBOX variable is automatically loaded from .env via justfile's dotenv-load
    if [[ -z "$FEELSBOX" ]]; then
        echo "Error: FEELSBOX variable not set in .env file"
        echo "Copy .env.example to .env and set FEELSBOX=128.199.58.200"
        exit 1
    fi
    
    echo ""
    echo "════════════════════════════════════════════════════════════════"
    echo "  Feelsbox Deployment Status"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    
    # Check if Next.js is running
    echo "Next.js Process:"
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX 'ps aux | grep "next-server" | grep -v grep' || echo "  ⚠ Not running"
    
    echo ""
    echo "Port 3000 Status:"
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX 'ss -tulpn | grep :3000' || echo "  ⚠ Port not in use"
    
    echo ""
    echo "Recent Logs (last 10 lines):"
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX 'tail -10 /var/log/feels-app.log 2>/dev/null' || echo "  ⚠ No logs found"
    
    echo ""
    echo "Production URL: https://feels.timewave.computer"
    echo "Direct Access: http://$FEELSBOX:3000"
    echo ""

# View deployment logs
deploy-logs:
    #!/usr/bin/env bash
    # FEELSBOX variable is automatically loaded from .env via justfile's dotenv-load
    if [[ -z "$FEELSBOX" ]]; then
        echo "Error: FEELSBOX variable not set in .env file"
        echo "Copy .env.example to .env and set FEELSBOX=128.199.58.200"
        exit 1
    fi
    
    echo "Tailing logs from feelsbox (Ctrl+C to stop)..."
    ssh -i ~/.ssh/id_rsa root@$FEELSBOX 'tail -f /var/log/feels-app.log'

# ===== Private Helper Functions =====

# Ensure IDL is generated (run canonical command if needed)
[private]
_ensure-idl-generated:
    #!/usr/bin/env bash
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-progress "IDL not found, generating via canonical command"
        just build idl
    else
        echo "  [OK] IDL already exists, using existing version"
    fi

# Copy generated IDL to frontend app directory
[private]
_copy-idl-to-app:
    #!/usr/bin/env bash
    if [[ -f "{{IDL_PATH}}" ]]; then
        mkdir -p {{app-dir}}/src/idl
        cp {{IDL_PATH}} {{app-dir}}/src/idl/feels.json
        echo "  IDL copied to app"
    else
        just show-error "IDL not found. Run 'just build idl' first"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# Fix IDL for browser compatibility
[private]
_fix-app-idl:
    #!/usr/bin/env bash
    if [[ -f "{{app-dir}}/src/lib/fix-idl.js" ]]; then
        node {{app-dir}}/src/lib/fix-idl.js \
            {{app-dir}}/src/idl/feels.json \
            {{app-dir}}/src/idl/feels.json
        echo "  IDL fixed for app usage"
    else
        just show-warning "fix-idl.js not found, skipping IDL fix"
    fi