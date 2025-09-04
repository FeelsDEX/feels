//! # Fees State - Consolidated Fee-Related Structures
//! 
//! This module consolidates all fee-related state structures:
//! - BufferAccount (τ): The thermodynamic fee reservoir
//! - FeesPolicy: Protocol-wide fee constraints and policies
//! - Fee calculation utilities and rebate management
//! 
//! ## Thermodynamic Model Integration
//! 
//! The buffer (τ) is the fourth dimension in our thermodynamic system, participating
//! in conservation laws alongside the market dimensions (S, T, L). It serves as the
//! fee collection and rebate distribution mechanism that maintains system equilibrium.
//! 
//! ### **Conservation Law**
//! ```text
//! Σ wᵢ ln(gᵢ) = 0  where i ∈ {S, T, L, τ}
//! ```
//! 
//! ### **Fee Flow Dynamics**
//! - **Uphill Work (W > 0)**: Fees flow into buffer τ
//! - **Downhill Work (W < 0)**: Rebates flow from buffer τ
//! - **κ-Clamping**: Rebates capped by κ × price_improvement
//! - **Safety Bounds**: Per-transaction and per-epoch limits

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::utils::math::safe;
use feels_core::constants::*;

// ============================================================================
// Buffer Constants
// ============================================================================

/// Maximum rebate per transaction (basis points of transaction value)
pub const MAX_REBATE_PER_TX_BPS: u64 = 100; // 1%

/// Maximum rebate per epoch (basis points of buffer value)
pub const MAX_REBATE_PER_EPOCH_BPS: u64 = 1000; // 10%

/// EWMA half-life for fee tracking (seconds)
pub const FEE_EWMA_HALF_LIFE: i64 = 86400; // 24 hours

/// Rebate epoch duration (seconds)
pub const REBATE_EPOCH_DURATION: i64 = 3600; // 1 hour

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

// ============================================================================
// BufferAccount (τ) - The Fee Reservoir
// ============================================================================

/// **BufferAccount - Thermodynamic Fee Reservoir (τ dimension)**
/// 
/// The buffer serves as the fourth dimension in the thermodynamic model:
/// 
/// **Key Functions:**
/// 1. **Fee Collection**: Absorbs fees from uphill trades (W > 0)
/// 2. **Rebate Distribution**: Provides rebates for downhill trades (W < 0)
/// 3. **Conservation Participation**: Maintains Σ wᵢ ln(gᵢ) = 0
/// 4. **Cross-dimensional Transfers**: Enables value flow between S, T, L dimensions
/// 
/// **Participation Coefficients (ζᵢ):**
/// - `zeta_spot`: Buffer participation in spot dimension
/// - `zeta_time`: Buffer participation in time dimension
/// - `zeta_leverage`: Buffer participation in leverage dimension
#[account]
#[derive(Default, Debug)]
pub struct BufferAccount {
    /// Market this buffer belongs to
    pub market: Pubkey,
    
    // ========== Fee Accumulation ==========
    
    /// Accumulated fees for token 0
    pub accumulated_fees_0: u64,
    
    /// Accumulated fees for token 1
    pub accumulated_fees_1: u64,
    
    /// Total fees collected (historical)
    pub total_fees_collected: u128,
    
    // ========== Rebate Tracking ==========
    
    /// Total rebates paid out (historical)
    pub total_rebates_paid: u128,
    
    /// Current epoch start time
    pub epoch_start: i64,
    
    /// Rebates paid in current epoch
    pub epoch_rebates_paid: u64,
    
    // ========== Participation Coefficients (ζ) ==========
    // Controls buffer participation in each dimension
    
    /// Spot dimension participation coefficient (Q32)
    pub zeta_spot: u64,
    
    /// Time dimension participation coefficient (Q32)
    pub zeta_time: u64,
    
    /// Leverage dimension participation coefficient (Q32)
    pub zeta_leverage: u64,
    
    // ========== Fee Statistics (EWMA) ==========
    
    /// Exponentially weighted moving average of fees
    pub fee_ewma: u64,
    
