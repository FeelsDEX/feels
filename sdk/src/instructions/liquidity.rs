use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_program};

/// Build instruction to add liquidity to a pool using the unified order system
#[allow(clippy::too_many_arguments)]
pub fn add_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
    leverage: Option<u64>,
    _amount_a_max: u64,
    _amount_b_max: u64,
) -> Instruction {
    // Use the unified Order context
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: system_program::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    // Use the unified order instruction with OrderType::Liquidity
    let params = feels::OrderParams {
        amount: liquidity_amount as u64, // Convert liquidity to amount
        rate_params: feels::RateParams::RateRange {
            tick_lower,
            tick_upper,
        },
        duration: feels::Duration::Weekly, // Default to weekly duration
        leverage: leverage.unwrap_or(1_000_000), // Default to 1x leverage
        order_type: feels::OrderType::Liquidity,
        limit_value: 0, // Not used for liquidity
    };

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to remove liquidity from a pool using the unified order system
#[allow(clippy::too_many_arguments)]
pub fn remove_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
    _amount_a_min: u64,
    _amount_b_min: u64,
) -> Instruction {
    // Use the unified Order context
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: system_program::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    // Use the unified order instruction with OrderType::Liquidity and negative amount for removal
    let params = feels::OrderParams {
        amount: liquidity_amount as u64, // Amount to remove
        rate_params: feels::RateParams::RateRange {
            tick_lower,
            tick_upper,
        },
        duration: feels::Duration::Swap, // Immediate removal
        leverage: 1_000_000, // No leverage for removal
        order_type: feels::OrderType::Liquidity,
        limit_value: 0, // Use amount_a_min and amount_b_min separately if needed
    };

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}