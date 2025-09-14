# IDL Builder for Anchor programs
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: rec {
  # Get the stable rust from solana-tools (compatible with solana-packet)
  solana-rust = inputs'.zero-nix.packages.solana-tools;
  
  # Get nightly rust for IDL generation  
  nightly-rust = inputs'.zero-nix.packages.nightly-rust;

  # Interactive IDL builder script that accepts program name
  idl-build = pkgs.writeShellScriptBin "idl-build" ''
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
        '') projectConfig.programs
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
      '') projectConfig.programs
    )}
    
    if [ "$PROGRAM_FOUND" = "false" ]; then
      echo "Error: Unknown program '$PROGRAM_NAME'"
      echo ""
      echo "Available programs:"
      ${pkgs.lib.concatStringsSep "\n" (
        pkgs.lib.mapAttrsToList (key: config: ''
          echo "  - ${config.name}"
        '') projectConfig.programs
      )}
      exit 1
    fi
    
    echo "Building IDL for $DISPLAY_NAME..."
    
    # Ensure we're in the project root
    ${if projectConfig.buildConfig.requireAnchorToml then ''
      if [ ! -f "Anchor.toml" ]; then
        echo "Error: Anchor.toml not found. Run this from the project root."
        exit 1
      fi
    '' else ""}
    
    # Create target/idl directory if it doesn't exist
    mkdir -p ${projectConfig.directories.idl}
    
    # Determine program directory
    PROGRAM_DIR=$(dirname "$CARGO_TOML")
    
    # Step 1: Build the program with stable Rust if needed
    echo "Step 1: Checking program build..."
    export PATH="${solana-rust}/bin:$PATH"
    export RUST_LOG=warn
    
    PROGRAM_BINARY="${projectConfig.directories.deploy}/$PROGRAM_NAME.so"
    
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
    export PATH="${nightly-rust}/bin:$PATH"
    export RUSTFLAGS='--cfg procmacro2_semver_exempt'
    
    # Run IDL generation test
    echo "Running IDL generation test..."
    if ${nightly-rust}/bin/cargo test --lib __anchor_private_print_idl --features idl-build -- --nocapture 2>&1 | tee /tmp/idl-output.log; then
      echo "IDL test completed"
      
      # Extract IDL JSON
      echo "Extracting IDL from test output..."
      
      # Find the line containing the main IDL JSON (after all events)
      # Look for the JSON object that starts with {"address":
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
        ' > "../../${projectConfig.directories.idl}/$PROGRAM_NAME.json"
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
        ' /tmp/idl-output.log > "../../${projectConfig.directories.idl}/$PROGRAM_NAME.json"
      fi
      
      # Restore original Cargo.toml
      mv Cargo.toml.orig Cargo.toml
      cd -
      
      # Verify valid JSON
      if [ -s "${projectConfig.directories.idl}/$PROGRAM_NAME.json" ] && ${pkgs.jq}/bin/jq . "${projectConfig.directories.idl}/$PROGRAM_NAME.json" >/dev/null 2>&1; then
        echo ""
        echo "IDL successfully generated!"
        echo "Location: ${projectConfig.directories.idl}/$PROGRAM_NAME.json"
        echo ""
        echo "IDL summary:"
        ${pkgs.jq}/bin/jq '. | {version, metadata, instructions: (.instructions | length), accounts: (.accounts | length), types: (.types | length)}' "${projectConfig.directories.idl}/$PROGRAM_NAME.json"
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