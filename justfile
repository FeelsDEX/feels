# ╔═══════════════════════════════════════════════════════════════════════════╗
# ║                    FEELS PROTOCOL - TASK RUNNER                           ║
# ╚═══════════════════════════════════════════════════════════════════════════╝
#
# This is the main entry point for development tasks in the Feels Protocol.
# Run `just` (no arguments) to see available commands.
#
# ═══════════════════════════════════════════════════════════════════════════
# JUSTFILE OVERVIEW
# ═══════════════════════════════════════════════════════════════════════════
#
# The justfile system is organized as follows:
#
# justfiles/
#   ├── common.just          - Shared utilities (nix-cmd, logging, validation)
#   ├── nix-env.just        - Configuration bridge (paths, ports, exit codes, colors)
#   ├── build.just          - Build, clean, IDL generation
#   ├── testing.just        - Test execution (unit, integration, e2e)
#   ├── development.just    - Dev workflow (format, lint, docs)
#   ├── solana-tools.just   - Solana CLI utilities
#   ├── frontend.just       - Next.js app commands (MODULE)
#   ├── services.just       - Services & Infrastructure (MODULE)
#   └── e2e.just           - End-to-end orchestration
#
# NIX INTEGRATION:
#   Nix provides hermetic, reproducible builds and development environments.
#   The just system automatically detects whether you're in a Nix shell and
#   routes commands appropriately via `nix-cmd` and `nix-shell-cmd` utilities.
#
#   Key components:
#   - flake.nix           - Main Nix flake defining devShells and build targets
#   - nix/                - Nix modules and configuration (see nix/README.md)
#   - justfiles/nix-env.just - Bridges Nix environment variables to just
#
#   Commands work both inside and outside `nix develop` shells. When outside,
#   they automatically wrap calls with `nix develop -c <command>`. This means
#   you can run `just build` directly or from within a Nix shell.
#
#   For more details on Nix setup and configuration, see: nix/README.md
#
# COMMAND PATTERNS:
#   - Parametrized: just build [target] [method]
#   - Module access: just frontend dev, just services pg-start
#   - Direct imports: Commands available at top level
#
# NAVIGATION:
#   just --list              - Show all available commands
#   just --list --unsorted   - Show commands in definition order
#   just frontend            - Show frontend module commands
#   just services            - Show services & infrastructure commands
#
# QUICK START:
#   just build               - Build all programs
#   just test                - Run tests
#   just run                 - Start complete E2E environment
#

# ═══════════════════════════════════════════════════════════════════════════
# MODULE IMPORTS
# ═══════════════════════════════════════════════════════════════════════════

# Core utilities and configuration
import 'justfiles/common.just'

# Build and test workflows
import 'justfiles/build.just'
import 'justfiles/testing.just'

# Development tools
import 'justfiles/solana-tools.just'

# Namespaced modules (access via: just <module> <command>)
mod frontend 'justfiles/frontend.just'
mod services 'justfiles/services.just'

# E2E orchestration (run, stop, status commands)
import 'justfiles/e2e.just'

# ═══════════════════════════════════════════════════════════════════════════
# DEFAULT COMMAND - HELP
# ═══════════════════════════════════════════════════════════════════════════

