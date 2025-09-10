//! SDK utility functions for MVP

use anchor_lang::prelude::*;
use crate::types::{Route, SwapQuote};

/// Calculate a swap quote
pub fn get_swap_quote(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    base_fee_bps: u16,
    token_in: Pubkey,
    token_out: Pubkey,
    feelssol_mint: Pubkey,
) -> Result<SwapQuote> {
    // Determine route
    let route = if token_in == feelssol_mint || token_out == feelssol_mint {
        Route::Direct {
            from: token_in,
            to: token_out,
        }
    } else {
        Route::TwoHop {
            from: token_in,
            intermediate: feelssol_mint,
            to: token_out,
        }
    };
    
    // Calculate fee
    let fee_amount = (amount_in as u128)
        .checked_mul(base_fee_bps as u128)
        .unwrap()
        .checked_div(10_000)
        .unwrap() as u64;
    
    let amount_in_after_fee = amount_in - fee_amount;
    
    // Calculate output (constant product)
    let amount_out = (reserve_out as u128)
        .checked_mul(amount_in_after_fee as u128)
        .unwrap()
        .checked_div(reserve_in as u128 + amount_in_after_fee as u128)
        .unwrap() as u64;
    
    // Calculate price impact
    let spot_price = (reserve_out as f64) / (reserve_in as f64);
    let execution_price = (amount_out as f64) / (amount_in as f64);
    let price_impact = ((spot_price - execution_price).abs() / spot_price * 10_000.0) as u16;
    
    Ok(SwapQuote {
        amount_in,
        amount_out,
        fee_amount,
        fee_bps: base_fee_bps,
        price_impact_bps: price_impact,
        route,
    })
}

/// Validate token pair includes FeelsSOL
pub fn validate_includes_feelssol(
    token_0: &Pubkey,
    token_1: &Pubkey,
    feelssol_mint: &Pubkey,
) -> bool {
    token_0 == feelssol_mint || token_1 == feelssol_mint
}

/// Calculate required amounts for adding liquidity
pub fn calculate_add_liquidity_amounts(
    amount_0_desired: u64,
    amount_1_desired: u64,
    reserve_0: u64,
    reserve_1: u64,
) -> Result<(u64, u64)> {
    if reserve_0 == 0 && reserve_1 == 0 {
        // First liquidity
        return Ok((amount_0_desired, amount_1_desired));
    }
    
    // Calculate proportional amounts
    let amount_1_optimal = (amount_0_desired as u128)
        .checked_mul(reserve_1 as u128)
        .unwrap()
        .checked_div(reserve_0 as u128)
        .unwrap() as u64;
    
    if amount_1_optimal <= amount_1_desired {
        Ok((amount_0_desired, amount_1_optimal))
    } else {
        let amount_0_optimal = (amount_1_desired as u128)
            .checked_mul(reserve_0 as u128)
            .unwrap()
            .checked_div(reserve_1 as u128)
            .unwrap() as u64;
        
        Ok((amount_0_optimal, amount_1_desired))
    }
}

/// Calculate slippage for a trade
pub fn calculate_slippage_bps(
    expected_out: u64,
    actual_out: u64,
) -> u16 {
    if expected_out == 0 {
        return 0;
    }
    
    let diff = if expected_out > actual_out {
        expected_out - actual_out
    } else {
        actual_out - expected_out
    };
    
    ((diff as u128 * 10_000) / expected_out as u128) as u16
}

/// Sort tokens for consistent ordering
pub fn sort_tokens(token_0: Pubkey, token_1: Pubkey) -> (Pubkey, Pubkey) {
    if token_0 < token_1 {
        (token_0, token_1)
    } else {
        (token_1, token_0)
    }
}

/// Derive pool address (market PDA)
pub fn derive_pool(
    token_0: &Pubkey,
    token_1: &Pubkey,
    fee_rate: u16,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    // For Feels, markets are derived from sorted tokens
    let (sorted_0, sorted_1) = sort_tokens(*token_0, *token_1);
    Pubkey::find_program_address(
        &[
            b"market",
            sorted_0.as_ref(),
            sorted_1.as_ref(),
            &fee_rate.to_le_bytes(),
        ],
        program_id,
    )
}