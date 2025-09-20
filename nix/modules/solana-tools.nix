# Solana development tools and environment
{ pkgs, inputs', lib, ... }:

{
  packages = with pkgs; [
    openssl
    pkg-config
    protobuf
    inputs'.crate2nix.packages.default  # For generating Cargo.nix
    jq
    just
    llvmPackages.libclang.lib
    cmake
    rust-analyzer  # Add rust-analyzer separately to avoid conflicts
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    libiconv
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];
  
  commands = [
    {
      name = "solana";
      package = inputs'.zero-nix.packages.solana-node;
      help = "Solana CLI and node tools";
    }
    {
      name = "anchor";
      package = inputs'.zero-nix.packages.solana-tools;
      help = "Anchor and SBF development tools";
    }
    {package = inputs'.zero-nix.packages.setup-solana;}
    # Metaplex download script
    {
      name = "download-metaplex";
      help = "Download Metaplex Token Metadata program for tests";
      command = ''
        ${pkgs.bash}/bin/bash ./scripts/download-metaplex.sh
      '';
    }
  ];
  
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
    {
      name = "RUST_SRC_PATH";
      value = "${inputs'.zero-nix.packages.solana-node}/platform-tools/rust/lib/rustlib/src/rust/library";
    }
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
    setup-solana = {
      deps = [];
      text = ''
        echo "Solana development tools loaded"
        echo "Available tools:"
        echo "  - solana: Solana CLI and validator"
        echo "  - anchor: Anchor framework for Solana development"
        echo ""
        echo "Build commands:"
        echo "  - cargo build-sbf           - Build Solana programs directly"
        echo ""
      '';
    };
  };
}
