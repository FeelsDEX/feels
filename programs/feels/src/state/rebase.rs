/// Virtual rebasing system with lazy evaluation for yield accrual and funding rates.
/// Implements the mathematical framework where all yield/funding is delivered via
/// multiplicative rebasing while preserving invariants.
use anchor_lang::prelude::*;
use crate::utils::{safe_mul_div_u128, safe_mul_div_u64};
use crate::error::FeelsProtocolError;

// ============================================================================
// Core Rebase State
// ============================================================================

/// Global rebase accumulator tracking all multiplicative factors
#[account(zero_copy)]
#[derive(Default)]
#[repr(C, packed)]
pub struct RebaseAccumulator {
    /// Current global rebase index for token 0 (scaled by 2^64)
    pub index_0: u128,
    
    /// Current global rebase index for token 1 (scaled by 2^64)
    pub index_1: u128,
    
    /// Timestamp of last rebase update
    pub last_update: i64,
    
    /// Padding to ensure proper alignment
    pub _padding1: [u8; 8],
    
    /// Cumulative funding rate index for longs (scaled by 2^64)
    pub funding_index_long: u128,
    
    /// Cumulative funding rate index for shorts (scaled by 2^64)
    pub funding_index_short: u128,
    
    /// Current supply rate for token 0 (basis points per year)
    pub supply_rate_a: u64,
    
    /// Current supply rate for token 1 (basis points per year)
    pub supply_rate_b: u64,
    
    /// Current funding rate (positive = longs pay shorts, basis points per year)
    pub funding_rate: i64,
    
    /// Pool weights for invariant preservation
    pub weight_a: u32,
    pub weight_b: u32,
    pub weight_long: u32,
    pub weight_short: u32,
    
    /// Reserved for future use
    pub _padding2: [u8; 32],
}

/// Position checkpoint for lazy evaluation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct RebaseCheckpoint {
    /// Rebase index for token 0 at position creation/update
    pub index_a_checkpoint: u128,
    
    /// Rebase index for token 1 at position creation/update
    pub index_b_checkpoint: u128,
    
    /// Funding index checkpoint for leveraged positions
    pub funding_index_checkpoint: u128,
    
    /// Timestamp of checkpoint
    pub checkpoint_timestamp: i64,
}

/// Weight rebase parameters for domain weight changes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WeightRebaseParams {
    /// Previous domain weights (basis points)
    pub old_weights: DomainWeights,
    
    /// New domain weights (basis points)
    pub new_weights: DomainWeights,
    
    /// Current domain values in numeraire
    pub domain_values: DomainValues,
}

// Use DomainWeights from market_state module
use super::market_state::DomainWeights;

/// Domain values in numeraire
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
#[allow(non_snake_case)]
pub struct DomainValues {
    pub S: u128,   // Spot value
    pub T: u128,   // Time value  
    pub L: u128,   // Leverage value
    pub tau: u128, // Buffer value
}

// ============================================================================
// Constants
// ============================================================================

/// Scale factor for rebase indices (2^64)
pub const REBASE_INDEX_SCALE: u128 = 1 << 64;

/// Seconds per year for rate calculations
pub const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;

// Use BPS_DENOMINATOR from constants module
use feels_core::constants::BPS_DENOMINATOR;

// ============================================================================
// Implementation
// ============================================================================

impl RebaseAccumulator {
    /// Initialize a new rebase accumulator
    pub fn initialize(&mut self, weight_a: u32, weight_b: u32, weight_long: u32, weight_short: u32) {
        self.index_0 = REBASE_INDEX_SCALE;
        self.index_1 = REBASE_INDEX_SCALE;
        self.funding_index_long = REBASE_INDEX_SCALE;
        self.funding_index_short = REBASE_INDEX_SCALE;
        self.last_update = Clock::get().unwrap().unix_timestamp;
        self.weight_a = weight_a;
        self.weight_b = weight_b;
        self.weight_long = weight_long;
        self.weight_short = weight_short;
    }
    
