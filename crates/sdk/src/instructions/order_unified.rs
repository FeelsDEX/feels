/// Unified order SDK - Matches the refactored on-chain order system
/// All operations go through the single order instruction with hub-and-spoke routing
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use feels::{OrderParams, CreateOrderParams, OrderType, PositionType};

// ============================================================================
// Token Swaps
// ============================================================================

/// Build a swap instruction with hub-and-spoke routing
/// Automatically determines if 1 or 2 hops are needed
pub fn swap(
    program_id: &Pubkey,
    route: Vec<Pubkey>,        // Pool keys (max 2)
    zero_for_one: Vec<bool>,   // Direction for each pool
    user: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    // Additional accounts needed based on route
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::Swap {
            route,
            min_amount_out,
            zero_for_one,
        },
        amount: amount_in,
    });

    let data = feels::instruction::Order { params };

    // Build accounts - simplified for now
    // In production, would need proper account construction
    let mut accounts = vec![
        AccountMeta::new(*user, true),
        // Market field, buffer, etc. based on route
    ];
    accounts.extend(remaining_accounts);

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

/// Build entry instruction: JitoSOL -> FeelsSOL
pub fn enter_protocol(
    program_id: &Pubkey,
    user: &Pubkey,
    jitosol_amount: u64,
    min_feelssol_out: u64,
    entry_pool: &Pubkey,
    // Token accounts
    user_jitosol: &Pubkey,
    user_feelssol: &Pubkey,
    pool_jitosol: &Pubkey,
    pool_feelssol: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::Entry {
            min_feelssol_out,
        },
        amount: jitosol_amount,
    });

    let data = feels::instruction::Order { params };

    // Build accounts for entry flow
    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*entry_pool, false),
        AccountMeta::new(*user_jitosol, false),
        AccountMeta::new(*user_feelssol, false),
        AccountMeta::new(*pool_jitosol, false),
        AccountMeta::new(*pool_feelssol, false),
        // Additional accounts: market_field, buffer, etc.
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

/// Build exit instruction: FeelsSOL -> JitoSOL
pub fn exit_protocol(
    program_id: &Pubkey,
    user: &Pubkey,
    feelssol_amount: u64,
    min_jitosol_out: u64,
    exit_pool: &Pubkey,
    // Token accounts
    user_feelssol: &Pubkey,
    user_jitosol: &Pubkey,
    pool_feelssol: &Pubkey,
    pool_jitosol: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::Exit {
            min_jitosol_out,
        },
        amount: feelssol_amount,
    });

    let data = feels::instruction::Order { params };

    // Build accounts for exit flow
    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*exit_pool, false),
        AccountMeta::new(*user_feelssol, false),
        AccountMeta::new(*user_jitosol, false),
        AccountMeta::new(*pool_feelssol, false),
        AccountMeta::new(*pool_jitosol, false),
        // Additional accounts: market_field, buffer, etc.
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

// ============================================================================
// Position Management
// ============================================================================

/// Enter a position from FeelsSOL
pub fn enter_position(
    program_id: &Pubkey,
    user: &Pubkey,
    feelssol_amount: u64,
    position_type: PositionType,
    min_position_tokens: u64,
    // Accounts
    user_feelssol: &Pubkey,
    position_vault: &Pubkey,
    position_mint: &Pubkey,
    user_position_token: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::EnterPosition {
            position_type,
            min_position_tokens,
        },
        amount: feelssol_amount,
    });

    let data = feels::instruction::Order { params };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*user_feelssol, false),
        AccountMeta::new(*position_vault, false),
        AccountMeta::new(*position_mint, false),
        AccountMeta::new(*user_position_token, false),
        // Additional accounts based on position type
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

/// Exit a position to FeelsSOL
pub fn exit_position(
    program_id: &Pubkey,
    user: &Pubkey,
    position_tokens: u64,
    position_mint: &Pubkey,
    min_feelssol_out: u64,
    // Accounts
    user_position_token: &Pubkey,
    user_feelssol: &Pubkey,
    position_vault: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::ExitPosition {
            position_mint: *position_mint,
            min_feelssol_out,
        },
        amount: position_tokens,
    });

    let data = feels::instruction::Order { params };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*user_position_token, false),
        AccountMeta::new(*user_feelssol, false),
        AccountMeta::new(*position_vault, false),
        AccountMeta::new(*position_mint, false),
        // Additional accounts
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

