/// Swap instruction builders using the unified API
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Build instruction to execute a swap
#[allow(clippy::too_many_arguments)]
pub fn swap(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_token_0_to_1: bool,
    sqrt_rate_limit: Option<u128>,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        pool_token_0: *pool_token_0,
        pool_token_1: *pool_token_1,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount: amount_in,
        config: feels::unified_order::OrderConfig::Swap {
            is_token_0_to_1,
            min_amount_out,
            sqrt_rate_limit,
        },
        advanced: None,
    };

    let data = feels::instruction::OrderUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to compute swap route
pub fn compute_swap_route(
    program_id: &Pubkey,
    pool: &Pubkey,
    is_token_0_to_1: bool,
    amount: u64,
) -> Instruction {
    let accounts = feels::accounts::OrderCompute { 
        pool: *pool,
        tick_array_router: None,
        authority: None,
    };

    let params = feels::UnifiedComputeParams {
        order_config: feels::unified_order::OrderConfig::Swap {
            is_token_0_to_1,
            min_amount_out: 0,
            sqrt_rate_limit: None,
        },
        route_preference: None,
    };

    let data = feels::instruction::OrderComputeUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}