    /// Update rebase indices with keeper-provided exact exponential factors
    /// 
    /// The keeper MUST provide exact rebase factors g = e^(rate·Δt/yr) that satisfy conservation:
    /// - For lending: w_A * ln(g_A) + w_D * ln(g_D) + w_tau * ln(g_tau) = 0
    /// - For leverage: w_long * ln(g_long) + w_short * ln(g_short) = 0
    /// 
    /// # Arguments
    /// * `conservation_proof` - Optional proof that factors satisfy conservation law
    pub fn update_indices_with_factors(
        &mut self,
        current_timestamp: i64,
        growth_factor_0: Option<u128>,
        growth_factor_1: Option<u128>,
        growth_factor_long: Option<u128>,
        growth_factor_short: Option<u128>,
        conservation_proof: Option<&crate::logic::conservation_check::ConservationProof>,
    ) -> Result<()> {
        let time_elapsed = current_timestamp.saturating_sub(self.last_update);
        if time_elapsed <= 0 {
            return Ok(());
        }
        
        // Verify conservation if proof provided
        if let Some(proof) = conservation_proof {
            use crate::logic::conservation_check::verify_rebase_conservation;
            
            // Determine operation type based on which factors are provided
            let operation_type = if growth_factor_0.is_some() || growth_factor_1.is_some() {
                "lending"
            } else if growth_factor_long.is_some() || growth_factor_short.is_some() {
                "leverage"
            } else {
                "unknown"
            };
            
            verify_rebase_conservation(operation_type, proof)?;
        }
        
        // Update yield indices with keeper-provided exact exponential factors
        // These MUST be g = e^(rate·Δt/yr), not linear approximations
        if let Some(growth_a) = growth_factor_0 {
            self.index_0 = safe_mul_div_u128(self.index_0, growth_a, REBASE_INDEX_SCALE)?;
            msg!("Updated index_0 with exact factor: {}", growth_a);
        }
        
        if let Some(growth_b) = growth_factor_1 {
            self.index_1 = safe_mul_div_u128(self.index_1, growth_b, REBASE_INDEX_SCALE)?;
            msg!("Updated index_1 with exact factor: {}", growth_b);
        }
        
        // Update funding indices with keeper-provided exact exponential factors
        if let Some(growth_long) = growth_factor_long {
            self.funding_index_long = safe_mul_div_u128(
                self.funding_index_long, 
                growth_long, 
                REBASE_INDEX_SCALE
            )?;
            msg!("Updated funding_index_long with exact factor: {}", growth_long);
        }
        
        if let Some(growth_short) = growth_factor_short {
            self.funding_index_short = safe_mul_div_u128(
                self.funding_index_short, 
                growth_short, 
                REBASE_INDEX_SCALE
            )?;
            msg!("Updated funding_index_short with exact factor: {}", growth_short);
        }
        
        self.last_update = current_timestamp;
        Ok(())
    }
    
    /// DEPRECATED: Update indices with linear approximation (emergency use only)
    /// This violates conservation laws and should only be used when keeper is unavailable
    pub fn update_indices_linear_unsafe(&mut self, current_timestamp: i64) -> Result<()> {
        let time_elapsed = current_timestamp.saturating_sub(self.last_update);
        if time_elapsed <= 0 {
            return Ok(());
        }
        
        // Update yield indices for tokens A and B
        if self.supply_rate_a > 0 {
            let growth_a = self.calculate_growth_factor_linear_unsafe(self.supply_rate_a, time_elapsed)?;
            self.index_0 = safe_mul_div_u128(self.index_0, growth_a, REBASE_INDEX_SCALE)?;
        }
        
        if self.supply_rate_b > 0 {
            let growth_b = self.calculate_growth_factor_linear_unsafe(self.supply_rate_b, time_elapsed)?;
            self.index_1 = safe_mul_div_u128(self.index_1, growth_b, REBASE_INDEX_SCALE)?;
        }
        
        // Update funding indices for leveraged positions
        if self.funding_rate != 0 {
            let (growth_long, growth_short) = self.calculate_funding_growth(time_elapsed)?;
            self.funding_index_long = safe_mul_div_u128(
                self.funding_index_long, 
                growth_long, 
                REBASE_INDEX_SCALE
            )?;
            self.funding_index_short = safe_mul_div_u128(
                self.funding_index_short, 
                growth_short, 
                REBASE_INDEX_SCALE
            )?;
        }
        
        self.last_update = current_timestamp;
        Ok(())
    }
    
    /// Calculate growth factor for a given rate and time period
    /// NOTE: This returns a LINEAR approximation which violates conservation laws!
    /// The keeper MUST provide exact exponential factors g = e^(rate * time / year)
    /// This function is kept only for emergency fallback when keeper is unavailable
    fn calculate_growth_factor_linear_unsafe(&self, rate_bps: u64, time_elapsed: i64) -> Result<u128> {
        // WARNING: Linear approximation e^x ≈ 1 + x is INCORRECT for conservation
        // This violates the weighted log-sum conservation law
        let rate_fraction = safe_mul_div_u128(
            rate_bps as u128 * time_elapsed as u128,
            REBASE_INDEX_SCALE,
            (BPS_DENOMINATOR as u128) * (SECONDS_PER_YEAR as u128)
        )?;
        
        Ok(REBASE_INDEX_SCALE + rate_fraction)
    }
    
