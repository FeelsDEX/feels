/// Unified API instruction builders for the Feels Protocol
/// This module provides simplified interfaces for all order operations
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

// ============================================================================
// Unified Order Instructions
// ============================================================================

/// Build instruction for a simple swap using the unified API
pub fn unified_swap(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_token_a_to_b: bool,
    sqrt_rate_limit: Option<u128>,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount: amount_in,
        config: feels::unified_order::OrderConfig::Swap {
            is_token_a_to_b,
            min_amount_out,
            sqrt_rate_limit,
        },
        advanced: None, // Use defaults
    };

    let data = feels::instruction::OrderUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction for adding liquidity using the unified API
#[allow(clippy::too_many_arguments)]
pub fn unified_add_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    position: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    max_amount_0: u64,
    max_amount_1: u64,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount: liquidity as u64, // Will be interpreted as liquidity amount
        config: feels::unified_order::OrderConfig::AddLiquidity {
            tick_lower,
            tick_upper,
            token_amounts: Some((max_amount_0, max_amount_1)),
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

/// Build instruction for creating a limit order using the unified API
pub fn unified_limit_order(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    amount: u64,
    is_buy: bool,
    target_sqrt_rate: u128,
    expiry: i64,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount,
        config: feels::unified_order::OrderConfig::LimitOrder {
            is_buy,
            target_sqrt_rate,
            expiry,
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

/// Build instruction for a flash loan using the unified API
pub fn unified_flash_loan(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    amount: u64,
    borrow_token_a: bool,
    callback_program: &Pubkey,
    callback_data: Vec<u8>,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount,
        config: feels::unified_order::OrderConfig::FlashLoan {
            borrow_token_a,
            callback_program: *callback_program,
            callback_data,
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

// ============================================================================
// Advanced Order Instructions
// ============================================================================

/// Build instruction for a leveraged swap using the unified API
pub fn unified_leveraged_swap(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_token_a_to_b: bool,
    leverage: u64, // 6 decimals, e.g., 3_000_000 = 3x
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount: amount_in,
        config: feels::unified_order::OrderConfig::Swap {
            is_token_a_to_b,
            min_amount_out,
            sqrt_rate_limit: None,
        },
        advanced: Some(feels::unified_order::AdvancedOrderParams {
            duration: feels::Duration::Swap,
            leverage,
            mev_protection: None,
            hook_data: None,
        }),
    };

    let data = feels::instruction::OrderUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction for adding liquidity with duration lock
#[allow(clippy::too_many_arguments)]
pub fn unified_add_liquidity_locked(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_a: &Pubkey,
    user_token_b: &Pubkey,
    pool_token_a: &Pubkey,
    pool_token_b: &Pubkey,
    position: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    duration: feels::Duration,
    leverage: Option<u64>,
) -> Instruction {
    let accounts = feels::accounts::Order {
        pool: *pool,
        user: *user,
        user_token_a: *user_token_a,
        user_token_b: *user_token_b,
        pool_token_a: *pool_token_a,
        pool_token_b: *pool_token_b,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
        hook_registry: None,
        hook_message_queue: None,
    };

    let params = feels::UnifiedOrderParams {
        amount: liquidity as u64,
        config: feels::unified_order::OrderConfig::AddLiquidity {
            tick_lower,
            tick_upper,
            token_amounts: None, // Let protocol calculate optimal amounts
        },
        advanced: Some(feels::unified_order::AdvancedOrderParams {
            duration,
            leverage: leverage.unwrap_or(1_000_000), // Default 1x
            mev_protection: None,
            hook_data: None,
        }),
    };

    let data = feels::instruction::OrderUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

// ============================================================================
// Order Computation Instructions
// ============================================================================

/// Build instruction to compute optimal routing using unified API
pub fn unified_compute_route(
    program_id: &Pubkey,
    pool: &Pubkey,
    is_swap: bool,
    is_token_a_to_b: bool,
    amount: u64,
) -> Instruction {
    let accounts = feels::accounts::OrderCompute { 
        pool: *pool,
        tick_array_router: None,
        authority: None,
    };

    let params = feels::UnifiedComputeParams {
        order_config: if is_swap {
            feels::unified_order::OrderConfig::Swap {
                is_token_a_to_b,
                min_amount_out: 0, // Just computing
                sqrt_rate_limit: None,
            }
        } else {
            // For liquidity, compute around current tick
            feels::unified_order::OrderConfig::AddLiquidity {
                tick_lower: -887_272, // Full range as example
                tick_upper: 887_272,
                token_amounts: None,
            }
        },
        route_preference: None, // Use default routing
    };

    let data = feels::instruction::OrderComputeUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

// ============================================================================
// Order Modification Instructions
// ============================================================================

/// Build instruction to modify an order using unified API
pub fn unified_modify_order(
    program_id: &Pubkey,
    pool: &Pubkey,
    owner: &Pubkey,
    position: &Pubkey,
    modification: feels::unified_order::OrderModification,
) -> Instruction {
    let accounts = feels::accounts::OrderModify {
        pool: *pool,
        owner: *owner,
        position: *position,
        user_token_a: None,
        user_token_b: None,
        pool_token_a: Pubkey::default(), // Required but not used for cancel
        pool_token_b: Pubkey::default(),
        token_program: spl_token_2022::ID,
    };

    let params = feels::UnifiedModifyParams {
        target: feels::unified_order::ModifyTarget::Position(*position),
        modification,
    };

    let data = feels::instruction::OrderModifyUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to cancel an order
pub fn unified_cancel_order(
    program_id: &Pubkey,
    pool: &Pubkey,
    owner: &Pubkey,
    position: &Pubkey,
) -> Instruction {
    unified_modify_order(
        program_id,
        pool,
        owner,
        position,
        feels::unified_order::OrderModification::Cancel,
    )
}

// ============================================================================
// Pool Configuration Instructions
// ============================================================================

/// Build instruction to configure pool using unified API
pub fn unified_configure_pool(
    program_id: &Pubkey,
    pool: &Pubkey,
    authority: &Pubkey,
    params: feels::PoolConfigParams,
) -> Instruction {
    let accounts = feels::accounts::ConfigurePool {
        pool: *pool,
        authority: *authority,
        hook_registry: None,
        oracle: None,
        system_program: solana_sdk::system_program::ID,
    };

    let data = feels::instruction::ConfigurePool { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}