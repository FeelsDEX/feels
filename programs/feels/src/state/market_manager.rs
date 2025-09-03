/// Market manager providing pool-like functionality for the physics model.
/// Bridges between tick-based AMM operations and gradient-based physics.
use anchor_lang::prelude::*;
use crate::state::{MarketField, BufferAccount};
use crate::state::reentrancy::ReentrancyStatus;

// ============================================================================
// Market Manager State
// ============================================================================

/// Extended market state for AMM operations
#[account(zero_copy)]
#[derive(Default)]
#[repr(C, packed)]
pub struct MarketManager {
    /// Market field this manager belongs to
    pub market: Pubkey,
    
    // ========== Price and Tick State ==========
    
    /// Current sqrt price (Q64.64)
    pub current_sqrt_rate: u128,
    
    /// Current tick
    pub current_tick: i32,
    
    /// Current liquidity
    pub liquidity: u128,
    
    // ========== Token Information ==========
    
    /// Token 0 mint
    pub token_0_mint: Pubkey,
    
    /// Token 1 mint
    pub token_1_mint: Pubkey,
    
    /// Token 0 decimals
    pub token_0_decimals: u8,
    
    /// Token 1 decimals
    pub token_1_decimals: u8,
    
    // ========== AMM Parameters ==========
    
    /// Tick spacing
    pub tick_spacing: i16,
    
    /// Fee rate (basis points)
    pub fee_rate: u16,
    
    /// Protocol fee percentage (of fees)
    pub protocol_fee_percentage: u16,
    
    // ========== Liquidity Tracking ==========
    
    /// Total liquidity locked in positions
    pub liquidity_locked: u128,
    
    /// Number of open positions
    pub position_count: u64,
    
    // ========== Oracle Integration ==========
    
    /// Oracle account (if configured)
    pub oracle: Pubkey,
    
    /// Oracle type
    pub oracle_type: u8,
    
    // ========== Admin ==========
    
    /// Market authority
    pub authority: Pubkey,
    
    /// Fee recipient
    pub fee_recipient: Pubkey,
    
    // ========== Status ==========
    
    /// Is market paused (0 = false, 1 = true)
    pub is_paused: u8,
    
    /// Reentrancy status
    pub reentrancy_status: u8,
    
    /// Last update timestamp
    pub last_update_timestamp: i64,
    
    /// Total volume for token 0
    pub total_volume_a: u128,
    
    /// Total volume for token 1
    pub total_volume_b: u128,
    
    // ========== Tick Array Bitmap ==========
    
    /// Bitmap tracking initialized tick arrays (16 u64s = 1024 bits)
    /// Each bit represents whether a tick array is initialized
    pub tick_array_bitmap: [u64; 16],
    
    /// Global fee growth for token 0 (Q64.64)
    pub fee_growth_global_0: u128,
    
    /// Global fee growth for token 1 (Q64.64)
    pub fee_growth_global_1: u128,
    
    /// Maximum leverage allowed (in basis points, e.g., 100000 = 10x)
    pub max_leverage_bps: u64,
    
    /// Current average leverage across all positions
    pub avg_leverage_bps: u64,
    
    /// Total leveraged notional value
    pub total_leveraged_notional: u128,
    
    /// Reserved for future use
    pub _reserved: [u8; 8],
}

impl MarketManager {
    /// Size of the MarketManager account
    pub const SIZE: usize = 32 + // market
        16 + // current_sqrt_rate
        4 + // current_tick
        16 + // liquidity
        32 + // token_0_mint
        32 + // token_1_mint
        1 + // token_0_decimals
        1 + // token_1_decimals
        2 + // tick_spacing
        2 + // fee_rate
        2 + // protocol_fee_percentage
        16 + // liquidity_locked
        8 + // position_count
        32 + // oracle
        1 + // oracle_type
        32 + // authority
        32 + // fee_recipient
        1 + // is_paused
        1 + // reentrancy_status
        8 + // last_update_timestamp
        16 + // total_volume_a
        16 + // total_volume_b
        (8 * 16) + // tick_array_bitmap
        16 + // fee_growth_global_0
        16 + // fee_growth_global_1
        8 + // max_leverage_bps
        8 + // avg_leverage_bps
        16 + // total_leveraged_notional
        8; // _reserved
    