    /// Calculate funding growth factors maintaining invariant
    fn calculate_funding_growth(&self, time_elapsed: i64) -> Result<(u128, u128)> {
        let funding_fraction = safe_mul_div_u128(
            self.funding_rate.abs() as u128 * time_elapsed as u128,
            REBASE_INDEX_SCALE,
            (BPS_DENOMINATOR as u128) * (SECONDS_PER_YEAR as u128)
        )?;
        
        // Maintain invariant: g_long^w_long * g_short^w_short = 1
        // If funding_rate > 0: longs pay shorts
        // If funding_rate < 0: shorts pay longs
        
        let (growth_long, growth_short) = if self.funding_rate > 0 {
            // Longs pay: g_long < 1, g_short > 1
            let g_long = REBASE_INDEX_SCALE.saturating_sub(funding_fraction);
            // Calculate g_short to maintain invariant
            let g_short = self.calculate_invariant_preserving_factor_linear_unsafe(
                g_long,
                self.weight_long,
                self.weight_short
            )?;
            (g_long, g_short)
        } else {
            // Shorts pay: g_short < 1, g_long > 1
            let g_short = REBASE_INDEX_SCALE.saturating_sub(funding_fraction);
            let g_long = self.calculate_invariant_preserving_factor_linear_unsafe(
                g_short,
                self.weight_short,
                self.weight_long
            )?;
            (g_long, g_short)
        };
        
        Ok((growth_long, growth_short))
    }
    
    /// Calculate the factor needed to preserve invariant
    /// Given g1 and weights w1, w2, find g2 such that g1^w1 * g2^w2 = 1
    /// 
    /// NOTE: This uses a LINEAR APPROXIMATION which violates conservation!
    /// The keeper must provide exact factors that satisfy: w1*ln(g1) + w2*ln(g2) = 0
    /// This implies: g2 = exp(-w1/w2 * ln(g1)) = g1^(-w1/w2)
    fn calculate_invariant_preserving_factor_linear_unsafe(
        &self,
        g1: u128,
        w1: u32,
        w2: u32,
    ) -> Result<u128> {
        // WARNING: Linear approximation for small deviations
        // This violates the conservation law!
        
        let deviation = if g1 > REBASE_INDEX_SCALE {
            g1 - REBASE_INDEX_SCALE
        } else {
            REBASE_INDEX_SCALE - g1
        };
        
        let scaled_deviation = safe_mul_div_u128(
            deviation,
            w1 as u128,
            w2 as u128
        )?;
        
        if g1 > REBASE_INDEX_SCALE {
            Ok(REBASE_INDEX_SCALE.saturating_sub(scaled_deviation))
        } else {
            Ok(REBASE_INDEX_SCALE + scaled_deviation)
        }
    }
    
    /// Set supply rates for yield accrual
    pub fn set_supply_rates(&mut self, rate_0: u64, rate_1: u64) {
        self.supply_rate_a = rate_0;
        self.supply_rate_b = rate_1;
    }
    
    /// Set funding rate
    pub fn set_funding_rate(&mut self, funding_rate: i64) {
        self.funding_rate = funding_rate;
    }
}

// ============================================================================
// Position Rebasing
// ============================================================================

/// Apply lazy rebase evaluation to a position
pub fn apply_position_rebase(
    position_value_a: u64,
    position_value_b: u64,
    checkpoint: &RebaseCheckpoint,
    accumulator: &RebaseAccumulator,
    is_leveraged: bool,
    is_long: bool,
) -> Result<(u64, u64)> {
    // Calculate rebase multipliers
    let rebase_a = safe_mul_div_u128(
        position_value_a as u128,
        accumulator.index_0,
        checkpoint.index_a_checkpoint
    )? as u64;
    
    let rebase_b = safe_mul_div_u128(
        position_value_b as u128,
        accumulator.index_1,
        checkpoint.index_b_checkpoint
    )? as u64;
    
    // Apply funding if leveraged
    let (final_a, final_b) = if is_leveraged {
        let funding_index = if is_long {
            accumulator.funding_index_long
        } else {
            accumulator.funding_index_short
        };
        
        let funding_multiplier = safe_mul_div_u128(
            REBASE_INDEX_SCALE,
            funding_index,
            checkpoint.funding_index_checkpoint
        )?;
        
        let funded_a = safe_mul_div_u128(
            rebase_a as u128,
            funding_multiplier,
            REBASE_INDEX_SCALE
        )? as u64;
        
        let funded_b = safe_mul_div_u128(
            rebase_b as u128,
            funding_multiplier,
            REBASE_INDEX_SCALE
        )? as u64;
        
        (funded_a, funded_b)
    } else {
        (rebase_a, rebase_b)
    };
    
    Ok((final_a, final_b))
}

