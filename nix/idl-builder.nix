{ pkgs, inputs' }:

let
  idl-build = pkgs.writeShellScriptBin "idl-build" ''
    set -euo pipefail
    
    echo "Building IDL files for Feels Protocol..."
    
    # Ensure we're in the project root
    if [ ! -f "Anchor.toml" ]; then
      echo "Error: Anchor.toml not found. Run this from the project root."
      exit 1
    fi
    
    # Create target/idl directory if it doesn't exist
    mkdir -p target/idl
    
    # Build the program first to ensure IDL is generated
    echo "Building program..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
    
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
  inherit idl-build;
}