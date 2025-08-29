/// Centralized fee management module that consolidates all fee-related calculations
/// and state mutations. This module serves as the single source of truth for fee
/// logic in the Feels Protocol, reducing code duplication and making the fee system
/// easier to maintain and audit.

use anchor_lang::prelude::*;
use crate::state::{Pool, FeeConfig, FeelsProtocolError, DynamicFeeConfig, VolatilityTracker, LendingMetrics, RiskProfile};
use crate::utils::{BASIS_POINTS_DENOMINATOR, FeeBreakdown, mul_div_u64, sqrt_u64};
use crate::logic::volatility_manager::VolatilityManager;

// ============================================================================
// Fee Types and Context
// ============================================================================

/// Fee types supported by the protocol
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeeType {
    Swap,         // Standard trading fees
    FlashLoan,    // Flash loan fees
    Lending,      // Lending/borrowing fees
    Leverage,     // Leveraged position fees
    Liquidation,  // Liquidation fees
}

/// Fee calculation context
pub struct FeeContext<'a> {
    pub fee_type: FeeType,
    pub amount: u64,
    pub fee_config: &'a Account<'a, FeeConfig>,
    pub dynamic_config: &'a DynamicFeeConfig,
    pub volatility_tracker: Option<&'a AccountInfo<'a>>,
    pub lending_metrics: Option<&'a AccountInfo<'a>>,
    pub position_data: Option<PositionFeeData>, // For position-specific fees
    pub pool: Option<&'a Pool>, // Optional pool reference for pool-specific calculations
    pub volatility_bps: Option<u64>, // Pre-calculated volatility if available
    pub volume_24h: Option<u128>, // Pre-calculated 24h volume if available
}

/// Position-specific fee data
#[derive(Debug, Clone, Copy)]
pub struct PositionFeeData {
    pub leverage: u64,
    pub duration: Option<crate::state::duration::Duration>,
}

// ============================================================================
// Fee Manager Implementation
// ============================================================================

pub struct FeeManager;

impl FeeManager {
    /// Universal fee calculation entry point
    pub fn calculate_fee(context: FeeContext) -> Result<FeeBreakdown> {
        match context.fee_type {
            FeeType::Swap => Self::calculate_swap_fee(context),
            FeeType::FlashLoan => Self::calculate_flash_loan_fee(context),
            FeeType::Lending => Self::calculate_lending_fee(context),
            FeeType::Leverage => Self::calculate_leverage_fee(context),
            FeeType::Liquidation => Self::calculate_liquidation_fee(context),
        }
    }
    
    /// Calculate swap fee with enhanced volatility
    fn calculate_swap_fee(context: FeeContext) -> Result<FeeBreakdown> {
        let mut fee_rate = context.fee_config.base_rate;
        
        // Apply dynamic adjustments if available
        if let (Some(vol), Some(volume)) = (context.volatility_bps, context.volume_24h) {
            if vol > 0 || volume > 0 {
                fee_rate = Self::calculate_dynamic_fee_rate(
                    context.dynamic_config,
                    vol,
                    volume,
                )?;
            }
        } else if let Some(_volatility_tracker_info) = context.volatility_tracker {
            // Fallback: In production, would deserialize the account and calculate multiplier
            // TODO: For now, apply a simple 10% increase as placeholder
            fee_rate = ((fee_rate as u64 * 11000) / 10000)
                .min(context.dynamic_config.max_fee as u64)
                .max(context.dynamic_config.min_fee as u64) as u16;
        }
        
        // Apply position-specific adjustments
        if let Some(position_data) = context.position_data {
            // Apply leverage adjustment
            if position_data.leverage > RiskProfile::LEVERAGE_SCALE {
                let fee_multiplier = sqrt_u64(position_data.leverage);
                fee_rate = ((fee_rate as u64 * fee_multiplier) / 1_000)
                    .min(1000) as u16; // Cap at 10%
            }
            
            // Apply duration discount
            if let Some(duration) = position_data.duration {
                let duration_multiplier = duration.fee_multiplier();
                fee_rate = ((fee_rate as u64 * duration_multiplier as u64) / 100)
                    .max(1) as u16; // Ensure at least 0.01%
            }
        }
        
        // Apply flash loan activity adjustment if available
        if let Some(_lending_metrics_info) = context.lending_metrics {
            // In production, would deserialize and check market stress
            // TODO: For now, we skip the adjustment
        }
        
        // Calculate fees
        Self::calculate_fee_breakdown(context.amount, fee_rate, context.fee_config.protocol_share)
    }
    
