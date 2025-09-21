# Common utility functions used across all modules

# Import constants
import 'constants.just'

# ===== Shell Configuration =====
set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true
set export := true

# ===== OS Detection =====
is-macos := if os() == "macos" { "true" } else { "false" }
is-linux := if os() == "linux" { "true" } else { "false" }  
is-windows := if os() == "windows" { "true" } else { "false" }

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

# ===== Command Checks =====
[private]
check-command cmd name="":
    #!/usr/bin/env bash
    if ! command -v {{cmd}} &> /dev/null; then
        just show-error "{{ if name != "" { name } else { cmd } }} is not installed"
        exit {{EXIT_MISSING_DEPS}}
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

# ===== Program ID Management =====
# Get program ID from deployed keypair, environment, or default
get-program-id:
    #!/usr/bin/env bash
    # Try deployed keypair first
    if [[ -f "{{DEPLOY_PATH}}/feels-keypair.json" ]]; then
        PROGRAM_ID=$(solana-keygen pubkey "{{DEPLOY_PATH}}/feels-keypair.json" 2>/dev/null || echo "")
        if [[ -n "$PROGRAM_ID" ]]; then
            echo "$PROGRAM_ID"
            exit 0
        fi
    fi
    
    # Try saved program ID file
    if [[ -f "{{E2E_PATH}}/.deployed-program-id" ]]; then
        PROGRAM_ID=$(cat "{{E2E_PATH}}/.deployed-program-id" 2>/dev/null || echo "")
        if [[ -n "$PROGRAM_ID" ]]; then
            echo "$PROGRAM_ID"
            exit 0
        fi
    fi
    
    # Fall back to environment or default
    echo "${FEELS_PROGRAM_ID:-{{DEFAULT_PROGRAM_ID}}}"

# Save program ID for future use
[private]
save-program-id id:
    @mkdir -p "{{E2E_PATH}}"
    @echo "{{id}}" > "{{E2E_PATH}}/.deployed-program-id"

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