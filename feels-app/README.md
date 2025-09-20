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

## System Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Feels Protocol Ecosystem                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │   Frontend App  │◄──►│  Feels Indexer  │◄──►│ Solana Chain │ │
│  │   (feels-app)   │    │                 │    │              │ │
│  └─────────────────┘    └─────────────────┘    └──────────────┘ │
│           │                       │                     │       │
│           ▼                       ▼                     ▼       │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │ Jupiter Swap API│    │ PostgreSQL DB   │    │ Feels Program│ │
│  │                 │    │ + Redis Cache   │    │              │ │
│  └─────────────────┘    └─────────────────┘    └──────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **User Interaction** → Frontend App
2. **Token Selection** → Automatic Route Detection
3. **Quote Requests** → Jupiter API + Feels Protocol
4. **Real-time Data** → Feels Indexer (PostgreSQL + Redis)
5. **Transaction Execution** → Solana Blockchain
6. **State Updates** → Indexer → Frontend (real-time)

## Component Interactions

### 1. Feels Indexer Integration

The frontend consumes real-time data from the Feels indexer:

```typescript
// API Client for indexer communication
class IndexerClient {
  async getProtocolStats(): Promise<ProtocolStats>
  async getMarkets(): Promise<Market[]>
  async getMarketData(address: string): Promise<MarketData>
  async getRecentSwaps(market: string): Promise<SwapTransaction[]>
}

// React hooks for data fetching
const { data: protocolStats } = useProtocolStats();
const { data: markets } = useMarkets();
const { data: swaps } = useMarketSwaps(marketAddress);
```

**Indexer Endpoints Used:**
- `GET /api/protocol/stats` - Protocol-wide statistics
- `GET /api/markets` - All available markets
- `GET /api/markets/{address}` - Specific market data
- `GET /api/markets/{address}/swaps` - Recent swap transactions

### 2. Jupiter Integration

Seamless integration with Jupiter's swap aggregation:

```typescript
// Jupiter client for cross-DEX swaps
class JupiterClient {
  async getQuote(inputMint, outputMint, amount, slippage): Promise<Quote>
  async buildSwapTransaction(quote, userPublicKey): Promise<Transaction>
  async executeSwap(params): Promise<string>
}

// Automatic route detection
function detectOptimalRoute(inputToken, outputToken) {
  // Determines if Jupiter, Feels, or both protocols needed
  // Returns optimal execution path automatically
}
```

**Jupiter API Integration:**
- Real-time quotes with 500ms debouncing
- Multi-hop routing for optimal prices
- Slippage protection and price impact warnings
- Priority fee management for faster execution

### 3. Feels Protocol SDK

Direct interaction with Feels Protocol smart contracts:

```typescript
// Generated SDK from Anchor IDL
import { FEELS_IDL, FEELS_PROGRAM_ID } from '@/lib/sdk';

// Program initialization
const program = new Program(FEELS_IDL, provider);

// Protocol interactions
await program.methods.deposit(amount).accounts({
  user: userPublicKey,
  jitosolMint: JITOSOL_MINT,
  feelsSolMint: FEELSSOL_MINT,
  // ... other accounts
}).rpc();
```

## Technical Stack

### Frontend Technologies
- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS + shadcn/ui
- **State Management**: React Hooks, Context, and Tanstack React Query for server state caching
- **Wallet Integration**: Solana Wallet Adapter
- **HTTP Client**: Custom hooks with useState/useEffect for indexer data, React Query setup available

### Blockchain Integration
- **Network**: Solana (Devnet/Mainnet)
- **Wallet Support**: Phantom, Solflare, Backpack, etc.
- **Program Interaction**: Anchor TypeScript SDK
- **Transaction Management**: @solana/web3.js

### External APIs
- **Jupiter Swap API**: Cross-DEX aggregation
- **Feels Indexer API**: Real-time protocol data (using custom hooks with useState/useEffect)
- **Solana RPC**: Blockchain state queries

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

Create `.env.local` in the feels-app directory:

```env
# Solana Network Configuration
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com

# Feels Protocol Configuration
NEXT_PUBLIC_FEELS_PROGRAM_ID=YourProgramIdHere
NEXT_PUBLIC_INDEXER_API_URL=http://localhost:8080

# Jupiter Configuration
NEXT_PUBLIC_JUPITER_API_URL=https://quote-api.jup.ag/v6
```

## Project Structure

