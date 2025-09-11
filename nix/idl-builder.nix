{ pkgs, inputs', projectConfig }:

let
  # Get nightly rust from zero-nix
  nightly-rust = inputs'.zero-nix.packages.nightly-rust;
  
  # Create a cargo wrapper that intercepts +nightly syntax
  cargo-wrapper = pkgs.writeShellScriptBin "cargo" ''
    # Check if first argument is +nightly
    if [ "$1" = "+nightly" ]; then
      # Remove +nightly and call the real nightly cargo
      shift
      exec ${nightly-rust}/bin/cargo "$@"
    else
      # Pass through to regular cargo
      exec ${nightly-rust}/bin/cargo "$@"
    fi
  '';
  
  idl-build = pkgs.writeShellScriptBin "idl-build" ''
    set -euo pipefail
    
    echo "Building IDL files for ${projectConfig.projectName}..."
    
    # Ensure we're in the project root (if configured)
    ${if projectConfig.buildConfig.requireAnchorToml then ''
      if [ ! -f "Anchor.toml" ]; then
        echo "Error: Anchor.toml not found. Run this from the project root."
        exit 1
      fi
    '' else ""}
    
    # Create target/idl directory if it doesn't exist
    mkdir -p ${projectConfig.directories.idl}
    
    # Setup environment with our cargo wrapper
    export PATH="${cargo-wrapper}/bin:${nightly-rust}/bin:${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
    export RUSTFLAGS='--cfg procmacro2_semver_exempt'
    
    # Create a temporary directory for our cargo wrapper
    WRAPPER_DIR=$(mktemp -d)
    
    # Create cargo wrapper in temp dir that will be first in PATH
    cat > "$WRAPPER_DIR/cargo" << 'CARGO_EOF'
#!/usr/bin/env bash
if [ "$1" = "+nightly" ]; then
  shift
  exec ${nightly-rust}/bin/cargo "$@"
else
  exec ${nightly-rust}/bin/cargo "$@"
fi
CARGO_EOF
    chmod +x "$WRAPPER_DIR/cargo"
    
    # Put our wrapper first in PATH
    export PATH="$WRAPPER_DIR:$PATH"
    
    # Clean up on exit
    trap "rm -rf \$WRAPPER_DIR" EXIT
    
    # First, build the program with cargo build-sbf
    echo "Building program with cargo build-sbf..."
    cargo build-sbf -p feels || echo "Warning: Build failed, but continuing with IDL generation"
    
    # Now generate the IDL by running the test directly
    echo "Generating IDL..."
    
    # Change to the program directory
    cd programs/feels
    
    # Run the IDL generation test directly
    echo "Running: cargo test __anchor_private_print_idl --features idl-build -- --nocapture"
    if cargo test __anchor_private_print_idl --features idl-build -- --nocapture 2>&1 | tee /tmp/idl-output.log; then
      echo "IDL test completed"
      
      # Extract the IDL JSON from the output
      # First try to find JSON between markers
      if grep -q "==== IDL JSON START ====" /tmp/idl-output.log; then
        sed -n '/==== IDL JSON START ====/,/==== IDL JSON END ====/p' /tmp/idl-output.log | sed '1d;$d' > "../../target/idl/feels.json"
      else
        # Otherwise extract the JSON object
        awk '/^{/{p=1} p{print} /^}/{p=0}' /tmp/idl-output.log > "../../target/idl/feels.json"
      fi
      
      # Verify we got valid JSON
      if [ -s "../../target/idl/feels.json" ] && grep -q '"version"' "../../target/idl/feels.json"; then
        echo "IDL extracted to target/idl/feels.json"
      else
        echo "Failed to extract valid IDL from output"
        echo "Output was:"
        cat /tmp/idl-output.log
        cd -
        exit 1
      fi
      
      cd -
    else
      echo "IDL test failed"
      cd -
      exit 1
    fi
    
    # Copy IDL files to target/idl
    if [ -d "target/types" ]; then
      echo "Copying IDL files..."
      cp target/types/*.ts target/idl/ 2>/dev/null || true
    fi
    
    # Generate JSON IDL if it exists
    if [ -f "target/idl/feels_protocol.json" ]; then
      echo "JSON IDL generated: target/idl/feels_protocol.json"
    fi
    
    # Generate TypeScript types if they exist
    if [ -f "target/types/feels_protocol.ts" ]; then
      echo "TypeScript types generated: target/types/feels_protocol.ts"
    fi
    
    echo "IDL build complete!"
  '';
in

{
  inherit idl-build cargo-wrapper;
}