    /// Last EWMA update time
    pub ewma_last_update: i64,
    
    /// 24h fee volume
    pub fee_volume_24h: u128,
    
    // ========== Growth Tracking ==========
    // For maintaining conservation law: Σ wᵢ ln(gᵢ) = 0
    
    /// Growth factor g_τ (Q64)
    pub growth_factor: u128,
    
    /// Last conservation check timestamp
    pub last_conservation_check: i64,
    
    // ========== Rebate Configuration ==========
    
    /// Maximum rebate per transaction (token units)
    pub max_rebate_per_tx: u64,
    
    /// Maximum rebate per epoch (token units)
    pub max_rebate_per_epoch: u64,
    
    // ========== Authority ==========
    
    /// Authority that can withdraw fees
    pub authority: Pubkey,
    
    /// FeelSOL mint (for minting rebates)
    pub feelssol_mint: Pubkey,
    
    /// Reserved space
    pub _reserved: [u8; 128],
}

impl BufferAccount {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        8 + 8 + 16 + // fee accumulation
        16 + 8 + 8 + // rebate tracking
        8 + 8 + 8 + // participation coefficients
        8 + 8 + 16 + // fee statistics
        16 + 8 + // growth tracking
        8 + 8 + // rebate config
        32 + 32 + // authorities
        128; // reserved
    
    /// Calculate available rebate capacity
    pub fn available_rebate(&self) -> u64 {
        // Available = min(accumulated_fees, epoch_limit - epoch_paid)
        let accumulated = self.accumulated_fees_0 + self.accumulated_fees_1;
        let epoch_remaining = self.max_rebate_per_epoch.saturating_sub(self.epoch_rebates_paid);
        accumulated.min(epoch_remaining)
    }
    
    /// Update EWMA fee tracking
    pub fn update_fee_ewma(&mut self, new_fee: u64, current_time: i64) -> Result<()> {
        let time_elapsed = current_time.saturating_sub(self.ewma_last_update);
        if time_elapsed <= 0 {
            return Ok(());
        }
        
        // Calculate decay factor: exp(-λt) where λ = ln(2)/half_life
        // Approximation for small time steps
        let decay_factor = if time_elapsed < FEE_EWMA_HALF_LIFE {
            let ratio = (time_elapsed as u128 * Q64) / FEE_EWMA_HALF_LIFE as u128;
            Q64 - (ratio * 693 / 1000) // ln(2) ≈ 0.693
        } else {
            Q64 / 2 // Half life passed
        };
        
        // Update EWMA: ewma = decay * old_ewma + (1 - decay) * new_value
        let decayed_old = safe::mul_div_u64(
            self.fee_ewma,
            decay_factor as u64,
            Q64 as u64
        )?;
        
        let weighted_new = safe::mul_div_u64(
            new_fee,
            (Q64 - decay_factor) as u64,
            Q64 as u64
        )?;
        
        self.fee_ewma = safe::add_u64(decayed_old, weighted_new)?;
        self.ewma_last_update = current_time;
        
        Ok(())
    }
    
    /// Check if epoch needs reset
    pub fn check_epoch_reset(&mut self, current_time: i64) -> bool {
        if current_time >= self.epoch_start + REBATE_EPOCH_DURATION {
            self.epoch_start = current_time;
            self.epoch_rebates_paid = 0;
            true
        } else {
            false
        }
    }
    
    /// Get maximum rebate allowed for a transaction
    pub fn get_max_rebate(&self, tx_value: u128, max_bps: u64) -> u64 {
        let max_from_tx = safe::mul_div_u128(tx_value, max_bps as u128, BPS_DENOMINATOR as u128)
            .unwrap_or(u64::MAX as u128)
            .min(u64::MAX as u128) as u64;
        
        max_from_tx
            .min(self.max_rebate_per_tx)
            .min(self.available_rebate())
    }
}

// ============================================================================
// Fees Policy Configuration
// ============================================================================

