/// Swap instruction builders for hub-constrained routing
/// All swaps must go through FeelsSOL as the hub token
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use crate::router::{HubRouter, PoolInfo};

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

/// Build hub-constrained swap instruction
/// This enforces that all swaps go through FeelsSOL hub
pub fn hub_swap(
    program_id: &Pubkey,
    token_in: &Pubkey,
    token_out: &Pubkey,
    feelssol_mint: &Pubkey,
    user: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    router: &HubRouter,
) -> Result<Vec<Instruction>, crate::errors::SdkError> {
    // Find route through hub
    let route = router.find_route(token_in, token_out)?;
    
    // Validate route
    router.validate_route(&route)?;
    
    let mut instructions = Vec::new();
    
    if route.hops == 1 {
        // Direct swap (one token must be FeelsSOL)
        let pool = &route.pools[0];
        
        // Determine token accounts based on pool
        let (user_token_0, user_token_1, pool_token_0, pool_token_1) = 
            if pool.token_a == *token_in {
                (token_in, token_out, &pool.token_a, &pool.token_b)
            } else {
                (token_out, token_in, &pool.token_b, &pool.token_a)
            };
            
        instructions.push(swap(
            program_id,
            &pool.address,
            user,
            user_token_0,
            user_token_1,
            pool_token_0,
            pool_token_1,
            amount_in,
            min_amount_out,
            pool.token_a == *token_in,
            None,
        ));
    } else {
        // Two-hop swap through FeelsSOL
        let pool1 = &route.pools[0];
        let pool2 = &route.pools[1];
        
        // First hop: token_in -> FeelsSOL
        instructions.push(swap(
            program_id,
            &pool1.address,
            user,
            token_in,
            feelssol_mint,
            &pool1.token_a,
            &pool1.token_b,
            amount_in,
            0, // No slippage on intermediate
            pool1.token_a == *token_in,
            None,
        ));
        
        // Second hop: FeelsSOL -> token_out
        instructions.push(swap(
            program_id,
            &pool2.address,
            user,
            feelssol_mint,
            token_out,
            &pool2.token_a,
            &pool2.token_b,
            0, // Amount will be determined by first hop
            min_amount_out,
            pool2.token_a == *feelssol_mint,
            None,
        ));
    }
    
    Ok(instructions)
}
