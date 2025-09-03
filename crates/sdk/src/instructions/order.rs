/// Unified order instruction builders for the Feels Protocol
/// All trading operations go through the unified order system
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

// Import types from the program
use feels::{OrderParams, OrderType, CreateOrderParams, PositionType};
use feels::state::{Duration, RiskProfile};

// ============================================================================
// Swap Instructions
// ============================================================================

/// Build instruction for a token swap
pub fn swap(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    market_token_0: &Pubkey,
    market_token_1: &Pubkey,
    route: Vec<Pubkey>,
    amount_in: u64,
    min_amount_out: u64,
    zero_for_one: Vec<bool>,
) -> Instruction {
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        market_token_0: *market_token_0,
        market_token_1: *market_token_1,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::Swap {
            route,
            min_amount_out,
            zero_for_one,
        },
        amount: amount_in,
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

// ============================================================================
// Position Instructions
// ============================================================================

/// Build instruction to enter a position from FeelsSOL
pub fn enter_position(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    user_feelssol: &Pubkey,
    position_mint: &Pubkey,
    position_type: PositionType,
    amount_in: u64,
    min_position_tokens: u64,
) -> Instruction {
    let market_feelssol = derive_market_vault(market_field, &feels::FEELSSOL_MINT);
    let user_position = get_associated_token_address(user, position_mint);
    
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: *user_feelssol,
        user_token_1: user_position,
        market_token_0: market_feelssol,
        market_token_1: *position_mint,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::EnterPosition {
            position_type,
            min_position_tokens,
        },
        amount: amount_in,
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to exit a position to FeelsSOL
pub fn exit_position(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    position_mint: &Pubkey,
    amount_in: u64,
    min_feelssol_out: u64,
) -> Instruction {
    let market_feelssol = derive_market_vault(market_field, &feels::FEELSSOL_MINT);
    let user_position = get_associated_token_address(user, position_mint);
    let user_feelssol = get_associated_token_address(user, &feels::FEELSSOL_MINT);
    
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: user_position,
        user_token_1: *user_feelssol,
        market_token_0: *position_mint,
        market_token_1: market_feelssol,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::ExitPosition {
            position_mint: *position_mint,
            min_feelssol_out,
        },
        amount: amount_in,
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to convert between positions
pub fn convert_position(
    program_id: &Pubkey,
    source_market: &Pubkey,
    dest_market: &Pubkey,
    user: &Pubkey,
    source_position: &Pubkey,
    target_position_type: PositionType,
    amount_in: u64,
    min_tokens_out: u64,
) -> Instruction {
    // For now, return a placeholder - this would need two separate order calls
    // through the FeelsSOL hub
    todo!("Position conversion requires two separate orders")
}

// ============================================================================
// Liquidity Instructions
// ============================================================================

/// Build instruction to add liquidity
pub fn add_liquidity(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    amount: u64,
) -> Instruction {
    let market_token_0 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_0));
    let market_token_1 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_1));
    
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        market_token_0,
        market_token_1,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::AddLiquidity {
            tick_lower,
            tick_upper,
            liquidity,
        },
        amount,
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Build instruction to remove liquidity
pub fn remove_liquidity(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    liquidity: u128,
    min_amounts: [u64; 2],
) -> Instruction {
    let market_token_0 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_0));
    let market_token_1 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_1));
    
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        market_token_0,
        market_token_1,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::RemoveLiquidity {
            liquidity,
            min_amounts,
        },
        amount: 0, // Not used for remove liquidity
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

// ============================================================================
// Limit Order Instructions
// ============================================================================

/// Build instruction to place a limit order
pub fn place_limit_order(
    program_id: &Pubkey,
    market_field: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    amount: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
    expiration: Option<i64>,
) -> Instruction {
    let market_token_0 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_0));
    let market_token_1 = derive_market_vault(market_field, &get_mint_from_token_account(user_token_1));
    
    let accounts = feels::accounts::Order {
        market_field: *market_field,
        market_manager: derive_market_manager(market_field),
        buffer_account: derive_buffer_account(market_field),
        user: *user,
        user_token_0: *user_token_0,
        user_token_1: *user_token_1,
        market_token_0,
        market_token_1,
        token_program: spl_token_2022::ID,
        system_program: solana_sdk::system_program::ID,
        tick_array_router: None,
    };

    let params = OrderParams::Create(CreateOrderParams {
        order_type: OrderType::LimitOrder {
            sqrt_price_limit,
            zero_for_one,
            expiration,
        },
        amount,
    });

    let data = feels::instruction::Order { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Derive market manager PDA
fn derive_market_manager(market_field: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"manager", market_field.as_ref()],
        &feels::ID,
    ).0
}

/// Derive buffer account PDA
fn derive_buffer_account(market_field: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"buffer", market_field.as_ref()],
        &feels::ID,
    ).0
}

/// Derive market vault PDA
fn derive_market_vault(market_field: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"vault", market_field.as_ref(), mint.as_ref()],
        &feels::ID,
    ).0
}

/// Get associated token address
fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(wallet, mint)
}

/// Get mint from token account (placeholder - would need to be passed in)
fn get_mint_from_token_account(_token_account: &Pubkey) -> Pubkey {
    // In practice, this would need to be passed in or looked up
    Pubkey::default()
}

// ============================================================================
// Convenience Builders
// ============================================================================

/// Enter system with JitoSOL -> FeelsSOL
pub fn enter_system(
    program_id: &Pubkey,
    hub_pool: &Pubkey,
    user: &Pubkey,
    amount_jitosol: u64,
    min_feelssol: u64,
) -> Instruction {
    // Entry uses a swap at the hub pool
    swap(
        program_id,
        hub_pool,
        user,
        &get_associated_token_address(user, &feels::JITOSOL_MINT),
        &get_associated_token_address(user, &feels::FEELSSOL_MINT),
        &derive_market_vault(hub_pool, &feels::JITOSOL_MINT),
        &derive_market_vault(hub_pool, &feels::FEELSSOL_MINT),
        vec![*hub_pool],
        amount_jitosol,
        min_feelssol,
        vec![true], // JitoSOL is token0 in hub pool
    )
}

/// Exit system with FeelsSOL -> JitoSOL
pub fn exit_system(
    program_id: &Pubkey,
    hub_pool: &Pubkey,
    user: &Pubkey,
    amount_feelssol: u64,
    min_jitosol: u64,
) -> Instruction {
    // Exit uses a swap at the hub pool
    swap(
        program_id,
        hub_pool,
        user,
        &get_associated_token_address(user, &feels::FEELSSOL_MINT),
        &get_associated_token_address(user, &feels::JITOSOL_MINT),
        &derive_market_vault(hub_pool, &feels::FEELSSOL_MINT),
        &derive_market_vault(hub_pool, &feels::JITOSOL_MINT),
        vec![*hub_pool],
        amount_feelssol,
        min_jitosol,
        vec![false], // FeelsSOL to JitoSOL
    )
}