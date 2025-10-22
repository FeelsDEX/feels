# Feels Protocol environment compositions
{ pkgs, inputs', lib, modules, projectConfig, idlBuilder, projectRoot, ... }:

let
  inherit (lib) mkDevShell mkPackages mkCommands mkEnvVars;
  
  # Import project-specific tools - temporarily disabled to fix Nix environment loading
  # feelsTools = import ./feels-tools.nix { inherit pkgs lib projectRoot inputs'; };
  feelsTools = { packages = []; commands = []; env = []; };
  
in {
  # Default development environment
  default = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
      feelsTools
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
      feelsTools
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
      feelsTools
    ];
    
    devshell.motd = ''

      ${projectConfig.devEnv.welcomeMessage}
      
      Quick start:
        just                  - List justfile tasks (build, test, deploy, setup)
        just build            - Build Solana BPF program
        just deploy [network] - Deploy program (localnet|devnet)
        just init protocol    - Initialize on-chain protocol
        feels init --help     - Protocol CLI (protocol, hub, market setup)
    '';
  };
  
  # Frontend development environment (includes WASM tools)
  frontend = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.frontend
      modules.wasm-tools
      feelsTools
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.frontend
      modules.wasm-tools
      feelsTools
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.frontend
      modules.wasm-tools
      feelsTools
    ];
    
    devshell.motd = ''
      ${projectConfig.devEnv.welcomeMessage}
      Frontend Development Environment
      
      Quick start:
        just frontend::generate-sdk  - Generate TypeScript SDK from IDL
        just frontend::dev            - Start Next.js dev server (localhost:3000)
        just vanity-miner-wasm/build - Build vanity miner WASM module
        feels init --help             - Protocol CLI (protocol, hub, market setup)
    '';
  };
  
  # Indexer development and testing environment
  indexer = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
      modules.indexer
      feelsTools
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
      modules.indexer
      feelsTools
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
      modules.indexer
      feelsTools
    ];
    
    devshell.motd = ''
      ${projectConfig.devEnv.welcomeMessage}
      Indexer Development Environment
      
      Quick start:
        just services::services-start  - Start PostgreSQL + Redis
        just services::services-stop   - Stop all database services
        just services::rocksdb-init    - Initialize RocksDB storage
        feels init --help              - Protocol CLI (protocol, hub, market setup)
      
      Service data: ./localnet/data/
      Service logs: ./localnet/logs/
    '';
  };
  
  # WASM development environment (isolated for vanity-miner builds)
  # Note: Uses rust-overlay instead of solana-tools to avoid collision
  wasm = {
    packages = lib.mkPackages [
      modules.wasm-tools    # Provides rust-overlay toolchain with rust-src and WASM tools
      feelsTools
    ];
    
    commands = lib.mkCommands [
      modules.wasm-tools
      feelsTools
    ];
    
    env = lib.mkEnvVars [
      modules.wasm-tools
      feelsTools
    ];
    
    devshell.motd = ''
      ${projectConfig.devEnv.welcomeMessage}
      WASM Development Environment
      
      Rust toolchain: rust-overlay (stable with rust-src)
      
      Quick start:
        cd vanity-miner-wasm
        just build       - Build WASM with parallel features
        just build-dev   - Development build with debug info
        just test        - Run WASM tests
        wasm-info        - Show environment details
    '';
  };
  
  # Complete E2E development environment
  e2e = {
    packages = lib.mkPackages [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
      feelsTools
    ];
    
    commands = lib.mkCommands [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
      feelsTools
    ];
    
    env = lib.mkEnvVars [
      modules.solana-tools
      modules.databases
      modules.indexer
      modules.frontend
      feelsTools
    ];
    
    devshell.motd = ''
      ${projectConfig.devEnv.welcomeMessage}
      Complete E2E Development Environment
      
      Quick start:
        just e2e::run     - Start validator + indexer + frontend + databases
        just e2e::status  - Check all service health
        just e2e::stop    - Stop all services
        feels init --help - Protocol CLI (protocol, hub, market setup)
    '';
  };
}
