//! Swap Builder for ergonomic swap construction
//!
//! Provides utilities for building swap instructions with automatic
//! tick array discovery and account management.

use crate::{
    fee_estimator::{FeeEstimate, FeeEstimateParams, FeeEstimator},
    program_id, SdkError, SdkResult,
};
use anchor_lang::prelude::*;
use solana_sdk::instruction::{AccountMeta, Instruction};

/// Swap direction for concentrated liquidity
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwapDirection {
    /// Swapping token 0 for token 1 (price decreases)
    ZeroForOne,
    /// Swapping token 1 for token 0 (price increases)
    OneForZero,
}

/// Parameters for building a swap
#[derive(Clone, Debug)]
pub struct SwapParams {
    /// Market address
    pub market: Pubkey,
    /// Oracle address (can be default if no oracle)
    pub oracle: Pubkey,
    /// Token vault 0 address
    pub vault_0: Pubkey,
    /// Token vault 1 address
    pub vault_1: Pubkey,
    /// Vault authority PDA
    pub vault_authority: Pubkey,
    /// Buffer PDA
    pub buffer: Pubkey,
    /// User's input token account
    pub user_token_in: Pubkey,
    /// User's output token account
    pub user_token_out: Pubkey,
    /// Protocol configuration PDA
    pub protocol_config: Pubkey,
    /// Protocol treasury token account (mandatory)
    pub protocol_treasury: Pubkey,
    /// Protocol token registry entry (optional)
    pub protocol_token: Option<Pubkey>,
    /// Creator token account (optional)
    pub creator_token_account: Option<Pubkey>,
    /// Amount to swap in
    pub amount_in: u64,
    /// Minimum amount out (slippage protection)
    pub minimum_amount_out: u64,
    /// Maximum ticks to cross (0 = no limit)
    pub max_ticks_crossed: u16,
    /// Maximum total fee in bps (0 = no limit)
    pub max_fee_bps: u16,
}

/// Builder for constructing swaps with automatic tick array management
pub struct SwapBuilder {
    params: SwapParams,
    tick_arrays: Vec<Pubkey>,
    current_tick: Option<i32>,
    tick_spacing: Option<u16>,
    liquidity: Option<u128>,
    base_fee_bps: Option<u16>,
}

impl SwapBuilder {
    /// Create a new swap builder
    pub fn new(params: SwapParams) -> Self {
        Self {
            params,
            tick_arrays: Vec::new(),
            current_tick: None,
            tick_spacing: None,
            liquidity: None,
            base_fee_bps: None,
        }
    }

    /// Set the current tick and tick spacing for array calculation
    pub fn with_tick_context(mut self, current_tick: i32, tick_spacing: u16) -> Self {
        self.current_tick = Some(current_tick);
        self.tick_spacing = Some(tick_spacing);
        self
    }
    
    /// Set market parameters for fee estimation
    pub fn with_market_params(mut self, liquidity: u128, base_fee_bps: u16) -> Self {
        self.liquidity = Some(liquidity);
        self.base_fee_bps = Some(base_fee_bps);
        self
    }
    
    /// Estimate fees for this swap
    pub fn estimate_fees(&self) -> SdkResult<FeeEstimate> {
        let base_fee = self.base_fee_bps.unwrap_or(30); // Default 0.3%
        let liquidity = self.liquidity.unwrap_or(1_000_000_000_000); // Default 1M
        let current_tick = self.current_tick.unwrap_or(0);
        let tick_spacing = self.tick_spacing.unwrap_or(1);
        
        let params = FeeEstimateParams {
            amount_in: self.params.amount_in,
            current_tick,
            liquidity,
            base_fee_bps: base_fee,
            tick_spacing,
        };
        
        FeeEstimator::estimate_fees(&params)
    }

    /// Manually add tick arrays (for advanced usage)
    pub fn with_tick_arrays(mut self, tick_arrays: Vec<Pubkey>) -> Self {
        self.tick_arrays = tick_arrays;
        self
    }

    /// Add a single tick array
    pub fn add_tick_array(mut self, tick_array: Pubkey) -> Self {
        self.tick_arrays.push(tick_array);
        self
    }

    /// Automatically derive tick arrays for a given tick range
    pub fn with_tick_range(
        mut self,
        tick_lower: i32,
        tick_upper: i32,
        tick_spacing: u16,
    ) -> SdkResult<Self> {
        let arrays = derive_tick_arrays_for_range(
            &self.params.market,
            tick_lower,
            tick_upper,
            tick_spacing,
        )?;
        self.tick_arrays.extend(arrays);
        Ok(self)
    }

