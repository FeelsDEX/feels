# Feels App Development Tools

This directory contains development and setup tools for the Feels Protocol frontend application.

## Directory Structure

```
tools/
├── debug/          # Debugging utilities
├── devbridge/      # WebSocket development bridge for CLI interaction
└── setup/          # Protocol and token setup scripts
```

## Debug Tools

Located in `tools/debug/`:

- **check-idl.ts** - Verify IDL file structure and contents
- **check-protocol.ts** - Check protocol deployment and configuration
- **get-declared-id.ts** - Debug program ID mismatches

Usage:
```bash
npm run debug:idl
npm run debug:protocol
npm run debug:program-id
```

## DevBridge

Located in `tools/devbridge/`:

A WebSocket-based development tool that enables CLI interaction with the browser. Useful for debugging and development workflows.

Usage:
```bash
npm run devbridge
```

See [DevBridge README](./devbridge/README.md) for detailed documentation.

## Setup Tools

Located in `tools/setup/`:

### Token Setup

1. **setup-jitosol.ts** - Creates mock JitoSOL and FeelsSOL mints for localnet testing
   ```bash
   npm run setup:jitosol
   # Or directly:
   tsx tools/setup/setup-jitosol.ts [rpc-url]
   ```
   
   After running, set the output mint addresses in your `.env.local`:
   ```env
   NEXT_PUBLIC_JITOSOL_MINT=<output-jitosol-mint>
   NEXT_PUBLIC_FEELSSOL_MINT=<output-feelssol-mint>
   ```

2. **mint-jitosol.ts** - Mint test JitoSOL to any wallet
   ```bash
   npm run mint:jitosol <jitosol-mint> <wallet-address> <amount> [rpc-url]
   # Example:
   npm run mint:jitosol BatGa... 7EL1Td... 100
   ```

### Protocol Initialization

1. **initialize-protocol.ts** - Initialize the Feels Protocol
   ```bash
   npm run init:protocol <program-id> <jitosol-mint> [rpc-url]
   # Example:
   npm run init:protocol 9dGWD... BatGa...
   ```

2. **initialize-hub.ts** - Initialize the FeelsSOL hub
   ```bash
   npm run init:hub <program-id> <feelssol-mint> [rpc-url]
   # Example:
   npm run init:hub GfEnp... AVipL...
   ```

## Environment Variables

All tools respect these environment variables:

- `SOLANA_WALLET` - Path to wallet keypair (defaults to `~/.config/solana/id.json`)
- `FEELS_PROGRAM_ID` - Feels Protocol program ID
- `NEXT_PUBLIC_FEELS_PROGRAM_ID` - Frontend program ID configuration
- `NEXT_PUBLIC_JITOSOL_MINT` - JitoSOL mint address (set after running setup)
- `NEXT_PUBLIC_FEELSSOL_MINT` - FeelsSOL mint address (set after running setup)

## Local Development Workflow

1. Start local validator (from root directory):
   ```bash
   just local-devnet
   ```

2. Setup tokens:
   ```bash
   npm run setup:jitosol
   # Copy output mint addresses to .env.local
   ```

3. Initialize protocol:
   ```bash
   npm run init:protocol <program-id> <jitosol-mint>
   npm run init:hub <program-id> <feelssol-mint>
   ```

4. Start the app:
   ```bash
   npm run dev
   ```

## Notes

- All scripts are written in TypeScript for type safety
- No persistent JSON configuration files - everything uses command line args and environment variables
- Scripts will output the values you need to set in your environment
- Always run setup scripts before starting the app on localnet