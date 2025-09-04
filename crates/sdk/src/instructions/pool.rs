use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_program, signature::Signature};

// ============================================================================
// Pool Result Types
// ============================================================================

/// Result of a pool creation operation
#[derive(Debug, Clone)]
pub struct PoolCreationResult {
    pub pool_pubkey: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub signature: Signature,
}

/// Result of a pool creation operation (alias)
pub type CreatePoolResult = PoolCreationResult;

/// Pool information
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pubkey: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub current_tick: i32,
    pub tick_spacing: i32,
}

/// Build instruction to initialize a pool
#[allow(clippy::too_many_arguments)]
pub fn initialize_pool(
    program_id: &Pubkey,
    pool: &Pubkey,
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
    feelssol: &Pubkey,
    token_0_vault: &Pubkey,
    token_1_vault: &Pubkey,
    protocol_state: &Pubkey,
    authority: &Pubkey,
    fee_rate: u16,
    initial_sqrt_price: u128,
    base_rate: u16,
    protocol_share: u16,
) -> Instruction {
    // Derive Phase 2 account addresses
    let (enhanced_oracle, _) =
        Pubkey::find_program_address(&[b"enhanced_oracle", pool.as_ref()], program_id);
    let (enhanced_oracle_data, _) =
        Pubkey::find_program_address(&[b"enhanced_oracle_data", pool.as_ref()], program_id);
    let (position_vault, _) =
        Pubkey::find_program_address(&[b"position_vault", pool.as_ref()], program_id);
    let (fee_config, _) =
        Pubkey::find_program_address(&[b"fee_config", pool.as_ref()], program_id);

    let accounts = feels::accounts::InitializePool {
        pool: *pool,
        fee_config,
        token_0_mint: *token_0_mint,
        token_1_mint: *token_1_mint,
        feelssol: *feelssol,
        token_0_vault: *token_0_vault,
        token_1_vault: *token_1_vault,
        oracle: enhanced_oracle,
        oracle_data: enhanced_oracle_data,
        position_vault,
        protocol_state: *protocol_state,
        authority: *authority,
        token_program: spl_token_2022::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
    };

    let data = feels::instruction::InitializePool {
        fee_rate,
        initial_sqrt_rate: initial_sqrt_price,
        base_rate,
        protocol_share,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to cleanup empty tick array
pub fn cleanup_empty_tick_array(
    program_id: &Pubkey,
    pool: &Pubkey,
    tick_array: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let accounts = feels::accounts::CleanupEmptyTickArray {
        pool: *pool,
        tick_array: *tick_array,
        beneficiary: *beneficiary,
    };

    let data = feels::instruction::CleanupEmptyTickArray {};

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
