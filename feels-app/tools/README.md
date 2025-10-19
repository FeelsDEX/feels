# Feels App Development Tools

This directory contains frontend-specific development tools for the Feels Protocol application.

## Directory Structure

```
tools/
├── debug/          # Debugging and diagnostic utilities (TypeScript)
└── devbridge/      # WebSocket development bridge for CLI interaction
```

## Protocol Setup

All protocol setup operations are handled by the Rust SDK CLI (`feels`) via justfiles. From the repository root:

```bash
# Complete E2E environment (validator, indexer, frontend, protocol setup)
just e2e::run

# Individual setup commands
just e2e::complete-protocol-setup   # Protocol + hub initialization
just e2e::initialize-protocol        # Protocol only
```

Configuration files are generated at `/localnet/config/`:
- **localnet-tokens.json** - Token mint addresses
- **localnet-protocol.json** - Protocol deployment state
- **localnet-metaplex.json** - Metaplex configuration

## Debug Tools

Located in `tools/debug/`:

All debug scripts are written in TypeScript for type safety:

- **check-idl.ts** - Verify IDL file structure and contents
  ```bash
  npm run debug:idl
  ```

- **check-protocol.ts** - Check protocol deployment and configuration
  ```bash
  npm run debug:protocol
  ```

- **get-declared-id.ts** - Debug program ID mismatches
  ```bash
  npm run debug:program-id
  ```

- **test-axis-browser.ts** - Browser console script for testing chart axis switching (Linear/Log)
  This TypeScript file contains code to paste directly into the browser console for debugging chart axis functionality.

## DevBridge

Located in `tools/devbridge/`:

A WebSocket-based development tool that enables CLI interaction with the browser. Useful for debugging and development workflows.

**Usage:**
```bash
# Start DevBridge CLI
npm run devbridge

# Listen to browser console logs in real-time
tsx tools/devbridge/listen-devbridge.ts
```

The **listen-devbridge.ts** utility connects to the DevBridge WebSocket server and displays browser console logs with timestamps and highlighting for axis-related messages.

See [DevBridge README](./devbridge/README.md) for detailed documentation.

## Environment Variables

Frontend tools respect these environment variables:

- `SOLANA_WALLET` - Path to wallet keypair (defaults to `~/.config/solana/id.json`)
- `FEELS_PROGRAM_ID` - Feels Protocol program ID
- `NEXT_PUBLIC_FEELS_PROGRAM_ID` - Frontend program ID configuration

## Local Development Workflow

1. **Start complete E2E environment** (from root directory):
   ```bash
   just e2e::run
   ```
   This starts the validator, deploys the program, runs protocol setup, and starts the frontend.

2. **Or start services individually**:
   ```bash
   # Start validator only
   just e2e::start-validator
   
   # Deploy and setup protocol
   just e2e::deploy-e2e
   just e2e::complete-protocol-setup
   
   # Start frontend
   just e2e::start-app
   ```

3. **Access the app** at http://localhost:3000

## Integration with Justfile System

All protocol operations use the justfile system from the repository root:

```bash
# Complete E2E environment
just e2e::run

# Frontend operations
just frontend::dev
just frontend::install
just frontend::build

# Protocol operations (via Rust SDK)
just e2e::complete-protocol-setup
just e2e::initialize-protocol
```

## Notes

- Protocol setup now uses the **Rust SDK CLI** (`feels`) for reliability and type safety
- All TypeScript setup scripts have been removed in favor of Rust CLI
- Configuration files are generated in `/localnet/config/` directory
- Debug tools help troubleshoot common issues with IDL, protocol state, and program IDs
- Test utilities are for frontend development and debugging real-time features
- DevBridge enables CLI interaction with the browser for advanced debugging workflows
