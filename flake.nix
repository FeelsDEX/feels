{
  description = "Feels Protocol - A concentrated liquidity AMM with unified position-based interactions";

  nixConfig.extra-experimental-features = "nix-command flakes";
  nixConfig.extra-substituters = "https://timewave.cachix.org";
  nixConfig.extra-trusted-public-keys = ''
    timewave.cachix.org-1:nu3Uqsm3sikI9xFK3Mt4AD4Q6z+j6eS9+kND1vtznq4=
  '';

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
    zero-nix.url = "github:timewave-computer/zero.nix/main";
    crate2nix.url = "github:timewave-computer/crate2nix";
  };

  outputs = { self, flake-parts, ... } @ inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ 
        inputs.devshell.flakeModule 
        inputs.crate2nix.flakeModule
      ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      flake = {
        lib = {
          buildBPFProgram = system: let
            pkgs = inputs.nixpkgs.legacyPackages.${system};
            inputs' = {
              zero-nix.packages = inputs.zero-nix.packages.${system};
              crate2nix.packages = inputs.crate2nix.packages.${system};
            };
            # Basic project config for BPF builder - will be overridden in perSystem
            projectConfig = {};
            bpfBuilderConfig = import ./nix/bpf-builder.nix { inherit pkgs inputs' projectConfig; };
          in bpfBuilderConfig.buildBPFProgram;
        };
      };

      perSystem = { config, pkgs, inputs', system, ... }:
        let
          projectConfig = {
            projectName = "Feels Protocol";
            
            programs = {
              main = {
                name = "feels";
                displayName = "Feels Protocol";
                cargoToml = "programs/feels/Cargo.toml";
                # Optional: Custom dependencies for IDL generation
                idlDependencies = ''
[package]
name = "feels"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "feels"

[features]
no-entrypoint = []
cpi = ["no-entrypoint"]
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]

[dependencies]
anchor-lang = { version = "0.31.1", features = ["idl-build"] }
anchor-spl = { version = "0.31.1", features = ["idl-build"] }
solana-program = { version = "2.2.1" }
borsh = { version = "0.10.3" }
bytemuck = { version = "1.14" }
spl-token = { version = "7.0" }
spl-token-2022 = { version = "6.0", default-features = false }
spl-associated-token-account = { version = "6.0" }
mpl-token-metadata = { version = "5.1.1", default-features = false }
orca_whirlpools_core = { version = "2.0.0", default-features = false }
ethnum = { version = "1.5.0", default-features = false }
fixed = { version = "1.28", default-features = false, features = ["serde"] }
num-traits = { version = "0.2", default-features = false }
micromath = { version = "2.1", default-features = false }
integer-sqrt = { version = "0.1" }
                '';
              };
              # Note: feels-jupiter-adapter is a library, not deployed on-chain
              # It provides the Jupiter AMM interface implementation
            };
            
            directories = {
              programs = "programs";
              target = "target";
              deploy = "target/deploy";
              idl = "target/idl";
              types = "target/types";
              logs = "./logs";
              ledger = "./test-ledger";
              keypairs = "./keypairs";
            };
            
            buildConfig = {
              # Whether to check for Anchor.toml at project root
              requireAnchorToml = true;
              # Whether to create client account stubs
              createClientStubs = true;
            };
            
            devEnv = {
              welcomeMessage = ''
                Feels Protocol Development Environment
                ======================================
              '';
              
              # Additional custom commands for the project
              customCommands = [];
            };
            
            # Local validator configuration
            validator = {
              rpcPort = 8899;
              wsPort = 8900;
              bindAddress = "0.0.0.0";
              portRange = "8000-8020";
              airdropAmount = 100;
            };
          };
          
          bpfBuilder = import ./nix/bpf-builder.nix { inherit pkgs inputs' projectConfig; };
          
          # Build all configured programs
          programPackages = pkgs.lib.mapAttrs (key: config:
            bpfBuilder.buildBPFProgram {
              name = config.name;
              src = ./.;
              cargoToml = config.cargoToml;
            }
          ) projectConfig.programs;
          
          # Default package - build all programs
          defaultPackage = pkgs.writeShellScriptBin "build-all" ''
            set -e

            echo "=== Building ${projectConfig.projectName} ==="
            echo ""

            export PATH="${inputs'.zero-nix.packages.solana-tools}/bin:$PATH"
            export RUST_BACKTRACE=1
            export MACOSX_DEPLOYMENT_TARGET=11.0

            echo "Building all programs..."
            ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
            echo ""
            echo "=== Build Complete ==="
            echo "Built artifacts available in: ${projectConfig.directories.deploy}/"
          '';
          
          devshellConfig = import ./nix/devshell.nix { inherit pkgs inputs' projectConfig; };
          devnetConfig = import ./nix/devnet.nix { inherit pkgs inputs' projectConfig; };
          idlBuilderConfig = import ./nix/idl-builder.nix { inherit pkgs inputs' projectConfig; };
          indexerTestConfig = import ./nix/indexer-test.nix { inherit pkgs inputs' projectConfig; };
          
          # Next.js development shell configuration
          nextjsDevShell = pkgs.mkShell {
            buildInputs = with pkgs; [
              # Node.js and package management
              nodejs_20
              pnpm
              yarn
              
              # Development tools
              nodePackages.typescript
              nodePackages.typescript-language-server
              nodePackages."@tailwindcss/language-server"
              nodePackages.eslint
              nodePackages.prettier
              nodePackages.next
              
              # Additional utilities
              jq
              curl
            ];
            
            shellHook = ''
              echo "Feels Protocol Next.js Development Environment"
              echo "Node.js version: $(node --version)"
              echo "pnpm version: $(pnpm --version)"
              echo ""
              echo "Available commands:"
              echo "  pnpm install     - Install dependencies"
              echo "  pnpm dev         - Start development server"
              echo "  pnpm build       - Build for production"
              echo "  pnpm lint        - Run ESLint"
              echo "  pnpm format      - Format code with Prettier"
              echo ""
              
              # Set up environment variables
              export NODE_ENV=development
              export NEXT_TELEMETRY_DISABLED=1
              
              # Ensure pnpm store is in project directory
              export PNPM_HOME="$PWD/.pnpm"
              export PATH="$PNPM_HOME:$PATH"
              
              # Navigate to feels-app directory if it exists
              if [ -d "feels-app" ]; then
                echo "Feels app directory found. Run 'cd feels-app && pnpm install && pnpm dev' to start."
              else
                echo "Feels app directory not found. You can create it with:"
                echo "  mkdir -p feels-app && cd feels-app"
                echo "  pnpm create next-app@latest . --typescript --tailwind --eslint --app --src-dir --import-alias '@/*'"
              fi
            '';
            
            # Environment variables are set in shellHook instead
          };
        in {
          # Configure crate2nix
          crate2nix = {
            cargoNix = ./Cargo.nix;
            devshell.name = "default";
            toolchain = {
              rust = inputs'.zero-nix.packages.nightly-rust;
              cargo = inputs'.zero-nix.packages.nightly-rust;
            };
          };
          devshells = {
            default = devshellConfig;
            nextjs = nextjsDevShell;
            indexer-test = indexerTestConfig;
          };

          packages = { 
            default = defaultPackage;
          } // programPackages 
            // config.crate2nix.packages;

          apps = {
            idl-build = {
              type = "app";
              program = "${idlBuilderConfig.idl-build}/bin/idl-build";
            };
            devnet = {
              type = "app";
              program = "${devnetConfig.devnet}/bin/devnet";
            };
            bpf-build = bpfBuilder.bpf-build;
          };
        };
    };
}