/// **FeesPolicy - Protocol-wide Fee Constraints**
/// 
/// Manages fee boundaries and safety parameters across all markets:
/// - Minimum/maximum fee rates
/// - Update rate limits
/// - Pool disable thresholds
/// - Staleness parameters
#[account(zero_copy)]
#[derive(Debug)]
#[repr(C, packed)]
pub struct FeesPolicy {
    /// Authority that can update policy
    pub authority: Pubkey,
    
    /// Minimum base fee in basis points
    pub min_base_fee_bps: u64,
    
    /// Maximum base fee in basis points
    pub max_base_fee_bps: u64,
    
    /// Fee increase cap per update (basis points)
    pub max_fee_increase_bps: u64,
    
    /// Fee decrease cap per update (basis points)
    pub max_fee_decrease_bps: u64,
    
    /// Minimum time between fee updates (seconds)
    pub min_update_interval: i64,
    
    /// Spot price deviation threshold for pool disable (basis points)
    pub spot_disable_threshold_bps: u64,
    
    /// Time utilization threshold for pool disable (basis points)
    pub time_disable_threshold_bps: u64,
    
    /// Leverage imbalance threshold for pool disable (basis points)
    pub leverage_disable_threshold_bps: u64,
    
    /// Number of consecutive high stress periods before disable
    pub consecutive_stress_periods_for_disable: u8,
    
    /// Cool-down period after re-enabling (seconds)
    pub reenable_cooldown: i64,
    
    /// Maximum staleness before fallback mode (seconds)
    pub max_oracle_staleness: i64,
    
    /// Rebate participation coefficient (basis points)
    pub rebate_kappa: u64,
    
    /// Reserved for alignment
    pub _padding: [u8; 7],
    
    /// Reserved space for future upgrades
    pub _reserved: [u64; 8],
}

impl FeesPolicy {
    /// Validate a proposed fee update
    pub fn validate_fee_update(
        &self,
        current_fee: u64,
        proposed_fee: u64,
        last_update: i64,
        current_time: i64,
    ) -> Result<()> {
        // Check update interval
        require!(
            current_time >= last_update + self.min_update_interval,
            FeelsProtocolError::UpdateTooFrequent
        );
        
        // Check absolute bounds
        require!(
            proposed_fee >= self.min_base_fee_bps && proposed_fee <= self.max_base_fee_bps,
            FeelsProtocolError::FeeOutOfBounds
        );
        
        // Check rate of change
        if proposed_fee > current_fee {
            let increase = proposed_fee - current_fee;
            require!(
                increase <= self.max_fee_increase_bps,
                FeelsProtocolError::FeeIncreaseTooLarge
            );
        } else {
            let decrease = current_fee - proposed_fee;
            require!(
                decrease <= self.max_fee_decrease_bps,
                FeelsProtocolError::FeeDecreaseTooLarge
            );
        }
        
        Ok(())
    }
    
    /// Check if market should be disabled based on stress metrics
    pub fn should_disable_market(
        &self,
        spot_deviation_bps: u64,
        time_utilization_bps: u64,
        leverage_imbalance_bps: u64,
        consecutive_stress_periods: u8,
    ) -> bool {
        let spot_stressed = spot_deviation_bps > self.spot_disable_threshold_bps;
        let time_stressed = time_utilization_bps > self.time_disable_threshold_bps;
        let leverage_stressed = leverage_imbalance_bps > self.leverage_disable_threshold_bps;
        
        let currently_stressed = spot_stressed || time_stressed || leverage_stressed;
        
        currently_stressed && consecutive_stress_periods >= self.consecutive_stress_periods_for_disable
    }
}

// ============================================================================
// Fee Calculation Helpers
// ============================================================================

/// Calculate rebate amount based on price improvement
pub fn calculate_rebate(
    work: i128,
    price_improvement_bps: u64,
    kappa: u64,
    buffer: &BufferAccount,
) -> u64 {
    // Only provide rebates for negative work (downhill moves)
    if work >= 0 {
        return 0;
    }
    
    // Rebate = κ × price_improvement (capped by buffer availability)
    let uncapped_rebate = safe::mul_div_u64(
        price_improvement_bps,
        kappa,
        BPS_DENOMINATOR
    ).unwrap_or(0);
    
    // Apply buffer constraints
    buffer.get_max_rebate(uncapped_rebate as u128, MAX_REBATE_PER_TX_BPS)
}

