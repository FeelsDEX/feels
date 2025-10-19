# Feels Protocol - Unified Trading Interface

A Next.js application providing a unified trading interface for the Feels Protocol concentrated liquidity AMM, featuring Jupiter aggregation integration and real-time indexer data.

## Overview

This application serves as the primary frontend for the Feels Protocol ecosystem, combining Jupiter's cross-DEX aggregation with Feels Protocol's concentrated liquidity pools to enable seamless multi-hop trading from any Solana token to meme coins via JitoSOL and FeelsSOL.

### Key Features

- **Unified Swap Interface** - Automatic route detection for optimal trading paths
- **Jupiter Integration** - Cross-DEX aggregation for best token prices
- **Real-time Data** - Live protocol statistics via Feels indexer
- **Intelligent Routing** - Automatic detection of optimal swap routes
- **Multi-hop Trading** - Complete trading chains: Any Token → JitoSOL → FeelsSOL → Meme Coins
- **Wallet Integration** - Support for all major Solana wallets
- **Professional UI** - Built with shadcn/ui and Tailwind CSS

## Project Structure

```
feels-app/
├── src/
│   ├── app/                    # Next.js App Router
│   ├── components/             # React components
│   │   ├── ui/                 # shadcn/ui components
│   │   ├── common/             # Common components
│   │   ├── market/             # Market-related components
│   │   ├── trading/            # Trading interface
│   │   ├── search/             # Search components
│   │   └── wallet/             # Wallet integration
│   ├── services/               # API clients
│   │   ├── indexer-client.ts   # Indexer API
│   │   ├── jupiter-client.ts   # Jupiter API
│   │   └── connection.ts       # Solana connection
│   ├── sdk/                    # Protocol SDK wrappers
│   │   ├── sdk.ts              # Feels Protocol SDK
│   │   └── program-workaround.ts # Anchor compatibility workaround
│   ├── hooks/                  # React hooks
│   ├── utils/                  # Utility functions
│   │   └── webpack/            # Webpack-specific workarounds
│   │       ├── ws-mock.js      # Mock for ws module (SSR)
│   │       ├── mermaid-mock.js # Mock for mermaid (SSR)
│   │       └── chunk-fix-plugin.js # Chunk loading fix
│   ├── constants/              # Application constants and config
│   │   ├── protocol.ts         # Protocol constants (program IDs, PDAs)
│   │   ├── localnet.ts         # Localnet token addresses
│   │   ├── app.ts              # App configuration
│   │   └── mock-tokens.ts      # Mock token data for UI
│   ├── types/                  # TypeScript type definitions
│   └── idl/                    # Anchor IDL files
├── tools/                      # Frontend development tools
│   ├── debug/                  # Debugging utilities
│   ├── test/                   # Frontend testing scripts
│   └── devbridge/              # CLI development bridge
├── components.json             # shadcn/ui configuration
├── tailwind.config.js          # Tailwind + shadcn/ui config
├── tsconfig.json              # TypeScript configuration
└── package.json               # Dependencies and scripts
```

## Directory Structure

### Constants (`src/constants/`)

All application constants consolidated in one location:

- **protocol.ts** - Protocol constants (program IDs, PDA seeds, market parameters)
- **localnet.ts** - Functions to get localnet token addresses and Metaplex program ID
- **app.ts** - App runtime configuration (default swap token, etc.)
- **mock-tokens.ts** - Mock token data for UI development and testing

### Localnet Configuration

Runtime configuration files are generated at `/localnet/config/` (in the repository root):

- **localnet-tokens.json** - Token mint addresses for local development (JitoSOL, FeelsSOL)
- **localnet-metaplex.json** - Metaplex Token Metadata program configuration  
- **localnet-protocol.json** - Complete protocol setup state (program IDs, mints, test tokens, markets)

These files are generated automatically by the Rust SDK CLI (`feels`) via justfiles and should not be manually edited.

## Tools Directory

The `tools/` directory contains frontend-specific development utilities:

### Debug Scripts (`tools/debug/`)
TypeScript utilities for troubleshooting:
- **check-idl.ts** - Verify IDL file structure
- **check-protocol.ts** - Check protocol deployment and configuration
- **get-declared-id.ts** - Debug program ID mismatches

### Test Scripts (`tools/test/`)
Frontend testing and debugging utilities for real-time features:
- WebSocket connection testing
- Chart axis testing
- DevBridge logging

### DevBridge (`tools/devbridge/`)
WebSocket-based development tool for CLI interaction with the browser. Enables real-time log streaming and command execution for debugging.

For detailed documentation, see [tools/README.md](tools/README.md).

