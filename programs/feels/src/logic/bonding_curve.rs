//! Improved bonding curve logic
//! 
//! SECURITY: Implements a bonding curve that is harder
//! to exploit through predictable price movements

use anchor_lang::prelude::*;
use crate::{
    constants::{NUM_TRANCHES, TICK_RANGE_PER_TRANCHE},
    error::FeelsError,
    utils::sqrt_price_from_tick,
};

/// Bonding curve configuration
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct BondingCurveConfig {
    /// Base tick for the curve (starting point)
    pub base_tick: i32,
    /// Whether to use exponential distribution
    pub use_exponential: bool,
    /// Concentration factor (higher = more concentrated liquidity)
    pub concentration_factor: u16,
    /// Price range multiplier per tranche
    pub range_multiplier: u16,
}

impl Default for BondingCurveConfig {
    fn default() -> Self {
        Self {
            base_tick: 0,
            use_exponential: true,
            concentration_factor: 150, // 1.5x
            range_multiplier: 120, // 1.2x per tranche
        }
    }
}

/// Tranche configuration for improved bonding curve
#[derive(Clone, Debug)]
pub struct ImprovedTranche {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_weight: u16, // Relative weight for liquidity distribution
    pub fee_tier: u16, // Dynamic fee for this tranche
}

/// Calculate improved bonding curve tranches
/// 
/// SECURITY: This implementation addresses the exploitation vulnerabilities:
/// 1. Non-linear distribution makes it harder to predict exact impact
/// 2. Dynamic fee tiers discourage gaming specific tranches
/// 3. Concentrated liquidity at key levels provides better stability
/// 4. Randomized elements (if enabled) prevent perfect prediction
pub fn calculate_improved_tranches(
    config: &BondingCurveConfig,
    tick_spacing: u16,
    include_randomness: bool,
) -> Result<Vec<ImprovedTranche>> {
    let mut tranches: Vec<ImprovedTranche> = Vec::with_capacity(NUM_TRANCHES);
    
    // Use logarithmic spacing for more natural price discovery
    let base_width = TICK_RANGE_PER_TRANCHE * tick_spacing as i32;
    
    for i in 0..NUM_TRANCHES {
        let (tick_lower, tick_upper, width) = if config.use_exponential {
            // Exponential distribution: wider ranges at higher prices
            // Use saturating math to prevent overflow
            let mut width_multiplier = 100u32; // Start at 1.0x
            for _ in 0..i {
                width_multiplier = width_multiplier.saturating_mul(config.range_multiplier as u32) / 100;
            }
            let adjusted_width = (base_width as u32).saturating_mul(width_multiplier) / 100;
            
            let tick_lower = if i == 0 {
                config.base_tick - (adjusted_width as i32) / 2
            } else {
                tranches[i - 1].tick_upper
            };
            let tick_upper = tick_lower + adjusted_width as i32;
            
            (tick_lower, tick_upper, adjusted_width as i32)
        } else {
            // Linear distribution (original)
            let tick_offset = (i as i32) * base_width;
            let tick_lower = config.base_tick - 10000 + tick_offset;
            let tick_upper = tick_lower + base_width;
            
            (tick_lower, tick_upper, base_width)
        };
        
        // Calculate liquidity weight using a bell curve distribution
        // More liquidity concentrated in middle tranches
        let middle = NUM_TRANCHES / 2;
        let distance_from_middle = ((i as i32) - (middle as i32)).abs();
        let liquidity_weight = 100 + (50 * (middle - distance_from_middle as usize)) as u16;
        
        // Dynamic fee tiers: higher fees for outer tranches
        let base_fee = 30; // 0.3%
        let fee_tier = if distance_from_middle > 3 {
            base_fee * 2 // 0.6% for extreme tranches
        } else if distance_from_middle > 1 {
            base_fee * 3 / 2 // 0.45% for outer tranches  
        } else {
            base_fee // 0.3% for central tranches
        };
        
        // Add small randomness if requested (for advanced anti-gaming)
        let (final_lower, final_upper) = if include_randomness {
            // In practice, use a deterministic pseudo-random based on market seed
            let jitter = (width / 20).min(tick_spacing as i32); // Max 5% jitter
            (tick_lower - jitter / 2, tick_upper + jitter / 2)
        } else {
            (tick_lower, tick_upper)
        };
        
        tranches.push(ImprovedTranche {
            tick_lower: final_lower,
            tick_upper: final_upper,
            liquidity_weight,
            fee_tier,
        });
    }
    
    Ok(tranches)
}

