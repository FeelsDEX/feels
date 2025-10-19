# Indexer Tests Justfile Refactoring

This justfile has been refactored to integrate with the main justfile system, eliminating redundancy in service management logic.

## Changes Made

1. **Added imports** for shared modules:
   - `common.just` - Common utilities and helpers
   - `services.just` - Centralized PostgreSQL and Redis management
   - `solana-tools.just` - Validator and Solana tooling

2. **Replaced duplicated service management**:
   - PostgreSQL start/stop now uses centralized `pg-start`, `pg-stop` recipes
   - Redis start/stop now uses centralized `redis-start`, `redis-stop` recipes
   - Database creation uses centralized `db-create` recipe
   - All services control uses `services-start` and `services-stop`

3. **Simplified test recipes**:
   - `test-integration` and `test-integration-geyser` now use centralized service management
   - Removed inline shell scripts for starting/stopping services
   - Maintained test-specific logic while delegating service control

4. **Maintained functionality**:
   - All original test commands work as before
   - Environment setup and cleanup preserved
   - Test-specific configurations retained

## Benefits

- Single source of truth for service management
- Consistent behavior across all modules
- Easier maintenance and updates
- Reduced code duplication

## Usage

All commands work the same as before:
```bash
just test-integration       # Run integration tests
just test-unit             # Run unit tests
just setup-test-env        # Setup test environment
just clean-test-env        # Clean test environment
```

The refactoring is transparent to users while providing better maintainability.