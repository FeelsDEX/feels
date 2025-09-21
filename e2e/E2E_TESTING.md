# End-to-End Testing Guide

This guide explains how to run the complete E2E tests that validate the entire Feels Protocol data pipeline from on-chain program through indexer to frontend.

## Overview

The E2E tests validate:
1. **On-chain program** execution and event emission
2. **Streaming adapter** capturing blockchain events
3. **Indexer** processing and storing data
4. **API endpoints** serving correct data
5. **WebSocket** real-time updates
6. **Frontend** integration via DevBridge

## Test Structure

### 1. Indexer Pipeline Tests (`test_indexer_pipeline_e2e.rs`)
- Tests complete data flow from chain to API
- Validates swap quotes and transaction building
- Tests Jupiter integration for entry/exit
- Verifies token balance queries
- Checks WebSocket endpoint availability

### 2. Frontend DevBridge Tests (`test_frontend_devbridge_e2e.rs`)
- Tests DevBridge connectivity and commands
- Validates navigation and routing
- Tests storage operations and feature flags
- Monitors real-time event streaming
- Tests error handling and resilience

## Prerequisites

### Required Services
All tests require the full E2E environment running:
- Solana validator (port 8899)
- Streaming adapter (port 10000)
- Indexer API (port 8080)
- Frontend app (port 3000)

### Environment Setup
For frontend tests, ensure DevBridge is enabled:
```bash
# In feels-app/.env.local
DEVBRIDGE_ENABLED=true
NEXT_PUBLIC_DEVBRIDGE_ENABLED=true
```

## Running Tests

### Quick Start
```bash
# Start the complete E2E environment
just dev-e2e

# In another terminal, run all pipeline tests
./e2e/run-pipeline-tests.sh
```

### Individual Test Commands

#### Indexer Tests
```bash
# Complete pipeline test
cargo test -p feels test_indexer_complete_pipeline -- --nocapture

# Market analytics test
cargo test -p feels test_indexer_market_stats_and_ohlcv -- --nocapture
```

#### Frontend Tests
```bash
# DevBridge integration test
cargo test -p feels test_frontend_complete_flow_via_devbridge -- --nocapture

# Real-time updates test
cargo test -p feels test_frontend_real_time_updates -- --nocapture

# Error handling test
cargo test -p feels test_frontend_error_handling -- --nocapture
```

### Running All E2E Tests
```bash
# Using just command
just test-e2e

# Or run specific e2e tests
cargo test -p feels e2e:: -- --nocapture --test-threads=1
```

## Test Scenarios

### Indexer Pipeline Test Flow
1. Check indexer health
2. Test entry quote (JitoSOL → FeelsSOL)
3. Create market and verify indexing
4. Get swap quote with routing
5. Build swap transaction
6. Execute swap and verify indexing
7. Query token balances
8. Test WebSocket connectivity

### Frontend DevBridge Test Flow
1. Connect to DevBridge WebSocket
2. Execute basic commands (ping, appInfo)
3. Test navigation between pages
4. Check storage operations
5. Toggle feature flags
6. Get performance metrics
7. Subscribe to events and monitor
8. Simulate UI flows

## Debugging Failed Tests

### Service Not Running
If tests fail with "service not running":
```bash
# Check service status
just dev-e2e-status

# View service logs
just dev-e2e-logs validator
just dev-e2e-logs streaming
just dev-e2e-logs indexer
just dev-e2e-logs app
```

### Indexer Issues
```bash
# Check indexer health
curl http://localhost:8080/health

# View indexer logs
tail -f logs/indexer.log
```

### Frontend/DevBridge Issues
```bash
# Check DevBridge is enabled
grep DEVBRIDGE feels-app/.env.local

# Test DevBridge connection
wscat -c ws://127.0.0.1:54040
> {"type":"ping"}
```

## CI Integration

For CI environments, tests gracefully skip when services aren't available:
```yaml
- name: Run E2E Tests
  run: |
    # Tests will detect missing services and skip
    cargo test -p feels e2e:: -- --nocapture
```

To run full E2E in CI:
```yaml
- name: Start E2E Environment
  run: just dev-e2e &
  
- name: Wait for Services
  run: sleep 60
  
- name: Run Pipeline Tests
  run: ./e2e/run-pipeline-tests.sh
```

## Writing New E2E Tests

### Adding Indexer Tests
1. Add test function to `test_indexer_pipeline_e2e.rs`
2. Use `IndexerE2ETest` helper for API calls
3. Always check service availability first

### Adding Frontend Tests
1. Add test function to `test_frontend_devbridge_e2e.rs`
2. Use `DevBridgeE2ETest` helper for WebSocket commands
3. Handle connection failures gracefully

### Best Practices
- Always check if services are running before testing
- Use timeouts for async operations
- Log detailed information for debugging
- Clean up test data when possible
- Make tests idempotent

## Troubleshooting

### Common Issues

1. **"Address already in use"**
   - Stop existing services: `just dev-e2e-stop`
   - Check for orphan processes: `lsof -i :8899`

2. **"Connection refused"**
   - Ensure E2E environment is running: `just dev-e2e`
   - Check firewall/network settings

3. **"Timeout waiting for indexer"**
   - Indexer may need more time to start
   - Check logs: `just dev-e2e-logs indexer`

4. **"DevBridge not available"**
   - Ensure environment variables are set
   - Restart frontend: `cd feels-app && npm run dev`

## Performance Considerations

- Tests use `--test-threads=1` to avoid concurrent conflicts
- Indexer tests wait for data propagation (2-5 seconds)
- Frontend tests use shorter timeouts for responsiveness
- Consider running heavy tests separately in CI