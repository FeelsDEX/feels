# Feels Protocol-specific tools and CLI utilities
{ pkgs, lib, projectRoot, inputs', ... }:
let
  # Metaplex Token Metadata program binary
  # Downloaded once from Solana mainnet and cached in Nix store
  metaplexBinary = pkgs.runCommand "mpl-token-metadata" {
    nativeBuildInputs = [ inputs'.zero-nix.packages.solana-node ];
    outputHashMode = "flat";
    outputHashAlgo = "sha256";
    # This hash corresponds to Metaplex Token Metadata v1.13.3
    # Hash obtained: 2025-01-14
    # To update if program changes on-chain:
    # 1. solana program dump -u mainnet-beta metaqbxx... output.so
    # 2. nix hash convert --hash-algo sha256 $(shasum -a 256 output.so | cut -d' ' -f1)
    # 3. Update hash below
    outputHash = "sha256-qeO9LJQpoDkAkAx7cD9Aotf1Rb0Z8utv9TqBuFEJ5uc=";
  } ''
    PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    echo "Downloading Metaplex Token Metadata from Solana mainnet..."
    echo "Program ID: $PROGRAM_ID"
    
    solana program dump -u mainnet-beta "$PROGRAM_ID" "$out"
    
    echo "Downloaded to: $out"
    ls -lh "$out"
  '';

  # Feels Protocol CLI binary
  # Built using Solana toolchain's Rust (1.84.1) which satisfies all dependencies
  feels-cli = pkgs.stdenv.mkDerivation {
    pname = "feels";
    version = "0.1.0";
    
    # Use the entire workspace as source since feels-sdk depends on workspace dependencies
    # Force Nix to include the feels-sdk directory by using a fresh source
    src = pkgs.lib.cleanSourceWith {
      src = projectRoot;
      filter = path: type: true;  # Include everything
    };
    
    nativeBuildInputs = [
      inputs'.zero-nix.packages.solana-tools  # Includes Rust 1.84.1
      pkgs.pkg-config
    ];
    
    buildInputs = with pkgs; [
      openssl
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
    ];
    
    # Set up Rust environment to use Solana toolchain
    preBuild = ''
      export HOME=$(mktemp -d)
      export CARGO_HOME="$HOME/.cargo"
      export RUSTC="${inputs'.zero-nix.packages.solana-tools}/bin/rustc"
      export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
    '';
    
    buildPhase = ''
      runHook preBuild
      
      echo "Building feels CLI with Solana Rust toolchain..."
      cargo build --release --manifest-path feels-sdk/Cargo.toml --bin feels
      
      runHook postBuild
    '';
    
    installPhase = ''
      runHook preInstall
      
      mkdir -p $out/bin
      cp target/release/feels $out/bin/
      
      runHook postInstall
    '';
    
    meta = {
      description = "Feels Protocol CLI for Solana";
      homepage = "https://github.com/timewave/feels-solana";
      license = pkgs.lib.licenses.mit;
    };
  };

in
{
  packages = [ feels-cli ];
  
  commands = [
    {
      name = "feels";
      package = feels-cli;
      help = "Feels Protocol CLI (try: feels init --help)";
    }
  ];
  
  env = [
    {
      name = "METAPLEX_BINARY_PATH";
      value = "${metaplexBinary}";
    }
  ];
  
  startup = {
    feels-tools = {
      deps = [];
      text = ''
        echo "  ✓ Feels Protocol CLI: feels (pre-built with Rust 1.84.1)"
        echo "    Usage: feels init protocol | feels init hub | feels init market"
        echo "    Quick: just protocol-init | just hub-init"
        echo ""
        echo "  ✓ Metaplex Token Metadata: ${metaplexBinary}"
        
        if [[ -f "${metaplexBinary}" ]]; then
          FILE_SIZE=$(stat -f%z "${metaplexBinary}" 2>/dev/null || stat -c%s "${metaplexBinary}" 2>/dev/null || echo "unknown")
          echo "    Size: ''${FILE_SIZE} bytes (cached in Nix store)"
        fi
      '';
    };
  };
}

