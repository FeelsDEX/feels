# Feels App Scripts

This directory contains utility scripts for managing and debugging the Feels Protocol frontend.

## Setup Scripts

- **initialize-protocol.js** - Initialize the Feels Protocol (run once before any operations)
  ```bash
  npm run init:protocol
  ```

- **initialize-hub.js** - Initialize the FeelsSOL hub
  ```bash
  npm run init:hub
  ```

- **setup-jitosol.js** - Set up JitoSOL for local development
  ```bash
  npm run setup:jitosol
  ```

- **mint-jitosol.js/ts** - Mint JitoSOL tokens for testing
  ```bash
  npm run mint:jitosol
  ```

- **setup-user-feelssol.js** - Set up user with FeelsSOL tokens
  ```bash
  node scripts/setup-user-feelssol.js
  ```

- **enter-feelssol.js** - Convert JitoSOL to FeelsSOL
  ```bash
  node scripts/enter-feelssol.js
  ```

## Market Creation

- **create-feelssol-mint.js** - Create the FeelsSOL mint
- **create-test-market.js** - Create test markets for development

## Metaplex Integration

- **deploy-metaplex-localnet.sh** - Deploy Metaplex to local network
- **download-and-deploy-metaplex.sh** - Download and deploy Metaplex
- **generate-metaplex-keypair.js** - Generate Metaplex keypair
- **find-metaplex-keypair.js** - Find existing Metaplex keypair
- **test-metaplex-keypair.js** - Test Metaplex keypair

## Development Tools

- **reset-localnet.js** - Reset local Solana validator
  ```bash
  node scripts/reset-localnet.js
  ```

## Debug Scripts (in `debug/` subdirectory)

- **check-idl.js** - Validate IDL file structure
  ```bash
  npm run debug:idl
  ```

- **check-protocol.js** - Check protocol account state
  ```bash
  npm run debug:protocol
  ```

- **get-declared-id.js** - Debug program ID mismatches
  ```bash
  npm run debug:program-id
  ```

## Configuration Files

- **localnet-tokens.json** - Token configuration for local development
- **metaplex-localnet.json** - Metaplex configuration for local network