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
    crate2nix.url = "github:nix-community/crate2nix";
  };

  outputs = { self, flake-parts, ... } @ inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      flake = {
        lib = {
          buildBPFProgram = system: let
            pkgs = inputs.nixpkgs.legacyPackages.${system};
            inputs' = {
              zero-nix.packages = inputs.zero-nix.packages.${system};
              crate2nix.packages = inputs.crate2nix.packages.${system};
            };
            bpfBuilderConfig = import ./nix/bpf-builder.nix { inherit pkgs inputs'; };
          in bpfBuilderConfig.buildBPFProgram;
        };
      };

      perSystem = { pkgs, inputs', system, ... }:
        let
          devshellConfig = import ./nix/devshell.nix { inherit pkgs inputs'; };
          packagesConfig = import ./nix/packages.nix { inherit pkgs inputs'; };
          localConfig = import ./nix/local.nix { inherit pkgs inputs'; };
          idlBuilderConfig = import ./nix/idl-builder.nix { inherit pkgs inputs'; };
          crate2nixConfig = import ./nix/crate2nix.nix { inherit pkgs inputs'; };
        in {
          devshells.default = devshellConfig;

          packages = packagesConfig // {
            inherit (packagesConfig) default feels-protocol;
          };

          apps = {
            generate-cargo-nix = crate2nixConfig.generate-cargo-nix;
            idl-build = {
              type = "app";
              program = "${idlBuilderConfig.idl-build}/bin/idl-build";
            };
            local-devnet = {
              type = "app";
              program = "${localConfig.local-devnet}/bin/local-devnet";
            };
            node = {
              type = "app";
              program = "${inputs'.zero-nix.packages.solana-node}/bin/solana-test-validator";
            };
          };
        };
    };
}


