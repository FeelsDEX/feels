#!/usr/bin/env bash
# Load environment variables and find authority keypair
# Usage: source scripts/load-env.sh

load_env_and_find_keypair() {
    # Load environment variables from .env if it exists
    if [ -f .env ]; then
        echo "Loading environment from .env..."
        export $(grep -v '^#' .env | xargs)
    fi
    
    if [ -n "$PROGRAM_AUTHORITY" ]; then
        echo "Using program authority: $PROGRAM_AUTHORITY"
        # Check for authority keypair
        for keypair in ~/.config/solana/id.json keypairs/*.json; do
            if [ -f "$keypair" ]; then
                pubkey=$(nix develop --command solana-keygen pubkey "$keypair" 2>/dev/null || true)
                if [ "$pubkey" = "$PROGRAM_AUTHORITY" ]; then
                    echo "Found authority keypair at: $keypair"
                    export ANCHOR_WALLET="$keypair"
                    return 0
                fi
            fi
        done
        echo "Warning: Could not find keypair for authority $PROGRAM_AUTHORITY"
        return 1
    fi
    return 0
}

# Call the function if script is sourced
if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    load_env_and_find_keypair
fi
