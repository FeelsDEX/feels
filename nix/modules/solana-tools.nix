# Solana development tools and environment
{ pkgs, inputs', lib, ... }:

let
  # Create a wrapper for solana binaries that excludes cargo
  # This prevents collision with the standalone cargo from nixpkgs
  solana-wrapped = pkgs.runCommand "solana-wrapped" {} ''
    mkdir -p $out/bin
    # Copy only solana-related binaries, exclude cargo and rust tools
    for bin in ${inputs'.zero-nix.packages.solana-node}/bin/*; do
      binname=$(basename "$bin")
      # Skip cargo and rust tools that might conflict
      if [[ "$binname" != "cargo"* ]] && [[ "$binname" != "rust"* ]]; then
        ln -s "$bin" "$out/bin/$binname"
      fi
    done
  '';
  
  # Wrap anchor/solana-tools to exclude conflicting binaries
  anchor-wrapped = pkgs.runCommand "anchor-wrapped" {} ''
    mkdir -p $out/bin
    # Copy only anchor and solana-related binaries, exclude cargo and rust tools
    for bin in ${inputs'.zero-nix.packages.solana-tools}/bin/*; do
      binname=$(basename "$bin")
      # Skip cargo and rust tools that might conflict
      if [[ "$binname" != "cargo"* ]] && [[ "$binname" != "rust"* ]]; then
        ln -s "$bin" "$out/bin/$binname"
      fi
    done
  '';
in {
  packages = with pkgs; [
    # Full Rust toolchain for IDE support (includes proc-macro server)
    # Using nixpkgs Rust which has complete toolchain including proc-macro server
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer
    
    # Wrapped Solana tools (without cargo collision)
    solana-wrapped
    anchor-wrapped
    
    # Build essentials
    openssl
    pkg-config
    protobuf
    inputs'.crate2nix.packages.default  # For generating Cargo.nix
    jq
    just
    llvmPackages.libclang.lib
    cmake
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    libiconv
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];
  
  commands = [];
  
  env = [
    {
      name = "PKG_CONFIG_PATH";
      value = "${pkgs.openssl.dev}/lib/pkgconfig";
    }
    {
      name = "OPENSSL_DIR";
      value = "${pkgs.openssl.dev}";
    }
    {
      name = "OPENSSL_LIB_DIR";
      value = "${pkgs.openssl.out}/lib";
    }
    {
      name = "OPENSSL_INCLUDE_DIR";
      value = "${pkgs.openssl.dev}/include";
    }
    {
      name = "MACOSX_DEPLOYMENT_TARGET";
      value = "11.0";
    }
    {
      name = "SOURCE_DATE_EPOCH";
      value = "1686858254";
    }
    {
      name = "LIBCLANG_PATH";
      value = "${pkgs.llvmPackages.libclang.lib}/lib";
    }
    {
      name = "BINDGEN_EXTRA_CLANG_ARGS";
      value = "-I${pkgs.llvmPackages.clang-unwrapped.lib}/lib/clang/${pkgs.llvmPackages.clang-unwrapped.version}/include";
    }
    # Note: RUST_SRC_PATH not needed - rust-analyzer discovers it from rustc automatically
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    {
      name = "LIBRARY_PATH";
      value = "${pkgs.libiconv}/lib:${pkgs.llvmPackages.libclang.lib}/lib";
    }
    {
      name = "DYLD_LIBRARY_PATH";
      value = "${pkgs.llvmPackages.libclang.lib}/lib";
    }
    {
      name = "LDFLAGS";
      value = "-L${pkgs.libiconv}/lib -L${pkgs.llvmPackages.libclang.lib}/lib";
    }
  ];
  
  startup = {
    solana-tools = {
      deps = [];
      text = ''
        echo "Solana Development Tools"
        echo "========================"
        echo ""
        echo "Build Tools:"
        echo "  anchor          - Anchor framework (build, test, deploy)"
        echo "  cargo           - Rust build system"
        echo "  cargo build-sbf - Build Solana programs (BPF)"
        echo ""
        echo "Solana CLI:"
        echo "  solana          - Main Solana CLI (program deploy, account info)"
        echo "  solana-validator - Run local test validator"
        echo "  spl-token       - SPL Token CLI"
        echo ""
        echo "Development:"
        echo "  rust-analyzer   - IDE support for Rust"
        echo "  clippy          - Rust linter"
        echo "  rustfmt         - Rust formatter"
        echo ""
        echo "Utilities:"
        echo "  just            - Task runner (see 'just --list')"
        echo "  jq              - JSON processor"
        echo ""
      '';
    };
  };
  
}
