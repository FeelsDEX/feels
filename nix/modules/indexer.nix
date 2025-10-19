# Indexer and streaming tools: gRPC, Yellowstone, monitoring
{ pkgs, inputs', lib, ... }:

let
  inherit (pkgs) stdenv;
  
  # Pre-fetch the macOS-compatible affinity source to avoid git network calls during build
  affinity-macos-src = pkgs.fetchFromGitHub {
    owner = "elast0ny";
    repo = "affinity";
    rev = "67b925db00575a35d839455964baea494ac86ec2";
    hash = "sha256-54Z45C751GCTadHJVorxWz40Igmk2D2QsQZwIQ9yAfc=";
  };

  # Build Yellowstone Dragon's Mouth from source
  yellowstoneDragonsMouth = pkgs.rustPlatform.buildRustPackage rec {
    pname = "yellowstone-grpc-geyser";
    version = "8.0.0+solana.2.3.3";

    src = pkgs.fetchFromGitHub {
      owner = "rpcpool";
      repo = "yellowstone-grpc";
      rev = "v${version}";
      hash = "sha256-rbGS0NLljGrv5Ffap0T+28tLN7sRYclMQYJA/BlmiNs=";
    };

    cargoHash = "sha256-0izlid1mXhXf9MpHIxFtQPMwpZdDLzOIMrY7KUnR2Uo=";
    
    nativeBuildInputs = with pkgs; [
      pkg-config
      protobuf
      cmake
      clang
      git
    ];
    
    buildInputs = with pkgs; [
      openssl
      zlib
      bzip2
      lz4
      zstd
      snappy
    ] ++ pkgs.lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
      libiconv
    ];
    
    # Set required environment variables
    PROTOC = "${pkgs.protobuf}/bin/protoc";
    PROTOC_INCLUDE = "${pkgs.protobuf}/include";
    
    # Set git version info for build script
    VERGEN_GIT_DESCRIBE = "v${version}";
    VERGEN_GIT_SHA = "unknown";
    SOURCE_DATE_EPOCH = "1686858254";
    
    # Patch the workspace Cargo.toml to use pre-fetched affinity source
    postPatch = ''
      # Copy the macOS-compatible affinity source into the vendor directory
      mkdir -p vendor/affinity
      cp -r ${affinity-macos-src}/* vendor/affinity/
      
      # Find and patch the workspace Cargo.toml to override affinity dependency
      if [ -f Cargo.toml ]; then
        echo "Patching workspace Cargo.toml..."
        sed -i 's|affinity = "0\.1\.2"|affinity = { path = "./vendor/affinity" }|g' Cargo.toml
        
        echo "Workspace Cargo.toml affinity section:"
        grep -A2 -B2 'affinity.*=' Cargo.toml || echo "affinity not found in workspace Cargo.toml"
      fi
      
      echo "Patched affinity dependency to use local macOS-compatible version"
    '';
    
    # Build only the geyser plugin
    cargoBuildFlags = [ "--package" "yellowstone-grpc-geyser" ];
    
    # Skip tests for faster builds
    doCheck = false;
    
    # Set up fake git repo for build script
    preBuild = ''
      git init
      git config user.email "nix@build.local"
      git config user.name "Nix Build"
      git add .
      git commit -m "nix build" --allow-empty
      git tag "v${version}"
    '';
    
    # The plugin is a dynamic library
    postInstall = ''
      mkdir -p $out/lib
      # Find and copy the yellowstone library files
      find target/release -name "libyellowstone_grpc_geyser.*" -type f -exec cp {} $out/lib/ \; || echo "Warning: No yellowstone library files found"
    '';
    
    # Enable features
    buildFeatures = [ ];
    
    meta = with pkgs.lib; {
      description = "Yellowstone Dragon's Mouth - Geyser gRPC Plugin";
      homepage = "https://github.com/rpcpool/yellowstone-grpc";
      license = licenses.agpl3Plus;
      platforms = platforms.unix;
    };
  };

  # Yellowstone Dragon's Mouth configuration
  geyserConfig = pkgs.writeText "geyser-config.json" ''
    {
      "libpath": "${yellowstoneDragonsMouth}/lib/libyellowstone_grpc_geyser.so",
      "log": {
        "level": "info"
      },
      "grpc": {
        "address": "0.0.0.0:10000",
        "channel_capacity": "100000",
        "unary_concurrency_limit": 100,
        "unary_disabled": false
      },
      "prometheus": {
        "address": "0.0.0.0:8999"
      },
      "block_fail_action": "log",
      "accounts_selector": {
        "owners": ["*"]
      }
    }
  '';

in {
  packages = [
    # gRPC and Protocol Buffers
    pkgs.protobuf
    pkgs.grpcurl
    
    # Development tools
    pkgs.curl
    pkgs.jq
    
    # Rust toolchain is provided by solana-tools module
    # Don't include rustc, cargo, clippy, or rustfmt to avoid conflicts
    
    # Yellowstone Dragon's Mouth
    yellowstoneDragonsMouth
  ];
  
  commands = [
    {
      name = "geyser-config";
      command = ''
        echo "Geyser configuration:"
        cat ${geyserConfig}
      '';
      help = "Show Geyser plugin configuration";
    }
  ];
  
  env = [
    {
      name = "RUST_LOG";
      value = "feels_indexer=debug,yellowstone_grpc_client=info";
    }
    {
      name = "RUST_BACKTRACE";
      value = "1";
    }
    {
      name = "GEYSER_CONFIG_PATH";
      value = "${geyserConfig}";
    }
  ];
  
  startup = {
    indexer = {
      deps = [];
      text = ''
        echo "Indexer & Streaming Tools"
        echo "========================="
        echo ""
        echo "Infrastructure:"
        echo "  yellowstone-grpc        - Geyser gRPC plugin"
        echo "  protoc                  - Protocol buffers compiler"
        echo "  grpcurl                 - gRPC CLI tool"
        echo ""
        echo "Development Commands (via just):"
        echo "  just indexer indexer-run        - Start indexer"
        echo "  just indexer indexer-test       - Run tests"
        echo "  just indexer indexer-migrate    - Run migrations"
        echo ""
        echo "Nix Commands:"
        echo "  geyser-config                   - Show Geyser config"
        echo ""
        echo "Default Endpoints:"
        echo "  Indexer API:    http://localhost:8080"
        echo "  Geyser gRPC:    http://localhost:10000"
        echo "  Prometheus:     http://localhost:8999"
        echo ""
      '';
    };
  };
  
  # Export the yellowstone package for use in other modules
  yellowstone = yellowstoneDragonsMouth;
  geyser-config = geyserConfig;
}