    /// Automatically derive tick arrays for a swap based on direction and amount
    pub fn with_auto_arrays(
        mut self,
        direction: SwapDirection,
        estimated_ticks: u32,
    ) -> SdkResult<Self> {
        if let (Some(current_tick), Some(tick_spacing)) = (self.current_tick, self.tick_spacing) {
            let arrays = derive_tick_arrays_for_swap(
                &self.params.market,
                current_tick,
                tick_spacing,
                direction,
                estimated_ticks,
            )?;
            self.tick_arrays.extend(arrays);
        }
        Ok(self)
    }

    /// Build the swap instruction
    pub fn build(self, user: &Pubkey) -> SdkResult<Instruction> {
        // Create account metas manually
        let mut account_metas = vec![
            // user account - signer and writable
            AccountMeta::new(*user, true),
            // market account - writable
            AccountMeta::new(self.params.market, false),
            // vault_0 - writable
            AccountMeta::new(self.params.vault_0, false),
            // vault_1 - writable
            AccountMeta::new(self.params.vault_1, false),
            // market_authority - readonly (note: renamed from vault_authority)
            AccountMeta::new_readonly(self.params.vault_authority, false),
            // buffer - writable
            AccountMeta::new(self.params.buffer, false),
            // oracle - writable
            AccountMeta::new(self.params.oracle, false),
            // user_token_in - writable
            AccountMeta::new(self.params.user_token_in, false),
            // user_token_out - writable
            AccountMeta::new(self.params.user_token_out, false),
            // token_program - readonly
            AccountMeta::new_readonly(spl_token::ID, false),
            // clock sysvar - readonly
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            // protocol_config - readonly
            AccountMeta::new_readonly(self.params.protocol_config, false),
            // protocol_treasury - writable
            AccountMeta::new(self.params.protocol_treasury, false),
        ];

        // Add optional protocol token (readonly if present)
        if let Some(protocol_token) = self.params.protocol_token {
            account_metas.push(AccountMeta::new_readonly(protocol_token, false));
        } else {
            // Add None placeholder for optional account
            account_metas.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }

        // Add optional creator token account (writable if present)
        if let Some(creator_account) = self.params.creator_token_account {
            account_metas.push(AccountMeta::new(creator_account, false));
        } else {
            // Add None placeholder for optional account
            account_metas.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }

        // Add tick arrays as remaining accounts
        for tick_array in &self.tick_arrays {
            account_metas.push(solana_sdk::instruction::AccountMeta::new(
                *tick_array,
                false,
            ));
        }

        // Build instruction data manually
        // The instruction discriminator for "swap" can be calculated from the method name
        let mut data = Vec::with_capacity(8 + 64);

        // Add discriminator (8 bytes): sha256("global:swap")[0..8]
        let preimage = b"global:swap";
        let h = solana_sdk::hash::hashv(&[preimage]);
        let disc = &h.to_bytes()[..8];
        data.extend_from_slice(disc);

        // Serialize SwapParams
        let params = feels::instructions::SwapParams {
            amount_in: self.params.amount_in,
            minimum_amount_out: self.params.minimum_amount_out,
            max_ticks_crossed: self.params.max_ticks_crossed as u8,
            max_total_fee_bps: 0, // 0 = no cap
        };

        // Serialize params using Anchor's serialization
        use anchor_lang::AnchorSerialize;
        params
            .serialize(&mut data)
            .map_err(|e| SdkError::SerializationError(e.to_string()))?;

        Ok(Instruction {
            program_id: program_id(),
            accounts: account_metas,
            data,
        })
    }
}

