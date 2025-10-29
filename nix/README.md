# Nix Configuration

This directory contains the Nix infrastructure for Feels Protocol development environments. The configuration is modular and composable, allowing different development contexts (program development, indexer, frontend, E2E testing) to share common tooling while maintaining their specific requirements.

## Rust Toolchain Strategy

Feels Protocol uses a dual Rust toolchain approach to support both excellent IDE experience and correct Solana program compilation:

### 1. nixpkgs Rust (Primary - IDE & Regular Builds)
- Full Rust toolchain with all components including proc-macro server
- Used by rust-analyzer for IDE features (no "proc-macro server exited" errors)
- Used for SDK, indexer, and other regular Rust code
- Provides: `rustc`, `cargo`, `rust-analyzer`, `clippy`, `rustfmt`
- From: Standard nixpkgs

### 2. Solana BPF Toolchain (Secondary - On-chain Programs)
- Specialized Rust from zero-nix optimized for Solana BPF compilation
- Automatically invoked by Anchor and `cargo build-sbf` commands
- Not used directly for IDE features
- From: zero-nix packages

This dual setup is configured in `modules/solana-tools.nix` and ensures:
- rust-analyzer works perfectly with proc-macros
- Solana programs build with the correct BPF toolchain
- SDK and indexer compile with standard Rust
- No conflicts between toolchains

See `../RUST-ANALYZER-SETUP.md` for IDE configuration details.

## Directory Structure

```
nix/
├── lib/           # Reusable library functions
├── modules/       # Modular development stacks
└── project/       # Project-specific configuration
```

## Library Functions (`lib/`)

**`lib/default.nix`** provides reusable abstractions for building development environments:

- `mkEnvVars`, `mkPackages`, `mkCommands` - Compose environment configurations from module lists
- `mkDevShell` - Merge development shell configurations with packages, commands, environment variables, and startup scripts
- `mkBPFProgram` - Generic builder for Solana BPF programs with proper environment setup for macOS and Linux
- `mkValidator` - Generates validator launcher scripts with automatic keypair management, program deployment, and cleanup
- `mkIDLBuilder` - Creates IDL generation tool that uses nightly Rust toolchain with proper dependency handling

These functions abstract away platform-specific details (macOS vs Linux, Apple Silicon specifics) and provide consistent interfaces for building Solana programs.

## Modules (`modules/`)

Each module provides a **generic, reusable** development stack with packages, commands, environment variables, and startup text. Modules are technology-specific (databases, frontend tools, etc.) but not project-specific. Modules can be combined to create different development environments.

**Note:** Project-specific tools and configurations belong in `project/`, not `modules/`.

### `databases.nix`

Database stack for indexer development:
- PostgreSQL 15, Redis, RocksDB
- Compression libraries (zlib, bzip2, lz4, zstd, snappy) with environment variables configured for builds
- Convenience commands: `services-start`, `services-stop`, `pg-start/stop`, `redis-start/stop`
- RocksDB utilities: `init-rocksdb`, `clean-rocksdb`
- Crate overrides for `librocksdb-sys` to handle macOS-specific build requirements

Database data stored in `localnet/data/` directory with logs in `localnet/logs/`.

### `frontend.nix`

Next.js and React development environment:
- Node.js 20 with TypeScript, ESLint, Prettier
- Tailwind CSS language server
- `app-setup` command for initializing Next.js projects
- `buildApp` function for creating production builds in Nix

Environment variables configured to disable telemetry and increase memory limits for large builds.

### `indexer.nix`

Indexer and streaming infrastructure:
- Yellowstone Dragon's Mouth (Geyser gRPC plugin) built from source
- Includes macOS-specific patches for `affinity` crate compatibility
- gRPC tools (protobuf, grpcurl)
- Pre-configured Geyser plugin configuration at port 10000 with Prometheus metrics at 8999

The Yellowstone package is built with proper platform-specific dependencies and includes workarounds for git-based build scripts.

### `solana-tools.nix`

Core Solana development environment with dual Rust toolchain:

