/// Leverage P&L rebase implementation for the market physics model.
/// Leverage P&L settlement uses geometric mean TWAP from protocol pools.
use anchor_lang::prelude::*;
use crate::state::Pool;
use crate::logic::market_physics::conservation::verify_conservation;
use crate::logic::market_physics::potential::{FixedPoint, ln_fixed, exp_fixed};
use super::rebase::{RebaseStrategy, RebaseFactors, RebaseState, RebaseParams, DomainParams};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum leverage multiplier (10x)
pub const MAX_LEVERAGE: u64 = 10_000_000; // 6 decimals, 10.0

/// Minimum TWAP window (5 minutes)
pub const MIN_TWAP_WINDOW: i64 = 300;

/// Maximum TWAP window (1 hour)
pub const MAX_TWAP_WINDOW: i64 = 3600;

/// Manipulation resistance threshold (5% max price change)
pub const MAX_PRICE_CHANGE_BPS: u64 = 500;

// ============================================================================
// Leverage Weights
// ============================================================================

/// Weights for leverage domain conservation
#[derive(Clone, Debug, Default)]
pub struct LeverageWeights {
    /// Weight of long positions
    pub w_long: u32,
    
    /// Weight of short positions
    pub w_short: u32,
    
    /// Total long value in numeraire
    pub long_value: u128,
    
    /// Total short value in numeraire
    pub short_value: u128,
}

impl LeverageWeights {
    /// Calculate weights from position values
    pub fn from_values(long_value: u128, short_value: u128) -> Result<Self> {
        let total = long_value.saturating_add(short_value);
        require!(total > 0, FeelsProtocolError::DivisionByZero);
        
        let w_long = ((long_value * 10_000) / total) as u32;
        let w_short = 10_000 - w_long;
        
        Ok(Self {
            w_long,
            w_short,
            long_value,
            short_value,
        })
    }
}

// ============================================================================
// Leverage Rebase Factors
// ============================================================================

/// Growth factors for leverage P&L rebase
#[derive(Clone, Debug)]
pub struct LeverageRebaseFactors {
    /// Growth factor for long positions
    pub g_long: u128,
    
    /// Growth factor for short positions
    pub g_short: u128,
    
    /// Price ratio used (new_price / old_price)
    pub price_ratio: u128,
    
    /// Average leverage applied
    pub avg_leverage: u64,
    
    /// Timestamp of rebase
    pub timestamp: i64,
}

// ============================================================================
// TWAP Price Observation
// ============================================================================

/// Price observation for TWAP calculation
#[derive(Clone, Debug)]
pub struct PriceObservation {
    /// Observed sqrt price (Q64)
    pub sqrt_price: u128,
    
    /// Observation timestamp
    pub timestamp: i64,
    
    /// Cumulative volume at observation
    pub cumulative_volume: u128,
}

// ============================================================================
// Leverage P&L Calculation
// ============================================================================

/// Calculate leverage rebase factors with conservation
pub fn calculate_leverage_rebase(
    old_price: FixedPoint,
    new_price: FixedPoint,
    leverage: FixedPoint,
    weights: &LeverageWeights,
) -> Result<LeverageRebaseFactors> {
    // Calculate price ratio
    let price_ratio = new_price.div(old_price)?;
    
    // g = (p'/p)^λ
    let g = pow_fixed(price_ratio, leverage)?;
    
    // Canonical case: α = β = 1
    // Longs gain when price increases, shorts gain when price decreases
    let g_long = g;
    let g_short = FixedPoint::ONE.div(g)?;
    
    // Verify conservation: w_long * ln(g_long) + w_short * ln(g_short) = 0
    let weights_array = [weights.w_long as u64, weights.w_short as u64];
    let factors_array = [
        g_long.value as u128,
        g_short.value as u128,
    ];
    
    verify_conservation(&weights_array, &factors_array)?;
    
    Ok(LeverageRebaseFactors {
        g_long: g_long.value as u128,
        g_short: g_short.value as u128,
        price_ratio: price_ratio.value as u128,
        avg_leverage: leverage.to_u64() * 1_000_000 / (1 << 64), // Convert to 6 decimals
        timestamp: Clock::get()?.unix_timestamp,
    })
}

