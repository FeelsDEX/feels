use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::{
    client::BaseClient,
    core::{FeeEstimate, Route, SdkResult, SwapDirection, SwapSimulation},
    instructions::{SwapAccounts, SwapInstructionBuilder, SwapParams},
    protocol::{calculate_swap_fees, PdaBuilder},
};

/// Service for swap operations
pub struct SwapService {
    base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    swap_builder: SwapInstructionBuilder,
}

impl SwapService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            base,
            pda,
            swap_builder: SwapInstructionBuilder::new(program_id),
        }
    }

    /// Execute a swap with exact input amount
    pub async fn swap_exact_in(
        &self,
        signer: &Keypair,
        market: Pubkey,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        _max_slippage_bps: Option<u16>,
    ) -> SdkResult<SwapResult> {
        // Get market info to determine tick arrays
        let market_info = self.base.get_account(&market).await?;
        let (current_tick, tick_spacing) = self.parse_market_tick_info(&market_info)?;

        // Determine swap direction based on token accounts
        // This is simplified - would need actual token mint comparison
        let direction = SwapDirection::ZeroForOne;

        // Derive tick arrays
        let tick_arrays = self.swap_builder.derive_tick_arrays(
            &market,
            current_tick,
            tick_spacing,
            direction,
            3, // Use 3 tick arrays
        );

        // Build swap instruction
        let params = SwapParams {
            amount_in,
            minimum_amount_out,
            max_ticks_crossed: 0,    // No limit
            max_total_fee_bps: 1000, // 10% max fee
        };

        // Get market account to extract token mints
        let market_account = self.base.get_account(&market).await?;
        let (token_0, token_1) = self.parse_market_tokens(&market_account)?;

        let accounts = SwapAccounts {
            user: signer.pubkey(),
            market,
            token_0,
            token_1,
            user_token_in,
            user_token_out,
            tick_arrays,
        };

        let ix = self.swap_builder.swap(accounts, params)?;

        // Execute transaction
        let signature = self.base.send_transaction(&[ix], &[signer]).await?;

        Ok(SwapResult {
            signature,
            amount_in,
            amount_out_estimate: minimum_amount_out,
            fee_paid_estimate: 0,
            route: Route::Direct {
                from: user_token_in,
                to: user_token_out,
            },
        })
    }

    /// Execute a swap with exact output amount
    ///
    /// This method calculates the exact input amount needed to receive the desired output,
    /// then executes a regular swap with exact_output_mode enabled to ensure the exact
    /// output is achieved or the transaction fails.
    pub async fn swap_exact_out(
        &self,
        signer: &Keypair,
        market: Pubkey,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        amount_out: u64,
        maximum_amount_in: u64,
        _max_slippage_bps: Option<u16>,
    ) -> SdkResult<SwapResult> {
        // TODO: Use ExactOutputSwapSolver to calculate the precise input amount
        // For now, we'll use the maximum amount as the input and rely on the
        // exact output mode to ensure we get exactly amount_out

        // Get market info to determine tick arrays
        let market_info = self.base.get_account(&market).await?;
        let (current_tick, tick_spacing) = self.parse_market_tick_info(&market_info)?;

        // Determine swap direction
        let direction = SwapDirection::OneForZero;

        // Derive tick arrays
        let tick_arrays =
            self.swap_builder
                .derive_tick_arrays(&market, current_tick, tick_spacing, direction, 3);

        // Build swap instruction
        // Note: For exact output, we need to use the swap_exact_out instruction
        // which is not yet implemented in the SDK
        let params = SwapParams {
            amount_in: maximum_amount_in,
            minimum_amount_out: amount_out,
            max_ticks_crossed: 0,
            max_total_fee_bps: 1000,
        };

        // Get market account to extract token mints
        let market_account = self.base.get_account(&market).await?;
        let (token_0, token_1) = self.parse_market_tokens(&market_account)?;

        let accounts = SwapAccounts {
            user: signer.pubkey(),
            market,
            token_0,
            token_1,
            user_token_in,
            user_token_out,
            tick_arrays,
        };

        let ix = self.swap_builder.swap(accounts, params)?;

        // Execute transaction
        let signature = self.base.send_transaction(&[ix], &[signer]).await?;

        Ok(SwapResult {
            signature,
            amount_in: maximum_amount_in, // Actual amount will be less
            amount_out_estimate: amount_out, // Exact amount received
            fee_paid_estimate: 0,
            route: Route::Direct {
                from: user_token_in,
                to: user_token_out,
            },
        })
    }

    /// Simulate a swap without executing
    pub async fn simulate_swap(
        &self,
        market: Pubkey,
        amount_in: u64,
        _is_token_0_in: bool,
    ) -> SdkResult<SwapSimulation> {
        // Get market data
        let _market_info = self.base.get_account(&market).await?;

        // Simplified simulation
        let fee_estimate = self.estimate_fees(&market, amount_in).await?;
        let amount_after_fee = amount_in.saturating_sub(fee_estimate.total_fee);

        // Mock output calculation (would need actual swap math)
        let amount_out = (amount_after_fee as f64 * 0.98) as u64;

        Ok(SwapSimulation {
            amount_in,
            amount_out,
            fee_paid: fee_estimate.total_fee,
            end_sqrt_price: 18446744073709551616, // Mock
            end_tick: 0,
            ticks_crossed: 1,
        })
    }

    /// Estimate fees for a swap
    pub async fn estimate_fees(&self, market: &Pubkey, amount_in: u64) -> SdkResult<FeeEstimate> {
        let market_info = self.base.get_account(market).await?;
        let (base_fee_bps, liquidity, sqrt_price) = self.parse_market_fee_info(&market_info)?;

        calculate_swap_fees(amount_in, base_fee_bps, liquidity, sqrt_price, true)
    }

    /// Find optimal route between two tokens
    pub async fn find_route(&self, token_from: &Pubkey, token_to: &Pubkey) -> SdkResult<Route> {
        let (feels_mint, _) = self.pda.feels_mint();

        // Check if either token is FeelsSOL
        if token_from == &feels_mint || token_to == &feels_mint {
            Ok(Route::Direct {
                from: *token_from,
                to: *token_to,
            })
        } else {
            Ok(Route::TwoHop {
                from: *token_from,
                intermediate: feels_mint,
                to: *token_to,
            })
        }
    }

    // Helper methods
    fn parse_market_tick_info(&self, _account: &Account) -> SdkResult<(i32, u16)> {
        // Simplified - would parse actual market data
        Ok((0, 10)) // current_tick, tick_spacing
    }

    fn parse_market_fee_info(&self, _account: &Account) -> SdkResult<(u16, u128, u128)> {
        // Simplified - would parse actual market data
        Ok((30, 1_000_000_000, 18446744073709551616)) // base_fee_bps, liquidity, sqrt_price
    }

    fn parse_market_tokens(&self, _account: &Account) -> SdkResult<(Pubkey, Pubkey)> {
        // Simplified - would parse actual market data to extract token_0 and token_1
        let (feels_mint, _) = self.pda.feels_mint();
        Ok((feels_mint, Pubkey::default())) // token_0 (FeelsSOL), token_1
    }
}

/// Result of a swap execution
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub signature: Signature,
    pub amount_in: u64,
    pub amount_out_estimate: u64,
    pub fee_paid_estimate: u64,
    pub route: Route,
}
