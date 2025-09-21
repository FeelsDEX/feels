# Feels Protocol project-specific configuration
{ pkgs, inputs', lib, ... }:

{
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
      echo "Feels Protocol Development Environment"
      echo "======================================"'';
    
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
}
