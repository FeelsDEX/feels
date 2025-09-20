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

pub const PROGRAM_ID: &str = "";

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
