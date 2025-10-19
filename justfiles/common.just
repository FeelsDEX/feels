# Common utility functions used across all modules

# Import configuration and constants
import 'nix-env.just'

# ===== Shell Configuration =====
set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true
set export := true

# ===== OS Detection =====
is-macos := if os() == "macos" { "true" } else { "false" }
is-linux := if os() == "linux" { "true" } else { "false" }  
is-windows := if os() == "windows" { "true" } else { "false" }

# ===== Nix Environment Helpers =====
# Run command with or without nix develop based on environment
nix-run command:
    #!/usr/bin/env bash
    if [[ -n "$IN_NIX_SHELL" ]]; then
        {{command}}
    else
        nix develop -c {{command}}
    fi

# Execute command with args in appropriate environment
# Usage: just nix-cmd cargo build --release
# Usage: just nix-cmd anchor deploy --provider.cluster localnet
[private]
nix-cmd cmd *args:
    #!/usr/bin/env bash
    if [[ -n "$IN_NIX_SHELL" ]]; then
        {{cmd}} {{args}}
    else
        nix develop --quiet -c {{cmd}} {{args}}
    fi

# Execute command in specific nix shell (e.g., frontend, indexer)
# Usage: just nix-shell-cmd frontend npm install
[private]
nix-shell-cmd shell cmd *args:
    #!/usr/bin/env bash
    nix develop --quiet {{PROJECT_ROOT}}#{{shell}} -c {{cmd}} {{args}}

# ===== Progress Indicators =====
[private]
@show-progress message:
    echo -e "{{YELLOW}}[...] {{message}}...{{NC}}"

[private]
@show-building:
    echo -e "{{YELLOW}}[BUILDING] Compiling programs...{{NC}}"

[private]
@show-deploying:
    echo -e "{{YELLOW}}[DEPLOYING] Deploying to network...{{NC}}"

[private]
@show-success message:
    echo -e "{{GREEN}}[OK] {{message}}{{NC}}"

[private]
@show-error message:
    echo -e "{{RED}}[ERROR] {{message}}{{NC}}" >&2

[private]
@show-warning message:
    echo -e "{{YELLOW}}[WARNING] {{message}}{{NC}}"

# ===== Error Handling =====
[private]
exit-with-error code message:
    @just show-error "{{message}}"
    @exit {{code}}

# ===== Validator Management =====
[private]
ensure-validator:
    #!/usr/bin/env bash
    # Check if validator is already running
    if solana cluster-version >/dev/null 2>&1; then
        just show-success "Validator already running"
    else
        just show-progress "Starting local validator"
        # Start validator in background
        solana-test-validator \
            --quiet \
            --reset \
            > {{LOGS_PATH}}/validator.log 2>&1 &
        
        # Wait for validator to start
        timeout=30
        while ! solana cluster-version >/dev/null 2>&1 && [ $timeout -gt 0 ]; do
            sleep 1
            ((timeout--))
        done
        
        if [ $timeout -eq 0 ]; then
            just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "Failed to start validator"
        else
            just show-success "Validator started"
        fi
    fi

# ===== Command Checks =====
[private]
check-command cmd name="":
    #!/usr/bin/env bash
    # Check command availability in Nix environment if not already in shell
    if [[ -n "$IN_NIX_SHELL" ]]; then
        if ! command -v {{cmd}} &> /dev/null; then
            just show-error "{{ if name != "" { name } else { cmd } }} is not installed"
            exit {{EXIT_MISSING_DEPS}}
        fi
    else
        if ! nix develop -c command -v {{cmd}} &> /dev/null; then
            just show-error "{{ if name != "" { name } else { cmd } }} is not installed"
            exit {{EXIT_MISSING_DEPS}}
        fi
    fi

# ===== File/Directory Checks =====
[private]
check-file path name="":
    #!/usr/bin/env bash
    if [[ ! -f "{{path}}" ]]; then
        just show-error "{{ if name != "" { name } else { path } }} not found"
        exit {{EXIT_CONFIG_ERROR}}
    fi

