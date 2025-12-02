//! Swap execution and trading helper

use super::super::*;
use crate::common::sdk_compat;
use feels::logic::SwapParams;
use feels::state::{Buffer, Market};
use solana_sdk::instruction::{AccountMeta, Instruction};

/// Helper for swap operations
pub struct SwapHelper {
    ctx: TestContext,
}

impl SwapHelper {
    pub fn new(ctx: TestContext) -> Self {
        Self { ctx }
    }

    /// Execute a simple swap
    pub async fn swap(
        &self,
        market: &Pubkey,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
        trader: &Keypair,
    ) -> TestResult<SwapResult> {
        // Get trader's token accounts
        let trader_token_in = self.ctx.create_ata(&trader.pubkey(), token_in).await?;
        let trader_token_out = self.ctx.create_ata(&trader.pubkey(), token_out).await?;

        // Get initial balances
        let initial_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let initial_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Get market state
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();

        // Determine swap direction
        let zero_for_one = token_in == &market_state.token_0;
        let current_tick = market_state.current_tick;
        let tick_spacing = market_state.tick_spacing as i32;

        // Calculate tick arrays needed for swap
        // We need tick arrays around the current tick
        let tick_array_size = 88; // TICK_ARRAY_SIZE constant

        // Calculate start indices for tick arrays
        let array_start =
            (current_tick / (tick_array_size * tick_spacing)) * tick_array_size * tick_spacing;

        let mut tick_arrays = Vec::new();

        // Add current tick array
        let (current_array, _) = utils::find_tick_array_address(market, array_start);
        tick_arrays.push(current_array);

        // Add next tick array in swap direction
        let next_start = if zero_for_one {
            array_start - (tick_array_size * tick_spacing)
        } else {
            array_start + (tick_array_size * tick_spacing)
        };
        let (next_array, _) = utils::find_tick_array_address(market, next_start);
        tick_arrays.push(next_array);

        // Add one more for safety
        let next_next_start = if zero_for_one {
            next_start - (tick_array_size * tick_spacing)
        } else {
            next_start + (tick_array_size * tick_spacing)
        };
        let (next_next_array, _) = utils::find_tick_array_address(market, next_next_start);
        tick_arrays.push(next_next_array);

        // Build swap instruction manually with correct accounts
        let (oracle, _) =
            Pubkey::find_program_address(&[b"oracle", market.as_ref()], &sdk_compat::program_id());
        let (vault_0, _) = sdk_compat::find_vault_address(market, &market_state.token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(market, &market_state.token_1);
        let (market_authority, _) = sdk_compat::find_vault_authority_address(market);

        let buffer_key = market_state.buffer;

        // Snapshot buffer tau_spot before swap to estimate impact fee paid
        let buf_before: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_before: u128 = buf_before.as_ref().map(|b| b.tau_spot).unwrap_or(0);

        // Create accounts list
        let mut accounts = vec![
            AccountMeta::new(trader.pubkey(), true),
            AccountMeta::new(*market, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new_readonly(market_authority, false),
            AccountMeta::new(buffer_key, false),
            AccountMeta::new(oracle, false),
            AccountMeta::new(trader_token_in, false),
            AccountMeta::new(trader_token_out, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ];

        // Add tick arrays as remaining accounts
        for tick_array in &tick_arrays {
            accounts.push(AccountMeta::new(*tick_array, false));
        }

        // Create instruction data
        let params = SwapParams {
            amount_in,
            minimum_amount_out: 0,
            max_ticks_crossed: 10,
            max_total_fee_bps: 0,
        };

        let data = {
            let discriminator = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]; // SWAP_DISCRIMINATOR
            let mut data = discriminator.to_vec();
            data.extend_from_slice(&params.try_to_vec().unwrap());
            data
        };

        let ix = Instruction {
            program_id: sdk_compat::program_id(),
            accounts,
            data,
        };

        // Execute swap
        self.ctx.process_instruction(ix, &[trader]).await?;

        // Get final balances
        let final_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let final_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Calculate actual amounts
        let amount_in = initial_balance_in - final_balance_in;
        let amount_out = final_balance_out - initial_balance_out;

        // Read buffer after swap
        let buf_after: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_after: u128 = buf_after.as_ref().map(|b| b.tau_spot).unwrap_or(tau_before);
        let fee_paid_est = tau_after.saturating_sub(tau_before) as u64;

        // Estimate price impact bps from pre-swap sqrt_price
        // price = (sqrt/2^64)^2; adjust depending on direction
        let q64 = 1u128 << 64;
        let sqrt = market_state.sqrt_price as f64 / q64 as f64;
        let price_01 = sqrt * sqrt; // token1 per token0
        let exec_price = amount_out as f64 / amount_in.max(1) as f64;
        let (spot, exec) = if zero_for_one {
            (price_01, exec_price)
        } else {
            (1.0 / price_01.max(1e-18), exec_price)
        };
        let price_impact_bps = if spot > 0.0 {
            ((spot - exec).abs() / spot) * 10_000.0
        } else {
            0.0
        };

        Ok(SwapResult {
            amount_in,
            amount_out,
            fee_amount: fee_paid_est,
            fee_paid: fee_paid_est,
            price_impact: price_impact_bps,
        })
    }

    /// Execute swap with exact output
    ///
    /// This uses the protocol's exact output mode where we provide the maximum
    /// input amount and the exact desired output. The transaction will fail if
    /// the exact output cannot be achieved.
    pub async fn swap_exact_out(
        &self,
        market: &Pubkey,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_out: u64,
        max_amount_in: u64,
        trader: &Keypair,
    ) -> TestResult<SwapResult> {
        // Get market state for validation
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();
        let zero_for_one = token_in == &market_state.token_0;

        // Get trader's token accounts
        let trader_token_in = self.ctx.create_ata(&trader.pubkey(), token_in).await?;
        let trader_token_out = self.ctx.create_ata(&trader.pubkey(), token_out).await?;

        // Get initial balances
        let initial_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let initial_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Ensure trader has enough input tokens
        if initial_balance_in < max_amount_in {
            return Err(format!(
                "Insufficient balance: have {}, need {}",
                initial_balance_in, max_amount_in
            )
            .into());
        }

        // Get tick arrays - determine direction based on token ordering
        let current_tick = market_state.current_tick;
        let tick_spacing = market_state.tick_spacing;
        let tick_arrays = self
            .ctx
            .derive_tick_arrays(market, current_tick, tick_spacing, zero_for_one)
            .await?;

        // Derive all needed PDAs
        let (vault_0, _) = self.ctx.derive_vault(market, &market_state.token_0, 0);
        let (vault_1, _) = self.ctx.derive_vault(market, &market_state.token_1, 1);
        let (market_authority, _) = self.ctx.derive_market_authority(market);
        let (buffer_key, _) = self.ctx.derive_buffer(market);
        let (oracle, _) = self.ctx.derive_oracle(market);

        // Read buffer state before swap
        let buf_before: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_before: u128 = buf_before.as_ref().map(|b| b.tau_spot).unwrap_or(0);

        // Build swap params with exact output mode enabled
        let params = SwapParams {
            amount_in: max_amount_in,
            minimum_amount_out: amount_out,
            max_ticks_crossed: 0,
            max_total_fee_bps: 1000, // 10% max fee
        };

        // Build accounts for instruction
        let mut accounts = vec![
            AccountMeta::new(trader.pubkey(), true),
            AccountMeta::new(*market, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new_readonly(market_authority, false),
            AccountMeta::new(buffer_key, false),
            AccountMeta::new(oracle, false),
            AccountMeta::new(trader_token_in, false),
            AccountMeta::new(trader_token_out, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
        ];

        // Add tick arrays
        for tick_array in &tick_arrays {
            accounts.push(AccountMeta::new(*tick_array, false));
        }

        // Build instruction data
        let data = {
            let discriminator = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]; // SWAP_DISCRIMINATOR
            let mut data = discriminator.to_vec();
            data.extend_from_slice(&params.try_to_vec().unwrap());
            data
        };

        let ix = Instruction {
            program_id: sdk_compat::program_id(),
            accounts,
            data,
        };

        // Execute swap
        self.ctx.process_instruction(ix, &[trader]).await?;

        // Get final balances
        let final_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let final_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Calculate actual amounts
        let amount_in = initial_balance_in - final_balance_in;
        let amount_out_actual = final_balance_out - initial_balance_out;

        // Verify we got exactly the requested output
        if amount_out_actual != amount_out {
            return Err(format!(
                "Exact output not achieved: got {}, expected {}",
                amount_out_actual, amount_out
            )
            .into());
        }

        // Read buffer after swap
        let buf_after: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_after: u128 = buf_after.as_ref().map(|b| b.tau_spot).unwrap_or(tau_before);
        let fee_paid_est = tau_after.saturating_sub(tau_before) as u64;

        // Calculate price impact
        let q64 = 1u128 << 64;
        let sqrt = market_state.sqrt_price as f64 / q64 as f64;
        let price_01 = sqrt * sqrt;
        let exec_price = amount_out as f64 / amount_in.max(1) as f64;
        let (spot, exec) = if zero_for_one {
            (price_01, exec_price)
        } else {
            (1.0 / price_01.max(1e-18), exec_price)
        };
        let price_impact_bps = if spot > 0.0 {
            ((spot - exec).abs() / spot) * 10_000.0
        } else {
            0.0
        };

        Ok(SwapResult {
            amount_in,
            amount_out: amount_out_actual,
            fee_amount: fee_paid_est,
            fee_paid: fee_paid_est,
            price_impact: price_impact_bps,
        })
    }

    /// Perform multiple swaps in sequence
    pub async fn multi_swap(&self, swaps: Vec<crate::common::sdk_compat::SwapParams>) -> TestResult<Vec<SwapResult>> {
        let mut results = Vec::new();

        for swap in swaps {
            let result = self
                .swap(
                    &swap.market,
                    &swap.token_in,
                    &swap.token_out,
                    swap.amount_in,
                    &swap.trader,
                )
                .await?;
            results.push(result);
        }

        Ok(results)
    }
}