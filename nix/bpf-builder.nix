# BPF Builder for Solana programs
{
  pkgs,
  inputs',
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
        
        # Check for Anchor generated files and create if missing for ALL programs
        # Find all programs that need stub files
        find programs -name "Cargo.toml" | while read cargo_file; do
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
  
  # Helper function to build all Valence programs
  buildValencePrograms = src: {
    core = buildBPFProgram {
      name = "valence-kernel";
      inherit src;
      cargoToml = "programs/valence-kernel/Cargo.toml";
    };
  };
  
  # App to build all Valence programs using Nix
  build-bpf-programs = {
    type = "app";
    program = "${pkgs.writeShellScriptBin "valence-build-bpf-programs" ''
      set -e
      
      echo "=== Building Valence BPF Programs with Nix ==="
      echo "This builds all Valence programs using the Nix BPF builder"
      echo ""
      
      # Colors for output
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      RED='\033[0;31m'
      NC='\033[0m' # No Color
      
      # Build programs using nix build
      echo -e "''${YELLOW}Building valence-kernel...''${NC}"
      nix build .#valence-kernel --out-link ./target/nix-core
      
      echo ""
      echo -e "''${GREEN}=== BPF Programs Built Successfully ===''${NC}"
      echo ""
      echo "Built programs available in:"
      echo "  - ./target/nix-core/deploy/"
      echo ""
      echo "To deploy programs:"
      echo "  solana program deploy ./target/nix-core/deploy/<program>.so"
    ''}/bin/valence-build-bpf-programs";
  };
  
  # Test BPF builder app
  test-bpf-builder = {
    type = "app";
    program = "${pkgs.writeShellScriptBin "valence-test-bpf-builder" ''
      set -e
      
      echo "=== Testing BPF Builder ==="
      echo ""
      
      # Test building the Valence programs using the BPF builder
      echo "Testing BPF builder with Valence programs..."
      nix build .#valence-kernel --out-link ./target/test-core-bpf
      
      echo ""
      echo "BPF builder test completed successfully"
      echo "Built programs available in:"
      echo "  - ./target/test-core-bpf/deploy/"
      
      if [ -d "./target/test-core-bpf/deploy" ]; then
        echo ""
        echo "Core programs:"
        ls -la ./target/test-core-bpf/deploy/ || echo "No files found"
      fi
    ''}/bin/valence-test-bpf-builder";
  };
}


