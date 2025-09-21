# Environment validation recipes

# Import dependencies
import 'common.just'

# ===== Main Validation Commands =====

# Validate everything
validate-all:
    @just validate-build
    @just validate-deploy localnet
    @just validate-e2e
    @echo ""
    @just show-success "All validations passed"

# Validate specific operation (internal - use 'just validate' from main justfile)
_validate operation="all":
    #!/usr/bin/env bash
    case "{{operation}}" in
        all) just validate-all ;;
        build) just validate-build ;;
        deploy-localnet) just validate-deploy localnet ;;
        deploy-devnet) just validate-deploy devnet ;;
        e2e) just validate-e2e ;;
        test) just validate-test all ;;
        *) just show-error "Unknown operation: {{operation}}" && exit {{EXIT_GENERAL_ERROR}} ;;
    esac

# ===== Build Environment =====
validate-build:
    @just show-progress "Validating build environment"
    @just check-command nix "Nix"
    @just check-command cargo "Cargo"
    @just check-command anchor "Anchor"
    @just check-command solana "Solana CLI"
    @just check-file Anchor.toml "Anchor configuration"
    @just check-file Cargo.toml "Cargo workspace"
    @just check-dir "{{PROGRAMS_PATH}}" "Programs directory"
    @just show-success "Build environment validated"

# ===== Deployment Environment =====
validate-deploy target="localnet":
    #!/usr/bin/env bash
    just show-progress "Validating {{target}} deployment environment"
    
    # Check for authority
    if [[ -n "${PROGRAM_AUTHORITY:-}" ]]; then
        echo "  Program authority: $PROGRAM_AUTHORITY"
        keypair=$(just find-authority-keypair)
        if [[ -n "$keypair" ]]; then
            echo "  Authority keypair: $keypair"
        fi
    else
        just show-warning "PROGRAM_AUTHORITY not set in environment"
    fi
    
    # For localnet, check if validator is running
    if [[ "{{target}}" == "localnet" ]]; then
        if just check-service "{{LOCALNET_RPC}}" "Solana validator"; then
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

# ===== E2E Environment =====
validate-e2e:
    @just show-progress "Validating E2E environment"
    @just check-dir "{{E2E_PATH}}" "E2E directory"
    @just check-dir "{{FEELS_APP_PATH}}" "Frontend app directory"
    @just check-dir "{{PROJECT_ROOT}}/feels-indexer" "Indexer directory"
    @just check-command node "Node.js"
    @just check-command npm "npm"
    @just validate-ports
    @just check-metaplex-config
    @just show-success "E2E environment validated"

# ===== Test Environment =====
validate-test type="all":
    #!/usr/bin/env bash
    just show-progress "Validating test environment for {{type}} tests"
    
    case "{{type}}" in
        unit|integration|property)
            just validate-build
            ;;
        e2e)
            just validate-e2e
            ;;
        localnet)
            just validate-deploy localnet
            ;;
        devnet)
            # Check network connectivity
            if curl -s "{{DEVNET_RPC}}" >/dev/null 2>&1; then
                echo "  Solana devnet: Reachable"
            else
                just show-error "Cannot reach Solana devnet"
                exit {{EXIT_ENV_SETUP_ERROR}}
            fi
            ;;
        all)
            just validate-build
            ;;
        *)
            just show-warning "Unknown test type: {{type}}"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac
    
    just show-success "Test environment validated"

# ===== Helper Functions =====

# Check if required ports are available
[private]
validate-ports:
    #!/usr/bin/env bash
    ports=({{VALIDATOR_RPC_PORT}} {{VALIDATOR_WS_PORT}} {{STREAMING_PORT}} {{INDEXER_PORT}} {{APP_PORT}})
    used_ports=()
    
    for port in "${ports[@]}"; do
        if just check-port $port; then
            used_ports+=($port)
        fi
    done
    
    if [[ ${#used_ports[@]} -gt 0 ]]; then
        just show-warning "The following ports are in use: ${used_ports[*]}"
        echo "  Consider running 'just stop-all' to free them"
    else
        echo "  All required ports are available"
    fi

# Check Metaplex configuration
[private]
check-metaplex-config:
    #!/usr/bin/env bash
    config_file="{{FEELS_APP_PATH}}/scripts/metaplex-localnet.json"
    if [[ -f "$config_file" ]]; then
        metaplex_id=$(jq -r '.programId // empty' "$config_file" 2>/dev/null)
        if [[ -n "$metaplex_id" ]]; then
            echo "  Metaplex program: $metaplex_id"
        else
            just show-warning "Invalid Metaplex configuration"
        fi
    else
        just show-warning "Metaplex not configured. Run 'just setup-metaplex' after starting validator."
    fi

# Pre-flight checks for common operations
run-preflight-checks checks:
    #!/usr/bin/env bash
    just show-progress "Running pre-flight checks"
    
    failed=0
    for check in {{checks}}; do
        case "$check" in
            solana) just check-command solana "Solana CLI" || ((failed++)) ;;
            anchor) just check-command anchor "Anchor framework" || ((failed++)) ;;
            node) just check-command node "Node.js" || ((failed++)) ;;
            cargo) just check-command cargo "Rust/Cargo" || ((failed++)) ;;
            validator)
                if ! just check-service "{{LOCALNET_RPC}}" "Solana validator"; then
                    just show-warning "Solana validator not running"
                    ((failed++))
                fi
                ;;
            indexer)
                if ! just check-service "http://localhost:{{INDEXER_PORT}}/health" "Indexer"; then
                    just show-warning "Indexer not running"
                    ((failed++))
                fi
                ;;
            streaming)
                if ! just check-service "http://localhost:{{STREAMING_PORT}}/status" "Streaming adapter"; then
                    just show-warning "Streaming adapter not running"
                    ((failed++))
                fi
                ;;
            app)
                if ! just check-service "http://localhost:{{APP_PORT}}" "Frontend app"; then
                    just show-warning "Frontend app not running"
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