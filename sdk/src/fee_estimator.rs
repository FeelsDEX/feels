//! Fee estimation module for Feels protocol
//!
//! Provides accurate fee estimation for swaps including base and impact fees

use crate::error::SdkResult;

/// Fee estimation parameters
#[derive(Clone, Debug)]
pub struct FeeEstimateParams {
    /// Input amount
    pub amount_in: u64,
    /// Current price tick
    pub current_tick: i32,
    /// Pool liquidity
    pub liquidity: u128,
    /// Pool base fee in bps
    pub base_fee_bps: u16,
    /// Tick spacing
    pub tick_spacing: u16,
}

/// Fee estimate result
#[derive(Clone, Debug)]
pub struct FeeEstimate {
    /// Base fee in bps
    pub base_fee_bps: u16,
    /// Estimated impact fee in bps
    pub impact_fee_bps: u16,
    /// Total fee in bps
    pub total_fee_bps: u16,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// Whether this is a degraded estimate
    pub is_degraded: bool,
}

/// Fee estimator for off-chain calculations
pub struct FeeEstimator;

impl FeeEstimator {
    /// Estimate fees for a swap
    pub fn estimate_fees(params: &FeeEstimateParams) -> SdkResult<FeeEstimate> {
        // Base fee is always known
        let base_fee_bps = params.base_fee_bps;
        
        // Estimate impact based on amount and liquidity
        let (impact_fee_bps, confidence) = Self::estimate_impact(
            params.amount_in,
            params.liquidity,
            params.tick_spacing,
        );
        
        // Apply impact floor (0.1%)
        const IMPACT_FLOOR_BPS: u16 = 10;
        let impact_fee_bps = impact_fee_bps.max(IMPACT_FLOOR_BPS);
        
        // Calculate total with bounds
        const MIN_TOTAL_FEE_BPS: u16 = 1; // 0.01%
        const MAX_TOTAL_FEE_BPS: u16 = 10000; // 100%
        
        let total_fee_bps = (base_fee_bps as u32 + impact_fee_bps as u32)
            .clamp(MIN_TOTAL_FEE_BPS as u32, MAX_TOTAL_FEE_BPS as u32) as u16;
        
        Ok(FeeEstimate {
            base_fee_bps,
            impact_fee_bps,
            total_fee_bps,
            confidence,
            is_degraded: confidence < 80,
        })
    }
    
    /// Estimate impact fee based on trade size relative to liquidity
    fn estimate_impact(amount_in: u64, liquidity: u128, _tick_spacing: u16) -> (u16, u8) {
        if liquidity == 0 {
            // No liquidity - max impact, low confidence
            return (5000, 10); // 50% impact, 10% confidence
        }
        
        // Calculate percentage of liquidity being traded
        let trade_ratio = (amount_in as u128 * 10000) / liquidity;
        
        // Estimate ticks moved using logarithmic approximation
        // This is a simplified model - real impact depends on tick distribution
        let estimated_ticks = match trade_ratio {
            0..=10 => 1,          // <0.1% of liquidity: ~1 tick
            11..=50 => 5,         // 0.1-0.5%: ~5 ticks
            51..=100 => 10,       // 0.5-1%: ~10 ticks
            101..=200 => 20,      // 1-2%: ~20 ticks
            201..=500 => 50,      // 2-5%: ~50 ticks
            501..=1000 => 100,    // 5-10%: ~100 ticks
            1001..=2000 => 200,   // 10-20%: ~200 ticks
            2001..=5000 => 500,   // 20-50%: ~500 ticks
            _ => 1000,            // >50%: ~1000 ticks
        };
        
        // Convert ticks to basis points using the table
        let impact_bps = Self::ticks_to_bps_estimate(estimated_ticks);
        
        // Calculate confidence based on trade size
        let confidence = match trade_ratio {
            0..=100 => 95,    // Small trades: high confidence
            101..=500 => 85,  // Medium trades: good confidence
            501..=1000 => 70, // Large trades: moderate confidence
            1001..=2000 => 50, // Very large: low confidence
            _ => 30,          // Extreme: very low confidence
        };
        
        (impact_bps, confidence)
    }
    
