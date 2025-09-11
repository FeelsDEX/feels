//! PDA derivation helpers
//! 
//! Single source of truth for all PDA derivations in the protocol

use anchor_lang::prelude::*;
use crate::constants::*;

/// Derive vault PDA for a specific market and token mint
pub fn derive_vault(market: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Derive market authority PDA (unified authority for vaults)
pub fn derive_market_authority(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MARKET_AUTHORITY_SEED, market.as_ref()],
        program_id,
    )
}

/// Derive buffer PDA for a token mint
pub fn derive_buffer(token_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[BUFFER_SEED, token_mint.as_ref()],
        program_id,
    )
}

/// Derive buffer authority PDA
pub fn derive_buffer_authority(token_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[BUFFER_AUTHORITY_SEED, token_mint.as_ref()],
        program_id,
    )
}

/// Derive position PDA from position mint
pub fn derive_position(position_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POSITION_SEED, position_mint.as_ref()],
        program_id,
    )
}

/// Derive tick array PDA
pub fn derive_tick_array(market: &Pubkey, start_tick_index: i32, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED,
            market.as_ref(),
            &start_tick_index.to_le_bytes(),
        ],
        program_id,
    )
}

/// Derive FeelsSOL mint authority PDA
pub fn derive_mint_authority(feelssol_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MINT_AUTHORITY_SEED, feelssol_mint.as_ref()],
        program_id,
    )
}

/// Derive JitoSOL vault PDA (holds JitoSOL backing for FeelsSOL)
pub fn derive_jitosol_vault(feelssol_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[JITOSOL_VAULT_SEED, feelssol_mint.as_ref()],
        program_id,
    )
}

/// Derive vault authority PDA (controls JitoSOL vault)
pub fn derive_vault_authority(feelssol_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, feelssol_mint.as_ref()],
        program_id,
    )
}

/// Derive oracle PDA for a market
pub fn derive_oracle(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"oracle", market.as_ref()],
        program_id,
    )
}

/// Derive epoch params PDA for a market
pub fn derive_epoch_params(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[EPOCH_PARAMS_SEED, market.as_ref()],
        program_id,
    )
}

/// Derive metadata PDA for SPL tokens
pub fn derive_metadata(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
        ],
        &mpl_token_metadata::ID,
    )
}