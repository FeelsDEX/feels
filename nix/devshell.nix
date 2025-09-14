# Development shell configuration
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: let
  inherit (pkgs) lib stdenv;
  idlBuilderConfig = import ./idl-builder.nix { inherit pkgs inputs' projectConfig; };
in {
  packages = with pkgs; [
    openssl
    pkg-config
    protobuf  # For off-chain builds
    inputs'.crate2nix.packages.default  # For generating Cargo.nix
    jq  # For JSON parsing in scripts
    just  # Task runner for build and test commands
    # Dependencies for librocksdb-sys and libclang (use solana-node's clang to avoid collision)
    llvmPackages.libclang.lib
    cmake
    # RocksDB system dependencies
    zlib
    bzip2
    lz4
    zstd
    snappy
  ] ++ lib.optionals stdenv.isDarwin [
    libiconv  # Required for macOS builds
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];
  
  commands = [
    # Separate Solana node and dev tools
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
    # Remove nightly-rust from commands to avoid collision
    {package = inputs'.zero-nix.packages.setup-solana;}
    # IDL builder and cargo wrapper
    {
      name = "idl-build";
      package = idlBuilderConfig.idl-build;
      help = "Build IDL for Anchor programs";
    }
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
    # RocksDB environment variables
    {
      name = "ZLIB_INCLUDE_DIR";
      value = "${pkgs.zlib.dev}/include";
    }
    {
      name = "ZLIB_LIB_DIR";
      value = "${pkgs.zlib}/lib";
    }
    {
      name = "BZIP2_INCLUDE_DIR";
      value = "${pkgs.bzip2.dev}/include";
    }
    {
      name = "BZIP2_LIB_DIR";
      value = "${pkgs.bzip2}/lib";
    }
    {
      name = "LZ4_INCLUDE_DIR";
      value = "${pkgs.lz4.dev}/include";
    }
    {
      name = "LZ4_LIB_DIR";
      value = "${pkgs.lz4}/lib";
    }
    {
      name = "ZSTD_INCLUDE_DIR";
      value = "${pkgs.zstd.dev}/include";
    }
    {
      name = "ZSTD_LIB_DIR";
      value = "${pkgs.zstd}/lib";
    }
    {
      name = "SNAPPY_INCLUDE_DIR";
      value = "${pkgs.snappy}/include";
    }
    {
      name = "SNAPPY_LIB_DIR";
      value = "${pkgs.snappy}/lib";
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
  
  devshell.startup.setup-solana = {
    deps = [];
    text = ''
      echo "${projectConfig.devEnv.welcomeMessage}"
      echo ""
      echo "Available tools:"
      echo "  - solana: Solana CLI and validator"
      echo "  - anchor: Anchor framework for Solana development"
      echo "  - idl-build: Build IDL for Anchor programs"
      echo "  - update-cargo-nix: Update Cargo.nix for Nix builds"
      echo "  - just: Task runner (run 'just' to see commands)"
      echo ""
      echo "Quick commands:"
      echo "  - just                      - Show all available commands"
      echo "  - just build                - Build all programs"
      echo "  - just test                 - Run tests"
      echo "  - idl-build                 - Generate IDL (uses cargo +nightly wrapper)"
      echo ""
      echo "Build commands:"
      echo "  - cargo build-sbf           - Build Solana programs directly"
      echo ""
      echo "Nix apps (run with 'nix run .#<app>'):"
      echo "  - devnet                    - Start local validator with auto-deploy"
      echo "  - bpf-build                 - Build all BPF programs using Nix"
      echo "  - idl-build                 - Generate IDL files"
      echo ""
    '';
  };
}