#!/usr/bin/env bash
# Deploy Feels Protocol with custom program authority from .env file

set -euo pipefail

# Function to load .env file
load_env() {
    if [ -f .env ]; then
        echo "Loading environment variables from .env file..."
        export $(grep -v '^#' .env | xargs)
    else
        echo "Warning: .env file not found"
    fi
}

# Function to validate program authority
validate_authority() {
    if [ -z "${PROGRAM_AUTHORITY:-}" ]; then
        echo "Error: PROGRAM_AUTHORITY not set in .env file"
        echo "Please add 'PROGRAM_AUTHORITY=<your-authority-pubkey>' to .env file"
        exit 1
    fi
    
    echo "Using program authority: $PROGRAM_AUTHORITY"
}

# Function to check if authority keypair exists
check_authority_keypair() {
    local authority_keypair=""
    
    # Check common locations for the authority keypair
    if [ -f "$HOME/.config/solana/id.json" ]; then
        local pubkey=$(solana-keygen pubkey "$HOME/.config/solana/id.json" 2>/dev/null || true)
        if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
            authority_keypair="$HOME/.config/solana/id.json"
            echo "Found matching keypair at: $authority_keypair"
        fi
    fi
    
    # Check keypairs directory
    for keypair in keypairs/*.json; do
        if [ -f "$keypair" ]; then
            local pubkey=$(solana-keygen pubkey "$keypair" 2>/dev/null || true)
            if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
                authority_keypair="$keypair"
                echo "Found matching keypair at: $authority_keypair"
                break
            fi
        fi
    done
    
    if [ -z "$authority_keypair" ]; then
        echo "Warning: Could not find keypair for authority $PROGRAM_AUTHORITY"
        echo "Make sure the keypair is available when deploying"
    fi
    
    echo "$authority_keypair"
}

# Main deployment function
deploy_program() {
    local cluster="${1:-localnet}"
    local program_name="${2:-feels}"
    
    echo "Deploying $program_name to $cluster..."
    
    # Load environment variables
    load_env
    
    # Validate authority
    validate_authority
    
    # Check for authority keypair
    local authority_keypair=$(check_authority_keypair)
    
    # Build deployment command
    local deploy_cmd="anchor deploy --provider.cluster $cluster"
    
    # Add program-specific options if deploying a specific program
    if [ "$program_name" != "all" ]; then
        deploy_cmd="$deploy_cmd --program-name $program_name"
    fi
    
    # If we found the authority keypair, use it
    if [ -n "$authority_keypair" ] && [ -f "$authority_keypair" ]; then
        echo "Deploying with authority keypair: $authority_keypair"
        export ANCHOR_WALLET="$authority_keypair"
    fi
    
    echo "Executing: $deploy_cmd"
    echo ""
    
    # Execute deployment
    $deploy_cmd
    
    echo ""
    echo "Deployment complete!"
    echo "Program authority: $PROGRAM_AUTHORITY"
}

# Parse command line arguments
CLUSTER="${1:-localnet}"
PROGRAM="${2:-all}"

# Run deployment
deploy_program "$CLUSTER" "$PROGRAM"