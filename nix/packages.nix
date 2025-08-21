# Package definitions for feels-solana
{
  pkgs,
  inputs',
  ...
}: let
  bpfBuilder = import ./bpf-builder.nix { inherit pkgs inputs'; };
  crate2nixConfig = import ./crate2nix.nix { inherit pkgs inputs'; };
  
  generate-cargo-nix = pkgs.writeShellScriptBin "generate-cargo-nix" ''
    set -euo pipefail
    
    echo "Generating Cargo.nix for fast incremental builds..."
    
    if [ ! -f "Cargo.toml" ]; then
      echo "Error: Cargo.toml not found. Run this from the project root."
      exit 1
    fi
    
    # Generate Cargo.nix using crate2nix
    ${inputs'.crate2nix.packages.crate2nix}/bin/crate2nix generate
    
    echo "Cargo.nix generated successfully!"
    echo "You can now use 'nix build .#cargo-deps' for fast incremental builds"
  '';
in {
  # Default package - build feels-protocol
  default = pkgs.writeShellScriptBin "feels-build-all" ''
    set -e

    echo "=== Building Feels Protocol ==="
    echo ""

    export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
    export RUST_BACKTRACE=1
    export MACOSX_DEPLOYMENT_TARGET=11.0

    echo "Building feels-protocol program..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
    echo ""
    echo "=== Build Complete ==="
    echo "Built artifacts available in: ./target/deploy/"
  '';

  feels-protocol = bpfBuilder.buildBPFProgram {
    name = "feels";
    src = ./..;
    cargoToml = "programs/feels/Cargo.toml";
  };
  
  generate-cargo-nix = crate2nixConfig.generate-cargo-nix.program;
}


