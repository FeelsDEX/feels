# Crate2nix-related commands for fast Nix builds
{
  pkgs,
  inputs',
  ...
}: let
  crate2nix = inputs'.crate2nix.packages.default;
in {
  # Generate or update Cargo.nix for optimized Nix builds
  generate-cargo-nix = {
    type = "app";
    meta.description = "Generate Cargo.nix for faster Nix builds";
    program = "${pkgs.writeShellScriptBin "generate-cargo-nix" ''
      set -e
      
      if [ -f Cargo.nix ]; then
        echo "Updating existing Cargo.nix for faster Nix builds..."
      else
        echo "Generating Cargo.nix for faster Nix builds..."
      fi
      echo ""
      
      if [ ! -f Cargo.toml ]; then
        echo "Error: No Cargo.toml found in current directory"
        exit 1
      fi
      
      # Set up nightly rust environment for Edition 2024 support
      export PATH="${inputs'.zero-nix.packages.nightly-rust}/bin:$PATH"
      export CARGO="${inputs'.zero-nix.packages.nightly-rust}/bin/cargo"
      export RUSTC="${inputs'.zero-nix.packages.nightly-rust}/bin/rustc"
      
      # Set up environment variables for macOS
      export MACOSX_DEPLOYMENT_TARGET=11.0
      export SOURCE_DATE_EPOCH=1686858254
      export RUST_BACKTRACE=1
      
      echo "Using nightly Rust: $(cargo --version)"
      echo "Processing Cargo workspace..."
      ${crate2nix}/bin/crate2nix generate \
        --output ./Cargo.nix
      
      echo ""
      echo "Cargo.nix updated successfully!"
      echo ""
      echo "This file contains Nix expressions for all Rust dependencies."
      echo "Commit Cargo.nix to version control for reproducible builds."
      echo ""
      echo "Usage:"
      echo "  - Run this command whenever you modify Cargo.toml or add/remove dependencies"
      echo "  - The generated Cargo.nix enables fast incremental builds with Nix caching"
      echo "  - Each dependency becomes a separate Nix derivation for maximum caching efficiency"
    ''}/bin/generate-cargo-nix";
  };
}