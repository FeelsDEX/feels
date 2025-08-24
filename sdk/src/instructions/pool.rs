use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_program,
};

/// Build instruction to initialize a pool
#[allow(clippy::too_many_arguments)]
pub fn initialize_pool(
    program_id: &Pubkey,
    pool: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    feelssol: &Pubkey,
    token_a_vault: &Pubkey,
    token_b_vault: &Pubkey,
    protocol_state: &Pubkey,
    authority: &Pubkey,
    fee_rate: u16,
    initial_sqrt_price: u128,
) -> Instruction {
    let accounts = feels::accounts::InitializePool {
        pool: *pool,
        token_a_mint: *token_a_mint,
        token_b_mint: *token_b_mint,
        feelssol: *feelssol,
        token_a_vault: *token_a_vault,
        token_b_vault: *token_b_vault,
        protocol_state: *protocol_state,
        authority: *authority,
        token_program: spl_token_2022::ID,
        associated_token_program: spl_associated_token_account::ID,
        system_program: system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
    };
    
    let data = feels::instruction::InitializePool {
        fee_rate,
        initial_sqrt_price,
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