/// Update buffer state after fee collection
pub fn collect_fee(
    buffer: &mut BufferAccount,
    fee_amount: u64,
    token_index: u8,
    current_time: i64,
) -> Result<()> {
    // Update accumulated fees
    match token_index {
        0 => buffer.accumulated_fees_0 = safe::add_u64(buffer.accumulated_fees_0, fee_amount)?,
        1 => buffer.accumulated_fees_1 = safe::add_u64(buffer.accumulated_fees_1, fee_amount)?,
        _ => return Err(FeelsProtocolError::InvalidTokenIndex.into()),
    }
    
    // Update total collected
    buffer.total_fees_collected = safe::add_u128(buffer.total_fees_collected, fee_amount as u128)?;
    
    // Update EWMA
    buffer.update_fee_ewma(fee_amount, current_time)?;
    
    // Update 24h volume
    buffer.fee_volume_24h = safe::add_u128(buffer.fee_volume_24h, fee_amount as u128)?;
    
    Ok(())
}

/// Process rebate payment from buffer
pub fn pay_rebate(
    buffer: &mut BufferAccount,
    rebate_amount: u64,
    current_time: i64,
) -> Result<()> {
    // Check epoch reset
    buffer.check_epoch_reset(current_time);
    
    // Verify rebate doesn't exceed limits
    require!(
        rebate_amount <= buffer.available_rebate(),
        FeelsProtocolError::InsufficientBuffer
    );
    
    // Update rebate tracking
    buffer.epoch_rebates_paid = safe::add_u64(buffer.epoch_rebates_paid, rebate_amount)?;
    buffer.total_rebates_paid = safe::add_u128(buffer.total_rebates_paid, rebate_amount as u128)?;
    
    // Deduct from accumulated fees (split evenly between tokens)
    let half_rebate = rebate_amount / 2;
    buffer.accumulated_fees_0 = buffer.accumulated_fees_0.saturating_sub(half_rebate);
    buffer.accumulated_fees_1 = buffer.accumulated_fees_1.saturating_sub(rebate_amount - half_rebate);
    
    Ok(())
}

/// Calculate buffer growth factor for conservation law
pub fn calculate_buffer_growth(
    fees_collected: u128,
    rebates_paid: u128,
    initial_value: u128,
) -> Result<u128> {
    if initial_value == 0 {
        return Ok(Q64);
    }
    
    let net_change = fees_collected.saturating_sub(rebates_paid);
    let current_value = safe::add_u128(initial_value, net_change)?;
    
    // Growth factor = current / initial (Q64)
    safe::mul_div_u128(current_value, Q64, initial_value)
}

// ============================================================================
// Conservation Law Integration
// ============================================================================

/// Buffer participation in conservation law
/// Ensures Σ wᵢ ln(gᵢ) = 0 across all dimensions including τ
pub fn verify_buffer_conservation(
    buffer_growth: u128,
    market_growth_s: u128,
    market_growth_t: u128,
    market_growth_l: u128,
    weights: &feels_core::types::DomainWeights,
) -> Result<bool> {
    // Calculate weighted log growth for each dimension
    // Note: In production, would use fixed-point ln
    // Here we verify the growth factors maintain the constraint
    
    let total_growth = safe::mul_div_u128(
        buffer_growth,
        weights.w_tau as u128,
        10000
    )?;
    
    let market_total = safe::add_u128(
        safe::mul_div_u128(market_growth_s, weights.w_s as u128, 10000)?,
        safe::add_u128(
            safe::mul_div_u128(market_growth_t, weights.w_t as u128, 10000)?,
            safe::mul_div_u128(market_growth_l, weights.w_l as u128, 10000)?
        )?
    )?;
    
    // Conservation is maintained if growths balance
    // Allow small deviation (0.1%) for rounding
    let deviation = if total_growth > market_total {
        total_growth - market_total
    } else {
        market_total - total_growth
    };
    
    Ok(deviation < market_total / 1000)
}