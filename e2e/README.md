# E2E Development Environment

Complete local development setup for Feels Protocol with integrated testing capabilities.

## Quick Start

```bash
# Start complete E2E environment
just dev-e2e

# Check status
just dev-e2e-status

# Stop all services
just dev-e2e-stop
```

## Architecture

```
┌─────────────────┐
│ Solana Validator│ ← Local blockchain (port 8899)
│ localhost:8899  │
└────────┬────────┘
         │ RPC calls
         ▼
┌─────────────────┐     gRPC Stream   ┌──────────────────┐
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

### streaming-adapter/
Yellowstone gRPC compatible service that polls Solana RPC and streams blockchain data via gRPC with protobuf messages for local development.

### Services
- **Validator**: Local Solana blockchain instance
- **Streaming Adapter**: Captures and streams blockchain events  
- **Indexer**: Processes and stores data with API endpoints
- **Frontend**: Next.js app with full protocol interface

## Commands

### Core Operations
```bash
# From project root
just dev-e2e              # Start everything
just dev-e2e-status       # Check service status
just dev-e2e-logs [service] # View logs
just dev-e2e-stop         # Stop all services

# From e2e/ directory  
just run                  # Start everything
just status               # Check status
just stop                 # Stop all services
just logs [service]       # View logs
```

### Individual Services
```bash
just validator            # Start validator only
just streaming           # Start streaming adapter only  
just indexer             # Start indexer only
just app                 # Start frontend only
```

### Setup & Deploy
```bash
just deploy              # Build and deploy program
just setup-metaplex      # Deploy Metaplex program
just setup-tokens        # Setup JitoSOL and FeelsSOL
just init-protocol       # Initialize protocol
```

## Testing

### Running E2E Tests
```bash
# All E2E tests
just test-e2e

# Specific test categories
cargo test -p feels e2e:: -- --nocapture --test-threads=1

# Individual tests
cargo test -p feels test_indexer_complete_pipeline -- --nocapture
cargo test -p feels test_frontend_complete_flow_via_devbridge -- --nocapture
```

### Test Coverage
E2E tests validate the complete data pipeline:
1. **On-chain program** execution and events
2. **Streaming adapter** capturing blockchain data
3. **Indexer** processing and API serving
4. **Frontend** integration via DevBridge
5. **WebSocket** real-time updates

### Prerequisites for Frontend Tests
```bash
# In feels-app/.env.local
DEVBRIDGE_ENABLED=true
NEXT_PUBLIC_DEVBRIDGE_ENABLED=true
```

## Configuration

### Environment Variables
- `FEELS_PROGRAM_ID` - Override default program ID
- `METAPLEX_ID` - Custom Metaplex program ID (auto-detected)

### Program ID Management
The environment automatically:
1. Generates program keypair if needed
2. Updates `declare_id!` in source code  
3. Deploys to generated address
4. Propagates ID to all services

## Logs

All services log to `logs/` directory:
```
logs/
├── validator.log         # Solana validator
├── streaming-adapter.log # Streaming service  
├── indexer.log          # Indexer service
├── app.log              # Frontend app
├── build.log            # Program compilation
├── metaplex-setup.log   # Metaplex deployment
└── protocol-init.log    # Protocol initialization
```

## Troubleshooting

### Service Issues
```bash
just status              # Check what's running
just logs [service]      # View specific logs
lsof -i :8899           # Check port conflicts
```

### Build Failures
Check `logs/build.log`. Build system tries:
1. Localnet build (if Metaplex configured)
2. Standard Nix build
3. Anchor build with reduced optimization

### Common Ports
- 8899: Solana RPC
- 10000: Streaming adapter  
- 8080: Indexer API
- 3000: Frontend app

### Reset Environment
```bash
just stop
rm -rf logs/ test-ledger/
rm -rf ~/.cache/solana/  # Optional
```

## Development Workflow

1. **Start**: `just dev-e2e`
2. **Develop**: Make code changes
3. **Deploy**: `just -f e2e/justfile deploy` 
4. **Test**: `just test-e2e`
5. **Debug**: `just dev-e2e-logs [service]`
6. **Stop**: `just dev-e2e-stop`

## Tips

- Use `just status` for quick health checks
- Run individual services during development for faster iteration
- Services start with health checks - next service waits for previous to be ready
- Tests gracefully skip when services aren't available
- All commands work from project root or e2e directory