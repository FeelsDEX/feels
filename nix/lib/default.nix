# Reusable library functions for Solana development
{ pkgs, inputs', ... }:

rec {
  # Environment variable builders
  mkEnvVars = stacks: pkgs.lib.flatten (map (stack: stack.env or []) stacks);
  
  # Package list builders  
  mkPackages = stacks: pkgs.lib.flatten (map (stack: stack.packages or []) stacks);
  
  # Command builders
  mkCommands = stacks: pkgs.lib.flatten (map (stack: stack.commands or []) stacks);
  
  # Merge development shell configurations
  mkDevShell = { packages ? [], commands ? [], env ? [], startup ? {}, ... }@args:
    let
      allPackages = mkPackages packages;
      allCommands = mkCommands commands;
      allEnv = mkEnvVars env;
    in {
      inherit packages;
      commands = allCommands;
      env = allEnv;
      devshell.startup = startup;
    };
  
  # Generic BPF program builder
  mkBPFProgram = { name, src, cargoToml ? null, extraBuildInputs ? [], ... }:
    pkgs.stdenv.mkDerivation {
      pname = "${name}-bpf";
      version = "0.1.0";
      
      inherit src;
      
      nativeBuildInputs = with pkgs; [
        inputs'.zero-nix.packages.solana-tools
        pkg-config
        openssl.dev
        git
        cacert
      ] ++ extraBuildInputs;
      
      buildInputs = with pkgs; [
        openssl
      ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
        darwin.apple_sdk.frameworks.Security
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];
      
      # Standard BPF build environment
      RUST_BACKTRACE = "1";
      SOLANA_BPF_OUT_DIR = "$out/deploy";
      MACOSX_DEPLOYMENT_TARGET = "11.0";
      SOURCE_DATE_EPOCH = "1686858254";
      PROTOC = "${pkgs.protobuf}/bin/protoc";
      PLATFORM_TOOLS_DIR = "${inputs'.zero-nix.packages.solana-node}/platform-tools";
      SBF_SDK_PATH = "${inputs'.zero-nix.packages.solana-node}/platform-tools";
      CARGO_TARGET_SBF_SOLANA_SOLANA_RUSTFLAGS = "-C link-arg=-undefined -C link-arg=dynamic_lookup";
      
      buildPhase = ''
        export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
        export HOME=$TMPDIR
        export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        export GIT_SSL_CAINFO="$SSL_CERT_FILE"
        
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
          cargo build-sbf --manifest-path ${cargoToml}
        '' else ''
          # Build all programs in workspace
          cargo build-sbf
        ''}
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
            for so in target/deploy/*.so; do
              if [ -f "$so" ]; then
                echo "Copying $so to $out/deploy/"
                cp "$so" $out/deploy/
              fi
            done
          fi
          
          # Also check sbf-solana-solana release directory
          if [ -d "target/sbf-solana-solana/release" ]; then
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
  
  # Generic validator launcher
  mkValidator = { projectName, directories, validator, programs ? {} }:
    pkgs.writeShellScriptBin "validator" ''
      set -euo pipefail
      
      echo "Starting ${projectName} local development environment..."
      
      # Configuration
      VALIDATOR_LOG_DIR="${directories.logs}"
      LEDGER_DIR="${directories.ledger}"
      KEYPAIR_DIR="${directories.keypairs}"
      
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
      echo "Building ${projectName}..."
      ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
      
      # Start the validator
      echo "Starting local validator..."
      ${inputs'.zero-nix.packages.solana-node}/bin/solana-test-validator \
        --ledger "$LEDGER_DIR" \
        --keypair "$KEYPAIR_DIR/payer.json" \
        --bind-address ${validator.bindAddress} \
        --rpc-port ${toString validator.rpcPort} \
        --rpc-bind-address ${validator.bindAddress} \
        --dynamic-port-range ${validator.portRange} \
        --enable-rpc-transaction-history \
        --enable-extended-tx-metadata-storage \
        --log "$VALIDATOR_LOG_DIR/validator.log" \
        --reset \
        --quiet &
      
      VALIDATOR_PID=$!
      
      # Wait for validator to start
      echo "Waiting for validator to start..."
      timeout=30
      while ! ${inputs'.zero-nix.packages.solana-tools}/bin/solana cluster-version --url http://localhost:${toString validator.rpcPort} >/dev/null 2>&1; do
        sleep 1
        timeout=$((timeout - 1))
        if [ $timeout -le 0 ]; then
          echo "Validator failed to start within 30 seconds"
          exit 1
        fi
      done
      
      echo "Validator is running!"
      
      # Configure Solana CLI
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --url http://localhost:${toString validator.rpcPort}
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --keypair "$KEYPAIR_DIR/payer.json"
      
      # Fund the payer account
      echo "Funding payer account..."
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana airdrop ${toString validator.airdropAmount} "$KEYPAIR_DIR/payer.json" --url http://localhost:${toString validator.rpcPort}
      
      # Deploy the program
      echo "Deploying ${projectName}..."
      if ${inputs'.zero-nix.packages.solana-tools}/bin/anchor deploy --provider.cluster localnet; then
        echo "Program deployed successfully!"
      else
        echo "Program deployment failed, but validator is still running"
      fi
      
      # Show useful information
      echo
      echo "${projectName} Local Development Environment"
      echo "=============================================="
      echo "RPC URL: http://localhost:${toString validator.rpcPort}"
      echo "WebSocket URL: ws://localhost:${toString validator.wsPort}"
      echo "Payer keypair: $KEYPAIR_DIR/payer.json"
      echo "Program keypair: $KEYPAIR_DIR/program.json"
      echo "Logs: $VALIDATOR_LOG_DIR/validator.log"
      echo
      echo "Useful commands:"
      echo "  solana balance                 - Check balance"
      echo "  solana logs                    - Stream program logs"
      echo "  anchor test --skip-local-validator - Run tests against local validator"
      echo
      echo "Press Ctrl+C to stop the validator"
      
      # Keep the script running
      wait $VALIDATOR_PID
    '';
  
  # IDL builder function
  mkIDLBuilder = { programs, directories }:
    pkgs.writeShellScriptBin "idl-build" ''
      set -euo pipefail
      
      # Check if program name was provided
      PROGRAM_NAME="''${1:-}"
      
      if [ -z "$PROGRAM_NAME" ]; then
        echo "Usage: idl-build <program-name>"
        echo ""
        echo "Available programs:"
        ${pkgs.lib.concatStringsSep "\n" (
          pkgs.lib.mapAttrsToList (key: config: ''
            echo "  - ${config.name}"
          '') programs
        )}
        exit 1
      fi
      
      # Find the program config
      PROGRAM_FOUND=false
      CARGO_TOML=""
      DISPLAY_NAME=""
      CUSTOM_DEPS=""
      
      ${pkgs.lib.concatStringsSep "\n" (
        pkgs.lib.mapAttrsToList (key: config: ''
          if [ "$PROGRAM_NAME" = "${config.name}" ]; then
            PROGRAM_FOUND=true
            CARGO_TOML="${config.cargoToml}"
            DISPLAY_NAME="${config.displayName}"
            ${if config ? idlDependencies then ''
              CUSTOM_DEPS=$(cat << 'EOF'
${config.idlDependencies}
EOF
            )
            '' else ""}
          fi
        '') programs
      )}
      
      if [ "$PROGRAM_FOUND" = "false" ]; then
        echo "Error: Unknown program '$PROGRAM_NAME'"
        echo ""
        echo "Available programs:"
        ${pkgs.lib.concatStringsSep "\n" (
          pkgs.lib.mapAttrsToList (key: config: ''
            echo "  - ${config.name}"
          '') programs
        )}
        exit 1
      fi
      
      echo "Building IDL for $DISPLAY_NAME..."
      
      # Create target/idl directory if it doesn't exist
      mkdir -p ${directories.idl}
      
      # Determine program directory
      PROGRAM_DIR=$(dirname "$CARGO_TOML")
      
      # Step 1: Build the program with stable Rust if needed
      echo "Step 1: Checking program build..."
      export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
      export RUST_LOG=warn
      
      PROGRAM_BINARY="${directories.deploy}/$PROGRAM_NAME.so"
      
      if [ ! -f "$PROGRAM_BINARY" ]; then
        echo "Building program first..."
        # Try anchor build first
        if [ -f "Anchor.toml" ] && anchor build --skip-lint 2>/dev/null; then
          echo "Program built with anchor"
        else
          # Fall back to cargo build-sbf
          cd "$PROGRAM_DIR"
          cargo build-sbf 2>&1 || true
          cd -
        fi
      else
        echo "Program binary already exists"
      fi
      
      # Step 2: Generate IDL with nightly Rust
      echo ""
      echo "Step 2: Generating IDL with nightly Rust..."
      
      cd "$PROGRAM_DIR"
      
      # Backup original Cargo.toml
      cp Cargo.toml Cargo.toml.orig
      
      # Use custom dependencies if provided
      if [ -n "$CUSTOM_DEPS" ]; then
        echo "$CUSTOM_DEPS" > Cargo.toml
      fi
      
      # Setup nightly environment
      export PATH="${inputs'.zero-nix.packages.nightly-rust}/bin:$PATH"
      export RUSTFLAGS='--cfg procmacro2_semver_exempt'
      
      # Run IDL generation test
      echo "Running IDL generation test..."
      if ${inputs'.zero-nix.packages.nightly-rust}/bin/cargo test --lib __anchor_private_print_idl --features idl-build -- --nocapture 2>&1 | tee /tmp/idl-output.log; then
        echo "IDL test completed"
        
        # Extract IDL JSON
        echo "Extracting IDL from test output..."
        
        # Find the line containing the main IDL JSON (after all events)
        IDL_START=$(grep -n '^{$' /tmp/idl-output.log | while read line_info; do
          LINE_NUM=$(echo "$line_info" | cut -d: -f1)
          # Check if next line contains "address":
          NEXT_LINE=$((LINE_NUM + 1))
          if sed -n "''${NEXT_LINE}p" /tmp/idl-output.log | grep -q '"address":'; then
            echo "$LINE_NUM"
            break
          fi
        done)
        
        if [ -n "$IDL_START" ]; then
          echo "Found IDL start at line $IDL_START"
          
          # Find the matching closing brace
          tail -n +$IDL_START /tmp/idl-output.log | awk '
          BEGIN { level = 0; in_json = 1 }
          {
              if (in_json) {
                  print
                  # Count braces
                  line = $0
                  gsub(/[^{}]/, "", line)
                  for (i = 1; i <= length(line); i++) {
                      c = substr(line, i, 1)
                      if (c == "{") level++
                      else if (c == "}") {
                          level--
                          if (level == 0) {
                              in_json = 0
                              exit
                          }
                      }
                  }
              }
          }
          ' > "../../${directories.idl}/$PROGRAM_NAME.json"
        else
          echo "WARNING: Could not find IDL start marker, trying fallback extraction..."
          # Fallback: look for a complete JSON object with version field
          awk '
          BEGIN { in_json = 0; level = 0 }
          /^{$/ && !in_json { in_json = 1; level = 1; print; next }
          in_json {
              print
              line = $0
              gsub(/[^{}]/, "", line)
              for (i = 1; i <= length(line); i++) {
                  c = substr(line, i, 1)
                  if (c == "{") level++
                  else if (c == "}") {
                      level--
                      if (level == 0) exit
                  }
              }
          }
          ' /tmp/idl-output.log > "../../${directories.idl}/$PROGRAM_NAME.json"
        fi
        
        # Restore original Cargo.toml
        mv Cargo.toml.orig Cargo.toml
        cd -
        
        # Verify valid JSON
        if [ -s "${directories.idl}/$PROGRAM_NAME.json" ] && ${pkgs.jq}/bin/jq . "${directories.idl}/$PROGRAM_NAME.json" >/dev/null 2>&1; then
          echo ""
          echo "IDL successfully generated!"
          echo "Location: ${directories.idl}/$PROGRAM_NAME.json"
          echo ""
          echo "IDL summary:"
          ${pkgs.jq}/bin/jq '. | {version, metadata, instructions: (.instructions | length), accounts: (.accounts | length), types: (.types | length)}' "${directories.idl}/$PROGRAM_NAME.json"
        else
          echo "ERROR: Failed to extract valid IDL"
          exit 1
        fi
      else
        echo "IDL generation failed"
        # Restore original Cargo.toml
        mv Cargo.toml.orig Cargo.toml
        cd -
        exit 1
      fi
    '';
}