[private]
check-dir path name="":
    #!/usr/bin/env bash
    if [[ ! -d "{{path}}" ]]; then
        just show-error "{{ if name != "" { name } else { path } }} directory not found"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# ===== Port Checks =====
[private]
check-port port service="":
    #!/usr/bin/env bash
    set +e  # Allow non-zero exit codes
    if lsof -Pi :{{port}} -sTCP:LISTEN -t >/dev/null 2>&1; then
        just show-warning "Port {{port}} already in use{{ if service != "" { " (expected: " + service + ")" } else { "" } }}"
        exit 0
    fi
    exit 1

# ===== Service Checks =====
[private]
check-service url name:
    #!/usr/bin/env bash
    if curl -s "{{url}}" >/dev/null 2>&1; then
        exit 0
    else
        exit 1
    fi

[private]
wait-for-service url name timeout="30":
    #!/usr/bin/env bash
    just show-progress "Waiting for {{name}}"
    timeout={{timeout}}
    while ! curl -s "{{url}}" >/dev/null 2>&1 && [ $timeout -gt 0 ]; do
        sleep 1
        ((timeout--))
    done
    
    if [ $timeout -eq 0 ]; then
        just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "{{name}} failed to start within {{timeout}} seconds"
    else
        just show-success "{{name}} ready"
    fi

# ===== Log Management =====
# View logs for a service
# Usage: just view-log "validator" (LOGS_PATH + "/validator.log")
[private]
view-log service log-file:
    #!/usr/bin/env bash
    if [[ -f "{{log-file}}" ]]; then
        tail -f "{{log-file}}"
    else
        just show-error "No {{service}} log found at {{log-file}}"
        echo "Start {{service}} first to generate logs"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# ===== Generic Service Management =====
# Check if a port-based service is running
# Returns 0 if running, 1 if not
[private]
is-service-running port:
    #!/usr/bin/env bash
    lsof -Pi :{{port}} -sTCP:LISTEN -t >/dev/null 2>&1

# Check if a URL-based service is healthy
# Returns 0 if healthy, 1 if not
[private]
is-service-healthy url:
    #!/usr/bin/env bash
    curl -s "{{url}}" >/dev/null 2>&1

# Wait for port to be available
[private]
wait-for-port port name timeout="30":
    #!/usr/bin/env bash
    just show-progress "Waiting for {{name}} on port {{port}}"
    timeout={{timeout}}
    while ! lsof -Pi :{{port}} -sTCP:LISTEN -t >/dev/null 2>&1 && [ $timeout -gt 0 ]; do
        sleep 1
        ((timeout--))
    done
    
    if [ $timeout -eq 0 ]; then
        just exit-with-error {{EXIT_SERVICE_START_FAILURE}} "{{name}} failed to start on port {{port}}"
    else
        just show-success "{{name}} ready on port {{port}}"
    fi

# ===== Program ID Management =====
# Get program ID from deployed keypair, environment, or default (use 'just program id' instead)
[private]
get-program-id:
    #!/usr/bin/env bash
    # Suppress any shell initialization output
    exec 2>/dev/null
    
    # Try deployed keypair first
    if [[ -f "{{DEPLOY_PATH}}/feels-keypair.json" ]]; then
        # Use nix develop only if not already in nix shell
        if [[ -n "$IN_NIX_SHELL" ]]; then
            PROGRAM_ID=$(solana-keygen pubkey "{{DEPLOY_PATH}}/feels-keypair.json" 2>/dev/null | tail -1 | grep -E '^[A-Za-z0-9]+$' || echo "")
        else
            PROGRAM_ID=$(nix develop --quiet -c solana-keygen pubkey "{{DEPLOY_PATH}}/feels-keypair.json" 2>/dev/null | tail -1 | grep -E '^[A-Za-z0-9]+$' || echo "")
        fi
        if [[ -n "$PROGRAM_ID" ]]; then
            echo "$PROGRAM_ID"
            exit 0
        fi
    fi
    
    # Try saved program ID file
    if [[ -f "{{KEYPAIRS_PATH}}/.deployed-program-id" ]]; then
        PROGRAM_ID=$(cat "{{KEYPAIRS_PATH}}/.deployed-program-id" 2>/dev/null | grep -E '^[A-Za-z0-9]+$' || echo "")
        if [[ -n "$PROGRAM_ID" ]]; then
            echo "$PROGRAM_ID"
            exit 0
        fi
    fi
    
    # Fall back to environment or default (just the ID)
    echo "{{DEFAULT_PROGRAM_ID}}"