/// Convert between positions (2 hops through FeelsSOL)
pub fn convert_position(
    program_id: &Pubkey,
    user: &Pubkey,
    amount: u64,
    source_position: &Pubkey,
    target_position_type: PositionType,
    min_tokens_out: u64,
    // Accounts for both legs
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::ConvertPosition {
            source_position: *source_position,
            target_position_type,
            min_tokens_out,
        },
        amount,
    });

    let data = feels::instruction::Order { params };

    let mut accounts = vec![
        AccountMeta::new(*user, true),
        // Position accounts
    ];
    accounts.extend(remaining_accounts);

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

// ============================================================================
// Liquidity Management
// ============================================================================

/// Add liquidity to a pool
pub fn add_liquidity(
    program_id: &Pubkey,
    user: &Pubkey,
    pool: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    // Token accounts
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
    position_nft: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::AddLiquidity {
            tick_lower,
            tick_upper,
            liquidity,
        },
        amount: 0, // Amount calculated from liquidity
    });

    let data = feels::instruction::Order { params };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*user_token_0, false),
        AccountMeta::new(*user_token_1, false),
        AccountMeta::new(*pool_token_0, false),
        AccountMeta::new(*pool_token_1, false),
        AccountMeta::new(*position_nft, false),
        // Additional accounts
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

/// Remove liquidity from a pool
pub fn remove_liquidity(
    program_id: &Pubkey,
    user: &Pubkey,
    pool: &Pubkey,
    liquidity: u128,
    min_amounts: [u64; 2],
    // Accounts
    position_nft: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::RemoveLiquidity {
            liquidity,
            min_amounts,
        },
        amount: 0,
    });

    let data = feels::instruction::Order { params };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*position_nft, false),
        AccountMeta::new(*user_token_0, false),
        AccountMeta::new(*user_token_1, false),
        AccountMeta::new(*pool_token_0, false),
        AccountMeta::new(*pool_token_1, false),
        // Additional accounts
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

// ============================================================================
// Limit Orders
// ============================================================================

/// Place a limit order
pub fn place_limit_order(
    program_id: &Pubkey,
    user: &Pubkey,
    pool: &Pubkey,
    amount: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
    expiration: Option<i64>,
    // Token accounts
    user_token_in: &Pubkey,
    pool_token_in: &Pubkey,
) -> Instruction {
    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::LimitOrder {
            sqrt_price_limit,
            zero_for_one,
            expiration,
        },
        amount,
    });

    let data = feels::instruction::Order { params };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*user_token_in, false),
        AccountMeta::new(*pool_token_in, false),
        // Additional accounts
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.data(),
    }
}

// ============================================================================
// Route Building Helpers
// ============================================================================

/// Build a route for token pair following hub-and-spoke constraint
pub fn build_swap_route(
    token_in: &Pubkey,
    token_out: &Pubkey,
    feelssol_mint: &Pubkey,
    pool_lookup: impl Fn(&Pubkey, &Pubkey) -> Option<Pubkey>,
) -> Result<(Vec<Pubkey>, Vec<bool>), crate::errors::SdkError> {
    use crate::errors::SdkError;
    
    // Case 1: Direct swap (one token is FeelsSOL)
    if *token_in == *feelssol_mint || *token_out == *feelssol_mint {
        if let Some(pool) = pool_lookup(token_in, token_out) {
            let zero_for_one = *token_in < *token_out;
            return Ok((vec![pool], vec![zero_for_one]));
        }
        return Err(SdkError::PoolNotFound);
    }
    
    // Case 2: Two-hop through FeelsSOL
    let pool1 = pool_lookup(token_in, feelssol_mint)
        .ok_or(SdkError::PoolNotFound)?;
    let pool2 = pool_lookup(feelssol_mint, token_out)
        .ok_or(SdkError::PoolNotFound)?;
    
    let zero_for_one_1 = *token_in < *feelssol_mint;
    let zero_for_one_2 = *feelssol_mint < *token_out;
    
    Ok((vec![pool1, pool2], vec![zero_for_one_1, zero_for_one_2]))
}