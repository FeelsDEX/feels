# Solana development tools and utilities

# Import dependencies
import 'common.just'

# ===== Network Management =====

# Start local validator (internal - use 'just validator' from main justfile)
_validator:
    @just show-progress "Starting local development network"
    @if command -v nix >/dev/null 2>&1; then \
        nix run .#devnet; \
    else \
        just show-warning "Nix not available, using system validator"; \
        solana-test-validator \
            --reset \
            --rpc-port {{VALIDATOR_RPC_PORT}} \
            --bind-address {{VALIDATOR_BIND_ADDRESS}} \
            --ledger {{TEST_LEDGER_PATH}} \
            --quiet; \
    fi

# ===== Program Management =====

# Deploy to specified network (internal - use 'just deploy' from main justfile)
_deploy network="localnet":
    #!/usr/bin/env bash
    just show-deploying
    
    case "{{network}}" in
        localnet)
            if ! just check-service "{{LOCALNET_RPC}}" "validator"; then
                just show-error "Local validator not running. Start with 'just validator' first"
                exit {{EXIT_SERVICE_START_FAILURE}}
            fi
            ;;
        devnet)
            if ! curl -s "{{DEVNET_RPC}}" >/dev/null 2>&1; then
                just show-error "Cannot reach Solana devnet"
                exit {{EXIT_ENV_SETUP_ERROR}}
            fi
            ;;
        *)
            just show-error "Unknown network: {{network}}"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac
    
    if anchor deploy --provider.cluster {{network}}; then
        PROGRAM_ID=$(just get-program-id)
        just save-program-id "$PROGRAM_ID"
        just show-success "Deployed to {{network}}: $PROGRAM_ID"
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment to {{network}} failed"
    fi

# ===== Account Management =====

# Show wallet balance (internal - use 'just balance' from main justfile)
_balance address="":
    #!/usr/bin/env bash
    if [[ -z "{{address}}" ]]; then
        solana balance
    else
        solana balance {{address}}
    fi

# Airdrop SOL
_airdrop amount="10" address="":
    #!/usr/bin/env bash
    just show-progress "Airdropping {{amount}} SOL"
    if [[ -z "{{address}}" ]]; then
        solana airdrop {{amount}}
    else
        solana airdrop {{amount}} {{address}}
    fi
    just show-success "Airdrop complete"

# ===== Authenticated Deployment =====

# Deploy to devnet using authenticated RPC from .env
deploy-devnet:
    #!/usr/bin/env bash
    set -euo pipefail
    just show-progress "Deploying to devnet with authenticated RPC"
    
    # Source .env file
    if [[ ! -f .env ]]; then
        just show-error ".env file not found"
        echo "Create .env from .env.example and configure DEVNET_ENDPOINT"
        exit {{EXIT_ENV_SETUP_ERROR}}
    fi
    source .env
    
    # Verify environment variables
    if [[ -z "${DEVNET_ENDPOINT:-}" ]]; then
        just show-error "DEVNET_ENDPOINT not set in .env"
        exit {{EXIT_ENV_SETUP_ERROR}}
    fi
    
    # Build the program
    just build feels
    
    # Copy devnet keypair
    if [[ ! -f localnet/keypairs/feels-program-devnet.json ]]; then
        just show-error "Devnet keypair not found: localnet/keypairs/feels-program-devnet.json"
        exit {{EXIT_MISSING_DEPS}}
    fi
    cp localnet/keypairs/feels-program-devnet.json target/deploy/feels-keypair.json
    
    # Get program ID
    PROGRAM_ID=$(just nix-cmd solana-keygen pubkey target/deploy/feels-keypair.json)
    echo "Program ID: $PROGRAM_ID"
    
    # Deploy
    if just nix-cmd solana program deploy \
        --url "$DEVNET_ENDPOINT" \
        --program-id target/deploy/feels-keypair.json \
        target/deploy/feels.so; then
        just show-success "Deployed to devnet: $PROGRAM_ID"
        echo "View on explorer: https://explorer.solana.com/address/$PROGRAM_ID?cluster=devnet"
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Devnet deployment failed"
    fi

