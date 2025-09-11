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
              };
              adapter = {
                name = "feels-jupiter-adapter";
                displayName = "Feels Jupiter Adapter";
                cargoToml = "programs/feels-jupiter-adapter/Cargo.toml";
              };
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
          devshells.default = devshellConfig;

          packages = { default = defaultPackage; } // programPackages // config.crate2nix.packages;

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


