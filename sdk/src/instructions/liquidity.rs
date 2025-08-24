use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_program,
};

/// Build instruction to add liquidity to a pool
#[allow(clippy::too_many_arguments)]
pub fn add_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    tick_position_metadata: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
    tick_array_lower: &Pubkey,
    tick_array_upper: &Pubkey,
    liquidity_amount: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    payer: &Pubkey,
) -> Instruction {
    let accounts = feels::accounts::AddLiquidity {
        pool: *pool,
        tick_position_metadata: *tick_position_metadata,
        tick_array_lower: *tick_array_lower,
        tick_array_upper: *tick_array_upper,
        user: *user,
        payer: *payer,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        pool_token_0: *pool_token_0,
        pool_token_1: *pool_token_1,
        token_program: spl_token_2022::ID,
        system_program: system_program::ID,
    };
    
    let data = feels::instruction::AddLiquidity {
        liquidity_amount,
        amount_0_max,
        amount_1_max,
    };
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to remove liquidity from a pool
#[allow(clippy::too_many_arguments)]
pub fn remove_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    position: &Pubkey,
    position_owner: &Pubkey,
    tick_array_lower: &Pubkey,
    tick_array_upper: &Pubkey,
    token_vault_0: &Pubkey,
    token_vault_1: &Pubkey,
    token_account_0: &Pubkey,
    token_account_1: &Pubkey,
    liquidity_amount: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Instruction {
    let accounts = feels::accounts::RemoveLiquidity {
        pool: *pool,
        position: *position,
        tick_array_lower: *tick_array_lower,
        tick_array_upper: *tick_array_upper,
        token_vault_0: *token_vault_0,
        token_vault_1: *token_vault_1,
        token_account_0: *token_account_0,
        token_account_1: *token_account_1,
        owner: *position_owner,
        token_program: spl_token_2022::ID,
    };
    
    let data = feels::instruction::RemoveLiquidity {
        liquidity_amount,
        amount_0_min,
        amount_1_min,
    };
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to collect fees from a position
#[allow(clippy::too_many_arguments)]
pub fn collect_fees(
    program_id: &Pubkey,
    pool: &Pubkey,
    position: &Pubkey,
    position_owner: &Pubkey,
    token_vault_0: &Pubkey,
    token_vault_1: &Pubkey,
    token_account_0: &Pubkey,
    token_account_1: &Pubkey,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Instruction {
    let accounts = feels::accounts::CollectFees {
        pool: *pool,
        position: *position,
        token_vault_0: *token_vault_0,
        token_vault_1: *token_vault_1,
        token_account_0: *token_account_0,
        token_account_1: *token_account_1,
        owner: *position_owner,
        token_program: spl_token_2022::ID,
    };
    
    let data = feels::instruction::CollectFees {
        amount_0_requested,
        amount_1_requested,
    };
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to collect protocol fees
#[allow(clippy::too_many_arguments)]
pub fn collect_protocol_fees(
    program_id: &Pubkey,
    pool: &Pubkey,
    authority: &Pubkey,
    token_vault_0: &Pubkey,
    token_vault_1: &Pubkey,
    recipient_0: &Pubkey,
    recipient_1: &Pubkey,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Instruction {
    let accounts = feels::accounts::CollectProtocolFees {
        pool: *pool,
        token_vault_0: *token_vault_0,
        token_vault_1: *token_vault_1,
        recipient_0: *recipient_0,
        recipient_1: *recipient_1,
        authority: *authority,
        token_program: spl_token_2022::ID,
    };
    
    let data = feels::instruction::CollectProtocolFees {
        amount_0_requested,
        amount_1_requested,
    };
    
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}