    /// Calculate flash loan fee
    fn calculate_flash_loan_fee(context: FeeContext) -> Result<FeeBreakdown> {
        // Flash loans have higher base fee (0.09%)
        let base_fee = 9; // basis points
        
        // Apply stress multiplier if market is volatile
        let fee_rate = if let Some(_lending_metrics_info) = context.lending_metrics {
            // In production would check burst_detected
            // TODO: For now use base rate
            base_fee
        } else {
            base_fee
        };
        
        Self::calculate_fee_breakdown(context.amount, fee_rate, 5000) // 50% protocol share
    }
    
    /// Calculate lending fee
    fn calculate_lending_fee(context: FeeContext) -> Result<FeeBreakdown> {
        // Lending fees based on utilization (simplified)
        let base_fee = 20; // 0.2% base
        Self::calculate_fee_breakdown(context.amount, base_fee, 2000) // 20% protocol share
    }
    
    /// Calculate leverage fee
    fn calculate_leverage_fee(context: FeeContext) -> Result<FeeBreakdown> {
        let base_fee = context.fee_config.base_rate;
        
        // Apply leverage multiplier if provided
        let fee_rate = if let Some(position_data) = context.position_data {
            let leverage_multiplier = (position_data.leverage / 1_000_000).max(1).min(10);
            (base_fee as u64 * leverage_multiplier).min(1000) as u16 // Cap at 10%
        } else {
            base_fee
        };
        
        Self::calculate_fee_breakdown(context.amount, fee_rate, context.fee_config.protocol_share)
    }
    
    /// Calculate liquidation fee
    fn calculate_liquidation_fee(context: FeeContext) -> Result<FeeBreakdown> {
        // Liquidation fees are higher to incentivize liquidators
        let fee_rate = 500; // 5%
        Self::calculate_fee_breakdown(context.amount, fee_rate, 1000) // 10% protocol share
    }
    
