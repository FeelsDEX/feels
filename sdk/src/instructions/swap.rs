use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Build instruction to execute a swap using the unified order system
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
    // Use the unified Order context
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        hook_registry: None,
        hook_message_queue: None,
    };

    // Use the unified order instruction with OrderType::Immediate
    let params = feels::OrderParams {
        amount: amount_in,
        rate_params: feels::RateParams::TargetRate {
            sqrt_rate_limit: sqrt_price_limit,
            is_token_a_to_b,
        },
        duration: feels::Duration::Swap,
        leverage: 1_000_000, // 1x leverage (no leverage)
        order_type: feels::OrderType::Immediate,
        limit_value: amount_out_minimum,
    };

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instructions for a routed swap (two-hop) using two separate order instructions
/// Returns a vector of two instructions that should be executed in sequence
#[allow(clippy::too_many_arguments)]
pub fn execute_routed_swap(
    program_id: &Pubkey,
    pool_1: &Pubkey,
    pool_2: &Pubkey,
    _feelssol: &Pubkey,
    _token_in_mint: &Pubkey,
    _token_out_mint: &Pubkey,
    user: &Pubkey,
    user_token_in: &Pubkey,
    user_token_intermediate: &Pubkey, // FeelsSOL account
    user_token_out: &Pubkey,
    pool_1_token_in: &Pubkey,
    pool_1_token_out: &Pubkey,
    pool_2_token_in: &Pubkey,
    pool_2_token_out: &Pubkey,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
    sqrt_price_limit_2: Option<u128>,
) -> Vec<Instruction> {
    // First swap: Token A -> FeelsSOL
    let swap1 = swap_execute(
        program_id,
        pool_1,
        user,
        user_token_in,
        user_token_intermediate,
        pool_1_token_in,
        pool_1_token_out,
        amount_in,
        0, // No minimum for intermediate swap
        sqrt_price_limit_1,
        true, // Assuming token A to FeelsSOL
    );

    // Second swap: FeelsSOL -> Token B
    let swap2 = swap_execute(
        program_id,
        pool_2,
        user,
        user_token_intermediate,
        user_token_out,
        pool_2_token_in,
        pool_2_token_out,
        u64::MAX, // Will use all received FeelsSOL
        amount_out_minimum,
        sqrt_price_limit_2.unwrap_or(0),
        true, // Assuming FeelsSOL to token B
    );

    vec![swap1, swap2]
}

/// Build instruction to compute order tick arrays using the unified system
pub fn get_swap_tick_arrays(
    program_id: &Pubkey,
    pool: &Pubkey,
    amount_in: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
) -> Instruction {
    let accounts = feels::accounts::OrderCompute { pool: *pool };

    let params = feels::OrderComputeParams {
        amount: amount_in,
        rate_params: feels::RateComputeParams::SwapPath {
            sqrt_rate_limit: sqrt_price_limit,
            is_token_a_to_b: zero_for_one,
        },
        duration: feels::Duration::Swap,
        leverage: 1_000_000, // 1x leverage
        order_type: feels::OrderType::Immediate,
    };

    let data = feels::instruction::OrderCompute { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
