# WASM tools for building WebAssembly modules with parallel support
{ pkgs, inputs', lib, ... }:

let
  # Use rust-overlay to get a specific tested nightly Rust with proper rust-src support
  # This specific nightly (2024-08-02) is tested and known to work with wasm-bindgen-rayon
  # See: https://github.com/RReverser/wasm-bindgen-rayon
  rustToolchain = pkgs.rust-bin.nightly."2024-08-02".default.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-unknown-unknown" ];
  };
  
in {
  packages = with pkgs; [
    # Rust toolchain with rust-src and wasm32 target (from rust-overlay)
    # This includes rustc, cargo, and rust-src in the proper location for build-std
    rustToolchain
    
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
        echo "============================"
        echo "  wasm-pack: $(wasm-pack --version | head -1)"
        echo "  cargo: $(cargo --version)"
        echo "  rustc: $(rustc --version)"
        echo ""
        echo "Configuration:"
        echo "  Build settings in vanity-miner-wasm/.cargo/config.toml"
        echo "  - atomics and bulk-memory enabled"
        echo "  - build-std for threading support"
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
    # Note: RUSTFLAGS and build-std configuration are in .cargo/config.toml
    # This allows wasm-pack to pick up the settings correctly
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
        echo "======================"
        echo ""
        echo "Tools available:"
        echo "  wasm-pack $(wasm-pack --version | head -1)"
        echo "  cargo $(cargo --version)"
        echo "  rustc $(rustc --version)"
        echo "  nodejs $(node --version)"
        echo ""
        # Verify rust-src is available
        RUSTC_SYSROOT=$(rustc --print sysroot)
        RUST_SRC_PATH="$RUSTC_SYSROOT/lib/rustlib/src/rust"
        if [[ -d "$RUST_SRC_PATH" ]]; then
          echo "Environment configured for parallel WASM builds:"
          echo "  ✓ Rust nightly with rust-src (rust-overlay)"
          echo "  ✓ WASM32 target installed"
          echo "  ✓ Configuration in .cargo/config.toml:"
          echo "    - atomics and bulk-memory features"
          echo "    - build-std for threading support"
        else
          echo "WARNING: rust-src not found in sysroot at $RUST_SRC_PATH"
        fi
        echo ""
        echo "Build commands (use via justfile):"
        echo "  cd vanity-miner-wasm"
        echo "  just build       - Production build with parallel features"
        echo "  just build-dev   - Development build with debug features"
        echo "  just test        - Run WASM tests"
        echo ""
      '';
    };
  };
}