**Rust Toolchains:**
- nixpkgs Rust: `cargo`, `rustc`, `rust-analyzer`, `clippy`, `rustfmt` (for IDE and regular builds)
- Solana BPF: Anchor framework with zero-nix Rust (for on-chain programs)

**Additional Tools:**
- Solana CLI and validator (via zero-nix packages)
- OpenSSL, protobuf, clang/LLVM for native builds
- Just, jq, cmake for build automation
- crate2nix for Cargo.nix generation

**Environment:**
- macOS-specific SDK paths and deployment target
- OpenSSL, libclang paths configured
- RUST_SRC_PATH automatically discovered by rust-analyzer

**Commands:**
- `download-metaplex` - Fetch Metaplex Token Metadata program

This module provides the foundation that all other environments build on. The dual toolchain ensures both excellent IDE support and correct Solana program compilation.

## Project Configuration (`project/`)

**All Feels Protocol-specific Nix code goes here.** This includes custom packages, tools, and configurations that are unique to this project.

### `config.nix`

Feels Protocol-specific configuration constants:
- Program definitions (name, display name, cargo manifest path)
- Custom IDL dependencies for proper build isolation
- Directory paths (programs, target, deploy, idl, logs, ledger, keypairs)
- Validator configuration (RPC port 8899, WebSocket port 8900, airdrop amount)

This configuration is consumed by library functions to generate project-specific tools.

### `environments.nix`

Composed development environments built from generic modules (from `modules/`) plus project-specific packages and tools:

**`default`** - Primary development environment
- Solana tools + databases
- IDL builder command
- Standard development workflow

**`frontend`** - Web application development
- Solana tools + Node.js stack
- Commands for generating SDK and setting up Next.js app

**`indexer`** - Indexer and streaming development
- All databases + Yellowstone gRPC
- PostgreSQL, Redis, RocksDB with compression
- Configured for local testing with proper environment variables

**`e2e`** - Complete integration testing
- All modules combined
- Full stack from validator through indexer to frontend
- Orchestration support via justfile commands

Each environment includes a custom startup message that lists available tools and commands.

## Usage

Development shells are accessed via the project's `flake.nix`:

```bash
# Default development environment
nix develop

# Frontend development
nix develop .#frontend

# Indexer development
nix develop .#indexer

# Full E2E environment
nix develop .#e2e
```

The `flake.nix` imports these modules and uses the library functions to construct the final development shells.

## Adding New Components

### Generic Modules (`modules/`)

To add a **generic, reusable** development stack (e.g., a new language runtime, database, or toolchain):

1. Create `modules/your-module.nix` with the standard structure:
   ```nix
   { pkgs, inputs', lib, ... }:
   {
     packages = [ /* list of packages */ ];
     commands = [ /* helper commands */ ];
     env = [ /* environment variables */ ];
     startup = { /* startup text */ };
   }
   ```

2. Import the module in `flake.nix`
3. Add it to appropriate environment compositions in `project/environments.nix`

### Project-Specific Tools (`project/`)

To add **Feels Protocol-specific** packages, tools, or configurations:

1. Create or modify files in `project/` (e.g., `project/feels-tools.nix`)
2. Build project-specific packages using `pkgs.rustPlatform.buildRustPackage`, `pkgs.buildNpmPackage`, etc.
3. Export packages, commands, and environment variables following the same structure as modules
4. Import and compose in `project/environments.nix`

**Rule of thumb:** If it could be used by other Solana projects → `modules/`. If it's Feels-specific → `project/`.

## Platform Notes

The configuration handles macOS (Apple Silicon) specifics:
- Darwin frameworks (Security, SystemConfiguration) included where needed
- `MACOSX_DEPLOYMENT_TARGET` set to 11.0
- libiconv and clang paths configured for native extensions
- RocksDB compression libraries properly linked via environment variables

Linux builds work without modification due to conditional inclusion of Darwin-specific packages.

## Build Artifacts

Nix builds produce:
- `result` symlink at project root pointing to build output
- Reproducible builds via pinned dependencies in `flake.lock`
- Binary cache support via Cachix (configured in flake)

The modular design allows incremental rebuilds when only specific modules change.

