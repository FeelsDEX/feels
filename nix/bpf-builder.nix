# BPF Builder for Solana programs
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: rec {
  # BPF Builder function for Solana programs using zero.nix tools
  buildBPFProgram = { name, src, cargoToml ? null, extraBuildInputs ? [] }:
    pkgs.stdenv.mkDerivation {
      pname = "${name}-bpf";
      version = "0.1.0";
      
      inherit src;
      
      nativeBuildInputs = with pkgs; [
        inputs'.zero-nix.packages.solana-tools  # This includes solana-node, anchor, and nightly-rust
        pkg-config
        openssl.dev
        git
        cacert
      ] ++ extraBuildInputs;
      
      buildInputs = with pkgs; [
        openssl
      ] ++ lib.optionals pkgs.stdenv.isDarwin [
        darwin.apple_sdk.frameworks.Security
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];
      
      # Set up Solana BPF build environment using zero.nix pattern
      RUST_BACKTRACE = "1";
      SOLANA_BPF_OUT_DIR = "$out/deploy";
      MACOSX_DEPLOYMENT_TARGET = "11.0";
      SOURCE_DATE_EPOCH = "1686858254";
      PROTOC = "${pkgs.protobuf}/bin/protoc";
      
      # Use platform tools from zero.nix
      PLATFORM_TOOLS_DIR = "${inputs'.zero-nix.packages.solana-node}/platform-tools";
      SBF_SDK_PATH = "${inputs'.zero-nix.packages.solana-node}/platform-tools";
      
      # Additional environment for getrandom workaround
      CARGO_TARGET_SBF_SOLANA_SOLANA_RUSTFLAGS = "-C link-arg=-undefined -C link-arg=dynamic_lookup";
      
      buildPhase = ''
        export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
        export HOME=$TMPDIR
        export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        export GIT_SSL_CAINFO="$SSL_CERT_FILE"
        
        # Check source structure
        echo "Checking source files for ${name}..."
        
        # Check for Anchor generated files and create if missing (if configured)
        ${if projectConfig.buildConfig.createClientStubs then ''
          # Find all programs that need stub files
          find ${projectConfig.directories.programs} -name "Cargo.toml" | while read cargo_file; do
            PROGRAM_DIR=$(dirname "$cargo_file")
            if [ ! -f "$PROGRAM_DIR/src/__client_accounts_crate.rs" ]; then
              echo "Creating __client_accounts_crate.rs stub in $PROGRAM_DIR..."
              echo "// Anchor client generation module - auto-generated stub" > "$PROGRAM_DIR/src/__client_accounts_crate.rs"
              echo "//" >> "$PROGRAM_DIR/src/__client_accounts_crate.rs"
              echo "// This module is required by Anchor's #[program] macro to generate client-side" >> "$PROGRAM_DIR/src/__client_accounts_crate.rs"
              echo "// TypeScript definitions and account structures." >> "$PROGRAM_DIR/src/__client_accounts_crate.rs"
              echo "pub use crate::*;" >> "$PROGRAM_DIR/src/__client_accounts_crate.rs"
              echo "Created stub file in $PROGRAM_DIR"
            fi
          done
        '' else ""}

        
        # Set up platform tools environment
        export PLATFORM_TOOLS_DIR="${inputs'.zero-nix.packages.solana-node}/platform-tools"
        export SBF_SDK_PATH="${inputs'.zero-nix.packages.solana-node}/platform-tools"
        export PATH="${inputs'.zero-nix.packages.solana-node}/platform-tools/rust/bin:${inputs'.zero-nix.packages.solana-node}/platform-tools/llvm/bin:$PATH"
        
        # Set up cargo cache
        export CARGO_HOME="$TMPDIR/.cargo"
        export RUSTUP_HOME="$TMPDIR/.rustup"
        mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
        
        # Configure cargo for BPF builds
        mkdir -p $CARGO_HOME
        echo "[target.sbf-solana-solana]" > $CARGO_HOME/config.toml
        echo 'rustflags = ["-C", "link-arg=-undefined", "-C", "link-arg=dynamic_lookup"]' >> $CARGO_HOME/config.toml
        echo "" >> $CARGO_HOME/config.toml
        echo "[net]" >> $CARGO_HOME/config.toml
        echo "git-fetch-with-cli = true" >> $CARGO_HOME/config.toml
        
        # Set environment variables for BPF compilation
        export CARGO_TARGET_SBF_SOLANA_SOLANA_RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup"
        export RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup"
        
        # Create output directory
        mkdir -p $out/deploy
        
        # Build the program
        echo "Building BPF program ${name}..."
        
        ${if cargoToml != null then ''
          # Build specific program
          echo "Running: cargo build-sbf --manifest-path ${cargoToml}"
          
          # Run the build
          cargo build-sbf --manifest-path ${cargoToml}
          echo "Build completed"
        '' else ''
          # Build all programs in workspace
          cargo build-sbf
        ''}
        
        # Verify we built something
        if [ -z "$(ls -A $out/deploy 2>/dev/null)" ]; then
          echo "No .so files found in output, checking workspace target..."
          # Try to copy from workspace target if build succeeded there
          if [ -d "target/deploy" ]; then
            cp target/deploy/*.so $out/deploy/ || true
          fi
        fi
        
        # Final verification
        if [ -z "$(ls -A $out/deploy 2>/dev/null)" ]; then
          echo "Warning: No BPF programs were built"
          echo "Checking for build errors..."
          find . -name "*.so" -type f || echo "No .so files found anywhere"
        fi
      '';
      
      installPhase = ''
        # Programs should be in target/deploy from cargo-build-sbf
        echo "Looking for built programs..."
        
        # Find all .so files
        SO_FILES=$(find target -name "*.so" -type f 2>/dev/null || true)
        
        if [ -n "$SO_FILES" ]; then
          echo "Found .so files:"
          echo "$SO_FILES"
          
          # Look in both target/deploy and target/sbf-solana-solana/release
          if [ -d "target/deploy" ]; then
            echo "Checking target/deploy directory:"
            ls -la target/deploy/ || echo "target/deploy is empty"
            
            # Copy .so files from target/deploy
            for so in target/deploy/*.so; do
              if [ -f "$so" ]; then
                echo "Copying $so to $out/deploy/"
                cp "$so" $out/deploy/
              fi
            done
          fi
          
          # Also check sbf-solana-solana release directory
          if [ -d "target/sbf-solana-solana/release" ]; then
            echo "Checking target/sbf-solana-solana/release directory:"
            
            # Find the main program .so (not deps)
            for so in target/sbf-solana-solana/release/*.so; do
              if [ -f "$so" ]; then
                echo "Copying $so to $out/deploy/"
                cp "$so" $out/deploy/
              fi
            done
          fi
          
          # Verify we copied something
          if [ -n "$(ls -A $out/deploy 2>/dev/null)" ]; then
            echo "Successfully copied programs to $out/deploy:"
            ls -la $out/deploy/
          else
            echo "Warning: No programs were copied to output directory"
            echo "Available .so files:"
            find target -name "*.so" -type f -ls
            exit 1
          fi
        else
          echo "Error: No .so files were built"
          echo "Build directory structure:"
          find target -type f -name "*.rs" | head -20
          exit 1
        fi
      '';
      
      meta = with pkgs.lib; {
        description = "Solana BPF program: ${name}";
        platforms = platforms.all;
        maintainers = [ ];
      };
    };
  
  # Helper function to build all configured programs
  buildAllPrograms = src:
    pkgs.lib.mapAttrs (key: config: 
      buildBPFProgram {
        name = config.name;
        inherit src;
        cargoToml = config.cargoToml;
      }
    ) projectConfig.programs;
  
  # App to build all programs using Nix
  bpf-build = {
    type = "app";
    program = let
      programBuildCommands = pkgs.lib.concatStringsSep "\n" (
        pkgs.lib.mapAttrsToList (key: config: ''
          echo -e "''${YELLOW}Building ${config.displayName}...''${NC}"
          nix build .#${config.name} --out-link ./target/nix-${key}
        '') projectConfig.programs
      );
      programOutputPaths = pkgs.lib.concatStringsSep "\n" (
        pkgs.lib.mapAttrsToList (key: config: ''
          echo "  - ./target/nix-${key}/deploy/"
        '') projectConfig.programs
      );
    in "${pkgs.writeShellScriptBin "bpf-build" ''
      set -e
      
      echo "=== Building ${projectConfig.projectName} BPF Programs with Nix ==="
      echo "This builds all programs using the Nix BPF builder"
      echo ""
      
      # Colors for output
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      RED='\033[0;31m'
      NC='\033[0m' # No Color
      
      # Build programs using nix build
      ${programBuildCommands}
      
      echo ""
      echo -e "''${GREEN}=== BPF Programs Built Successfully ===''${NC}"
      echo ""
      echo "Built programs available in:"
      ${programOutputPaths}
      echo ""
      echo "To deploy programs:"
      echo "  solana program deploy ./target/nix-<program>/deploy/<program>.so"
    ''}/bin/bpf-build";
  };
}