# Show available commands and usage
default:
    @echo "Feels Protocol Development Commands"
    @echo "==================================="
    @echo ""
    @echo "Build & Deploy:"
    @echo "  just build [target] [method] - Build (targets: all|feels|indexer|frontend|idl)"
    @echo "  just build idl [program]     - Generate IDL + TypeScript/Rust clients"
    @echo "  just deploy [network]        - Deploy to network (localnet|devnet)"
    @echo "  just clean [target]          - Clean artifacts (targets: all|feels|indexer|frontend|nix)"
    @echo ""
    @echo "Protocol Setup:"
    @echo "  just program init protocol   - Deploy ProtocolConfig account (0.3% fee)"
    @echo "  just program init hub        - Deploy FeelsSOL hub (requires JITOSOL_MINT)"
    @echo "  just program init market <mint> - Initialize a new market"
    @echo ""
    @echo "Development Workflow:"
    @echo "  just validator               - Start local Solana validator"
    @echo "  just reset                   - Reset entire development environment"
    @echo ""
    @echo "Testing:"
    @echo "  just test [subcommand]       - Run tests (all, unit, integration, e2e, property, localnet)"
    @echo "  just test filter <pattern>   - Run tests matching pattern"
    @echo "  just test advanced           - Show advanced testing capabilities"
    @echo ""
    @echo "E2E Environment:"
    @echo "  just run                     - Start complete stack (validator + indexer + frontend)"
    @echo "  just stop                    - Stop all services"
    @echo "  just status                  - Check service status"
    @echo "  just logs [service]          - View logs (validator|indexer|frontend|geyser)"
    @echo "  just verify                  - Verify protocol setup"
    @echo ""
    @echo "Modules (use 'just <module> <command>'):"
    @echo "  just frontend                - Next.js app (dev, build, test, clean)"
    @echo "  just services                - Services & Infrastructure (pg-start, indexer-build, etc.)"
    @echo ""
    @echo "Utilities:"
    @echo "  just docs                    - Generate and open documentation"
    @echo "  just workspace [subcommand]  - Workspace management (clean, reset, status, info)"
    @echo "  just validate [subcommand]   - Environment validation (all, build, deploy, preflight)"
    @echo "  just nix [subcommand]        - Nix environment (info, shell, run, build, show, check)"
    @echo "  just program [subcommand]    - Program management (id, init, keypair, authority, info)"
    @echo "  just localnet [subcommand]   - Local validator (start, stop, status, setup, airdrop, clean)"
    @echo "  just localnet clean          - Clean localnet data directories"
    @echo "  just --list                  - Show all available commands"

# ═══════════════════════════════════════════════════════════════════════════
# BUILD & DEPLOYMENT
# ═══════════════════════════════════════════════════════════════════════════
#
# Core build and deployment commands. Most are imported from build.just.
#
# Available targets: all (default), feels, indexer, frontend
# Available methods: anchor (default), nix
#
# Examples:
#   just build                    # Build all with anchor
#   just build indexer            # Build just the indexer
#   just build all nix            # Build all with hermetic nix
#   just clean frontend           # Clean just frontend artifacts
#   just reset                    # Clean everything and reset environment
#
# The following are imported from build.just:
#   - build [target] [method]  (targets include: all, feels, indexer, frontend, idl)
#   - clean [target]
#   - reset

# Deploy program to Solana network
# Networks: localnet (default), devnet
deploy network="localnet":
    #!/usr/bin/env bash
    case "{{network}}" in
        localnet)
            # Ensure local validator is running
            just ensure-validator
            # Run pre-deployment checks
            just run-preflight-checks "anchor .env"
            # Deploy to localnet
            just show-deploying
            if just nix-cmd anchor deploy --provider.cluster localnet; then
                just show-success "Deployed to localnet"
            else
                just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
            fi
            ;;
        devnet)
            # Deploy to Solana devnet (no validator needed)
            just run-preflight-checks "anchor .env"
            just show-deploying
            if just nix-cmd anchor deploy --provider.cluster devnet; then
                just show-success "Deployed to devnet"
            else
                just exit-with-error {{EXIT_DEPLOY_FAILURE}} "Deployment failed"
            fi
            ;;
        *)
            just show-error "Unknown network: {{network}}"
            echo "Available networks: localnet, devnet"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# LOCAL DEVELOPMENT
# ═══════════════════════════════════════════════════════════════════════════

# Start local Solana validator for development
# Uses Nix-provided solana-test-validator with optimal settings
validator:
    @echo "Starting local development network..."
    @nix run .#devnet