    /// Initialize market manager
    pub fn initialize(
        &mut self,
        market: Pubkey,
        token_0: Pubkey,
        token_1: Pubkey,
        tick_spacing: i16,
        initial_sqrt_price: u128,
    ) -> Result<()> {
        self.market = market;
        self.token_0_mint = token_0;
        self.token_1_mint = token_1;
        self.tick_spacing = tick_spacing;
        self.current_sqrt_rate = initial_sqrt_price;
        self.current_tick = crate::utils::math::amm::TickMath::get_tick_at_sqrt_ratio(initial_sqrt_price)?;
        self.fee_rate = 30; // 0.3% default
        self.protocol_fee_percentage = 3000; // 30% of fees
        
        Ok(())
    }
    
    /// Update current price and tick
    pub fn update_price(&mut self, new_sqrt_price: u128) -> Result<()> {
        self.current_sqrt_rate = new_sqrt_price;
        self.current_tick = crate::utils::math::amm::TickMath::get_tick_at_sqrt_ratio(new_sqrt_price)?;
        self.last_update_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
    
    /// Check if oracle is configured
    pub fn has_oracle(&self) -> bool {
        self.oracle != Pubkey::default()
    }
    
    /// Get maximum leverage (returns None if leverage not configured)
    pub fn get_max_leverage(&self) -> Option<u64> {
        if self.max_leverage_bps > 0 {
            Some(self.max_leverage_bps)
        } else {
            None
        }
    }
    
    /// Set maximum leverage
    pub fn set_max_leverage(&mut self, max_leverage_bps: u64) -> Result<()> {
        require!(
            max_leverage_bps >= 10000, // Minimum 1x
            crate::error::FeelsError::InvalidParameter
        );
        require!(
            max_leverage_bps <= 1000000, // Maximum 100x
            crate::error::FeelsError::InvalidParameter
        );
        self.max_leverage_bps = max_leverage_bps;
        Ok(())
    }
    
    /// Update average leverage tracking
    pub fn update_leverage_tracking(
        &mut self,
        position_notional: u128,
        leverage_bps: u64,
        is_open: bool,
    ) -> Result<()> {
        if is_open {
            // Opening position - add to tracking
            let new_total_notional = self.total_leveraged_notional
                .saturating_add(position_notional);
            
            // Calculate new weighted average
            if self.position_count > 0 && self.avg_leverage_bps > 0 {
                let old_weight = self.total_leveraged_notional;
                let new_weight = position_notional;
                let weighted_avg = (self.avg_leverage_bps as u128 * old_weight + leverage_bps as u128 * new_weight)
                    / (old_weight + new_weight);
                self.avg_leverage_bps = weighted_avg as u64;
            } else {
                self.avg_leverage_bps = leverage_bps;
            }
            
            self.total_leveraged_notional = new_total_notional;
        } else {
            // Closing position - remove from tracking
            self.total_leveraged_notional = self.total_leveraged_notional
                .saturating_sub(position_notional);
            
            // If no more leveraged positions, reset average
            if self.total_leveraged_notional == 0 {
                self.avg_leverage_bps = 10000; // Default 1x
            }
        }
        
        Ok(())
    }
    
    /// Check if market is active
    pub fn is_active(&self) -> bool {
        self.is_paused == 0 && self.reentrancy_status == 0
    }
    
    /// Set reentrancy status
    pub fn set_reentrancy_status(&mut self, status: ReentrancyStatus) -> Result<()> {
        self.reentrancy_status = status as u8;
        Ok(())
    }
    
    /// Get reentrancy status
    pub fn get_reentrancy_status(&self) -> ReentrancyStatus {
        match self.reentrancy_status {
            1 => ReentrancyStatus::Locked,
            _ => ReentrancyStatus::Unlocked,
        }
    }
    
    /// Accumulate fee growth for both tokens
    pub fn accumulate_fee_growth(&mut self, fee_amount: u64, zero_for_one: bool) -> Result<()> {
        let fee_growth_delta = if self.liquidity > 0 {
            // Use safe math for critical fee calculation - fix operator precedence and overflow
            let shifted_fee = crate::utils::math::safe::safe_shl_u128(fee_amount as u128, 64)?;
            crate::utils::math::safe::div_u128(shifted_fee, self.liquidity)?
        } else {
            0
        };
        
        if zero_for_one {
            self.fee_growth_global_0 = self.fee_growth_global_0.wrapping_add(fee_growth_delta);
        } else {
            self.fee_growth_global_1 = self.fee_growth_global_1.wrapping_add(fee_growth_delta);
        }
        
        Ok(())
    }
}

// ============================================================================
// Reentrancy Status
// ============================================================================

// ReentrancyStatus is imported from state::reentrancy module

// ============================================================================
// Market View
// ============================================================================

/// Combined view of market state for order execution
pub struct MarketView<'info> {
    pub manager: &'info MarketManager,
    pub field: &'info MarketField,
    pub buffer: &'info BufferAccount,
}

