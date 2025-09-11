//! Buffer (τ) state for MVP
//! 
//! Protocol buffer that collects fees and manages floor LP

use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;

/// Protocol buffer (τ) account
#[account]
pub struct Buffer {
    /// Associated market
    pub market: Pubkey,
    
    /// Authority that can manage buffer
    pub authority: Pubkey,
    
    /// FeelsSOL mint (for reference)
    pub feelssol_mint: Pubkey,
    
    /// Token balances (u128 to prevent overflow in high-volume scenarios)
    pub fees_token_0: u128, // Total fees collected in token 0
    pub fees_token_1: u128, // Total fees collected in token 1
    
    /// τ partition counters (virtual partitions, u128 for overflow safety)
    pub tau_spot: u128,     // τ_s - spot domain allocation
    pub tau_time: u128,     // τ_t - time domain allocation (0 for MVP)
    pub tau_leverage: u128, // τ_l - leverage domain allocation (0 for MVP)
    
    /// Floor LP configuration
    /// DEPRECATED: This field is no longer used. POMM width is now derived from market tick spacing.
    /// Kept for backwards compatibility only.
    pub floor_tick_spacing: i32,      // How wide to place floor LP (e.g., 1000 = ±10%)
    pub floor_placement_threshold: u64, // Minimum value to trigger floor placement
    pub last_floor_placement: i64,    // Timestamp of last floor placement
    
    /// Epoch tracking for buffer
    pub last_rebase: i64,             // Last rebase timestamp
    pub total_distributed: u128,        // Total ever distributed from buffer (u128 for cumulative safety)
    
    /// Canonical bump for buffer authority PDA
    /// SECURITY: Storing the bump prevents ambiguity and improves performance
    pub buffer_authority_bump: u8,
    
    /// Reserved space for future expansion
    pub _reserved: [u8; 8],
}

impl Buffer {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        32 + // authority
        32 + // feelssol_mint
        16 + // fees_token_0 (u128)
        16 + // fees_token_1 (u128)
        16 + // tau_spot (u128)
        16 + // tau_time (u128)
        16 + // tau_leverage (u128)
        4 + // floor_tick_spacing
        8 + // floor_placement_threshold
        8 + // last_floor_placement
        8 + // last_rebase
        16 + // total_distributed (u128)
        1 + // buffer_authority_bump
        8; // _reserved
    
    /// Get total τ across all partitions
    pub fn get_total_tau(&self) -> u128 {
        self.tau_spot.saturating_add(self.tau_time).saturating_add(self.tau_leverage)
    }
    
    /// Check if floor placement is due
    /// SECURITY: Uses u128 for calculations to prevent overflow
    pub fn floor_placement_due(&self, token_0_value: u64, token_1_value: u64) -> bool {
        // Convert to u128 to prevent overflow in addition
        let total_value: u128 = (token_0_value as u128)
            .saturating_add(token_1_value as u128);
        total_value >= (self.floor_placement_threshold as u128)
    }
    
    /// Collect fee to appropriate partition (MVP only uses spot)
    /// SECURITY: This function is transactional - it only modifies state if all operations succeed
    pub fn collect_fee(&mut self, amount: u64, token_index: usize, domain: FeeDomain) -> Result<()> {
        let amount_u128 = amount as u128;
        
        // First, calculate all new values without modifying state
        let new_tau = match domain {
            FeeDomain::Spot => {
                self.tau_spot.checked_add(amount_u128)
                    .ok_or(crate::error::FeelsError::MathOverflow)?
            }
            FeeDomain::Time => {
                // Not used in MVP
                self.tau_time.checked_add(amount_u128)
                    .ok_or(crate::error::FeelsError::MathOverflow)?
            }
            FeeDomain::Leverage => {
                // Not used in MVP
                self.tau_leverage.checked_add(amount_u128)
                    .ok_or(crate::error::FeelsError::MathOverflow)?
            }
        };
        
        // Calculate new fee totals
        let new_fees = if token_index == 0 {
            self.fees_token_0.checked_add(amount_u128)
                .ok_or(crate::error::FeelsError::MathOverflow)?
        } else {
            self.fees_token_1.checked_add(amount_u128)
                .ok_or(crate::error::FeelsError::MathOverflow)?
        };
        
        // Only modify state after all checks pass
        match domain {
            FeeDomain::Spot => self.tau_spot = new_tau,
            FeeDomain::Time => self.tau_time = new_tau,
            FeeDomain::Leverage => self.tau_leverage = new_tau,
        }
        
        if token_index == 0 {
            self.fees_token_0 = new_fees;
        } else {
            self.fees_token_1 = new_fees;
        }
        
        Ok(())
    }
}

/// Domain for fee attribution
#[derive(Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum FeeDomain {
    Spot = 0,
    Time = 1,
    Leverage = 2,
}