# ═══════════════════════════════════════════════════════════════════════════
# TESTING
# ═══════════════════════════════════════════════════════════════════════════
#
# Test commands are imported from testing.just.
#
# Test hierarchy:
#   - test           : All in-memory tests (default for CI)
#   - test-unit      : Fast isolated tests
#   - test-integration : Multi-component tests
#   - test-property  : Property-based invariant tests
#   - test-e2e       : Full protocol flow tests
#   - test-localnet  : Full localnet test suite (requires validator)
#   - test-devnet    : Devnet tests (requires connection)
#
# All test commands are available directly (no prefix needed).

# ═══════════════════════════════════════════════════════════════════════════
# END-TO-END ENVIRONMENT
# ═══════════════════════════════════════════════════════════════════════════
#
# E2E commands orchestrate the complete development stack:
#   - Solana validator (port 8899)
#   - Streaming adapter (port 8081)
#   - Indexer (port 8080)
#   - Frontend app (port 3000)
#
# Commands imported from e2e.just:
#   - run     : Start complete environment
#   - stop    : Stop all services
#   - status  : Check service status
#   - logs [service] : View service logs
#   - verify  : Verify protocol setup
#
# The E2E environment automatically:
#   - Starts validator with proper configuration
#   - Deploys programs
#   - Initializes protocol
#   - Starts indexer with databases
#   - Starts frontend with proper environment

# ═══════════════════════════════════════════════════════════════════════════
# MODULES
# ═══════════════════════════════════════════════════════════════════════════
#
# Two namespaced modules for specific components:
#
# FRONTEND (Next.js application)
#   just frontend dev              - Start dev server with hot reload
#   just frontend build            - Build for production
#   just frontend test             - Run frontend tests
#   just frontend lint             - Run ESLint
#   just frontend clean            - Clean build artifacts
#   just frontend generate-sdk     - Generate TypeScript SDK
#
# SERVICES (Services & Infrastructure)
#   just services services-start   - Start all services (PostgreSQL + Redis)
#   just services services-stop    - Stop all services
#   just services pg-start         - Start PostgreSQL only
#   just services redis-start      - Start Redis only
#   just services rocksdb-init     - Initialize RocksDB directory
#   just services indexer-build    - Build indexer
#   just services indexer-run      - Run indexer service
#   just services indexer-test     - Run indexer tests
#   just services indexer-migrate  - Run database migrations
#
# Use `just <module>` to see all commands in that module.


# ═══════════════════════════════════════════════════════════════════════════
# DOCUMENTATION AND WORKSPACE
# ═══════════════════════════════════════════════════════════════════════════

# Generate and open documentation
docs:
    @just show-progress "Generating documentation"
    @just nix-cmd cargo doc --no-deps --open

# Clean all workspace artifacts (legacy command - use 'just workspace clean' instead)
[private]
clean-workspace: clean
    @just show-progress "Cleaning all workspace artifacts"
    @cd {{FEELS_APP_PATH}} && just clean 2>/dev/null || true
    @cd {{PROJECT_ROOT}}/feels-indexer && cargo clean 2>/dev/null || true
    @rm -rf {{GENERATED_SDK_PATH}}
    @rm -rf {{TEST_LEDGER_PATH}}
    @just show-success "Workspace cleaned"

# ═══════════════════════════════════════════════════════════════════════════
# VALIDATION
# ═══════════════════════════════════════════════════════════════════════════

# Validate environment and configuration with subcommands
validate *args:
    #!/usr/bin/env bash
    set -euo pipefail

    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="all"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi

    case "$subcommand" in
        build)
            just validate-build
            ;;
        deploy)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                target="localnet"
            else
                target=${remaining_args_array[0]}
            fi
            just validate-deploy "$target"
            ;;
        preflight)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                checks="anchor .env"
            else
                checks="$remaining_args"
            fi
            just run-preflight-checks "$checks"
            ;;
        all)
            just show-progress "Running full environment validation"
            just validate-build
            just validate-deploy localnet
            just run-preflight-checks "anchor .env"
            just show-success "Full environment validation completed"
            ;;
        *)
            echo "Error: Unknown validate subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  all       - Run all validation checks (default)"
            echo "  build     - Validate build environment"
            echo "  deploy    - Validate deployment environment [target]"
            echo "  preflight - Run pre-flight checks [checks]"
            echo ""
            echo "Usage:"
            echo "  just validate [subcommand]"
            echo "  just validate deploy [localnet|devnet]"
            echo "  just validate preflight [check1 check2 ...]"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# NIX ENVIRONMENT