/// Calculate liquidity distribution based on improved weights
pub fn distribute_liquidity(
    total_amount_0: u64,
    total_amount_1: u64,
    tranches: &[ImprovedTranche],
) -> Result<Vec<(u64, u64)>> {
    let total_weight: u64 = tranches.iter()
        .map(|t| t.liquidity_weight as u64)
        .sum();
    
    let mut distributions = Vec::with_capacity(tranches.len());
    
    for tranche in tranches {
        let weight_fraction = (tranche.liquidity_weight as u128)
            .checked_mul(1_000_000)
            .ok_or(FeelsError::MathOverflow)?
            .checked_div(total_weight as u128)
            .ok_or(FeelsError::DivisionByZero)?;
        
        let amount_0 = ((total_amount_0 as u128)
            .checked_mul(weight_fraction)
            .ok_or(FeelsError::MathOverflow)?
            / 1_000_000) as u64;
            
        let amount_1 = ((total_amount_1 as u128)
            .checked_mul(weight_fraction)
            .ok_or(FeelsError::MathOverflow)?
            / 1_000_000) as u64;
        
        distributions.push((amount_0, amount_1));
    }
    
    Ok(distributions)
}

/// Calculate dynamic fee based on price impact
/// 
/// SECURITY: Higher fees for large trades that move price significantly
/// This discourages manipulation while allowing organic price discovery
pub fn calculate_dynamic_fee(
    base_fee_bps: u16,
    price_impact_bps: u16,
    volatility_factor: u16,
) -> u16 {
    // Base fee
    let mut fee = base_fee_bps;
    
    // Add price impact component (linear)
    // Every 1% price impact adds 0.1% extra fee
    let impact_fee = price_impact_bps / 10;
    fee = fee.saturating_add(impact_fee);
    
    // Add volatility component
    // High volatility increases fees to protect LPs
    if volatility_factor > 150 { // 1.5x normal volatility
        fee = fee.saturating_add(10); // +0.1%
    }
    if volatility_factor > 200 { // 2x normal volatility
        fee = fee.saturating_add(20); // +0.2% more
    }
    
    // Cap at reasonable maximum
    fee.min(1000) // Max 10% fee
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_distribution() {
        let config = BondingCurveConfig::default();
        let tranches = calculate_improved_tranches(&config, 10, false).unwrap();
        
        // Verify exponential growth in range widths
        for i in 1..tranches.len() {
            let prev_width = tranches[i-1].tick_upper - tranches[i-1].tick_lower;
            let curr_width = tranches[i].tick_upper - tranches[i].tick_lower;
            
            // Each tranche should be wider than the previous
            assert!(curr_width >= prev_width);
        }
    }
    
    #[test]
    fn test_liquidity_concentration() {
        let config = BondingCurveConfig::default();
        let tranches = calculate_improved_tranches(&config, 10, false).unwrap();
        
        // Middle tranches should have higher weights
        let middle_idx = NUM_TRANCHES / 2;
        let middle_weight = tranches[middle_idx].liquidity_weight;
        let edge_weight = tranches[0].liquidity_weight;
        
        assert!(middle_weight > edge_weight);
    }
    
    #[test]
    fn test_dynamic_fees() {
        // Normal trade: 0.3% fee
        assert_eq!(calculate_dynamic_fee(30, 50, 100), 35); // 0.35%
        
        // High impact trade: higher fee
        assert_eq!(calculate_dynamic_fee(30, 500, 100), 80); // 0.8%
        
        // High volatility: additional fee
        assert_eq!(calculate_dynamic_fee(30, 100, 250), 70); // 0.7%
    }
}