### Protocol Setup

All protocol setup operations are now handled by the Rust SDK CLI (`feels`) via justfiles at the repository root. TypeScript setup scripts have been removed in favor of the more reliable Rust implementation.

## Getting Started

### Prerequisites

- Node.js 20+
- pnpm (recommended) or npm
- Solana wallet browser extension

### Development Setup

#### Option 1: Using Nix (Recommended)

```bash
# Enter the development environment
nix develop

# Install dependencies
cd feels-app && pnpm install

# Start development server
pnpm dev
```

#### Option 2: Traditional Setup

```bash
# Navigate to app directory
cd feels-app

# Install dependencies
pnpm install

# Start development server
pnpm dev

# Build for production
pnpm build
```

### Environment Configuration

Create `.env.local` in the feels-app directory. The following examples cover different deployment scenarios:

#### Localnet Development

```env
# Solana Network Configuration
NEXT_PUBLIC_SOLANA_NETWORK=localnet
NEXT_PUBLIC_SOLANA_RPC_URL=http://localhost:8899

# Feels Protocol Configuration
NEXT_PUBLIC_FEELS_PROGRAM_ID=YourProgramIdHere

# Token Mints (set after running setup for localnet)
NEXT_PUBLIC_JITOSOL_MINT=
NEXT_PUBLIC_FEELSSOL_MINT=

# Indexer Configuration
NEXT_PUBLIC_INDEXER_API_URL=http://localhost:8080

# Jupiter Configuration
NEXT_PUBLIC_JUPITER_API_URL=https://quote-api.jup.ag/v6

# DevBridge Configuration (development only)
DEVBRIDGE_ENABLED=false
NEXT_PUBLIC_DEVBRIDGE_ENABLED=false
```

#### Devnet/Mainnet Deployment

```env
# Solana Network Configuration
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com

# Feels Protocol Configuration
NEXT_PUBLIC_FEELS_PROGRAM_ID=YourProgramIdHere

# Indexer Configuration
NEXT_PUBLIC_INDEXER_API_URL=https://your-indexer-url.com

# Jupiter Configuration
NEXT_PUBLIC_JUPITER_API_URL=https://quote-api.jup.ag/v6
```

#### IPFS/Pinata Configuration (for metadata uploads)

```env
# IPFS/Pinata Configuration
PINATA_JWT=your_jwt_here
UPLOAD_CACHE_TTL=1800000  # 30 minutes in ms
RATE_LIMIT_WINDOW=60000   # 1 minute
RATE_LIMIT_MAX=10         # max uploads per window
```

## Development Commands

```bash
# Development
pnpm dev              # Build content and start dev server
pnpm dev:watch        # Watch content files and auto-rebuild
pnpm dev:all          # Start everything (content + dev server + devbridge)
pnpm build            # Build content and Next.js for production
pnpm start            # Start production server

# Code Quality
pnpm lint             # Run ESLint
pnpm type-check       # TypeScript type checking
pnpm format           # Format with Prettier

# DevBridge (development debugging tool)
pnpm devbridge        # Start DevBridge CLI
pnpm dev:bridge       # Start DevBridge server
tsx tools/devbridge/listen-devbridge.ts  # Listen to browser console logs

# Debugging utilities
npm run debug:idl        # Debug IDL issues
npm run debug:protocol   # Check protocol state
npm run debug:program-id # Check program ID mismatches

# Protocol setup is now via justfiles at repo root
# From repository root: just e2e::run
```

## Technical Details

### Workaround Files

The codebase includes several workaround files for compatibility:

**src/sdk/program-workaround.ts**
- Workaround for Anchor 0.31.1 account parsing bugs
- Strips account definitions from IDL to avoid deserialization errors
- Will be removed when Anchor fixes upstream issues

**src/utils/webpack/** (Webpack-specific workarounds)
- **ws-mock.js** - Mock for Node.js 'ws' module (prevents bundling server-only code)
- **mermaid-mock.js** - Mock for mermaid library (prevents SSR errors)
- **chunk-fix-plugin.js** - Fixes chunk loading errors with Solana dependencies

These files are necessary for current dependencies and will be removed when upstream fixes are available.

## Related Components

- **[Feels Protocol Core](../programs/feels/)** - Main Solana program
- **[Feels Indexer](../feels-indexer/)** - Real-time data indexing
- **[Feels SDK](../feels-sdk/)** - Rust SDK for protocol interaction
- **[Jupiter Adapter](../feels-jupiter-adapter/)** - Jupiter integration layer

For more information about the complete Feels Protocol ecosystem, see the [main repository README](../README.md).
