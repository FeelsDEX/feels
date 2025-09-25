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

# Start validator with custom options
validator-custom flags="":
    @just show-progress "Starting validator with custom flags"
    @solana-test-validator {{flags}}

# Get validator logs
validator-logs:
    #!/usr/bin/env bash
    if [[ -f {{LOGS_PATH}}/validator.log ]]; then
        tail -f {{LOGS_PATH}}/validator.log
    else
        just show-error "No validator log found. Start validator first"
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
        mainnet)
            just show-error "Mainnet deployment not supported through this command"
            exit {{EXIT_GENERAL_ERROR}}
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

# Show program address
program-id program="feels":
    #!/usr/bin/env bash
    echo "{{program}} Program ID:"
    if [[ -f {{DEPLOY_PATH}}/{{program}}-keypair.json ]]; then
        solana-keygen pubkey {{DEPLOY_PATH}}/{{program}}-keypair.json
    else
        just show-error "Program keypair not found. Build first with 'just build'"
    fi

# Show all deployed programs
programs:
    @echo "Deployed Programs:"
    @echo "=================="
    @for keypair in {{DEPLOY_PATH}}/*-keypair.json; do \
        if [[ -f "$keypair" ]]; then \
            name=$(basename "$keypair" -keypair.json); \
            id=$(solana-keygen pubkey "$keypair" 2>/dev/null); \
            echo "  $name: $id"; \
        fi; \
    done

# ===== Account Management =====

# Show wallet balance (internal - use 'just balance' from main justfile)
_balance address="":
    #!/usr/bin/env bash
    if [[ -z "{{address}}" ]]; then
        echo "Default wallet balance:"
        solana balance
    else
        echo "Balance for {{address}}:"
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

# Generate new keypair
keygen name output-dir="":
    #!/usr/bin/env bash
    # Use KEYPAIRS_PATH if no output-dir specified
    if [[ -z "{{output-dir}}" ]]; then
        output_dir="{{KEYPAIRS_PATH}}"
    else
        output_dir="{{output-dir}}"
    fi
    
    mkdir -p "$output_dir"
    output_file="$output_dir/{{name}}.json"
    
    just show-progress "Generating keypair: {{name}}"
    solana-keygen new -o "$output_file" --no-bip39-passphrase --force
    
    pubkey=$(solana-keygen pubkey "$output_file")
    just show-success "Keypair generated: $pubkey"
    echo "  Saved to: $output_file"
    echo "  Pubkey: $pubkey"

# Show keypair info
keyinfo path:
    #!/usr/bin/env bash
    if [[ ! -f "{{path}}" ]]; then
        just show-error "Keypair not found: {{path}}"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    echo "Keypair Information:"
    echo "  Path: {{path}}"
    echo "  Pubkey: $(solana-keygen pubkey {{path}})"

# ===== Transaction Tools =====

# Simulate transaction
simulate tx-file:
    @just show-progress "Simulating transaction"
    @solana simulate -C {{tx-file}}

# Send transaction
send-tx tx-file:
    @just show-progress "Sending transaction"
    @solana confirm -v {{tx-file}}

# Get transaction details
tx-info signature:
    @solana transaction-info {{signature}} --verbose

# ===== Account Inspection =====

# Show account info
account address:
    @just ensure-validator
    @solana account {{address}} --output json-compact

# Decode account data
decode-account address program="feels":
    @just ensure-validator
    @just show-progress "Decoding account data"
    @anchor account --provider.cluster localnet --program-id $(just program-id {{program}}) {{address}}

# ===== RPC Utilities =====

# Get cluster version
cluster-version:
    @just ensure-validator
    @solana cluster-version

# Get recent blockhash
blockhash:
    @just ensure-validator
    @solana block | grep "^Blockhash:" | cut -d' ' -f2

# Get epoch info
epoch-info:
    @just ensure-validator
    @solana epoch-info

# Get slot
slot:
    @just ensure-validator
    @solana slot

# ===== Development Utilities =====

# Close buffer accounts (cleanup)
close-buffers:
    #!/usr/bin/env bash
    just ensure-validator
    just show-progress "Closing program buffer accounts"
    buffers=$(solana program show --buffers | grep -E '^[A-Za-z0-9]{44}' || true)
    if [[ -z "$buffers" ]]; then
        echo "  No buffer accounts found"
    else
        for buffer in $buffers; do
            echo "  Closing buffer: $buffer"
            solana program close "$buffer" || true
        done
        just show-success "Buffer accounts closed"
    fi

# Set Solana CLI config
set-config key value:
    @just show-progress "Setting Solana config: {{key}}={{value}}"
    @solana config set --{{key}} {{value}}

# Show current Solana CLI config
config:
    @solana config get

# ===== Log Analysis =====

# Parse program logs
parse-logs signature:
    @just show-progress "Parsing program logs for {{signature}}"
    @solana logs {{signature}} | grep -E "(Program log:|Program .* invoke|Program .* success)"

# Stream program logs
stream-logs program-id="":
    #!/usr/bin/env bash
    if [[ -z "{{program-id}}" ]]; then
        PROGRAM_ID=$(just get-program-id)
    else
        PROGRAM_ID="{{program-id}}"
    fi
    
    just show-progress "Streaming logs for $PROGRAM_ID"
    solana logs --url localhost $PROGRAM_ID

# ===== External Programs =====

# Download Metaplex Token Metadata program for tests
download-metaplex:
    #!/usr/bin/env bash
    # Create directory for external programs
    EXTERNAL_PROGRAMS_DIR="{{DEPLOY_PATH}}/external-programs"
    mkdir -p "$EXTERNAL_PROGRAMS_DIR"
    
    just show-progress "Downloading Metaplex Token Metadata program"
    
    # The program ID for Token Metadata
    PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    PROGRAM_NAME="mpl_token_metadata"
    OUTPUT_FILE="${EXTERNAL_PROGRAMS_DIR}/${PROGRAM_NAME}.so"
    
    # Check if already downloaded
    if [[ -f "$OUTPUT_FILE" ]] && [[ -s "$OUTPUT_FILE" ]]; then
        FILE_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
        if [[ $FILE_SIZE -gt 1000 ]]; then
            just show-success "Metaplex program already downloaded (${FILE_SIZE} bytes)"
            exit 0
        fi
    fi
    
    # Download using solana CLI
    just check-command solana "Solana CLI"
    
    echo "Using solana CLI to dump the program..."
    if solana program dump -u mainnet-beta ${PROGRAM_ID} ${OUTPUT_FILE}; then
        FILE_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
        just show-success "Metaplex Token Metadata program downloaded"
        echo "  Location: ${OUTPUT_FILE}"
        echo "  Size: ${FILE_SIZE} bytes"
    else
        just exit-with-error {{EXIT_GENERAL_ERROR}} "Failed to download Metaplex program"
    fi