# Deploy to mainnet using authenticated RPC from .env
deploy-mainnet:
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Safety check
    echo "⚠️  WARNING: You are about to deploy to MAINNET ⚠️"
    echo "This will cost real SOL and is irreversible."
    echo ""
    read -p "Type 'DEPLOY TO MAINNET' to continue: " confirmation
    if [[ "$confirmation" != "DEPLOY TO MAINNET" ]]; then
        echo "Deployment cancelled"
        exit 0
    fi
    
    just show-progress "Deploying to mainnet with authenticated RPC"
    
    # Source .env file
    if [[ ! -f .env ]]; then
        just show-error ".env file not found"
        echo "Create .env from .env.example and configure MAINNET_ENDPOINT"
        exit {{EXIT_ENV_SETUP_ERROR}}
    fi
    source .env
    
    # Verify environment variables
    if [[ -z "${MAINNET_ENDPOINT:-}" ]]; then
        just show-error "MAINNET_ENDPOINT not set in .env"
        exit {{EXIT_ENV_SETUP_ERROR}}
    fi
    
    # Build the program
    just build feels
    
    # Copy or create mainnet keypair
    if [[ ! -f localnet/keypairs/feels-program-mainnet.json ]]; then
        just show-warning "Mainnet keypair not found, creating new one"
        just nix-cmd solana-keygen new -o localnet/keypairs/feels-program-mainnet.json
    fi
    cp localnet/keypairs/feels-program-mainnet.json target/deploy/feels-keypair.json
    
    # Get program ID
    PROGRAM_ID=$(just nix-cmd solana-keygen pubkey target/deploy/feels-keypair.json)
    echo "Program ID: $PROGRAM_ID"
    
    # Check balance
    BALANCE=$(just nix-cmd solana balance --url "$MAINNET_ENDPOINT" | awk '{print $1}')
    echo "Deployer balance: $BALANCE SOL"
    
    # Final confirmation
    echo ""
    echo "Final confirmation required:"
    echo "  Program ID: $PROGRAM_ID"
    echo "  Network: Mainnet"
    echo "  Estimated cost: ~11 SOL"
    echo ""
    read -p "Proceed with deployment? (yes/no): " final_confirm
    if [[ "$final_confirm" != "yes" ]]; then
        echo "Deployment cancelled"
        exit 0
    fi
    
    # Deploy
    if just nix-cmd solana program deploy \
        --url "$MAINNET_ENDPOINT" \
        --program-id target/deploy/feels-keypair.json \
        target/deploy/feels.so; then
        just show-success "Deployed to mainnet: $PROGRAM_ID"
        echo "View on explorer: https://explorer.solana.com/address/$PROGRAM_ID"
    else
        just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Mainnet deployment failed"
    fi

# ===== RPC Queries =====

# Query program on devnet using authenticated RPC
query-devnet program_id:
    #!/usr/bin/env bash
    set -euo pipefail
    source .env 2>/dev/null || true
    
    if [[ -n "${DEVNET_ENDPOINT:-}" ]]; then
        just show-progress "Querying devnet with authenticated RPC"
        just nix-cmd solana program show --url "$DEVNET_ENDPOINT" {{program_id}}
    else
        just show-progress "Querying devnet with public RPC"
        just nix-cmd solana program show --url devnet {{program_id}}
    fi

# Query program on mainnet using authenticated RPC
query-mainnet program_id:
    #!/usr/bin/env bash
    set -euo pipefail
    source .env 2>/dev/null || true
    
    if [[ -n "${MAINNET_ENDPOINT:-}" ]]; then
        just show-progress "Querying mainnet with authenticated RPC"
        just nix-cmd solana program show --url "$MAINNET_ENDPOINT" {{program_id}}
    else
        just show-progress "Querying mainnet with public RPC"
        just nix-cmd solana program show --url mainnet-beta {{program_id}}
    fi

