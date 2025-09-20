# Feels Indexer Nix configuration
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: let
  rocksdbConfig = import ./rocksdb.nix { inherit pkgs inputs' projectConfig; };
in {
  packages = rocksdbConfig.packages ++ [
    # gRPC and Protocol Buffers
    pkgs.protobuf
    pkgs.grpcurl
    
    # Development tools
    pkgs.curl
    pkgs.jq
    
    # Rust toolchain (if not already available)
    pkgs.rustc
    pkgs.cargo
    pkgs.clippy
    pkgs.rustfmt
  ];
  
  devShell = pkgs.mkShell {
    buildInputs = rocksdbConfig.packages ++ [
      pkgs.protobuf
      pkgs.grpcurl
      pkgs.curl
      pkgs.jq
      pkgs.rustc
      pkgs.cargo
      pkgs.clippy
      pkgs.rustfmt
    ];
    
    shellHook = rocksdbConfig.devShell.shellHook + ''
      echo ""
      echo "Feels Indexer development environment loaded"
      echo "Available commands:"
      echo "  cargo run --bin feels-indexer    - Start indexer"
      echo "  cargo test                       - Run tests"
      echo "  cargo clippy                     - Run linter"
      echo "  cargo fmt                        - Format code"
      echo ""
      echo "API endpoints (when running):"
      echo "  http://localhost:8080/markets    - List markets"
      echo "  http://localhost:8080/health     - Health check"
      echo "  http://localhost:9090/metrics    - Prometheus metrics"
      echo ""
      echo "Configuration:"
      echo "  Edit indexer.toml to customize settings"
      echo "  RocksDB data will be stored in: ./data/rocksdb"
    '';
    
    # Environment variables for development
    RUST_LOG = "feels_indexer=debug,yellowstone_grpc_client=info";
    RUST_BACKTRACE = "1";
  };
  
  # TODO: Add indexer-binary and indexer-docker when needed
}