# ═══════════════════════════════════════════════════════════════════════════

# Nix environment and development shell management
nix *args:
    #!/usr/bin/env bash
    set -euo pipefail

    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="info"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi

    case "$subcommand" in
        info)
            just nix-info
            ;;
        shell)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                shell="default"
            else
                shell=${remaining_args_array[0]}
            fi
            echo "Entering Nix development shell: $shell"
            nix develop ".#$shell"
            ;;
        run)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                echo "Error: App name required"
                echo "Usage: just nix run <app>"
                echo "Available apps: bpf-build, devnet, idl-build"
                exit {{EXIT_GENERAL_ERROR}}
            fi
            app=${remaining_args_array[0]}
            echo "Running Nix app: $app"
            nix run ".#$app"
            ;;
        build)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                target="bpf-build"
            else
                target=${remaining_args_array[0]}
            fi
            echo "Running Nix build: $target"
            nix run ".#$target"
            ;;
        show)
            echo "Available Nix outputs:"
            nix flake show
            ;;
        check)
            echo "Checking Nix flake:"
            nix flake check
            ;;
        *)
            echo "Error: Unknown nix subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  info     - Show Nix environment information (default)"
            echo "  shell    - Enter development shell [default|frontend|indexer|e2e]"
            echo "  run      - Run Nix app [bpf-build|devnet|idl-build]"
            echo "  build    - Run Nix build [bpf-build]"
            echo "  show     - Show available Nix outputs"
            echo "  check    - Check Nix flake validity"
            echo ""
            echo "Usage:"
            echo "  just nix [subcommand]"
            echo "  just nix shell [shell-name]"
            echo "  just nix run <app-name>"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# PROGRAM MANAGEMENT
# ═══════════════════════════════════════════════════════════════════════════