# Save program ID for future use
[private]
save-program-id id:
    @mkdir -p "{{KEYPAIRS_PATH}}"
    @echo "{{id}}" > "{{KEYPAIRS_PATH}}/.deployed-program-id"

# ===== Environment Loading =====
# Load environment and find authority keypair
[private]
find-authority-keypair:
    #!/usr/bin/env bash
    if [[ -n "${PROGRAM_AUTHORITY:-}" ]]; then
        for keypair in ~/.config/solana/id.json {{KEYPAIRS_PATH}}/*.json; do
            if [[ -f "$keypair" ]]; then
                pubkey=$(solana-keygen pubkey "$keypair" 2>/dev/null || true)
                if [[ "$pubkey" = "$PROGRAM_AUTHORITY" ]]; then
                    echo "$keypair"
                    exit 0
                fi
            fi
        done
        just show-warning "Could not find keypair for authority $PROGRAM_AUTHORITY"
    fi
    
    # Return default wallet if exists
    if [[ -f ~/.config/solana/id.json ]]; then
        echo ~/.config/solana/id.json
    fi

# ═══════════════════════════════════════════════════════════════════════════
# Environment Validation
# ═══════════════════════════════════════════════════════════════════════════

# Validate build environment (use 'just validate build' instead)
[private]
validate-build:
    @just show-progress "Validating build environment"
    @just check-command cargo "Cargo"
    @just check-command anchor "Anchor"
    @just check-command solana "Solana CLI"
    @just check-file Anchor.toml "Anchor configuration"
    @just check-file Cargo.toml "Cargo workspace"
    @just show-success "Build environment validated"

# Validate deployment environment (use 'just validate deploy' instead)
[private]
validate-deploy target="localnet":
    #!/usr/bin/env bash
    just show-progress "Validating {{target}} deployment environment"
    
    # For localnet, check if validator is running
    if [[ "{{target}}" == "localnet" ]]; then
        if just check-service "{{LOCALNET_RPC}}" "Solana validator" 2>/dev/null; then
            just show-success "Local validator running"
        else
            just show-error "Local validator not running. Run 'just validator' first."
            exit {{EXIT_SERVICE_START_FAILURE}}
        fi
    fi
    
    # Check for built program
    if [[ -f "{{DEPLOY_PATH}}/feels.so" ]]; then
        echo "  Program binary: Found"
    else
        just show-warning "Program not built. Run 'just build' first."
    fi
    
    just show-success "Deployment environment validated"

# Run pre-flight checks (use 'just validate preflight' instead)
[private]
run-preflight-checks checks:
    #!/usr/bin/env bash
    just show-progress "Running pre-flight checks"
    
    failed=0
    for check in {{checks}}; do
        case "$check" in
            solana) just check-command solana "Solana CLI" || ((failed++)) ;;
            anchor) 
                just check-command anchor "Anchor framework" || ((failed++))
                ;;
            validator)
                if ! just check-service "{{LOCALNET_RPC}}" "Solana validator"; then
                    just show-warning "Solana validator not running"
                    ((failed++))
                fi
                ;;
            "anchor.toml") just check-file "Anchor.toml" "Anchor configuration" || ((failed++)) ;;
            ".env") 
                if ! just check-file ".env" "Environment configuration" 2>/dev/null; then
                    just show-warning ".env file not found"
                    ((failed++))
                fi
                ;;
        esac
    done
    
    if [[ $failed -gt 0 ]]; then
        just exit-with-error {{EXIT_ENV_SETUP_ERROR}} "Pre-flight checks failed ($failed issues)"
    else
        just show-success "All pre-flight checks passed"
    fi