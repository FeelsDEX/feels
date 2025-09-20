#!/usr/bin/env bash
# Environment validation script for Feels Protocol

source scripts/exit-codes.sh

# Validate build environment
validate_build_env() {
    local errors=0
    
    show_progress "Validating build environment"
    
    # Check for Nix
    if ! command -v nix &> /dev/null; then
        show_error "Nix is not installed. Please install Nix first."
        ((errors++))
    fi
    
    # Check for required files
    if ! check_file "Anchor.toml" "Anchor configuration"; then
        show_error "Anchor.toml not found in project root"
        ((errors++))
    fi
    
    if ! check_file "Cargo.toml" "Cargo workspace"; then
        show_error "Cargo.toml not found in project root"
        ((errors++))
    fi
    
    # Check for programs directory
    if ! check_dir "programs" "Programs directory"; then
        show_error "programs/ directory not found"
        ((errors++))
    fi
    
    # Check rust version in nix shell
    if command -v nix &> /dev/null; then
        local rust_version=$(nix develop --command rustc --version 2>/dev/null | head -1)
        if [[ -n "$rust_version" ]]; then
            echo "  Rust version: $rust_version"
        else
            show_warning "Could not determine Rust version in Nix shell"
        fi
    fi
    
    if [[ $errors -eq 0 ]]; then
        show_success "Build environment validated"
        return 0
    else
        return 1
    fi
}

# Validate deployment environment
validate_deploy_env() {
    local target=$1  # "localnet" or "devnet"
    local errors=0
    
    show_progress "Validating deployment environment for $target"
    
    # Check for .env file
    if check_file ".env"; then
        # Load and validate environment
        source scripts/load-env.sh
        if [[ -z "$PROGRAM_AUTHORITY" ]]; then
            show_warning "PROGRAM_AUTHORITY not set in .env"
            ((errors++))
        else
            echo "  Program authority: $PROGRAM_AUTHORITY"
        fi
    else
        show_warning ".env file not found - deployment may fail"
        ((errors++))
    fi
    
    # Check for keypairs
    local found_wallet=false
    for keypair in ~/.config/solana/id.json keypairs/*.json; do
        if [[ -f "$keypair" ]]; then
            found_wallet=true
            break
        fi
    done
    
    if ! $found_wallet; then
        show_warning "No wallet keypairs found"
        ((errors++))
    fi
    
    # For localnet, check if validator is running
    if [[ "$target" == "localnet" ]]; then
        if ! check_service "http://localhost:8899" "Solana validator"; then
            show_error "Local validator not running. Run 'just validator' first."
            ((errors++))
        else
            echo "  Local validator: Running"
        fi
    fi
    
    # Check for built program
    if ! check_file "target/deploy/feels.so"; then
        show_warning "Program not built. Run 'just build' first."
        ((errors++))
    else
        echo "  Program binary: Found"
    fi
    
    if [[ $errors -eq 0 ]]; then
        show_success "Deployment environment validated"
        return 0
    else
        return 1
    fi
}

# Validate E2E environment
validate_e2e_env() {
    local errors=0
    
    show_progress "Validating E2E environment"
    
    # Check for required directories
    for dir in "e2e" "feels-app" "feels-indexer"; do
        if ! check_dir "$dir"; then
            show_error "$dir/ directory not found"
            ((errors++))
        fi
    done
    
    # Check for Node.js
    if ! check_command "node" "Node.js"; then
        ((errors++))
    else
        local node_version=$(node --version)
        echo "  Node.js version: $node_version"
    fi
    
    # Check for npm
    if ! check_command "npm" "npm"; then
        ((errors++))
    fi
    
    # Check port availability
    local ports=(8899 8900 8080 10000 3000)
    local used_ports=()
    
    for port in "${ports[@]}"; do
        if check_port $port; then
            used_ports+=($port)
        fi
    done
    
    if [[ ${#used_ports[@]} -gt 0 ]]; then
        show_warning "The following ports are in use: ${used_ports[*]}"
        echo "  Consider running 'just dev-e2e-stop' to free them"
    fi
    
    # Check for Metaplex config
    if check_file "feels-app/scripts/metaplex-localnet.json"; then
        local metaplex_id=$(jq -r '.programId // empty' feels-app/scripts/metaplex-localnet.json 2>/dev/null)
        if [[ -n "$metaplex_id" ]]; then
            echo "  Metaplex program: $metaplex_id"
        fi
    else
        show_warning "Metaplex not configured. Run 'just setup-metaplex' after starting validator."
    fi
    
    if [[ $errors -eq 0 ]]; then
        show_success "E2E environment validated"
        return 0
    else
        return 1
    fi
}

# Validate test environment
validate_test_env() {
    local test_type=$1
    local errors=0
    
    show_progress "Validating test environment for $test_type tests"
    
    case $test_type in
        "unit"|"integration"|"property")
            # These tests run in-memory, just need build env
            validate_build_env
            return $?
            ;;
        "e2e")
            # E2E tests need full environment
            validate_e2e_env
            return $?
            ;;
        "localnet")
            # Localnet tests need validator and deployed program
            validate_deploy_env "localnet"
            return $?
            ;;
        "devnet")
            # Devnet tests need network access
            if ! curl -s https://api.devnet.solana.com >/dev/null 2>&1; then
                show_error "Cannot reach Solana devnet"
                ((errors++))
            else
                echo "  Solana devnet: Reachable"
            fi
            ;;
        *)
            show_warning "Unknown test type: $test_type"
            return 1
            ;;
    esac
    
    if [[ $errors -eq 0 ]]; then
        show_success "Test environment validated"
        return 0
    else
        return 1
    fi
}

# Main validation function
validate_environment() {
    local operation=$1
    
    echo "=== Feels Protocol Environment Validation ==="
    echo ""
    
    case $operation in
        "build")
            validate_build_env
            ;;
        "deploy-localnet")
            validate_deploy_env "localnet"
            ;;
        "deploy-devnet")
            validate_deploy_env "devnet"
            ;;
        "e2e")
            validate_e2e_env
            ;;
        "test")
            validate_test_env "${2:-all}"
            ;;
        "all")
            validate_build_env
            echo ""
            validate_deploy_env "localnet"
            echo ""
            validate_e2e_env
            ;;
        *)
            echo "Usage: $0 {build|deploy-localnet|deploy-devnet|e2e|test|all}"
            exit 1
            ;;
    esac
}

# If script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    validate_environment "$@"
fi