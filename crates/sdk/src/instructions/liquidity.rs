/// Liquidity instruction builders using the unified API
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Build instruction to add liquidity
#[allow(clippy::too_many_arguments)]
pub fn add_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
    position: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    max_amount_0: u64,
    max_amount_1: u64,
    duration: Option<feels::Duration>,
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
        amount: liquidity as u64,
        config: feels::unified_order::OrderConfig::AddLiquidity {
            tick_lower,
            tick_upper,
            token_amounts: Some((max_amount_0, max_amount_1)),
        },
        advanced: duration.map(|d| feels::unified_order::AdvancedOrderParams {
            duration: d,
            leverage: 1_000_000, // 1x leverage
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

/// Build instruction to remove liquidity
#[allow(clippy::too_many_arguments)]
pub fn remove_liquidity(
    program_id: &Pubkey,
    pool: &Pubkey,
    user: &Pubkey,
    position: &Pubkey,
    user_token_0: &Pubkey,
    user_token_1: &Pubkey,
    pool_token_0: &Pubkey,
    pool_token_1: &Pubkey,
    liquidity_amount: u128,
    min_amount_0: u64,
    min_amount_1: u64,
) -> Instruction {
    let accounts = feels::accounts::OrderModify {
        pool: *pool,
        owner: *user,
        position: *position,
        user_token_0: Some(*user_token_0),
        user_token_1: Some(*user_token_1),
        pool_token_0: *pool_token_0,
        pool_token_1: *pool_token_1,
        token_program: spl_token_2022::ID,
    };

    let params = feels::UnifiedModifyParams {
        target: feels::unified_order::ModifyTarget::Position(*position),
        modification: feels::unified_order::OrderModification::Update {
            amount: Some(liquidity_amount as u64), // Amount to remove (negative handled internally)
            rate: None,
            leverage: None,
            duration: None,
        },
    };

    let data = feels::instruction::OrderModifyUnified { params };

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}