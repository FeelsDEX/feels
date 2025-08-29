use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_program};

/// Build instruction to add liquidity to a pool
#[allow(clippy::too_many_arguments)]
pub fn add_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    tick_position_metadata: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    tick_array_lower: &Pubkey,
    tick_array_upper: &Pubkey,
    liquidity_amount: u128,
    leverage: Option<u64>,
    amount_a_max: u64,
    amount_b_max: u64,
    payer: &Pubkey,
) -> Instruction {
    let accounts = feels::accounts::AddLiquidity {
        pool: *pool,
        tick_position_metadata: *tick_position_metadata,
        tick_array_lower: *tick_array_lower,
        tick_array_upper: *tick_array_upper,
        user: *user,
        payer: *payer,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: system_program::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    let data = feels::instruction::AddLiquidity {
        liquidity_amount,
        leverage,
        amount_a_max,
        amount_b_max,
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
    amount_a_min: u64,
    amount_b_min: u64,
) -> Instruction {
    let accounts = feels::accounts::RemoveLiquidity {
        pool: *pool,
        position: *position,
        tick_array_lower: *tick_array_lower,
        tick_array_upper: *tick_array_upper,
        token_vault_a: *token_vault_0,
        token_vault_b: *token_vault_1,
        token_account_a: *token_account_0,
        token_account_b: *token_account_1,
        owner: *position_owner,
        token_program: spl_token_2022::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    let data = feels::instruction::RemoveLiquidity {
        liquidity_amount,
        amount_a_min,
        amount_b_min,
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
    amount_a_requested: u64,
    amount_b_requested: u64,
) -> Instruction {
    let accounts = feels::accounts::CollectFees {
        pool: *pool,
        position: *position,
        token_vault_a: *token_vault_0,
        token_vault_b: *token_vault_1,
        token_account_a: *token_account_0,
        token_account_b: *token_account_1,
        owner: *position_owner,
        token_program: spl_token_2022::ID,
    };

    let data = feels::instruction::CollectFees {
        amount_a_requested,
        amount_b_requested,
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
    amount_a_requested: u64,
    amount_b_requested: u64,
) -> Instruction {
    let accounts = feels::accounts::CollectProtocolFees {
        pool: *pool,
        token_vault_a: *token_vault_0,
        token_vault_b: *token_vault_1,
        recipient_a: *recipient_0,
        recipient_b: *recipient_1,
        authority: *authority,
        token_program: spl_token_2022::ID,
    };

    let data = feels::instruction::CollectProtocolFees {
        amount_a_requested,
        amount_b_requested,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