# Program management and utilities
program *args:
    #!/usr/bin/env bash
    set -euo pipefail

    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="id"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi

    case "$subcommand" in
        id)
            echo "Program ID: $(just get-program-id)"
            ;;
        init)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                component=""
            else
                component=${remaining_args_array[0]}
                # Remove component from args for passing to subcommands
                init_remaining_args="${remaining_args_array[@]:1}"
            fi

            case "$component" in
                protocol)
                    # Initialize Feels Protocol (deploy ProtocolConfig account with 0.3% default fee)
                    PROGRAM_ID=$(just get-program-id)
                    TREASURY=$(solana address)
                    RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"

                    echo "Initializing Feels Protocol..."
                    echo "  Program ID: $PROGRAM_ID"
                    echo "  Treasury: $TREASURY"
                    echo "  RPC URL: $RPC_URL"
                    echo ""

                    feels init protocol \
                        --treasury "$TREASURY" \
                        --base-fee-bps 30 \
                        --program-id "$PROGRAM_ID" \
                        --rpc-url "$RPC_URL"
                    ;;
                hub)
                    # Initialize FeelsSOL hub (requires JITOSOL_MINT environment variable)
                    if [[ -z "${JITOSOL_MINT:-}" ]]; then
                        echo "Error: JITOSOL_MINT environment variable not set"
                        echo ""
                        echo "Usage:"
                        echo "  export JITOSOL_MINT=<jitosol-mint-address>"
                        echo "  just program init hub"
                        echo ""
                        echo "Or provide as environment variable:"
                        echo "  JITOSOL_MINT=<address> just program init hub"
                        exit 1
                    fi

                    PROGRAM_ID=$(just get-program-id)
                    RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"

                    echo "Initializing FeelsSOL Hub..."
                    echo "  Program ID: $PROGRAM_ID"
                    echo "  JitoSOL Mint: $JITOSOL_MINT"
                    echo "  RPC URL: $RPC_URL"
                    echo ""

                    feels init hub \
                        --jitosol-mint "$JITOSOL_MINT" \
                        --program-id "$PROGRAM_ID" \
                        --rpc-url "$RPC_URL"
                    ;;
                market)
                    # Initialize a new market (requires token mint address)
                    init_remaining_args_array=($init_remaining_args)
                    if [[ ${#init_remaining_args_array[@]} -eq 0 ]]; then
                        echo "Error: Token mint address required"
                        echo ""
                        echo "Usage:"
                        echo "  just program init market <token_mint>"
                        exit 1
                    fi

                    TOKEN_MINT=${init_remaining_args_array[0]}
                    PROGRAM_ID=$(just get-program-id)
                    RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"

                    echo "Initializing Market..."
                    echo "  Program ID: $PROGRAM_ID"
                    echo "  Token Mint: $TOKEN_MINT"
                    echo "  RPC URL: $RPC_URL"
                    echo ""

                    feels init market \
                        --token-mint "$TOKEN_MINT" \
                        --program-id "$PROGRAM_ID" \
                        --rpc-url "$RPC_URL"
                    ;;
                "")
                    echo "Error: No component specified for init"
                    echo ""
                    echo "Available init components:"
                    echo "  protocol - Initialize ProtocolConfig account"
                    echo "  hub      - Initialize FeelsSOL hub"
                    echo "  market   - Initialize a new market"
                    echo ""
                    echo "Usage:"
                    echo "  just program init protocol"
                    echo "  just program init hub"
                    echo "  just program init market <token_mint>"
                    exit {{EXIT_GENERAL_ERROR}}
                    ;;
                *)
                    echo "Error: Unknown init component: $component"
                    echo ""
                    echo "Available init components:"
                    echo "  protocol - Initialize ProtocolConfig account"
                    echo "  hub      - Initialize FeelsSOL hub"
                    echo "  market   - Initialize a new market"
                    echo ""
                    echo "Usage:"
                    echo "  just program init protocol"
                    echo "  just program init hub"
                    echo "  just program init market <token_mint>"
                    exit {{EXIT_GENERAL_ERROR}}
                    ;;
            esac
            ;;
        keypair)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                echo "Deployed keypair location: {{DEPLOY_PATH}}/feels-keypair.json"
                if [[ -f "{{DEPLOY_PATH}}/feels-keypair.json" ]]; then
                    echo "Keypair exists: YES"
                    echo "Public key: $(just get-program-id)"
                else
                    echo "Keypair exists: NO"
                fi
            else
                action=${remaining_args_array[0]}
                case "$action" in
                    show)
                        if [[ -f "{{DEPLOY_PATH}}/feels-keypair.json" ]]; then
                            echo "Keypair contents:"
                            cat "{{DEPLOY_PATH}}/feels-keypair.json"
                        else
                            echo "No deployed keypair found"
                        fi
                        ;;
                    pubkey)
                        echo "$(just get-program-id)"
                        ;;
                    *)
                        echo "Error: Unknown keypair action: $action"
                        echo "Available actions: show, pubkey"
                        exit {{EXIT_GENERAL_ERROR}}
                        ;;
                esac
            fi
            ;;
        authority)
            echo "Finding program authority keypair..."
            authority_keypair=$(just find-authority-keypair)
            if [[ -n "$authority_keypair" ]]; then
                echo "Authority keypair: $authority_keypair"
                if [[ -f "$authority_keypair" ]]; then
                    pubkey=$(solana-keygen pubkey "$authority_keypair" 2>/dev/null || echo "Unable to read")
                    echo "Authority pubkey: $pubkey"
                else
                    echo "Keypair file not found"
                fi
            else
                echo "No authority keypair found"
            fi
            ;;
        info)
            echo "Program Information:"
            echo "==================="
            echo "Program ID: $(just get-program-id)"
            echo "Program binary: {{DEPLOY_PATH}}/feels.so"
            if [[ -f "{{DEPLOY_PATH}}/feels.so" ]]; then
                echo "Binary exists: YES"
                binary_size=$(stat -f%z "{{DEPLOY_PATH}}/feels.so" 2>/dev/null || stat -c%s "{{DEPLOY_PATH}}/feels.so" 2>/dev/null || echo "0")
                echo "Binary size: $binary_size bytes"
            else
                echo "Binary exists: NO"
            fi
            echo "IDL file: {{PROJECT_ROOT}}/target/idl/feels.json"
            if [[ -f "{{PROJECT_ROOT}}/target/idl/feels.json" ]]; then
                echo "IDL exists: YES"
            else
                echo "IDL exists: NO"
            fi
            ;;
        *)
            echo "Error: Unknown program subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  id         - Show program ID (default)"
            echo "  init       - Initialize protocol components [protocol|hub|market]"
            echo "  keypair    - Show deployed keypair information [show|pubkey]"
            echo "  authority  - Show program authority information"
            echo "  info       - Show comprehensive program information"
            echo ""
            echo "Usage:"
            echo "  just program [subcommand]"
            echo "  just program init <component>"
            echo "  just program keypair [action]"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# WORKSPACE MANAGEMENT