# Query account using curl with authentication
query-rpc-curl network program_id:
    #!/usr/bin/env bash
    set -euo pipefail
    source .env 2>/dev/null || true
    
    case "{{network}}" in
        devnet)
            ENDPOINT="${DEVNET_ENDPOINT:-https://api.devnet.solana.com}"
            TOKEN="${DEVNET_TOKEN:-}"
            ;;
        mainnet)
            ENDPOINT="${MAINNET_ENDPOINT:-https://api.mainnet-beta.solana.com}"
            TOKEN="${MAINNET_TOKEN:-}"
            ;;
        *)
            just show-error "Unknown network: {{network}}"
            echo "Use: devnet or mainnet"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac
    
    just show-progress "Querying {{network}}: {{program_id}}"
    
    if [[ -n "$TOKEN" ]]; then
        curl -X POST "$ENDPOINT" \
          -H "Content-Type: application/json" \
          -H "Authorization: Bearer $TOKEN" \
          -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"{{program_id}}\",{\"encoding\":\"jsonParsed\"}]}" \
          | jq .
    else
        curl -X POST "$ENDPOINT" \
          -H "Content-Type: application/json" \
          -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"{{program_id}}\",{\"encoding\":\"jsonParsed\"}]}" \
          | jq .
    fi

# Test RPC authentication
test-rpc-auth network:
    #!/usr/bin/env bash
    set -euo pipefail
    source .env 2>/dev/null || true
    
    case "{{network}}" in
        devnet)
            ENDPOINT="${DEVNET_ENDPOINT:-https://api.devnet.solana.com}"
            TOKEN="${DEVNET_TOKEN:-}"
            ;;
        mainnet)
            ENDPOINT="${MAINNET_ENDPOINT:-https://api.mainnet-beta.solana.com}"
            TOKEN="${MAINNET_TOKEN:-}"
            ;;
        *)
            just show-error "Unknown network: {{network}}"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac
    
    just show-progress "Testing {{network}} RPC authentication"
    echo "Endpoint: $ENDPOINT"
    echo "Token present: $([ -n "$TOKEN" ] && echo "yes" || echo "no")"
    echo ""
    
    if [[ -n "$TOKEN" ]]; then
        curl -X POST "$ENDPOINT" \
          -H "Content-Type: application/json" \
          -H "Authorization: Bearer $TOKEN" \
          -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
          | jq .
    else
        curl -X POST "$ENDPOINT" \
          -H "Content-Type: application/json" \
          -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
          | jq .
    fi

# ===== External Programs =====

# Setup Metaplex Token Metadata program for tests (use 'just localnet setup metaplex' instead)
[private]
setup-metaplex:
    #!/usr/bin/env bash
    just show-progress "Setting up Metaplex Token Metadata for tests"
    
    # Check if we're in Nix environment
    if [[ -z "$METAPLEX_BINARY_PATH" ]]; then
        just show-error "METAPLEX_BINARY_PATH not set"
        echo "Ensure you're in a Nix shell: nix develop"
        exit {{EXIT_MISSING_DEPS}}
    fi
    
    if [[ ! -f "$METAPLEX_BINARY_PATH" ]]; then
        just show-error "Metaplex binary not found at: $METAPLEX_BINARY_PATH"
        echo "This should be provided by the Nix environment."
        exit {{EXIT_MISSING_DEPS}}
    fi
    
    # Copy to expected test locations
    mkdir -p "{{PROJECT_ROOT}}/programs/feels/target/external-programs"
    mkdir -p "{{DEPLOY_PATH}}/external-programs"
    
    cp "$METAPLEX_BINARY_PATH" "{{PROJECT_ROOT}}/programs/feels/target/external-programs/mpl_token_metadata.so"
    cp "$METAPLEX_BINARY_PATH" "{{DEPLOY_PATH}}/external-programs/mpl_token_metadata.so"
    
    FILE_SIZE=$(stat -f%z "$METAPLEX_BINARY_PATH" 2>/dev/null || stat -c%s "$METAPLEX_BINARY_PATH" 2>/dev/null)
    just show-success "Metaplex binary copied to test locations (${FILE_SIZE} bytes)"