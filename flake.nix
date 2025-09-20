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
            lib = import ./nix/lib { inherit pkgs inputs'; };
          in lib.mkBPFProgram;
        };
      };

      perSystem = { config, pkgs, inputs', system, ... }:
        let
          # Import the new modular structure
          lib = import ./nix/lib { inherit pkgs inputs'; };
          projectConfig = import ./nix/project/config.nix { inherit pkgs inputs' lib; };
          
          # Import technology modules
          modules = {
            solana-tools = import ./nix/modules/solana-tools.nix { inherit pkgs inputs' lib; };
            databases = import ./nix/modules/databases.nix { inherit pkgs inputs' lib; };
            frontend = import ./nix/modules/frontend.nix { inherit pkgs inputs' lib; };
            indexer = import ./nix/modules/indexer.nix { inherit pkgs inputs' lib; };
          };
          
          # Import environment compositions
          environments = import ./nix/project/environments.nix { 
            inherit pkgs inputs' lib modules projectConfig idlBuilder; 
          };
          
          # Build all configured programs using the new lib functions
          programPackages = pkgs.lib.mapAttrs (key: config:
            lib.mkBPFProgram {
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
          
          # Create validator launcher using lib function
          validatorLauncher = lib.mkValidator {
            inherit (projectConfig) projectName directories validator programs;
          };
          
          # Create IDL builder using lib function
          idlBuilder = lib.mkIDLBuilder {
            inherit (projectConfig) programs directories;
          };
          
          # BPF build app using lib function
          bpfBuildApp = {
            type = "app";
            program = let
              programBuildCommands = pkgs.lib.concatStringsSep "\n" (
                pkgs.lib.mapAttrsToList (key: config: ''
                  echo -e "''${YELLOW}Building ${config.displayName}...''${NC}"
                  nix build .#${config.name} --out-link ./target/nix-${key}
                '') projectConfig.programs
              );
              programOutputPaths = pkgs.lib.concatStringsSep "\n" (
                pkgs.lib.mapAttrsToList (key: config: ''
                  echo "  - ./target/nix-${key}/deploy/"
                '') projectConfig.programs
              );
            in "${pkgs.writeShellScriptBin "bpf-build" ''
              set -e
              
              echo "=== Building ${projectConfig.projectName} BPF Programs with Nix ==="
              echo "This builds all programs using the Nix BPF builder"
              echo ""
              
              # Colors for output
              GREEN='\033[0;32m'
              YELLOW='\033[1;33m'
              RED='\033[0;31m'
              NC='\033[0m' # No Color
              
              # Build programs using nix build
              ${programBuildCommands}
              
              echo ""
              echo -e "''${GREEN}=== BPF Programs Built Successfully ===''${NC}"
              echo ""
              echo "Built programs available in:"
              ${programOutputPaths}
              echo ""
              echo "To deploy programs:"
              echo "  solana program deploy ./target/nix-<program>/deploy/<program>.so"
            ''}/bin/bpf-build";
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
            default = environments.default;
            frontend = environments.frontend;
            indexer = environments.indexer;
            e2e = environments.e2e;
          };

          packages = { 
            default = defaultPackage;
          } // programPackages 
            // config.crate2nix.packages;

          apps = {
            idl-build = {
              type = "app";
              program = "${idlBuilder}/bin/idl-build";
            };
            devnet = {
              type = "app";
              program = "${validatorLauncher}/bin/validator";
            };
            bpf-build = bpfBuildApp;
          };
        };
    };
}


