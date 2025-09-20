# E2E Local Development Environment

This directory contains the complete end-to-end development environment for Feels Protocol, managed through a unified justfile system.

## Overview

The E2E environment provides a complete local development setup that includes:

1. **Solana Test Validator** - Local blockchain instance
2. **Feels Program** - The main protocol smart contract
3. **Streaming Adapter** - Simulates Geyser/Fumarole for real-time data streaming
4. **Indexer** - Processes and stores blockchain data
5. **Frontend Application** - Next.js app with full protocol interface

## Quick Start

### From Project Root

```bash
# Start the complete E2E environment
just dev-e2e

# Check status of all services
just dev-e2e-status

# View logs
just dev-e2e-logs           # Show available logs
just dev-e2e-logs validator # View specific service logs

# Stop all services
just dev-e2e-stop
```

### From E2E Directory

```bash
cd e2e

# See all available commands
just

# Start everything
just run

# Check status
just status
```

## Available Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `just run` | Start the complete E2E environment with all services |
| `just stop` | Stop all running E2E services |
| `just status` | Check the status of all services |
| `just check` | Quick status check with helpful hints |
| `just logs [service]` | View logs (validator/streaming/indexer/app/all) |

### Individual Components

| Command | Description |
|---------|-------------|
| `just build-localnet` | Build program with localnet feature (custom Metaplex ID) |
| `just validator` | Start Solana test validator only |
| `just deploy` | Build and deploy program to localnet |
| `just streaming` | Start streaming adapter only |
| `just indexer` | Start indexer service only |
| `just app` | Start frontend application only |

### Setup Commands

| Command | Description |
|---------|-------------|
| `just setup-metaplex` | Deploy Metaplex Token Metadata program |
| `just setup-tokens` | Setup JitoSOL and FeelsSOL tokens |
| `just init-protocol` | Initialize the Feels Protocol |
| `just idl-generate` | Generate and copy IDL to frontend |

## Architecture

```
┌─────────────────┐
│ Solana Validator│
│ localhost:8899  │
└────────┬────────┘
         │ RPC calls
         ▼
┌─────────────────┐     SSE Stream    ┌──────────────────┐
│Streaming Adapter│─────────────────▶│  Feels Indexer   │
│ localhost:10000 │                  │ localhost:8080   │
└─────────────────┘                  └──────────────────┘
                                              │ API
                                              ▼
                                     ┌──────────────────┐
                                     │   Next.js App    │
                                     │ localhost:3000   │
                                     └──────────────────┘
```

## Components

### minimal-streaming-adapter/

A lightweight Rust service that:
- Polls the Solana RPC for program accounts and slot updates
- Streams data via Server-Sent Events (SSE) 
- Simulates Geyser plugin functionality for local development
- Configurable polling interval and program filtering

### justfile

The main orchestration tool that provides:
- **DRY Implementation**: No duplicate code between commands
- **Automatic Health Checks**: Waits for services to be ready
- **Colored Output**: Clear visual feedback (green=success, yellow=warning, red=error)
- **Error Handling**: Proper error propagation and cleanup
- **Flexible Execution**: Run all services or individual components
- **Smart Defaults**: Automatically finds program binaries and handles different build outputs

## Configuration

### Environment Variables

- `FEELS_PROGRAM_ID` - Override the default program ID
- `METAPLEX_ID` - Custom Metaplex Token Metadata program ID (auto-detected from config)

### Program ID Management

The E2E environment automatically:
1. Generates a program keypair if none exists
2. Updates `declare_id!` in the source code
3. Deploys to the generated program address
4. Propagates the ID to all services

### Metaplex Integration

When `feels-app/scripts/metaplex-localnet.json` exists:
- Builds with `localnet` feature flag
- Uses custom Metaplex program ID
- Enables NFT position tracking features

## Logs

All services output logs to the `logs/` directory:

```
logs/
├── validator.log        # Solana validator output
├── build.log           # Program compilation logs
├── streaming-adapter.log # Streaming service logs  
├── indexer.log         # Indexer service logs
├── app.log             # Frontend application logs
├── metaplex-setup.log  # Metaplex deployment logs
├── token-setup.log     # Token creation logs
└── protocol-init.log   # Protocol initialization logs
```

View logs with:
```bash
just logs validator   # Tail specific log
just logs            # List all available logs
```

## Troubleshooting

### Build Failures

The build system tries multiple strategies in order:
1. Localnet build with custom Metaplex (if configured)
2. Standard Nix build via `just build`
3. Anchor build with reduced optimization flags

Check `logs/build.log` for detailed error messages.

### Service Not Starting

1. Check service status: `just status`
2. View service logs: `just logs [service]`
3. Ensure ports are free:
   - 8899 (Solana RPC)
   - 10000 (Streaming adapter)
   - 8080 (Indexer API)
   - 3000 (Frontend app)

### Stack Size Errors

If you see "Stack offset exceeded" errors during build:
- These are warnings and can often be ignored for local development
- The build system automatically tries reduced optimization levels
- For production builds, refactor large functions to reduce stack usage

### Cleanup

To fully reset the environment:
```bash
just stop                    # Stop all services
rm -rf logs/ test-ledger/    # Remove logs and blockchain data
rm -rf ~/.cache/solana/      # Clear Solana cache (optional)
```

## Development Workflow

1. **Start E2E**: `just dev-e2e`
2. **Make changes** to your code
3. **Rebuild & Deploy**: 
   ```bash
   just -f e2e/justfile deploy     # Rebuild and deploy program
   just -f e2e/justfile app         # Restart frontend if needed
   ```
4. **Check logs**: `just dev-e2e-logs [service]`
5. **Stop when done**: `just dev-e2e-stop`

## Tips

- Run individual services for faster iteration during development
- Use `just status` to quickly check what's running
- The streaming adapter simulates real Geyser plugins - useful for testing indexer logic
- All commands can be run from either the project root or e2e directory
- Services are started with health checks - the next service won't start until the previous is ready