    /// Convert estimated ticks to basis points
    fn ticks_to_bps_estimate(ticks: i32) -> u16 {
        // Using the same table from the contract
        match ticks {
            0..=10 => ticks as u16,
            11..=20 => 10 + (ticks - 10) as u16,
            21..=30 => 20 + (ticks - 20) as u16,
            31..=40 => 30 + (ticks - 30) as u16,
            41..=50 => 40 + (ticks - 40) as u16,
            51..=60 => 50 + (ticks - 50) as u16,
            61..=70 => 60 + (ticks - 60) as u16,
            71..=80 => 70 + ((ticks - 70) as u16 * 11 / 10),
            81..=90 => 81 + ((ticks - 80) as u16 * 10 / 10),
            91..=100 => 91 + ((ticks - 90) as u16 * 9 / 10),
            101..=200 => 100 + ((ticks - 100) as u16 * 101 / 100),
            201..=300 => 201 + ((ticks - 200) as u16 * 102 / 100),
            301..=400 => 303 + ((ticks - 300) as u16 * 103 / 100),
            401..=500 => 406 + ((ticks - 400) as u16 * 104 / 100),
            501..=1000 => 510 + ((ticks - 500) as u16 * 106 / 100),
            1001..=2000 => 1046 + ((ticks - 1000) as u16 * 116 / 100),
            _ => 2204 + ((ticks - 2000).min(8000) as u16 * 120 / 100),
        }
    }
    
    /// Estimate fees for a multi-hop route
    pub fn estimate_route_fees(
        hop1_params: &FeeEstimateParams,
        hop2_params: Option<&FeeEstimateParams>,
    ) -> SdkResult<FeeEstimate> {
        let hop1 = Self::estimate_fees(hop1_params)?;
        
        if let Some(hop2_params) = hop2_params {
            let hop2 = Self::estimate_fees(hop2_params)?;
            
            // Combine fees (not perfectly accurate due to compounding)
            let total_bps = hop1.total_fee_bps.saturating_add(hop2.total_fee_bps);
            let avg_confidence = (hop1.confidence as u16 + hop2.confidence as u16) / 2;
            
            Ok(FeeEstimate {
                base_fee_bps: hop1.base_fee_bps + hop2.base_fee_bps,
                impact_fee_bps: hop1.impact_fee_bps + hop2.impact_fee_bps,
                total_fee_bps: total_bps.min(10000), // Cap at 100%
                confidence: avg_confidence.min(100) as u8,
                is_degraded: hop1.is_degraded || hop2.is_degraded,
            })
        } else {
            Ok(hop1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_estimation() {
        // Small trade relative to liquidity
        let params = FeeEstimateParams {
            amount_in: 1_000_000, // 1 token
            current_tick: 0,
            liquidity: 1_000_000_000_000, // 1M tokens
            base_fee_bps: 30,
            tick_spacing: 1,
        };
        
        let estimate = FeeEstimator::estimate_fees(&params).unwrap();
        assert_eq!(estimate.base_fee_bps, 30);
        assert!(estimate.impact_fee_bps >= 10); // At least floor
        assert!(estimate.confidence > 90); // High confidence for small trade
        assert!(!estimate.is_degraded);
    }
    
    #[test]
    fn test_large_trade_impact() {
        // Large trade relative to liquidity
        let params = FeeEstimateParams {
            amount_in: 100_000_000_000, // 100k tokens
            current_tick: 0,
            liquidity: 1_000_000_000_000, // 1M tokens
            base_fee_bps: 30,
            tick_spacing: 1,
        };
        
        let estimate = FeeEstimator::estimate_fees(&params).unwrap();
        assert!(estimate.impact_fee_bps >= 100); // Significant impact (100 bps)
        assert!(estimate.confidence < 80); // Lower confidence
        assert!(estimate.is_degraded); // Degraded estimate
    }
}