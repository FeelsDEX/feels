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