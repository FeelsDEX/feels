# Nix environment variable bridge & Constants
# This module provides a bridge between Nix-defined configuration and justfiles
# and defines all constants used across the justfile system

# ===== Exit Codes =====
# These remain constant regardless of environment
export EXIT_SUCCESS := "0"
export EXIT_GENERAL_ERROR := "1"
export EXIT_MISSING_DEPS := "2"
export EXIT_BUILD_FAILURE := "3"
export EXIT_DEPLOY_FAILURE := "4"
export EXIT_TEST_FAILURE := "5"
export EXIT_CONFIG_ERROR := "10"
export EXIT_ENV_SETUP_ERROR := "11"
export EXIT_SERVICE_START_FAILURE := "12"

# ===== Colors =====
# Terminal colors for output
export GREEN := '\033[0;32m'
export YELLOW := '\033[1;33m'
export RED := '\033[0;31m'
export NC := '\033[0m' # No Color

# ===== Path Configuration =====
# Use Nix-provided paths if available, otherwise fall back to defaults

# Project paths - use invocation directory as root
export PROJECT_ROOT := env_var_or_default("NIX_PROJECT_ROOT", invocation_directory())
export PROGRAMS_PATH := env_var_or_default("NIX_PROGRAMS_PATH", PROJECT_ROOT + "/programs")

# Build artifacts
export TARGET_PATH := env_var_or_default("NIX_TARGET_PATH", PROJECT_ROOT + "/target")
export DEPLOY_PATH := env_var_or_default("NIX_DEPLOY_PATH", TARGET_PATH + "/deploy")
export IDL_PATH := env_var_or_default("NIX_IDL_PATH", TARGET_PATH + "/idl/feels.json")
export TYPES_PATH := env_var_or_default("NIX_TYPES_PATH", TARGET_PATH + "/types")

# Development directories
export LOGS_PATH := env_var_or_default("NIX_LOGS_PATH", PROJECT_ROOT + "/localnet/logs")
export TEST_LEDGER_PATH := env_var_or_default("NIX_LEDGER_PATH", PROJECT_ROOT + "/localnet/validator-ledger")
export KEYPAIRS_PATH := env_var_or_default("NIX_KEYPAIRS_PATH", PROJECT_ROOT + "/localnet/keypairs")
export LOCALNET_CONFIG_PATH := env_var_or_default("NIX_CONFIG_PATH", PROJECT_ROOT + "/localnet/config")

# Application paths
export GENERATED_SDK_PATH := env_var_or_default("NIX_GENERATED_SDK_PATH", PROJECT_ROOT + "/feels-sdk-generated")
export FEELS_APP_PATH := env_var_or_default("NIX_FEELS_APP_PATH", PROJECT_ROOT + "/feels-app")

# ===== Network Configuration =====
# Use Nix-provided network config if available

export VALIDATOR_RPC_PORT := env_var_or_default("NIX_VALIDATOR_RPC_PORT", "8899")
export VALIDATOR_WS_PORT := env_var_or_default("NIX_VALIDATOR_WS_PORT", "8900")
export VALIDATOR_BIND_ADDRESS := env_var_or_default("NIX_VALIDATOR_BIND_ADDRESS", "0.0.0.0")
export VALIDATOR_PORT_RANGE := env_var_or_default("NIX_VALIDATOR_PORT_RANGE", "8000-8020")

# Network endpoints
export LOCALNET_RPC := env_var_or_default("NIX_LOCALNET_RPC", "http://localhost:" + VALIDATOR_RPC_PORT)
export DEVNET_RPC := env_var_or_default("NIX_DEVNET_RPC", "https://api.devnet.solana.com")
export MAINNET_RPC := env_var_or_default("NIX_MAINNET_RPC", "https://api.mainnet-beta.solana.com")

# ===== Service Ports =====
export STREAMING_PORT := env_var_or_default("NIX_STREAMING_PORT", "10000")
export INDEXER_PORT := env_var_or_default("NIX_INDEXER_PORT", "8080")
export APP_PORT := env_var_or_default("NIX_APP_PORT", "3000")

# ===== Program Configuration =====
export DEFAULT_PROGRAM_ID := env_var_or_default("NIX_DEFAULT_PROGRAM_ID", "BLjLS7TzUBncLxXMjFYxezioeg4RVdc5vRpVRYDq8GyQ")
export MAINNET_METAPLEX_ID := env_var_or_default("NIX_MAINNET_METAPLEX_ID", "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")

# ===== Build Configuration =====
export USE_NIX_BUILD := env_var_or_default("NIX_BUILD_ENABLED", "true")
export NIX_IDL_BUILDER := env_var_or_default("NIX_IDL_BUILDER", "true")

# ===== Test Configuration =====
export TEST_THREADS := env_var_or_default("NIX_TEST_THREADS", "1")
export TEST_TIMEOUT := env_var_or_default("NIX_TEST_TIMEOUT", "300")

# ===== Helper Functions =====

# Check if running in Nix environment
[private]
in-nix-env:
    #!/usr/bin/env bash
    if [[ -n "${IN_NIX_SHELL:-}" ]]; then
        echo "true"
    else
        echo "false"
    fi

# Get Nix environment info
# Nix environment info (use 'just nix info' instead)
[private]
nix-info:
    #!/usr/bin/env bash
    echo "Nix Environment Information:"
    echo "=========================="
    if [[ -n "${IN_NIX_SHELL:-}" ]]; then
        echo "Running in Nix shell: YES"
        echo "Nix shell name: ${name:-default}"
        echo ""
        echo "Nix-provided paths:"
        env | grep "^NIX_" | grep -E "(PATH|PORT)" | sort
    else
        echo "Running in Nix shell: NO"
        echo "Using default paths from justfile"
    fi