```
feels-app/
├── src/
│   ├── app/                    # Next.js App Router
│   │   ├── globals.css         # Global styles + shadcn/ui
│   │   ├── layout.tsx          # Root layout with providers
│   │   ├── page.tsx            # Main application page
│   │   ├── token/[address]/    # Token detail pages
│   │   ├── search/             # Search functionality
│   │   └── control/            # Admin/control panel
│   ├── components/             # React components
│   │   ├── ui/                 # shadcn/ui components
│   │   ├── common/             # Common components (NavBar, ConnectionStatus)
│   │   ├── market/             # Market-related components
│   │   ├── trading/            # Trading interface components
│   │   ├── search/             # Search components
│   │   └── wallet/             # Wallet integration components
│   ├── contexts/               # React contexts
│   │   ├── DataSourceContext.tsx # Data source management
│   │   └── SearchContext.tsx   # Search state management
│   ├── assets/                 # Static assets
│   │   ├── fonts/              # Custom fonts
│   │   └── images/             # Images and icons
│   ├── services/               # API clients
│   │   ├── indexer-client.ts   # Indexer API client
│   │   ├── jupiter-client.ts   # Jupiter API client
│   │   └── connection.ts       # Solana connection
│   ├── sdk/                    # Protocol SDK wrappers
│   │   ├── sdk.ts              # Feels Protocol SDK wrapper
│   │   └── program-workaround.ts # Program compatibility
│   ├── hooks/                  # React hooks
│   │   ├── useIndexer.ts       # Indexer data hooks
│   │   ├── useTokenSearch.ts   # Token search functionality
│   │   └── use-toast.ts        # Toast notifications
│   ├── utils/                  # Utility functions
│   │   ├── swap-router.ts      # Multi-hop routing engine
│   │   ├── token-search.ts     # Token search utilities
│   │   └── utils.ts            # shadcn/ui utilities
│   ├── constants/              # Application constants
│   ├── config/                 # Configuration files
│   ├── types/                  # TypeScript type definitions
│   └── idl/                    # Anchor IDL files
├── components.json             # shadcn/ui configuration
├── tailwind.config.js          # Tailwind + shadcn/ui config
├── tsconfig.json              # TypeScript configuration
└── package.json               # Dependencies and scripts
```

## Swap Route Detection

The application automatically detects optimal trading routes based on token selection:

### Route Types

1. **Direct Swap** (`jupiter-direct`)
   - Any Jupiter-supported token pair
   - Single-hop via Jupiter aggregation

2. **To JitoSOL** (`jupiter-to-jitosol`)
   - Any token → JitoSOL
   - Optimized for staking yield capture

3. **Full Onboarding** (`full-feels-onboard`)
   - Any token → JitoSOL → FeelsSOL
   - Complete Feels Protocol onboarding

4. **Feels to Meme** (`feels-to-meme`)
   - FeelsSOL → Meme coins
   - Via Feels Protocol concentrated liquidity pools

5. **Full Chain** (`full-chain`)
   - Any token → JitoSOL → FeelsSOL → Meme coin
   - Complete multi-hop trading chain

### Route Detection Logic

```typescript
function detectOptimalRoute(inputToken: TokenInfo, outputToken: TokenInfo): SwapRoute {
  // 1. Check for direct Jupiter support
  if (isJupiterSupported(input) && isJupiterSupported(output)) {
    return outputToken.symbol === 'JitoSOL' ? 'jupiter-to-jitosol' : 'jupiter-direct';
  }
  
  // 2. Check for Feels onboarding
  if (outputToken.symbol === 'FeelsSOL') {
    return 'full-feels-onboard';
  }
  
  // 3. Check for Feels pool trading
  if (inputToken.symbol === 'FeelsSOL' && isFeelsPoolToken(output)) {
    return 'feels-to-meme';
  }
  
  // 4. Check for full chain routing
  if (isJupiterSupported(input) && isFeelsPoolToken(output)) {
    return 'full-chain';
  }
  
  return 'jupiter-direct'; // Fallback
}
```

## Data Management

### Real-time Updates

The application maintains real-time synchronization with the Feels indexer:

```typescript
// Protocol statistics (updated every 30s)
const useProtocolStats = () => {
  return useQuery({
    queryKey: ['protocol-stats'],
    queryFn: () => indexerClient.getProtocolStats(),
    refetchInterval: 30000,
  });
};

// Market data (updated every 10s)
const useMarketData = (marketAddress: string) => {
  return useQuery({
    queryKey: ['market-data', marketAddress],
    queryFn: () => indexerClient.getMarketData(marketAddress),
    refetchInterval: 10000,
  });
};
```

### Caching Strategy

- **React Query**: Client-side caching with automatic background updates
- **Indexer Redis**: Server-side caching for frequently accessed data
- **Jupiter Quotes**: 500ms debouncing to prevent API spam

## Development Commands

```bash
# Development
pnpm dev              # Start development server (runs both Next.js and DevBridge via npm-run-all)
pnpm build            # Build for production
pnpm start            # Start production server

# Code Quality
pnpm lint             # Run ESLint
pnpm type-check       # TypeScript type checking
pnpm format           # Format with Prettier

# Testing
pnpm test             # Run test suite (when implemented)

# Feels-specific commands (via justfile)
just app-dev          # Start with Nix environment
just app-build        # Build in Nix environment
just app-lint         # Lint in Nix environment
```

