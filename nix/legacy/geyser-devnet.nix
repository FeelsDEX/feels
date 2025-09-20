# Local Solana test validator with Yellowstone Dragon's Mouth Geyser plugin
{ pkgs, inputs', projectConfig }:

let
  inherit (pkgs) lib stdenv;
  
  # Pre-fetch the macOS-compatible affinity source to avoid git network calls during build
  affinity-macos-src = pkgs.fetchFromGitHub {
    owner = "elast0ny";
    repo = "affinity";
    rev = "67b925db00575a35d839455964baea494ac86ec2";
    hash = "sha256-54Z45C751GCTadHJVorxWz40Igmk2D2QsQZwIQ9yAfc=";
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

  # Build Yellowstone Dragon's Mouth from source - exact copy from working almanac version
  yellowstoneDragonsMouth = pkgs.rustPlatform.buildRustPackage rec {
    pname = "yellowstone-grpc-geyser";
    version = "8.0.0+solana.2.3.3";

    src = pkgs.fetchFromGitHub {
      owner = "rpcpool";
      repo = "yellowstone-grpc";
      rev = "v${version}";
      hash = "sha256-rbGS0NLljGrv5Ffap0T+28tLN7sRYclMQYJA/BlmiNs=";
    };

    cargoHash = "sha256-KWY0qQya9k674WPm5Oj3rOdKl2PfFnAGT5pS66rQUFc=";
    
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
    ] ++ lib.optionals stdenv.isDarwin [
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
      cp target/release/libyellowstone_grpc_geyser.* $out/lib/
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

  geyserDevnet = pkgs.writeShellScriptBin "geyser-devnet" ''
    set -euo pipefail
    
    echo "Starting ${projectConfig.projectName} local development environment with Geyser..."
    
    # Configuration
    VALIDATOR_LOG_DIR="${projectConfig.directories.logs}"
    LEDGER_DIR="${projectConfig.directories.ledger}"
    KEYPAIR_DIR="${projectConfig.directories.keypairs}"
    
    # Create directories
    mkdir -p "$VALIDATOR_LOG_DIR" "$LEDGER_DIR" "$KEYPAIR_DIR"
    
    # Function to cleanup on exit
    cleanup() {
      echo "Cleaning up..."
      pkill -f solana-test-validator || true
      echo "Cleanup complete"
    }
    trap cleanup EXIT INT TERM
    
    # Generate or use existing keypairs
    if [ ! -f "$KEYPAIR_DIR/payer.json" ]; then
      echo "Generating development keypairs..."
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR_DIR/payer.json"
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR_DIR/program.json"
    fi
    
    # Build the program
    echo "Building ${projectConfig.projectName}..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
    
    # Start the validator with Geyser plugin
    echo "Starting local validator with Geyser plugin..."
    ${inputs'.zero-nix.packages.solana-node}/bin/solana-test-validator \
      --ledger "$LEDGER_DIR" \
      --keypair "$KEYPAIR_DIR/payer.json" \
      --bind-address ${projectConfig.validator.bindAddress} \
      --rpc-port ${toString projectConfig.validator.rpcPort} \
      --rpc-bind-address ${projectConfig.validator.bindAddress} \
      --dynamic-port-range ${projectConfig.validator.portRange} \
      --enable-rpc-transaction-history \
      --enable-extended-tx-metadata-storage \
      --geyser-plugin-config ${geyserConfig} \
      --log "$VALIDATOR_LOG_DIR/validator.log" \
      --reset \
      --quiet &
    
    VALIDATOR_PID=$!
    
    # Wait for validator to start
    echo "Waiting for validator to start..."
    timeout=30
    while ! ${inputs'.zero-nix.packages.solana-tools}/bin/solana cluster-version --url http://localhost:${toString projectConfig.validator.rpcPort} >/dev/null 2>&1; do
      sleep 1
      timeout=$((timeout - 1))
      if [ $timeout -le 0 ]; then
        echo "Validator failed to start within 30 seconds"
        exit 1
      fi
    done
    
    echo "Validator is running!"
    
    # Wait for Geyser to be ready
    echo "Waiting for Geyser gRPC to be ready..."
    timeout=20
    while ! nc -z localhost 10000 2>/dev/null; do
      sleep 1
      timeout=$((timeout - 1))
      if [ $timeout -le 0 ]; then
        echo "Warning: Geyser gRPC not responding on port 10000"
        break
      fi
    done
    
    if nc -z localhost 10000 2>/dev/null; then
      echo "Geyser gRPC is ready on port 10000!"
    fi
    
    # Configure Solana CLI
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --url http://localhost:${toString projectConfig.validator.rpcPort}
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --keypair "$KEYPAIR_DIR/payer.json"
    
    # Fund the payer account
    echo "Funding payer account..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana airdrop ${toString projectConfig.validator.airdropAmount} "$KEYPAIR_DIR/payer.json" --url http://localhost:${toString projectConfig.validator.rpcPort}
    
    # Deploy the program
    echo "Deploying ${projectConfig.projectName}..."
    if ${inputs'.zero-nix.packages.solana-tools}/bin/anchor deploy --provider.cluster localnet; then
      echo "Program deployed successfully!"
    else
      echo "Program deployment failed, but validator is still running"
    fi
    
    # Show useful information
    echo
    echo "${projectConfig.projectName} Local Development Environment with Geyser"
    echo "========================================================"
    echo "RPC URL: http://localhost:${toString projectConfig.validator.rpcPort}"
    echo "WebSocket URL: ws://localhost:${toString projectConfig.validator.wsPort}"
    echo "Geyser gRPC URL: http://localhost:10000"
    echo "Geyser Prometheus: http://localhost:8999/metrics"
    echo "Payer keypair: $KEYPAIR_DIR/payer.json"
    echo "Program keypair: $KEYPAIR_DIR/program.json"
    echo "Logs: $VALIDATOR_LOG_DIR/validator.log"
    echo
    echo "Useful commands:"
    echo "  solana balance                 - Check balance"
    echo "  solana logs                    - Stream program logs"
    echo "  grpcurl -plaintext localhost:10000 list - List gRPC services"
    echo "  anchor test --skip-local-validator - Run tests against local validator"
    echo
    echo "Press Ctrl+C to stop the validator"
    
    # Wait indefinitely
    wait $VALIDATOR_PID
  '';
in {
  devnet = geyserDevnet;
  yellowstone = yellowstoneDragonsMouth;
}