/// Create a new checkpoint at current indices
pub fn create_checkpoint(accumulator: &RebaseAccumulator, is_long: bool) -> RebaseCheckpoint {
    RebaseCheckpoint {
        index_a_checkpoint: accumulator.index_0,
        index_b_checkpoint: accumulator.index_1,
        funding_index_checkpoint: if is_long {
            accumulator.funding_index_long
        } else {
            accumulator.funding_index_short
        },
        checkpoint_timestamp: accumulator.last_update,
    }
}

// ============================================================================
// Rate Calculations
// ============================================================================

/// Calculate supply rate based on utilization
pub fn calculate_supply_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
    reserve_factor_bps: u64,
) -> u64 {
    // supply_rate = borrow_rate * utilization * (1 - reserve_factor)
    let borrow_rate = calculate_borrow_rate(utilization_bps, base_rate_bps);
    
    safe_mul_div_u64(
        safe_mul_div_u64(
            borrow_rate,
            utilization_bps,
            BPS_DENOMINATOR
        ).unwrap_or(0),
        BPS_DENOMINATOR - reserve_factor_bps,
        BPS_DENOMINATOR
    ).unwrap_or(0)
}

/// Calculate borrow rate based on utilization
pub fn calculate_borrow_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
) -> u64 {
    // Simple linear model: rate = base + utilization * slope
    // At 80% utilization, rate = 4x base rate
    let slope = base_rate_bps * 3 / 80; // 3x increase over 80%
    
    base_rate_bps + safe_mul_div_u64(
        utilization_bps,
        slope,
        100 // Convert from percentage
    ).unwrap_or(0)
}

/// Calculate funding rate based on long/short imbalance
pub fn calculate_funding_rate(
    long_value: u128,
    short_value: u128,
    max_funding_rate_bps: u64,
) -> i64 {
    let total = long_value.saturating_add(short_value);
    if total == 0 {
        return 0;
    }
    
    // Imbalance ratio: positive if more longs, negative if more shorts
    let long_ratio = safe_mul_div_u128(
        long_value,
        BPS_DENOMINATOR as u128,
        total
    ).unwrap_or(5000) as i64; // Default to 50%
    
    let imbalance = long_ratio - 5000; // -5000 to +5000
    
    // Linear funding rate based on imbalance
    // Max funding at 100% imbalance (all long or all short)
    (imbalance * max_funding_rate_bps as i64) / 5000
}

// ============================================================================
// Weight Rebase for Domain Weight Changes
// ============================================================================

/// Calculate rebase factors when domain weights change
/// 
/// When domain weights change (e.g., from governance), we need to rebase
/// the domain values to maintain the accounting invariant K_acct = constant.
/// 
/// The keeper must solve for factors h_S, h_T, h_L, h_tau such that:
/// 1. K_acct remains constant: S'^w's * T'^w't * L'^w'l * tau'^w'tau = K_acct
/// 2. Price continuity: The spot price S_a/S_b remains unchanged
/// 
/// Where S' = S * h_S, T' = T * h_T, etc.
/// 
/// This requires solving a system with Newton's method off-chain.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct WeightRebaseFactors {
    pub h_S: u128,    // Spot rebase factor
    pub h_T: u128,    // Time rebase factor
    pub h_L: u128,    // Leverage rebase factor
    pub h_tau: u128,  // Buffer rebase factor
}

impl WeightRebaseFactors {
    /// Validate that factors maintain the invariant
    /// NOTE: Actual validation requires ln calculations done off-chain
    pub fn validate(&self, params: &WeightRebaseParams) -> Result<()> {
        // Basic sanity checks
        require!(
            self.h_S > 0 && self.h_T > 0 && self.h_L > 0 && self.h_tau > 0,
            FeelsProtocolError::InvalidInput
        );
        
        // Ensure weights sum to 10000 (100%)
        let old_sum = params.old_weights.w_s + params.old_weights.w_t + 
                     params.old_weights.w_l + params.old_weights.w_tau;
        let new_sum = params.new_weights.w_s + params.new_weights.w_t +
                     params.new_weights.w_l + params.new_weights.w_tau;
                     
        require!(
            old_sum == 10000 && new_sum == 10000,
            FeelsProtocolError::InvalidInput
        );
        
        // NOTE: The actual invariant check K'_acct = K_acct requires
        // computing K = S^w_s * T^w_t * L^w_l * tau^w_tau
        // which involves logarithms and must be done off-chain
        
        Ok(())
    }
    
