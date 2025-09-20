#!/usr/bin/env bash
# Standardized exit codes for Feels Protocol justfile commands

# Exit code definitions
export EXIT_SUCCESS=0
export EXIT_GENERAL_ERROR=1
export EXIT_MISSING_DEPS=2
export EXIT_BUILD_FAILURE=3
export EXIT_DEPLOY_FAILURE=4
export EXIT_TEST_FAILURE=5
export EXIT_CONFIG_ERROR=10
export EXIT_ENV_SETUP_ERROR=11
export EXIT_SERVICE_START_FAILURE=12

# Helper function to exit with code and message
exit_with_code() {
    local code=$1
    local message=$2
    
    case $code in
        0)  echo -e "\033[0;32m[SUCCESS] $message\033[0m" ;;
        1)  echo -e "\033[0;31m[ERROR] $message\033[0m" >&2 ;;
        2)  echo -e "\033[0;31m[MISSING DEPS] $message\033[0m" >&2 ;;
        3)  echo -e "\033[0;31m[BUILD FAILED] $message\033[0m" >&2 ;;
        4)  echo -e "\033[0;31m[DEPLOY FAILED] $message\033[0m" >&2 ;;
        5)  echo -e "\033[0;31m[TEST FAILED] $message\033[0m" >&2 ;;
        10) echo -e "\033[0;31m[CONFIG ERROR] $message\033[0m" >&2 ;;
        11) echo -e "\033[0;31m[ENV SETUP ERROR] $message\033[0m" >&2 ;;
        12) echo -e "\033[0;31m[SERVICE START FAILED] $message\033[0m" >&2 ;;
        *)  echo -e "\033[0;31m[ERROR $code] $message\033[0m" >&2 ;;
    esac
    
    exit $code
}

# Progress indicator functions
show_progress() {
    local message=$1
    echo -e "\033[1;33m[...] $message...\033[0m"
}

show_building() {
    echo -e "\033[1;33m[BUILDING] Compiling programs...\033[0m"
}

show_deploying() {
    echo -e "\033[1;33m[DEPLOYING] Deploying to network...\033[0m"
}

show_success() {
    local message=$1
    echo -e "\033[0;32m[OK] $message\033[0m"
}

show_error() {
    local message=$1
    echo -e "\033[0;31m[ERROR] $message\033[0m"
}

show_warning() {
    local message=$1
    echo -e "\033[1;33m[WARNING] $message\033[0m"
}

# Validation functions
check_command() {
    local cmd=$1
    local name=${2:-$cmd}
    if ! command -v "$cmd" &> /dev/null; then
        exit_with_code $EXIT_MISSING_DEPS "$name is not installed"
    fi
}

check_file() {
    local file=$1
    local name=${2:-$file}
    if [[ ! -f "$file" ]]; then
        return 1
    fi
    return 0
}

check_dir() {
    local dir=$1
    local name=${2:-$dir}
    if [[ ! -d "$dir" ]]; then
        return 1
    fi
    return 0
}

check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        return 0  # Port is in use
    else
        return 1  # Port is free
    fi
}

check_service() {
    local url=$1
    local name=$2
    if curl -s "$url" >/dev/null 2>&1; then
        return 0  # Service is running
    else
        return 1  # Service is not running
    fi
}

# Pre-flight check function
run_preflight_checks() {
    local checks=("$@")
    local failed=0
    
    show_progress "Running pre-flight checks"
    
    for check in "${checks[@]}"; do
        case $check in
            "solana")
                if ! check_command "solana" "Solana CLI"; then
                    ((failed++))
                fi
                ;;
            "anchor")
                if ! check_command "anchor" "Anchor framework"; then
                    ((failed++))
                fi
                ;;
            "node")
                if ! check_command "node" "Node.js"; then
                    ((failed++))
                fi
                ;;
            "cargo")
                if ! check_command "cargo" "Rust/Cargo"; then
                    ((failed++))
                fi
                ;;
            "validator")
                if ! check_service "http://localhost:8899" "Solana validator"; then
                    show_warning "Solana validator not running on port 8899"
                    ((failed++))
                fi
                ;;
            "indexer")
                if ! check_service "http://localhost:8080/health" "Indexer"; then
                    show_warning "Indexer not running on port 8080"
                    ((failed++))
                fi
                ;;
            "streaming")
                if ! check_service "http://localhost:10000/status" "Streaming adapter"; then
                    show_warning "Streaming adapter not running on port 10000"
                    ((failed++))
                fi
                ;;
            "app")
                if ! check_service "http://localhost:3000" "Frontend app"; then
                    show_warning "Frontend app not running on port 3000"
                    ((failed++))
                fi
                ;;
            "anchor.toml")
                if ! check_file "Anchor.toml" "Anchor configuration"; then
                    show_warning "Anchor.toml not found"
                    ((failed++))
                fi
                ;;
            ".env")
                if ! check_file ".env" "Environment configuration"; then
                    show_warning ".env file not found"
                    ((failed++))
                fi
                ;;
        esac
    done
    
    if [[ $failed -gt 0 ]]; then
        exit_with_code $EXIT_ENV_SETUP_ERROR "Pre-flight checks failed ($failed issues)"
    else
        show_success "All pre-flight checks passed"
    fi
}

# Export all functions for use in justfiles
export -f exit_with_code
export -f show_progress
export -f show_building
export -f show_deploying
export -f show_success
export -f show_error
export -f show_warning
export -f check_command
export -f check_file
export -f check_dir
export -f check_port
export -f check_service
export -f run_preflight_checks