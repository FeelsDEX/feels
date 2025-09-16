//! Exact Output Swap Implementation
//!
//! Since the Feels protocol doesn't natively support exact output swaps,
//! this module provides a binary search implementation to find the right
//! input amount that yields the desired output.

use crate::{
    instructions::swap,
    program_id, SdkError, SdkResult,
};
use anchor_lang::prelude::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Parameters for exact output swap
#[derive(Clone, Debug)]
pub struct ExactOutputSwapParams {
    /// Desired output amount
    pub amount_out: u64,
    /// Maximum input amount willing to pay
    pub max_amount_in: u64,
    /// Minimum input amount to try (helps with binary search)
    pub min_amount_in: u64,
    /// Tolerance for output amount (in basis points)
    pub tolerance_bps: u16,
    /// Maximum iterations for binary search
    pub max_iterations: u8,
}

impl Default for ExactOutputSwapParams {
    fn default() -> Self {
        Self {
            amount_out: 0,
            max_amount_in: u64::MAX,
            min_amount_in: 0,
            tolerance_bps: 10, // 0.1% tolerance
            max_iterations: 20,
        }
    }
}

/// Result of exact output swap simulation
#[derive(Clone, Debug)]
pub struct SimulationResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_paid: u64,
    pub price_impact_bps: u16,
}

/// Exact output swap solver using binary search
pub struct ExactOutputSwapSolver<'a> {
    client: &'a dyn anchor_lang::solana_program::account_info::Account,
    market: Pubkey,
    token_in: Pubkey,
    token_out: Pubkey,
    trader: &'a Keypair,
}

impl<'a> ExactOutputSwapSolver<'a> {
    pub fn new(
        client: &'a dyn anchor_lang::solana_program::account_info::Account,
        market: Pubkey,
        token_in: Pubkey,
        token_out: Pubkey,
        trader: &'a Keypair,
    ) -> Self {
        Self {
            client,
            market,
            token_in,
            token_out,
            trader,
        }
    }

    /// Find the input amount that yields the desired output using binary search
    pub async fn solve(&self, params: ExactOutputSwapParams) -> SdkResult<SimulationResult> {
        let mut low = params.min_amount_in;
        let mut high = params.max_amount_in;
        let mut best_result = None;
        let mut iterations = 0;

        // Calculate tolerance in absolute terms
        let tolerance = (params.amount_out as u128 * params.tolerance_bps as u128 / 10_000) as u64;

        while low <= high && iterations < params.max_iterations {
            let mid = low + (high - low) / 2;
            
            // Simulate swap with current input amount
            match self.simulate_swap(mid).await {
                Ok(result) => {
                    let diff = if result.amount_out > params.amount_out {
                        result.amount_out - params.amount_out
                    } else {
                        params.amount_out - result.amount_out
                    };

                    // Check if we're within tolerance
                    if diff <= tolerance {
                        best_result = Some(result.clone());
                        
                        // Try to optimize further - get closer to exact amount
                        if result.amount_out > params.amount_out {
                            // We got too much output, try less input
                            high = mid - 1;
                        } else if result.amount_out < params.amount_out {
                            // We got too little output, try more input
                            low = mid + 1;
                        } else {
                            // Exact match!
                            return Ok(result);
                        }
                    } else if result.amount_out < params.amount_out {
                        // Need more output, increase input
                        low = mid + 1;
                    } else {
                        // Too much output, decrease input
                        high = mid - 1;
                    }
                },
                Err(_) => {
                    // Swap failed, likely too much input causing slippage
                    // Try with less input
                    high = mid - 1;
                }
            }
            
            iterations += 1;
        }

        best_result.ok_or_else(|| {
            SdkError::SwapSimulationFailed(
                "Could not find input amount that yields desired output within tolerance".to_string()
            )
        })
    }

    /// Simulate a swap without executing it
    async fn simulate_swap(&self, amount_in: u64) -> SdkResult<SimulationResult> {
        // In a real implementation, this would:
        // 1. Clone the current market state
        // 2. Simulate the swap through the tick arrays
        // 3. Calculate the exact output including fees
        // 4. Return the simulation result
        
        // For now, we'll return a placeholder that shows the structure
        // This would need to be implemented with actual swap math
        Err(SdkError::NotImplemented(
            "Swap simulation requires integration with on-chain program simulation".to_string()
        ))
    }
}

/// High-level function to build an exact output swap instruction
pub async fn build_exact_output_swap(
    client: &dyn anchor_lang::solana_program::account_info::Account,
    market: Pubkey,
    token_in: Pubkey,
    token_out: Pubkey,
    trader: &Keypair,
    amount_out: u64,
    max_amount_in: u64,
) -> SdkResult<(Instruction, u64)> {
    let solver = ExactOutputSwapSolver::new(client, market, token_in, token_out, trader);
    
    let params = ExactOutputSwapParams {
        amount_out,
        max_amount_in,
        min_amount_in: 0,
        ..Default::default()
    };
    
    let result = solver.solve(params).await?;
    
    // Build regular swap instruction with the calculated input amount
    // For the swap instruction, we need to determine which token is token_0 and token_1
    // This requires looking up the market to get the correct ordering
    // For now, we'll return an error indicating this needs proper implementation
    Err(SdkError::NotImplemented(
        "build_exact_output_swap requires market state lookup to determine token ordering".to_string()
    ))
}

/// Price-based estimation for initial binary search bounds
pub fn estimate_input_for_output(
    amount_out: u64,
    sqrt_price: u128,
    zero_for_one: bool,
    fee_bps: u16,
) -> (u64, u64) {
    // Convert sqrt price to actual price
    let q64 = 1u128 << 64;
    let price = (sqrt_price as f64 / q64 as f64).powi(2);
    
    // Estimate based on direction
    let base_estimate = if zero_for_one {
        (amount_out as f64 * price) as u64
    } else {
        (amount_out as f64 / price) as u64
    };
    
    // Account for fees and add buffer
    let fee_multiplier = 1.0 + (fee_bps as f64 / 10_000.0);
    let with_fees = (base_estimate as f64 * fee_multiplier) as u64;
    
    // Return range with 20% buffer on each side
    let min = (with_fees as f64 * 0.8) as u64;
    let max = (with_fees as f64 * 1.2) as u64;
    
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_input_for_output() {
        // Test with price = 1.0 (sqrt_price = 2^64)
        let sqrt_price = 1u128 << 64;
        let (min, max) = estimate_input_for_output(1_000_000, sqrt_price, true, 30);
        
        // With 1:1 price and 0.3% fee, we expect ~1,003,000 base estimate
        // With 20% buffer: min ~802,400, max ~1,203,600
        assert!(min < 1_000_000);
        assert!(max > 1_000_000);
        assert!(min < max);
    }

    #[test]
    fn test_exact_output_params_default() {
        let params = ExactOutputSwapParams::default();
        assert_eq!(params.tolerance_bps, 10);
        assert_eq!(params.max_iterations, 20);
    }
}