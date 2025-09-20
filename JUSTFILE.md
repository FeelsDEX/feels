# Justfile Architecture

This document explains the justfile-based build system used in the Feels Protocol project.

## Overview

The Feels Protocol uses [just](https://github.com/casey/just) as its command runner, providing a consistent interface for development, testing, and deployment tasks. The justfile system is organized in a modular hierarchy to separate concerns and maintain clarity.

## Justfile Structure

```
.
├── justfile                    # Main project justfile
├── e2e/justfile               # E2E development environment orchestration
├── feels-app/justfile         # Frontend application commands
└── programs/feels/tests/justfile  # Test-specific commands (imported by main)
```

### Main Justfile (`./justfile`)

The root justfile serves as the primary entry point for all project commands. It's organized into clear sections:

1. **Variables** - Centralized path configuration
2. **Build & Deploy** - Program compilation and deployment
3. **IDL & Client Generation** - Interface and SDK generation
4. **Testing** - Comprehensive test suite commands
5. **Frontend Application** - Forwarded to feels-app/justfile
6. **E2E Development** - Forwarded to e2e/justfile
7. **Utilities** - Helper commands and tools

Key features:
- Imports test commands from `programs/feels/tests/justfile`
- Uses environment script `scripts/load-env.sh` for consistent configuration
- Forwards frontend commands to dedicated justfile

### E2E Justfile (`e2e/justfile`)

Manages the complete local development environment including:
- Solana validator
- Program deployment
- Metaplex setup
- Token initialization
- Streaming adapter
- Indexer service
- Frontend application

Key features:
- Dynamic program ID detection from deployed keypair
- Fallback to environment variable or default
- Saves deployed program ID for subsequent commands
- Color-coded output for better visibility

### Frontend Justfile (`feels-app/justfile`)

Dedicated to Next.js application management:
- Development server with hot reload
- Production builds
- Linting and formatting
- Type checking
- Indexer integration modes

## Command Categories

### Essential Commands

```bash
# Start complete E2E environment
just dev-e2e

# Run all tests
just test

# Build the program
just build

# Deploy to localnet
just deploy
```

### Development Workflow

```bash
# 1. Start local development environment
just dev-e2e

# 2. Make changes to code

# 3. Run tests
just test-unit        # Fast unit tests
just test-integration # Integration tests

# 4. Deploy changes
just deploy

# 5. Check logs
just dev-e2e-logs
```

### Testing Commands

The testing infrastructure is comprehensive:

```bash
just test              # All in-memory tests
just test-all          # ALL tests including network tests
just test-unit         # Unit tests only
just test-integration  # Integration tests only
just test-e2e         # End-to-end tests only
just test-property    # Property-based tests
just test-localnet    # Localnet-dependent tests
```

## Environment Variables

### Required Configuration

Create a `.env` file in the project root:

```env
# Program deployment authority
PROGRAM_AUTHORITY=<your-authority-pubkey>

# Optional: Override default program ID
FEELS_PROGRAM_ID=<custom-program-id>
```

### Dynamic Program ID

The E2E environment automatically detects the deployed program ID:
1. Reads from `target/deploy/feels-keypair.json` if available
2. Falls back to `FEELS_PROGRAM_ID` environment variable
3. Uses default ID as last resort

## Exit Codes

Standardized exit codes for automation:

- `0` - Success
- `1` - General error
- `2` - Missing dependencies
- `3` - Build failure
- `4` - Deployment failure
- `5` - Test failure
- `10` - Configuration error
- `11` - Environment setup error
- `12` - Service start failure

## Pre-flight Checks

Commands perform validation before execution:

1. **Environment checks** - Verifies required tools and configuration
2. **Service status** - Ensures dependencies are running
3. **File existence** - Validates required files and directories
4. **Network connectivity** - Checks RPC endpoints

## Progress Indicators

Long-running commands provide visual feedback:
- 🔨 Building...
- 🚀 Deploying...
- ⏳ Waiting for service...
- ✅ Success
- ❌ Error

## Tips and Tricks

### Running Specific Tests
```bash
# Run tests matching a pattern
just filter test_swap

# Run with verbose output
just verbose

# Run with specific thread count
just parallel 4
```

### Viewing Logs
```bash
# All E2E logs
just dev-e2e-logs

# Specific service
just dev-e2e-logs validator
just dev-e2e-logs indexer
```

### Quick Status Check
```bash
# Check all services
just dev-e2e-status

# Check environment
just check-env
```

## Troubleshooting

### Common Issues

1. **Build failures**
   - Check `logs/build.log`
   - Ensure Nix environment: `nix develop`
   - Verify Rust toolchain

2. **Deployment failures**
   - Check validator is running: `just dev-e2e-status`
   - Verify keypair exists: `just check-env`
   - Check program ID matches

3. **Service start failures**
   - Check port availability
   - Review service logs: `just dev-e2e-logs [service]`
   - Ensure dependencies are built

### Debug Mode

Enable verbose output:
```bash
# Add to any command
JUST_VERBOSE=1 just build

# Or use built-in verbose commands
just verbose
```

## Contributing

When adding new commands:

1. Follow naming conventions:
   - Use hyphens for multi-word commands
   - Prefix with category (e.g., `test-`, `app-`)
   - Keep names descriptive but concise

2. Add appropriate documentation:
   - Command description in help text
   - Usage examples for complex commands
   - Exit codes for error conditions

3. Implement pre-flight checks:
   - Validate dependencies
   - Check service availability
   - Verify file existence

4. Use consistent output:
   - Color coding for status
   - Progress indicators for long operations
   - Clear error messages

## Architecture Decisions

1. **Modular justfiles** - Separate files for different domains
2. **Command forwarding** - Main justfile delegates to specialized files
3. **Environment scripts** - Centralized configuration loading
4. **Dynamic detection** - Program IDs and service status
5. **Fail-fast approach** - Early validation and clear errors

This architecture provides a scalable, maintainable build system that grows with the project while keeping complexity manageable.