# ═══════════════════════════════════════════════════════════════════════════

# Workspace management and cleanup
workspace *args:
    #!/usr/bin/env bash
    set -euo pipefail

    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="clean"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi

    case "$subcommand" in
        clean)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                target="all"
            else
                target=${remaining_args_array[0]}
            fi
            just clean "$target"
            ;;
        reset)
            just reset
            ;;
        status)
            echo "Workspace Status:"
            echo "================="
            echo "Project root: {{PROJECT_ROOT}}"
            echo "Target directory: $(if [[ -d target/ ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Anchor artifacts: $(if [[ -d .anchor/ ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "IDL file: $(if [[ -f target/idl/feels.json ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Program binary: $(if [[ -f target/deploy/feels.so ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Frontend build: $(if [[ -d {{FEELS_APP_PATH}}/.next ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Indexer build: $(if [[ -d feels-indexer/target ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Test ledger: $(if [[ -d {{TEST_LEDGER_PATH}} ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            ;;
        info)
            echo "Workspace Information:"
            echo "====================="
            echo "Project name: Feels Protocol"
            echo "Project root: {{PROJECT_ROOT}}"
            echo "Build targets:"
            echo "  - feels program: programs/feels/"
            echo "  - indexer: feels-indexer/"
            echo "  - frontend: feels-app/"
            echo "  - jupiter adapter: feels-jupiter-adapter/"
            echo "Generated artifacts:"
            echo "  - Program binary: target/deploy/feels.so"
            echo "  - IDL: target/idl/feels.json"
            echo "  - TypeScript client: generated-sdk/feels.ts"
            echo "  - Rust client: generated-sdk/rust/src/lib.rs"
            ;;
        *)
            echo "Error: Unknown workspace subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  clean    - Clean workspace artifacts [all|feels|indexer|frontend|nix]"
            echo "  reset    - Reset complete development environment"
            echo "  status   - Show workspace build status"
            echo "  info     - Show workspace information"
            echo ""
            echo "Usage:"
            echo "  just workspace [subcommand]"
            echo "  just workspace clean [target]"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac

# ═══════════════════════════════════════════════════════════════════════════
# LOCALNET MANAGEMENT
# ═══════════════════════════════════════════════════════════════════════════

# Local validator and development network management
localnet *args:
    #!/usr/bin/env bash
    set -euo pipefail

    args_array=({{args}})
    if [[ ${#args_array[@]} -eq 0 ]]; then
        subcommand="start"
    else
        subcommand=${args_array[0]}
        remaining_args="${args_array[@]:1}"
    fi

    case "$subcommand" in
        start)
            just validator
            ;;
        stop)
            just stop-localnet
            ;;
        restart)
            just stop-localnet
            sleep 2
            just validator
            ;;
        setup)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                setup_type="metaplex"
            else
                setup_type=${remaining_args_array[0]}
            fi
            case "$setup_type" in
                metaplex)
                    just setup-metaplex
                    ;;
                *)
                    echo "Error: Unknown setup type: $setup_type"
                    echo "Available setup types: metaplex"
                    exit {{EXIT_GENERAL_ERROR}}
                    ;;
            esac
            ;;
        status)
            echo "Localnet Status:"
            echo "==============="
            if just check-service "{{LOCALNET_RPC}}" "validator" 2>/dev/null; then
                echo "Validator: RUNNING"
                echo "RPC endpoint: {{LOCALNET_RPC}}"
                echo "WebSocket endpoint: ws://localhost:{{VALIDATOR_WS_PORT}}"
            else
                echo "Validator: STOPPED"
            fi
            echo "Test ledger: $(if [[ -d {{TEST_LEDGER_PATH}} ]]; then echo "EXISTS"; else echo "NOT FOUND"; fi)"
            echo "Log file: {{LOGS_PATH}}/validator.log"
            ;;
        logs)
            if [[ -f "{{LOGS_PATH}}/validator.log" ]]; then
                echo "Validator logs (press Ctrl+C to exit):"
                echo "======================================"
                tail -f "{{LOGS_PATH}}/validator.log" 2>/dev/null
            else
                echo "No validator log found. Start the validator first with: just localnet start"
            fi
            ;;
        airdrop)
            remaining_args_array=($remaining_args)
            if [[ ${#remaining_args_array[@]} -eq 0 ]]; then
                amount="10"
            else
                amount=${remaining_args_array[0]}
            fi
            echo "Airdropping $amount SOL to default wallet..."
            solana airdrop "$amount"
            ;;
        info)
            echo "Localnet Information:"
            echo "===================="
            echo "RPC URL: {{LOCALNET_RPC}}"
            echo "WebSocket URL: ws://localhost:{{VALIDATOR_WS_PORT}}"
            echo "Bind address: {{VALIDATOR_BIND_ADDRESS}}"
            echo "Port range: {{VALIDATOR_PORT_RANGE}}"
            echo "Test ledger path: {{TEST_LEDGER_PATH}}"
            echo "Logs path: {{LOGS_PATH}}/validator.log"
            if just check-service "{{LOCALNET_RPC}}" "validator" 2>/dev/null; then
                echo "Cluster version: $(solana cluster-version 2>/dev/null || echo 'Unable to fetch')"
                echo "Slot: $(solana slot 2>/dev/null || echo 'Unable to fetch')"
            fi
            ;;
        clean)
            echo "Cleaning localnet data..."
            # Stop validator first if running
            if just check-service "{{LOCALNET_RPC}}" "validator" 2>/dev/null; then
                echo "Stopping validator..."
                just stop-localnet
                sleep 2
            fi
            # Clean localnet directories
            rm -rf {{TEST_LEDGER_PATH}}/
            rm -rf {{LOGS_PATH}}/*
            rm -rf {{PROJECT_ROOT}}/localnet/data/*
            rm -rf {{PROJECT_ROOT}}/localnet/indexer-storage/*
            echo "Localnet data cleaned successfully"
            ;;
        *)
            echo "Error: Unknown localnet subcommand: $subcommand"
            echo ""
            echo "Available subcommands:"
            echo "  start     - Start local validator (default)"
            echo "  stop      - Stop local validator"
            echo "  restart   - Restart local validator"
            echo "  setup     - Setup external programs [metaplex]"
            echo "  status    - Show validator status"
            echo "  logs      - View validator logs"
            echo "  airdrop   - Airdrop SOL to wallet [amount]"
            echo "  info      - Show localnet configuration"
            echo "  clean     - Clean localnet data directories"
            echo ""
            echo "Usage:"
            echo "  just localnet [subcommand]"
            echo "  just localnet airdrop [amount]"
            echo "  just localnet setup [type]"
            exit {{EXIT_GENERAL_ERROR}}
            ;;
    esac
