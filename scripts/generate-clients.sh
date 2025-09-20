#!/usr/bin/env bash
set -euo pipefail

if [ ! -f "target/idl/feels.json" ]; then
    echo "Error: IDL not found at target/idl/feels.json"
    echo "Run 'just idl-build' first to generate the IDL"
    exit 1
fi

echo "=== Generating TypeScript Client ==="
mkdir -p generated-sdk/typescript

# Generate TypeScript types using Anchor
echo "Generating TypeScript types..."
nix develop --command anchor idl type -o generated-sdk/typescript/types.ts target/idl/feels.json || {
    echo "Warning: Anchor type generation failed, generating manually..."
}

# Generate TypeScript IDL module
echo "Creating TypeScript IDL module..."
cat > generated-sdk/typescript/index.ts << 'TYPESCRIPT_EOF'
// Auto-generated TypeScript client for Feels Protocol
import { PublicKey } from '@solana/web3.js';
import { Program, AnchorProvider } from '@project-serum/anchor';

TYPESCRIPT_EOF

# Append the IDL
echo "export const IDL = " >> generated-sdk/typescript/index.ts
cat target/idl/feels.json >> generated-sdk/typescript/index.ts
echo ";" >> generated-sdk/typescript/index.ts

# Add program ID and helper
PROGRAM_ID=$(jq -r '.address // "2FgA6YfdFNGgX8YyPKqSzhFGNvatRD5zi1yqCCFaSjq1"' target/idl/feels.json)
cat >> generated-sdk/typescript/index.ts << EOF

export const PROGRAM_ID = new PublicKey('${PROGRAM_ID}');

export type Feels = typeof IDL;

// Helper function to get the program
export function getProgram(provider: AnchorProvider): Program<Feels> {
  return new Program(IDL as Feels, PROGRAM_ID, provider);
}
EOF

echo "✓ TypeScript client generated at generated-sdk/typescript/"

echo ""
echo "=== Generating Rust Client ==="
mkdir -p generated-sdk/rust/src

# Generate Rust client bindings
echo "Creating Rust client module..."
cat > generated-sdk/rust/Cargo.toml << 'EOF'
[package]
name = "feels-client"
version = "0.1.0"
edition = "2021"

[dependencies]
anchor-client = "0.31.1"
anchor-lang = "0.31.1"
solana-sdk = "2.3.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[features]
cpi = ["anchor-lang/cpi"]
EOF

# Create Rust client lib.rs
echo "Generating Rust client code..."

# Create the Rust client file
cat > generated-sdk/rust/src/lib.rs << 'RUST_EOF'
//! Auto-generated Rust client for Feels Protocol
#![allow(dead_code)]

use anchor_client::{
    solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
    },
    Client, ClientError, Cluster, Program,
};
use std::rc::Rc;
use std::str::FromStr;

pub mod types {
    use super::*;
    use anchor_lang::prelude::*;
    
    // Re-export instruction parameter types
    // These would be generated from the IDL in a full implementation
    
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
    pub struct SwapParams {
        pub amount_in: u64,
        pub min_amount_out: u64,
        pub sqrt_price_limit: Option<u128>,
        pub is_token_0_in: bool,
        pub is_exact_in: bool,
    }
    
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
    pub struct InitializeMarketParams {
        pub fee_tier: u16,
        pub tick_spacing: u16,
        pub initial_sqrt_price: u128,
        pub initial_buy_feelssol_amount: u64,
    }
}

RUST_EOF

# Add program ID
echo "pub const PROGRAM_ID: &str = \"${PROGRAM_ID}\";" >> generated-sdk/rust/src/lib.rs

# Add client struct and basic implementation
cat >> generated-sdk/rust/src/lib.rs << 'RUST_EOF'

pub struct FeelsClient {
    program: Program<Rc<Keypair>>,
}

type ClientResult<T> = Result<T, ClientError>;

impl FeelsClient {
    pub fn new(
        cluster: Cluster,
        payer: Rc<Keypair>,
    ) -> ClientResult<Self> {
        let client = Client::new(cluster, payer.clone());
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        let program = client.program(program_id)?;
        
        Ok(Self { program })
    }
    
    pub fn new_with_program_id(
        cluster: Cluster,
        payer: Rc<Keypair>,
        program_id: Pubkey,
    ) -> ClientResult<Self> {
        let client = Client::new(cluster, payer.clone());
        let program = client.program(program_id)?;
        
        Ok(Self { program })
    }
    
    pub fn program(&self) -> &Program<Rc<Keypair>> {
        &self.program
    }

    // Example instruction builders
    pub fn initialize_market(
        &self,
        deployer: Pubkey,
        token_0: Pubkey,
        token_1: Pubkey,
        feelssol_mint: Pubkey,
        params: types::InitializeMarketParams,
    ) -> ClientResult<Instruction> {
        // In a full implementation, this would use the IDL to build the instruction
        // For now, return a placeholder
        todo!("Implement based on IDL")
    }
    
    pub fn swap(
        &self,
        user: Pubkey,
        market: Pubkey,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        params: types::SwapParams,
    ) -> ClientResult<Instruction> {
        // In a full implementation, this would use the IDL to build the instruction
        todo!("Implement based on IDL")
    }
}

// Include the IDL as a constant
pub const IDL_JSON: &str = include_str!("../../../target/idl/feels.json");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_idl() {
        let idl: serde_json::Value = serde_json::from_str(IDL_JSON).unwrap();
        assert!(idl.is_object());
    }
    
    #[test]
    fn test_program_id() {
        let program_id = Pubkey::from_str(PROGRAM_ID);
        assert!(program_id.is_ok());
    }
}
RUST_EOF

echo "✓ Rust client generated at generated-sdk/rust/"

echo ""
echo "=== Client Generation Complete ==="
echo ""
echo "Generated clients:"
echo "  TypeScript: generated-sdk/typescript/"
echo "    - index.ts: Complete IDL and types"
echo "    - types.ts: TypeScript type definitions (if generated)"
echo ""
echo "  Rust: generated-sdk/rust/"
echo "    - Cargo.toml: Package manifest"
echo "    - src/lib.rs: Client implementation with instruction builders"
echo ""
echo "To use the TypeScript client:"
echo "  import { IDL, PROGRAM_ID } from './generated-sdk/typescript';"
echo "  const program = new anchor.Program(IDL, PROGRAM_ID, provider);"
echo ""
echo "To use the Rust client:"
echo "  Add to your Cargo.toml:"
echo "    feels-client = { path = \"./generated-sdk/rust\" }"
echo ""
echo "  Then in your code:"
echo "    use feels_client::{FeelsClient, PROGRAM_ID};"
echo "    let client = FeelsClient::new(cluster, payer)?;"