    /// Apply weight rebase atomically to all domain values
    /// This maintains K_acct constant and preserves price continuity
    pub fn apply_to_values(&self, values: &mut DomainValues) -> Result<()> {
        // Apply multiplicative rebase factors
        values.S = safe_mul_div_u128(values.S, self.h_S, REBASE_INDEX_SCALE)?;
        values.T = safe_mul_div_u128(values.T, self.h_T, REBASE_INDEX_SCALE)?;
        values.L = safe_mul_div_u128(values.L, self.h_L, REBASE_INDEX_SCALE)?;
        values.tau = safe_mul_div_u128(values.tau, self.h_tau, REBASE_INDEX_SCALE)?;
        
        msg!("Applied weight rebase factors:");
        msg!("  S: {} -> {}", values.S / self.h_S * REBASE_INDEX_SCALE, values.S);
        msg!("  T: {} -> {}", values.T / self.h_T * REBASE_INDEX_SCALE, values.T);
        msg!("  L: {} -> {}", values.L / self.h_L * REBASE_INDEX_SCALE, values.L);
        msg!("  tau: {} -> {}", values.tau / self.h_tau * REBASE_INDEX_SCALE, values.tau);
        
        Ok(())
    }
    
    /// Create weight rebase factors with conservation proof
    #[allow(non_snake_case)]
    pub fn new_with_proof(
        h_S: u128,
        h_T: u128,
        h_L: u128,
        h_tau: u128,
        conservation_proof: &crate::logic::conservation_check::ConservationProof,
    ) -> Result<Self> {
        use crate::logic::conservation_check::verify_rebase_conservation;
        
        // Verify conservation before creating factors
        verify_rebase_conservation("weight_rebase", conservation_proof)?;
        
        Ok(Self {
            h_S,
            h_T,
            h_L,
            h_tau,
        })
    }
}

/// Apply weight rebase to maintain invariant when weights change
/// 
/// When governance changes domain weights, we must rebase all positions
/// to maintain K_acct = constant. This requires solving for h_S, h_T, h_L, h_tau
/// such that:
/// 
/// S'^(w's) * T'^(w't) * L'^(w'l) * tau'^(w'tau) = S^(ws) * T^(wt) * L^(wl) * tau^(wtau)
/// 
/// Where S' = S * h_S, etc. and w' are the new weights.
/// 
/// The keeper solves this system off-chain using Newton's method and provides
/// the factors along with a conservation proof.
pub fn apply_weight_rebase(
    accumulator: &mut RebaseAccumulator,
    current_values: &mut DomainValues,
    new_weights: &DomainWeights,
    rebase_factors: &WeightRebaseFactors,
    conservation_proof: &crate::logic::conservation_check::ConservationProof,
) -> Result<()> {
    // Create params for validation
    let params = WeightRebaseParams {
        old_weights: DomainWeights {
            w_s: accumulator.weight_a,
            w_t: accumulator.weight_b,
            w_l: accumulator.weight_long,
            w_tau: 10000 - accumulator.weight_a - accumulator.weight_b - accumulator.weight_long,
        },
        new_weights: new_weights.clone(),
        domain_values: current_values.clone(),
    };
    
    // Validate factors maintain invariant
    rebase_factors.validate(&params)?;
    
    // Verify conservation
    use crate::logic::conservation_check::verify_rebase_conservation;
    verify_rebase_conservation("weight_rebase", conservation_proof)?;
    
    // Apply rebase factors atomically
    rebase_factors.apply_to_values(current_values)?;
    
    // Update weights in accumulator
    accumulator.weight_a = new_weights.w_s;
    accumulator.weight_b = new_weights.w_t;
    accumulator.weight_long = new_weights.w_l;
    accumulator.weight_short = new_weights.w_tau;
    
    msg!("Weight rebase completed successfully");
    msg!("  New weights: S={}, T={}, L={}, tau={}", 
        new_weights.w_s, new_weights.w_t, new_weights.w_l, new_weights.w_tau);
    
    Ok(())
}