    /// Helper to calculate fee breakdown
    fn calculate_fee_breakdown(
        amount: u64,
        fee_rate: u16,
        protocol_share: u16,
    ) -> Result<FeeBreakdown> {
        // Validate inputs
        require!(
            fee_rate <= BASIS_POINTS_DENOMINATOR as u16,
            FeelsProtocolError::InvalidFeeRate
        );
        require!(
            protocol_share <= BASIS_POINTS_DENOMINATOR as u16,
            FeelsProtocolError::InvalidProtocolFeeRate
        );
        
        // Calculate total fee
        let total_fee = (amount as u128)
            .checked_mul(fee_rate as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        require!(
            total_fee <= u64::MAX as u128,
            FeelsProtocolError::MathOverflow
        );
        let total_fee = total_fee as u64;
        
        // Calculate protocol fee
        let protocol_fee = (total_fee as u128)
            .checked_mul(protocol_share as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        require!(
            protocol_fee <= u64::MAX as u128,
            FeelsProtocolError::MathOverflow
        );
        let protocol_fee = protocol_fee as u64;
        
        // Calculate LP fee
        let lp_fee = total_fee
            .checked_sub(protocol_fee)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
        
        Ok(FeeBreakdown {
            total_fee,
            lp_fee,
            protocol_fee,
        })
    }
    
    /// Calculate order fees based on the fee configuration
    /// 
    /// # Arguments
    /// * `amount_in` - The input amount for the order
    /// * `fee_config` - The fee configuration for the pool
    /// 
    /// # Returns
    /// * `Result<FeeBreakdown>` - The calculated fee breakdown
    pub fn calculate_order_fee(
        amount_in: u64,
        fee_config: &Account<FeeConfig>,
    ) -> Result<FeeBreakdown> {
        // Validate fee configuration
        require!(
            fee_config.base_rate <= BASIS_POINTS_DENOMINATOR as u16,
            FeelsProtocolError::InvalidFeeRate
        );
        require!(
            fee_config.protocol_share <= BASIS_POINTS_DENOMINATOR as u16,
            FeelsProtocolError::InvalidProtocolFeeRate
        );

        // Calculate total fee
        let total_fee = (amount_in as u128)
            .checked_mul(fee_config.base_rate as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;

        // Ensure fee doesn't exceed u64::MAX
        require!(
            total_fee <= u64::MAX as u128,
            FeelsProtocolError::MathOverflow
        );
        let total_fee = total_fee as u64;

        // Calculate protocol fee from total fee
        let protocol_fee = (total_fee as u128)
            .checked_mul(fee_config.protocol_share as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;

        require!(
            protocol_fee <= u64::MAX as u128,
            FeelsProtocolError::MathOverflow
        );
        let protocol_fee = protocol_fee as u64;

        // Calculate LP fee (remaining after protocol fee)
        let lp_fee = total_fee
            .checked_sub(protocol_fee)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;

        Ok(FeeBreakdown {
            total_fee,
            lp_fee,
            protocol_fee,
        })
    }

    /// Update global fee growth for liquidity providers
    /// 
    /// This function should be called after each swap to distribute fees
    /// to liquidity providers through the fee growth mechanism.
    pub fn update_fee_growth(
        pool: &mut Pool,
        fee_amount: u64,
        is_token_a: bool,
    ) -> Result<()> {
        pool.update_fee_growth(fee_amount, is_token_a)
    }

    // ============================================================================
    // Convenience Methods (backwards compatibility)
    // ============================================================================

    /// Build a FeeContext with all necessary parameters
    /// Note: dynamic_config must be provided externally to avoid lifetime issues
    pub fn build_fee_context<'a>(
        fee_type: FeeType,
        amount: u64,
        fee_config: &'a Account<'a, FeeConfig>,
        dynamic_config: &'a DynamicFeeConfig,
        pool: Option<&'a Pool>,
        remaining_accounts: &[AccountInfo<'a>],
        position_data: Option<PositionFeeData>,
        volatility_bps: Option<u64>,
        volume_24h: Option<u128>,
    ) -> FeeContext<'a> {
        // Parse volatility tracker and flash loan TWAV from remaining_accounts if available
        let (volatility_tracker, lending_metrics) = if remaining_accounts.len() >= 2 {
            (Some(&remaining_accounts[0]), Some(&remaining_accounts[1]))
        } else {
            (None, None)
        };
        
        FeeContext {
            fee_type,
            amount,
            fee_config,
            dynamic_config,
            volatility_tracker,
            lending_metrics,
            position_data,
            pool,
            volatility_bps,
            volume_24h,
        }
    }
    
    /// Calculate complete fee breakdown for a swap amount
    /// This method is provided for backwards compatibility
    pub fn calculate_swap_fees(
        pool: &Pool,
        amount_in: u64,
        remaining_accounts: &[AccountInfo],
    ) -> Result<FeeBreakdown> {
        // For backwards compatibility, use the base fee calculation
        // In production, this would parse volatility data from remaining_accounts
        Self::calculate_fee_breakdown(amount_in, pool.fee_rate, pool.protocol_fee_rate)
    }

    // ============================================================================
    // Dynamic Fee Calculations
    // ============================================================================

    /// Calculate dynamic fee rate based on market conditions
    fn calculate_dynamic_fee_rate(
        config: &DynamicFeeConfig,
        volatility_bps: u64,
        volume_24h: u128,
    ) -> Result<u16> {
        let mut fee_rate = config.base_fee as u64;

        // Adjust for volatility
        if volatility_bps > 0 {
            // fee_adjustment = volatility_bps * coefficient / 10000
            let volatility_adjustment = volatility_bps
                .saturating_mul(config.volatility_coefficient as u64)
                .saturating_div(10_000);
            
            fee_rate = fee_rate.saturating_add(volatility_adjustment);
        }

        // Apply volume discount if above threshold
        if volume_24h > config.volume_discount_threshold {
            // 10% discount for high volume
            fee_rate = fee_rate.saturating_mul(90).saturating_div(100);
        }

        // Clamp to configured bounds
        let final_fee = fee_rate
            .max(config.min_fee as u64)
            .min(config.max_fee as u64);

        Ok(final_fee as u16)
    }

    /// Calculate effective fee rate for a pool
    /// This considers base rate and any dynamic adjustments
    pub fn get_effective_fee_rate(pool: &Pool) -> Result<u16> {
        // Phase 1: Return base fee rate
        // Phase 2+: Would implement dynamic adjustments based on volume/volatility
        // Phase 2: Would implement dynamic adjustments based on volume/volatility
        // For now, return base rate
        Ok(pool.fee_rate)
    }

    // ============================================================================
    // Fee Collection and Distribution
    // ============================================================================

    /// Accumulate protocol fees after a swap
    pub fn accumulate_protocol_fees(
        pool: &mut Pool,
        fee_breakdown: &FeeBreakdown,
        is_token_a: bool,
    ) -> Result<()> {
        pool.accumulate_protocol_fees(fee_breakdown.protocol_fee, is_token_a)
    }

    /// Calculate fee growth delta using high-precision math
    pub fn calculate_fee_growth_delta(
        fee_amount: u64,
        liquidity: u128,
    ) -> Result<[u64; 4]> {
        crate::utils::calculate_fee_growth_delta(fee_amount, liquidity)
    }

    // ============================================================================
    // Fee Validation and Configuration
    // ============================================================================

    /// Validate fee configuration for a pool
    pub fn validate_fee_configuration(pool: &Pool) -> Result<()> {
        use crate::utils::FeeMath;
        FeeMath::validate_fee_rate(pool.fee_rate)?;
        FeeMath::validate_protocol_fee_rate(pool.protocol_fee_rate)?;
        Ok(())
    }

    /// Initialize fee configuration for a new pool
    pub fn initialize_fees(
        pool: &mut Pool,
        fee_rate: u16,
    ) -> Result<()> {
        use crate::state::FeeConfig;
        let (validated_fee_rate, protocol_fee_rate, tick_spacing) =
            FeeConfig::create_for_pool(fee_rate)?;

        pool.fee_rate = validated_fee_rate;
        pool.protocol_fee_rate = protocol_fee_rate;
        pool.tick_spacing = tick_spacing;

        Ok(())
    }

    /// Update dynamic fee configuration (Phase 2)
    pub fn update_dynamic_fee_config(
        pool: &mut Pool,
        config: DynamicFeeConfig,
    ) -> Result<()> {
        // Validate configuration
        require!(
            config.min_fee <= config.base_fee && config.base_fee <= config.max_fee,
            FeelsProtocolError::InvalidFeeRate
        );
        require!(
            config.max_fee <= 1000, // Max 10%
            FeelsProtocolError::InvalidFeeRate
        );

        // Update dynamic fee config directly on pool
        pool.dynamic_fee_config = config;

        Ok(())
    }

    // ============================================================================
    // Fee Collection Instructions
    // ============================================================================

    /// Collect fees for a position
    pub fn collect_position_fees(
        pool: &Pool,
        position_liquidity: u128,
        fee_growth_inside_0: [u64; 4],
        fee_growth_inside_1: [u64; 4],
        fees_owed_0: u64,
        fees_owed_1: u64,
    ) -> Result<(u64, u64)> {
        use crate::utils::FeeGrowthMath;
        
        // Calculate fees earned since last collection
        let fee_growth_delta_0 = FeeGrowthMath::sub_fee_growth(
            pool.fee_growth_global_a,
            fee_growth_inside_0,
        )?;
        
        let fee_growth_delta_1 = FeeGrowthMath::sub_fee_growth(
            pool.fee_growth_global_b,
            fee_growth_inside_1,
        )?;

        // Calculate fees earned
        let fees_earned_0 = mul_div_u64(
            position_liquidity as u64,
            fee_growth_delta_0[0], // Use lowest 64 bits for calculation
            1u64 << 32, // Scale factor
        )?;

        let fees_earned_1 = mul_div_u64(
            position_liquidity as u64,
            fee_growth_delta_1[0],
            1u64 << 32,
        )?;

        // Total fees to collect
        let total_fees_0 = fees_owed_0
            .checked_add(fees_earned_0)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        let total_fees_1 = fees_owed_1
            .checked_add(fees_earned_1)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        Ok((total_fees_0, total_fees_1))
    }

    /// Collect protocol fees
    pub fn collect_protocol_fees(
        pool: &mut Pool,
    ) -> Result<(u64, u64)> {
        let fees_0 = pool.protocol_fees_a;
        let fees_1 = pool.protocol_fees_b;

        // Reset protocol fees
        pool.protocol_fees_a = 0;
        pool.protocol_fees_b = 0;

        Ok((fees_0, fees_1))
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Calculate the amount after deducting fees
    pub fn calculate_amount_after_fee(
        pool: &Pool,
        amount_in: u64,
    ) -> Result<u64> {
        let fee_breakdown = Self::calculate_fee_breakdown(amount_in, pool.fee_rate, pool.protocol_fee_rate)?;
        amount_in
            .checked_sub(fee_breakdown.total_fee)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow.into())
    }

    /// Get the appropriate fee calculation method based on pool state
    /// This method creates a proper FeeContext and uses the generic calculate_fee
    pub fn get_fee_breakdown(
        pool: &Pool,
        amount_in: u64,
        volatility_bps: Option<u64>,
        volume_24h: Option<u128>,
        average_leverage: Option<u64>,
        duration: Option<crate::state::duration::Duration>,
        fee_config: &Account<FeeConfig>,
    ) -> Result<FeeBreakdown> {
        // Build position data if leverage or duration is specified
        let position_data = if average_leverage.is_some() || duration.is_some() {
            Some(PositionFeeData {
                leverage: average_leverage.unwrap_or(RiskProfile::LEVERAGE_SCALE),
                duration,
            })
        } else {
            None
        };
        
        // Get dynamic config directly from pool
        let dynamic_config = &pool.dynamic_fee_config;
        
        // Build context
        let context = FeeContext {
            fee_type: FeeType::Swap,
            amount: amount_in,
            fee_config,
            dynamic_config,
            volatility_tracker: None,
            lending_metrics: None,
            position_data,
            pool: Some(pool),
            volatility_bps,
            volume_24h,
        };
        
        // Use the generic calculate_fee method
        Self::calculate_fee(context)
    }
    
    /// Calculate dynamic swap fees (deprecated - use calculate_fee with proper context)
    #[deprecated(since = "2.0.0", note = "Use calculate_fee with proper FeeContext instead")]
    #[allow(dead_code)]
    pub fn calculate_dynamic_swap_fees(
        pool: &Pool,
        amount_in: u64,
        volatility_bps: u64,
        volume_24h: u128,
    ) -> Result<FeeBreakdown> {
        // For backwards compatibility, get dynamic config from pool
        let dynamic_config = pool.dynamic_fee_config;
        
        let fee_rate = Self::calculate_dynamic_fee_rate(&dynamic_config, volatility_bps, volume_24h)?;
        Self::calculate_fee_breakdown(amount_in, fee_rate, pool.protocol_fee_rate)
    }
    
    /// Calculate swap fees with leverage (deprecated - use calculate_fee with proper context)
    #[deprecated(since = "2.0.0", note = "Use calculate_fee with proper FeeContext instead")]
    #[allow(dead_code)]
    pub fn calculate_swap_fees_with_leverage(
        pool: &Pool,
        amount_in: u64,
        average_leverage: u64,
    ) -> Result<FeeBreakdown> {
        // Base fee calculation
        let mut fee_breakdown = Self::calculate_fee_breakdown(amount_in, pool.fee_rate, pool.protocol_fee_rate)?;

        // Apply leverage multiplier if leverage > 1x
        if average_leverage > RiskProfile::LEVERAGE_SCALE {
            // Calculate fee multiplier: sqrt(leverage) for gradual increase
            let fee_multiplier = sqrt_u64(average_leverage);

            // Apply multiplier to fees
            fee_breakdown.lp_fee = fee_breakdown
                .lp_fee
                .checked_mul(fee_multiplier)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(1_000)
                .ok_or(FeelsProtocolError::MathOverflow)?;

            fee_breakdown.protocol_fee = fee_breakdown
                .protocol_fee
                .checked_mul(fee_multiplier)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(1_000)
                .ok_or(FeelsProtocolError::MathOverflow)?;

            fee_breakdown.total_fee = fee_breakdown
                .lp_fee
                .checked_add(fee_breakdown.protocol_fee)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }

        Ok(fee_breakdown)
    }
}
