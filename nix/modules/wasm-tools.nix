# WASM tools for building WebAssembly modules with parallel support
{ pkgs, inputs', lib, ... }:

let
  # Get nightly Rust with proper WASM targets and rust-src for build-std
  zero-nix-rust = inputs'.zero-nix.packages.rust or null;
  
  # Fallback to nixpkgs rust if zero-nix is not available
  rust-toolchain = if zero-nix-rust != null then zero-nix-rust else pkgs.rustc;
  
in {
  packages = with pkgs; [
    # Rust toolchain with WASM support
    rust-toolchain
    cargo
    
    # WASM-specific tools
    wasm-pack
    wasm-bindgen-cli
    binaryen  # For wasm-opt
    
    # Additional utilities for building
    pkg-config
    openssl
    
    # Required for multi-threaded WASM builds
    nodejs  # For wasm-pack testing and worker support
  ];
  
  commands = [
    {
      name = "wasm-info";
      command = ''
        echo "WASM Environment Information"
        echo "=========================="
        echo "  wasm-pack: $(wasm-pack --version | head -1)"
        echo "  cargo: $(cargo --version)"
        echo "  rustc: $(rustc --version)"
        echo ""
        echo "Environment variables:"
        echo "  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS = $CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS"
        echo ""
        echo "Use 'just' commands for building WASM modules"
      '';
      help = "Show WASM environment information";
    }
  ];
  
  env = [
    {
      name = "RUST_LOG";
      value = "info";
    }
    {
      name = "WASM_BINDGEN_TEST_TIMEOUT";
      value = "60";
    }
    {
      name = "RUSTUP_HOME";
      value = "$HOME/.rustup";
    }
    # WASM-specific compilation flags for atomic features
    {
      name = "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS";
      value = "-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--export-table";
    }
    # Rust backtrace for debugging
    {
      name = "RUST_BACKTRACE";
      value = "1";
    }
  ];
  
  startup = {
    wasm = {
      deps = [];
      text = ''
        echo "WASM Environment Ready"
        echo "====================="
        echo ""
        echo "Tools available:"
        echo "  wasm-pack $(wasm-pack --version | head -1)"
        echo "  cargo $(cargo --version)"
        echo "  nodejs $(node --version)"
        echo ""
        echo "Environment configured for parallel WASM builds:"
        echo "  ✓ Atomic operations (+atomics,+bulk-memory,+mutable-globals)"
        echo "  ✓ Shared memory and import/export table"
        echo "  ✓ Nightly Rust with build-std support"
        echo ""
        echo "Build commands (use via justfile):"
        echo "  just build       - Production build with parallel features"
        echo "  just build-dev   - Development build with debug features"
        echo "  just build-simple - Single-threaded fallback build"
        echo "  just test        - Run WASM tests"
        echo ""
        # Ensure wasm32 target is installed if rustup is available
        if command -v rustup &> /dev/null; then
          if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
            echo "Installing wasm32-unknown-unknown target..."
            rustup target add wasm32-unknown-unknown
          fi
          # Check for nightly toolchain with rust-src
          if ! rustup component list --toolchain nightly | grep -q "rust-src (installed)"; then
            echo "Installing rust-src component for nightly..."
            rustup component add rust-src --toolchain nightly
          fi
        fi
      '';
    };
  };
}