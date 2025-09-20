# Feels Protocol environment compositions
{ pkgs, inputs', lib, modules, projectConfig, idlBuilder, ... }:

let
  inherit (lib) mkDevShell mkPackages mkCommands mkEnvVars;
  
in {
  # Default development environment
  default = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
    ] ++ [
      {
        name = "idl-build";
        package = idlBuilder;
        help = "Build IDL for Anchor programs";
      }
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
    ];
    
    devshell.startup = {
      setup-feels = {
        deps = [];
        text = ''
          ${projectConfig.devEnv.welcomeMessage}
          echo ""
          echo "Available tools:"
          echo "  - solana: Solana CLI and validator"
          echo "  - anchor: Anchor framework for Solana development"
          echo "  - idl-build: Build IDL for Anchor programs"
          echo "  - just: Task runner (run 'just' to see commands)"
          echo ""
          echo "Database services:"
          echo "  - services-start/services-stop - Control all services"
          echo "  - init-rocksdb/clean-rocksdb   - Manage RocksDB data"
          echo ""
          echo "Quick commands:"
          echo "  - just                      - Show all available commands"
          echo "  - just build                - Build all programs"
          echo "  - just test                 - Run tests"
          echo "  - idl-build                 - Generate IDL (uses cargo +nightly wrapper)"
          echo ""
          echo "Build commands:"
          echo "  - cargo build-sbf           - Build Solana programs directly"
          echo ""
          echo "Nix apps (run with 'nix run .#<app>'):"
          echo "  - devnet                    - Start local validator with auto-deploy"
          echo "  - bpf-build                 - Build all BPF programs using Nix"
          echo "  - idl-build                 - Generate IDL files"
          echo ""
        '';
      };
    };
  };
  
  # Frontend development environment
  frontend = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.frontend
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.frontend
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.frontend
    ];
    
    devshell.startup = {
      setup-frontend = {
        deps = [];
        text = ''
          ${projectConfig.devEnv.welcomeMessage}
          echo ""
          echo "Frontend Development Environment"
          echo "================================"
          echo ""
          echo "This environment includes:"
          echo "  ✓ Solana development tools"
          echo "  ✓ Next.js and React development stack"
          echo "  ✓ TypeScript and Tailwind CSS"
          echo ""
          echo "Quick start:"
          echo "  1. Generate SDK: just generate-sdk"
          echo "  2. Set up app: app-setup"
          echo "  3. Start dev server: cd feels-app && npm run dev"
          echo ""
        '';
      };
    };
  };
  
  # Indexer development and testing environment
  indexer = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
      modules.indexer
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
      modules.indexer
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
      modules.indexer
    ];
    
    devshell.startup = {
      setup-indexer = {
        deps = [];
        text = ''
          ${projectConfig.devEnv.welcomeMessage}
          echo ""
          echo "Indexer Development Environment"
          echo "==============================="
          echo ""
          echo "This environment includes:"
          echo "  ✓ Solana validator and tools"
          echo "  ✓ Geyser gRPC streaming support"
          echo "  ✓ PostgreSQL 15"
          echo "  ✓ Redis"
          echo "  ✓ RocksDB with all compression libs"
          echo "  ✓ Rust development tools"
          echo ""
          echo "Quick commands:"
          echo "  services-start    - Start PostgreSQL and Redis"
          echo "  services-stop     - Stop all services"
          echo ""
          echo "Manual service control:"
          echo "  pg-start/pg-stop     - Control PostgreSQL"
          echo "  redis-start/redis-stop - Control Redis"
          echo ""
          echo "Test data will be stored in: ./test-data/"
          echo ""
        '';
      };
    };
  };
  
  # Complete E2E development environment
  e2e = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
    ];
    
    devshell.startup = {
      setup-e2e = {
        deps = [];
        text = ''
          ${projectConfig.devEnv.welcomeMessage}
          echo ""
          echo "Complete E2E Development Environment"
          echo "===================================="
          echo ""
          echo "This environment includes everything:"
          echo "  ✓ Solana development tools"
          echo "  ✓ Database stack (PostgreSQL, Redis, RocksDB)"
          echo "  ✓ Indexer and streaming tools"
          echo "  ✓ Frontend development stack"
          echo ""
          echo "Quick start for full stack development:"
          echo "  1. Start services: services-start"
          echo "  2. Start validator: just local-devnet"
          echo "  3. Deploy programs: just deploy"
          echo "  4. Start indexer: cd feels-indexer && cargo run"
          echo "  5. Start frontend: cd feels-app && npm run dev"
          echo ""
          echo "Or use the E2E orchestration:"
          echo "  just dev-e2e     - Start complete environment"
          echo ""
        '';
      };
    };
  };
}