/// Derive tick arrays needed to cover a specific tick range
pub fn derive_tick_arrays_for_range(
    market: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    tick_spacing: u16,
) -> SdkResult<Vec<Pubkey>> {
    const TICK_ARRAY_SIZE: i32 = 64;
    let tick_array_spacing = tick_spacing as i32 * TICK_ARRAY_SIZE;

    // Align to tick array boundaries (round down for negative, up for positive)
    let start_index_lower = if tick_lower >= 0 {
        (tick_lower / tick_array_spacing) * tick_array_spacing
    } else {
        ((tick_lower - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
    };

    let start_index_upper = if tick_upper >= 0 {
        (tick_upper / tick_array_spacing) * tick_array_spacing
    } else {
        ((tick_upper - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
    };

    let mut arrays = Vec::new();
    let mut current_start = start_index_lower;

    while current_start <= start_index_upper {
        let (tick_array_pda, _) = find_tick_array_address(market, current_start);
        arrays.push(tick_array_pda);
        current_start += tick_array_spacing;
    }

    Ok(arrays)
}

/// Derive tick arrays needed for a swap based on direction and estimated tick movement
pub fn derive_tick_arrays_for_swap(
    market: &Pubkey,
    current_tick: i32,
    tick_spacing: u16,
    direction: SwapDirection,
    estimated_ticks: u32,
) -> SdkResult<Vec<Pubkey>> {
    const TICK_ARRAY_SIZE: i32 = 64;
    const SAFETY_BUFFER: u32 = 10; // Extra arrays for safety

    let tick_array_spacing = tick_spacing as i32 * TICK_ARRAY_SIZE;
    let current_array_start = current_tick - (current_tick % tick_array_spacing);

    let mut arrays = Vec::new();

    // Always include current array
    let (current_array, _) = find_tick_array_address(market, current_array_start);
    arrays.push(current_array);

    // Calculate how many additional arrays we need
    let total_tick_movement = estimated_ticks + SAFETY_BUFFER;
    let arrays_needed = (total_tick_movement as i32 / TICK_ARRAY_SIZE) + 2; // +2 for safety

    match direction {
        SwapDirection::ZeroForOne => {
            // Price decreasing, tick decreasing
            for i in 1..=arrays_needed {
                let start_index = current_array_start - (i * tick_array_spacing);
                let (array, _) = find_tick_array_address(market, start_index);
                arrays.push(array);
            }
        }
        SwapDirection::OneForZero => {
            // Price increasing, tick increasing
            for i in 1..=arrays_needed {
                let start_index = current_array_start + (i * tick_array_spacing);
                let (array, _) = find_tick_array_address(market, start_index);
                arrays.push(array);
            }
        }
    }

    Ok(arrays)
}

/// Find tick array PDA for a given market and start tick index
pub fn find_tick_array_address(market: &Pubkey, start_tick_index: i32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"tick_array",
            market.as_ref(),
            &start_tick_index.to_le_bytes(),
        ],
        &program_id(),
    )
}

/// Helper to estimate ticks crossed for a given swap amount
pub fn estimate_ticks_for_swap(
    amount_in: u64,
    current_liquidity: u128,
    _sqrt_price: u128,
    _direction: SwapDirection,
) -> u32 {
    // Rough estimation - this could be made more sophisticated
    if current_liquidity == 0 {
        return 100; // Conservative fallback
    }

    let price_impact = (amount_in as u128 * 1_000_000) / current_liquidity;
    let estimated_ticks = (price_impact / 10_000) as u32; // Very rough approximation

    // Minimum safety buffer
    std::cmp::max(estimated_ticks, 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_tick_arrays_for_range() {
        let market = Pubkey::new_unique();
        let arrays = derive_tick_arrays_for_range(&market, -1000, 1000, 64).unwrap();

        // Should cover the range with appropriate arrays
        assert!(!arrays.is_empty());
        assert!(arrays.len() >= 2); // At minimum should have arrays for negative and positive range
    }

    #[test]
    fn test_swap_builder() {
        let params = SwapParams {
            market: Pubkey::new_unique(),
            oracle: Pubkey::default(),
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            vault_authority: Pubkey::new_unique(),
            buffer: Pubkey::new_unique(),
            user_token_in: Pubkey::new_unique(),
            user_token_out: Pubkey::new_unique(),
            protocol_config: Pubkey::new_unique(),
            protocol_treasury: Pubkey::new_unique(),
            protocol_token: None,
            creator_token_account: None,
            amount_in: 1_000_000,
            minimum_amount_out: 990_000,
            max_ticks_crossed: 0,
            max_fee_bps: 0,
        };

        let builder = SwapBuilder::new(params)
            .with_tick_context(-100, 64)
            .with_auto_arrays(SwapDirection::ZeroForOne, 50)
            .unwrap();

        let user = Pubkey::new_unique();
        let instruction = builder.build(&user).unwrap();

        assert_eq!(instruction.program_id, program_id());
        assert!(!instruction.accounts.is_empty());
    }
}