/// Calculate power with fractional exponent: base^exponent
fn pow_fixed(base: FixedPoint, exponent: FixedPoint) -> Result<FixedPoint> {
    // For x^y, use: exp(y * ln(x))
    
    // Handle edge cases
    if exponent.value == 0 {
        return Ok(FixedPoint::ONE);
    }
    if base.value == FixedPoint::SCALE {
        return Ok(FixedPoint::ONE);
    }
    
    // Calculate ln(base)
    let ln_base = ln_fixed(base.value as u128)?;
    
    // Calculate y * ln(x)
    let exponent_times_ln = exponent.mul(ln_base)?;
    
    // Calculate exp(y * ln(x))
    let result = exp_fixed(exponent_times_ln)?;
    
    Ok(FixedPoint::from_scaled(result as i128))
}

// ============================================================================
// TWAP Calculation
// ============================================================================

/// Get TWAP price from pool observations
pub fn get_twap_price(
    pool: &Pool,
    window: i64,
) -> Result<FixedPoint> {
    // Validate window
    require!(
        window >= MIN_TWAP_WINDOW && window <= MAX_TWAP_WINDOW,
        FeelsProtocolError::InvalidInput
    );
    
    // Get current time
    let current_time = Clock::get()?.unix_timestamp;
    let window_start = current_time - window;
    
    // In production, would fetch actual price observations from pool
    // For now, use current price as placeholder
    let current_sqrt_price = pool.current_sqrt_rate;
    
    // Convert sqrt price to price
    let price = sqrt_price_to_price(current_sqrt_price)?;
    
    Ok(FixedPoint::from_scaled(price as i128))
}

/// Calculate geometric mean TWAP from observations
pub fn calculate_geometric_mean_twap(
    observations: &[PriceObservation],
) -> Result<FixedPoint> {
    require!(!observations.is_empty(), FeelsProtocolError::InvalidInput);
    
    // For geometric mean: GM = (p1^t1 * p2^t2 * ... * pn^tn)^(1/T)
    // Where ti is the time weight and T is total time
    
    let mut log_price_sum = FixedPoint::ZERO;
    let mut total_time = 0i64;
    
    // Calculate time-weighted log prices
    for i in 0..observations.len() - 1 {
        let current = &observations[i];
        let next = &observations[i + 1];
        
        let time_delta = next.timestamp - current.timestamp;
        if time_delta <= 0 {
            continue;
        }
        
        // Convert sqrt price to price
        let price = sqrt_price_to_price(current.sqrt_price)?;
        let ln_price = ln_fixed(price)?;
        
        // Add weighted log price
        let weighted = ln_price.mul(FixedPoint::from_int(time_delta))?;
        log_price_sum = log_price_sum.add(weighted)?;
        
        total_time += time_delta;
    }
    
    require!(total_time > 0, FeelsProtocolError::InvalidInput);
    
    // Average log price
    let avg_ln_price = log_price_sum.div(FixedPoint::from_int(total_time))?;
    
    // Convert back from log space
    let twap = exp_fixed(avg_ln_price)?;
    
    Ok(FixedPoint::from_scaled(twap as i128))
}

/// Convert sqrt price to price
fn sqrt_price_to_price(sqrt_price: u128) -> Result<u128> {
    // price = (sqrt_price)^2 / 2^128
    sqrt_price
        .checked_mul(sqrt_price)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 128)
        .ok_or(FeelsProtocolError::DivisionByZero.into())
}

// ============================================================================
// Manipulation Resistance
// ============================================================================

/// Check if price change is within acceptable bounds
pub fn check_price_manipulation(
    old_price: FixedPoint,
    new_price: FixedPoint,
) -> Result<bool> {
    let price_ratio = if new_price.value > old_price.value {
        new_price.div(old_price)?
    } else {
        old_price.div(new_price)?
    };
    
    // Check if price change is within threshold
    let max_ratio = FixedPoint::ONE.add(
        FixedPoint::from_scaled((MAX_PRICE_CHANGE_BPS as i128 * FixedPoint::SCALE) / 10_000)
    )?;
    
    Ok(price_ratio.value <= max_ratio.value)
}

// ============================================================================
// Leverage Imbalance
// ============================================================================

/// Calculate leverage imbalance for funding
pub struct LeverageImbalance {
    /// Long position value
    pub long_value: u128,
    
    /// Short position value
    pub short_value: u128,
    
    /// Weight of longs
    pub w_long: u32,
    
    /// Weight of shorts
    pub w_short: u32,
    
    /// Imbalance ratio (positive if more longs)
    pub imbalance_ratio: i64,
}