## Deployment

### Production Checklist

1. **Environment Variables**
   ```env
   NEXT_PUBLIC_SOLANA_NETWORK=mainnet-beta
   NEXT_PUBLIC_SOLANA_RPC_URL=https://your-mainnet-rpc
   NEXT_PUBLIC_FEELS_PROGRAM_ID=MainnetProgramId
   NEXT_PUBLIC_INDEXER_API_URL=https://indexer.feels.so
   ```

2. **Build Optimization**
   ```bash
   npm run build
   npm run start
   ```

3. **Monitoring Setup**
   - Error tracking (Sentry recommended)
   - Performance monitoring
   - RPC endpoint health checks

### Deployment Targets

- **Vercel**: Recommended for Next.js applications
- **Netlify**: Alternative with good Solana support
- **Self-hosted**: Docker container available

## Integration Points

### With Feels Indexer

The indexer provides real-time protocol data:

- **Protocol Statistics**: TVL, volume, fees collected
- **Market Data**: Current tick, liquidity, price ranges
- **Transaction History**: Recent swaps, deposits, withdrawals
- **User Positions**: Portfolio tracking (when implemented)

### With Jupiter

Jupiter provides cross-DEX aggregation:

- **Quote API**: Best prices across all Solana DEXs
- **Swap API**: Transaction building and execution
- **Token List**: Comprehensive token metadata
- **Route Optimization**: Multi-hop routing for best prices

### With Feels Protocol

Direct smart contract interaction:

- **Deposit/Withdraw**: JitoSOL ↔ FeelsSOL conversion
- **Pool Swaps**: Trading in concentrated liquidity pools
- **Position Management**: LP position creation/management
- **Governance**: Protocol parameter voting (future)

## Security Considerations

### Wallet Security
- Never store private keys in the application
- Use secure wallet adapters with proper validation
- Implement transaction simulation before signing

### API Security
- Rate limiting on all external API calls
- Input validation for all user-provided data
- Secure RPC endpoint configuration

### Smart Contract Interaction
- Transaction simulation before execution
- Slippage protection on all swaps
- Proper error handling and user feedback

## Performance Optimization

### Bundle Optimization
- Code splitting for different routes
- Dynamic imports for heavy components
- Tree shaking for unused dependencies

### API Optimization
- Request debouncing for real-time quotes
- Caching strategies for static data
- Background data refresh for better UX

### User Experience
- Loading states for all async operations
- Error boundaries for graceful failure handling
- Responsive design for all screen sizes

## Development Tools

### DevBridge - CLI Development Tool

DevBridge is a WebSocket-based development tool that enables CLI interaction with the running application. It's particularly useful for debugging and LLM-assisted development.

**Features:**
- Real-time log streaming from browser to CLI
- Command execution from CLI to browser
- Event monitoring and debugging
- Zero production footprint (completely disabled in production)

**Usage:**
```bash
# Enable in .env.local
DEVBRIDGE_ENABLED=true
NEXT_PUBLIC_DEVBRIDGE_ENABLED=true

# Start development server with DevBridge
npm run dev

# Use the CLI tool
npm run devbridge            # Start interactive mode
npm run devbridge run ping    # Run single command
```

**Available Commands:**
- `ping` - Test connection
- `navigate` - Navigate to route: `{"path": "/token/abc"}`
- `refresh` - Refresh current page
- `getPath` - Get current pathname
- `windowInfo` - Get window dimensions
- `perfMetrics` - Get performance metrics

For more details, see [DevBridge documentation](src/devbridge/README.md).

## Contributing

### Development Workflow

1. **Setup**: Use Nix development environment
2. **Coding**: Follow TypeScript and React best practices
3. **Testing**: Ensure all features work with wallet integration
4. **Documentation**: Update README for any architectural changes

### Code Style

- **TypeScript**: Strict mode enabled
- **React**: Functional components with hooks
- **Styling**: Tailwind CSS with shadcn/ui components
- **Formatting**: Prettier with ESLint integration

## License

This application is part of the Feels Protocol ecosystem. See the root repository for license information.

---

## Related Components

- **[Feels Protocol Core](../programs/feels/)** - Main Solana program
- **[Feels Indexer](../feels-indexer/)** - Real-time data indexing
- **[Feels SDK](../sdk/)** - Rust SDK for protocol interaction
- **[Jupiter Adapter](../programs/feels-jupiter-adapter/)** - Jupiter integration layer

For more information about the complete Feels Protocol ecosystem, see the [main repository README](../README.md).