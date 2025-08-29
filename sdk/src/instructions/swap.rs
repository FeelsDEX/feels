use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Build instruction to execute a swap
#[allow(clippy::too_many_arguments)]
pub fn swap_execute(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit: u128,
    is_token_a_to_b: bool,
) -> Instruction {
    let accounts = feels::accounts::ExecuteOrder {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    let data = feels::instruction::OrderExecute {
        amount_in,
        amount_out_minimum,
        sqrt_price_limit,
        is_token_a_to_b,
        duration: None,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to execute a routed swap (two-hop)
#[allow(clippy::too_many_arguments)]
pub fn execute_routed_swap(
    program_id: &Pubkey,
    pool_1: &Pubkey,
    pool_2: &Pubkey,
    feelssol: &Pubkey,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
    user: &Pubkey,
    user_token_in: &Pubkey,
    user_token_out: &Pubkey,
    pool_1_token_in: &Pubkey,
    pool_1_token_out: &Pubkey,
    pool_2_token_in: &Pubkey,
    pool_2_token_out: &Pubkey,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
    sqrt_price_limit_2: Option<u128>,
) -> Instruction {
    let accounts = feels::accounts::ExecuteRoutedSwap {
        pool_1: *pool_1,
        pool_2: *pool_2,
        feelssol: *feelssol,
        token_in_mint: *token_in_mint,
        token_out_mint: *token_out_mint,
        user: *user,
        user_token_in: *user_token_in,
        user_token_out: *user_token_out,
        pool_1_token_in: *pool_1_token_in,
        pool_1_token_out: *pool_1_token_out,
        pool_2_token_in: *pool_2_token_in,
        pool_2_token_out: *pool_2_token_out,
        token_program: spl_token_2022::ID,
    };

    let data = feels::instruction::ExecuteRoutedSwap {
        amount_in,
        amount_out_minimum,
        sqrt_price_limit_1,
        sqrt_price_limit_2,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to get swap tick arrays info
pub fn get_swap_tick_arrays(
    program_id: &Pubkey,
    pool: &Pubkey,
    amount_in: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
) -> Instruction {
    let accounts = feels::accounts::GetOrderTickArrays { pool: *pool };

    let data = feels::instruction::GetOrderTickArrays {
        amount_in,
        sqrt_price_limit,
        zero_for_one,
    };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