impl<'info> MarketView<'info> {
    /// Create market view from components
    pub fn new(
        manager: &'info MarketManager,
        field: &'info MarketField,
        buffer: &'info BufferAccount,
    ) -> Self {
        Self {
            manager,
            field,
            buffer,
        }
    }
    
    /// Get current sqrt price
    pub fn current_sqrt_rate(&self) -> u128 {
        self.manager.current_sqrt_rate
    }
    
    /// Get current tick
    pub fn current_tick(&self) -> i32 {
        self.manager.current_tick
    }
    
    /// Get current liquidity
    pub fn liquidity(&self) -> u128 {
        self.manager.liquidity
    }
    
    /// Get tick spacing
    pub fn tick_spacing(&self) -> i16 {
        self.manager.tick_spacing
    }
    
    /// Check if oracle is configured
    pub fn has_oracle(&self) -> bool {
        self.manager.oracle != Pubkey::default()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert physics position to tick
pub fn position_to_tick(position: &crate::utils::types::Position3D) -> Result<i32> {
    use crate::utils::math::amm::TickMath;
    use crate::constant::Q64;
    
    // Position.S is already in Q64 format representing price
    // Need to convert price to sqrt_price for tick calculation
    // sqrt_price = sqrt(price) * Q64
    let price = position.S;
    
    // Use integer square root
    let sqrt_price_raw = integer_sqrt::IntegerSquareRoot::integer_sqrt(&price);
    // Scale back to Q64
    let sqrt_price = (sqrt_price_raw as u128 * Q64) / (1u128 << 32);
    
    let tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price)?;
    Ok(tick)
}

/// Convert tick to physics position
pub fn tick_to_position(
    tick: i32,
    duration: &crate::state::duration::Duration,
    leverage_bps: u64,
) -> Result<crate::utils::types::Position3D> {
    use crate::utils::math::amm::TickMath;
    use crate::constant::Q64;
    
    // Get sqrt price from tick
    let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick)?;
    
    // Convert sqrt_price to price: price = (sqrt_price)² / Q64
    let price = sqrt_price
        .checked_mul(sqrt_price)
        .ok_or(crate::error::FeelsError::MathOverflow)?
        .checked_div(Q64)
        .ok_or(crate::error::FeelsError::MathOverflow)?;
    
    // Get time factor from duration
    let time_factor = duration.to_time_factor()?;
    
    // Convert leverage from basis points to Q64
    // leverage_bps: 10000 = 1x leverage
    let leverage_q64 = (leverage_bps as u128 * Q64) / 10000;
    
    Ok(crate::utils::types::Position3D::new(
        price,
        time_factor,
        leverage_q64,
    ))
}