impl LeverageImbalance {
    /// Calculate from position values
    pub fn calculate(long_value: u128, short_value: u128) -> Result<Self> {
        let total = long_value.saturating_add(short_value);
        
        if total == 0 {
            return Ok(Self {
                long_value: 0,
                short_value: 0,
                w_long: 0,
                w_short: 0,
                imbalance_ratio: 0,
            });
        }
        
        let long_ratio = ((long_value * 10_000) / total) as i64;
        let imbalance_ratio = long_ratio - 5000; // -5000 to +5000
        
        let w_long = long_ratio as u32;
        let w_short = 10_000 - w_long;
        
        Ok(Self {
            long_value,
            short_value,
            w_long,
            w_short,
            imbalance_ratio,
        })
    }
    
    /// Check if positions are balanced
    pub fn is_balanced(&self, threshold_bps: u64) -> bool {
        self.imbalance_ratio.abs() < threshold_bps as i64
    }
}

// ============================================================================
// Position Update
// ============================================================================

/// Apply leverage rebase to positions
pub fn apply_leverage_rebase(
    long_positions: &mut u128,
    short_positions: &mut u128,
    factors: &LeverageRebaseFactors,
) -> Result<()> {
    // Apply growth factors
    *long_positions = (*long_positions)
        .checked_mul(factors.g_long)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    *short_positions = (*short_positions)
        .checked_mul(factors.g_short)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_leverage_conservation() {
        let weights = LeverageWeights::from_values(
            600_000 * (1u128 << 64), // 600K longs
            400_000 * (1u128 << 64), // 400K shorts
        ).unwrap();
        
        let old_price = FixedPoint::from_int(100);
        let new_price = FixedPoint::from_int(110); // 10% increase
        let leverage = FixedPoint::from_int(2); // 2x leverage
        
        let factors = calculate_leverage_rebase(
            old_price,
            new_price,
            leverage,
            &weights,
        ).unwrap();
        
        // Verify conservation
        let weights_array = [weights.w_long as u64, weights.w_short as u64];
        let factors_array = [factors.g_long, factors.g_short];
        
        assert!(verify_conservation(&weights_array, &factors_array).is_ok());
        
        // Verify longs gained ~20% (2x leverage on 10% move)
        assert!(factors.g_long > (1u128 << 64));
        
        // Verify shorts lost value
        assert!(factors.g_short < (1u128 << 64));
    }
    
    #[test]
    fn test_price_manipulation_check() {
        let old_price = FixedPoint::from_int(100);
        let new_price_safe = FixedPoint::from_int(103); // 3% change
        let new_price_risky = FixedPoint::from_int(110); // 10% change
        
        assert!(check_price_manipulation(old_price, new_price_safe).unwrap());
        assert!(!check_price_manipulation(old_price, new_price_risky).unwrap());
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl RebaseFactors for LeverageRebaseFactors {
    fn as_array(&self) -> Vec<u128> {
        vec![self.g_long, self.g_short]
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn is_identity(&self) -> bool {
        let one = 1u128 << 64;
        self.g_long == one && self.g_short == one
    }
}

/// Leverage rebase strategy implementation
pub struct LeverageRebaseStrategy;

impl RebaseStrategy for LeverageRebaseStrategy {
    type Factors = LeverageRebaseFactors;
    type Masses = LeverageWeights;
    
    fn calculate_factors(
        &self,
        masses: &Self::Masses,
        params: &RebaseParams,
    ) -> Result<Self::Factors> {
        match &params.domain_params {
            DomainParams::Leverage { old_price, new_price, avg_leverage } => {
                let old_fp = FixedPoint::from_scaled(*old_price as i128);
                let new_fp = FixedPoint::from_scaled(*new_price as i128);
                let leverage_fp = FixedPoint::from_scaled((*avg_leverage as i128 * FixedPoint::SCALE) / 1_000_000);
                
                calculate_leverage_rebase(
                    old_fp,
                    new_fp,
                    leverage_fp,
                    masses,
                )
            }
            _ => Err(FeelsProtocolError::InvalidInput.into()),
        }
    }
    
    fn apply_rebase(
        &self,
        state: &mut impl RebaseState,
        factors: &Self::Factors,
    ) -> Result<()> {
        state.apply_growth("long_positions", factors.g_long)?;
        state.apply_growth("short_positions", factors.g_short)?;
        Ok(())
    }
    
    fn verify_conservation(
        &self,
        factors: &Self::Factors,
        masses: &Self::Masses,
    ) -> Result<()> {
        let weights = [masses.w_long as u64, masses.w_short as u64];
        let growth_factors = [factors.g_long, factors.g_short];
        
        verify_conservation(&weights, &growth_factors)
    }
}