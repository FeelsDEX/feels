//! Feels Protocol SDK - MVP Version
//! 
//! Minimal SDK for interacting with the Feels

pub mod client;
pub mod config;
pub mod error;
pub mod instructions;
pub mod router;
pub mod swap_builder;
pub mod testing;
pub mod types;
pub mod utils;

pub use client::FeelsClient;
pub use config::SdkConfig;
pub use error::{SdkError, SdkResult};
pub use instructions::*;
pub use router::{HubRouter, PoolInfo};
pub use swap_builder::{SwapBuilder, SwapParams, SwapDirection};
pub use testing::{TestCoverage, TestAccountBuilder, SwapTestCase, PositionTestCase, StressTestScenario};
pub use types::*;
pub use utils::*;

use anchor_lang::prelude::*;

/// Program ID for Feels Protocol
pub const PROGRAM_ID: &str = "FEELSjMmBW8cB9SsoNXdQiKtFYbNVUe2tTEKKZmu6E1";

/// Get the program ID as a Pubkey
pub fn program_id() -> Pubkey {
    PROGRAM_ID.parse().unwrap()
}

/// Seeds for common PDAs
pub mod seeds {
    pub const MARKET: &[u8] = b"market";
    pub const BUFFER: &[u8] = b"buffer";
    pub const EPOCH_PARAMS: &[u8] = b"epoch_params";
    pub const VAULT: &[u8] = b"vault";
    pub const VAULT_AUTHORITY: &[u8] = b"vault_authority";
    pub const BUFFER_VAULT: &[u8] = b"buffer_vault";
    pub const BUFFER_AUTHORITY: &[u8] = b"buffer_authority";
    pub const JITOSOL_VAULT: &[u8] = b"jitosol_vault";
    pub const MINT_AUTHORITY: &[u8] = b"mint_authority";
}

/// Find PDA for market
pub fn find_market_address(token_0: &Pubkey, token_1: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::MARKET, token_0.as_ref(), token_1.as_ref()],
        &program_id(),
    )
}

/// Find PDA for buffer
pub fn find_buffer_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::BUFFER, market.as_ref()],
        &program_id(),
    )
}

/// Find PDA for epoch params
pub fn find_epoch_params_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::EPOCH_PARAMS, market.as_ref()],
        &program_id(),
    )
}

/// Find PDA for vault
pub fn find_vault_address(market: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::VAULT, market.as_ref(), mint.as_ref()],
        &program_id(),
    )
}

/// Find PDA for vault authority
pub fn find_vault_authority_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::VAULT_AUTHORITY, market.as_ref()],
        &program_id(),
    )
}