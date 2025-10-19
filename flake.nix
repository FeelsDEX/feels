{
  description = "Feels Protocol - A concentrated liquidity AMM with unified position-based interactions";

  # Note: Nix configuration is managed globally via Home Manager
  # See ~/.config/nix/nix.conf for substituters and trusted keys

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
            wasm-tools = import ./nix/modules/wasm-tools.nix { inherit pkgs inputs' lib; };
          };
          
          # Import environment compositions
          environments = import ./nix/project/environments.nix { 
            inherit pkgs inputs' lib modules projectConfig idlBuilder; 
            projectRoot = ./.;
          };
          
          # Build all configured programs using the new lib functions
          programPackages = pkgs.lib.mapAttrs (key: config:
            lib.mkBPFProgram {
              name = config.name;
              src = ./.;
              cargoToml = config.cargoToml;
            }
          ) projectConfig.programs;
          
          # Default package - build using justfile instead of Nix derivations
          defaultPackage = pkgs.writeShellScriptBin "build-all" ''
            set -e

            echo "=== Building ${projectConfig.projectName} ==="
            echo ""
            echo "Building using justfile (Nix derivations temporarily disabled)..."
            just build
            echo ""
            echo "=== Build Complete ==="
            echo "Use 'just build' for program compilation"
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
                  nix build .#${key} --out-link ./.nix-build/${key}
                '') projectConfig.programs
              );
              programOutputPaths = pkgs.lib.concatStringsSep "\n" (
                pkgs.lib.mapAttrsToList (key: config: ''
                  echo "  - ./.nix-build/${key}/deploy/"
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
              
              # Create .nix-build directory
              mkdir -p .nix-build
              
              # Build programs using nix build
              ${programBuildCommands}
              
              echo ""
              echo -e "''${GREEN}=== BPF Programs Built Successfully ===''${NC}"
              echo ""
              echo "Built programs available in:"
              ${programOutputPaths}
              echo ""
              echo "To deploy programs:"
              echo "  solana program deploy ./.nix-build/<program>/deploy/<program>.so"
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
          } // programPackages; 
            # Temporarily disabled until Cargo.nix is regenerated
            # // config.crate2nix.packages;

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


