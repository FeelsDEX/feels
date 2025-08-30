/// Centralized fee management module that consolidates all fee-related calculations
/// and state mutations. This module serves as the single source of truth for fee
/// logic in the Feels Protocol, reducing code duplication and making the fee system
/// easier to maintain and audit.

use anchor_lang::prelude::*;
use crate::state::{Pool, FeeConfig, FeelsProtocolError, RiskProfile};
use crate::utils::{BASIS_POINTS_DENOMINATOR, FeeBreakdown, mul_div_u64, sqrt_u64};

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
    pub volatility_tracker: Option<&'a AccountInfo<'a>>,
    pub lending_metrics: Option<&'a AccountInfo<'a>>,
    pub position_data: Option<PositionFeeData>, // For position-specific fees
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
        // Get fee rate from FeeConfig (base or dynamic)
        let mut fee_rate = context.fee_config.get_fee_rate(
            context.volatility_bps,
            context.volume_24h
        );
        
        // Apply volatility tracker adjustments if available but no direct volatility data
        if context.volatility_bps.is_none() && context.volatility_tracker.is_some() {
            // Fallback: In production, would deserialize the account and calculate multiplier
            // TODO: For now, apply a simple 10% increase as placeholder
            let dynamic_config = &context.fee_config.dynamic_config;
            fee_rate = ((fee_rate as u64 * 11000) / 10000)
                .min(dynamic_config.max_fee as u64)
                .max(dynamic_config.min_fee as u64) as u16;
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
        
        // Calculate fees using FeeConfig
        let total_fee = mul_div_u64(context.amount, fee_rate as u64, BASIS_POINTS_DENOMINATOR)?;
        let protocol_fee = context.fee_config.calculate_protocol_fee(total_fee);
        let liquidity_fee = context.fee_config.calculate_lp_fee(total_fee);
        
        Ok(FeeBreakdown {
            total_fee,
            liquidity_fee,
            protocol_fee,
        })
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
        
        // Calculate fees - flash loans have 50% protocol share
        let total_fee = mul_div_u64(context.amount, fee_rate as u64, BASIS_POINTS_DENOMINATOR)?;
        let protocol_fee = total_fee / 2; // 50% protocol share
        let liquidity_fee = total_fee - protocol_fee;
        
        Ok(FeeBreakdown {
            total_fee,
            liquidity_fee,
            protocol_fee,
        })
    }
    
    /// Calculate lending fee
    fn calculate_lending_fee(context: FeeContext) -> Result<FeeBreakdown> {
        // Lending fees based on utilization (simplified)
        let base_fee = 20; // 0.2% base
        
        let total_fee = mul_div_u64(context.amount, base_fee as u64, BASIS_POINTS_DENOMINATOR)?;
        let protocol_fee = total_fee / 5; // 20% protocol share
        let liquidity_fee = total_fee - protocol_fee;
        
        Ok(FeeBreakdown {
            total_fee,
            liquidity_fee,
            protocol_fee,
        })
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
        
        // Calculate fees using FeeConfig
        let total_fee = mul_div_u64(context.amount, fee_rate as u64, BASIS_POINTS_DENOMINATOR)?;
        let protocol_fee = context.fee_config.calculate_protocol_fee(total_fee);
        let liquidity_fee = context.fee_config.calculate_lp_fee(total_fee);
        
        Ok(FeeBreakdown {
            total_fee,
            liquidity_fee,
            protocol_fee,
        })
    }
    
    /// Calculate liquidation fee
    fn calculate_liquidation_fee(context: FeeContext) -> Result<FeeBreakdown> {
        // Liquidation fees are higher to incentivize liquidators
        let fee_rate = 500; // 5%
        
        let total_fee = mul_div_u64(context.amount, fee_rate as u64, BASIS_POINTS_DENOMINATOR)?;
        let protocol_fee = total_fee / 10; // 10% protocol share
        let liquidity_fee = total_fee - protocol_fee;
        
        Ok(FeeBreakdown {
            total_fee,
            liquidity_fee,
            protocol_fee,
        })
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
            FeelsProtocolError::InvalidFeeRate
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
            liquidity_fee: lp_fee,
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
            FeelsProtocolError::InvalidFeeRate
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
            liquidity_fee: lp_fee,
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
        pool.accumulate_fee_growth(fee_amount, is_token_a)
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Build a FeeContext with all necessary parameters
    pub fn build_fee_context<'a>(
        fee_type: FeeType,
        amount: u64,
        fee_config: &'a Account<'a, FeeConfig>,
        _remaining_accounts: &'a [AccountInfo<'a>],
        position_data: Option<PositionFeeData>,
        volatility_bps: Option<u64>,
        volume_24h: Option<u128>,
    ) -> FeeContext<'a> {
        // Parse volatility tracker and flash loan TWAV from remaining_accounts if available
        let (volatility_tracker, lending_metrics) = if _remaining_accounts.len() >= 2 {
            (Some(&_remaining_accounts[0]), Some(&_remaining_accounts[1]))
        } else {
            (None, None)
        };
        
        FeeContext {
            fee_type,
            amount,
            fee_config,
            volatility_tracker,
            lending_metrics,
            position_data,
            volatility_bps,
            volume_24h,
        }
    }

    // ============================================================================
    // Dynamic Fee Calculations
    // ============================================================================

    /// Calculate effective fee rate for a pool
    /// This method requires the FeeConfig account to be provided
    pub fn get_effective_fee_rate(
        fee_config: &FeeConfig,
        volatility_bps: Option<u64>,
        volume_24h: Option<u128>,
    ) -> Result<u16> {
        Ok(fee_config.get_fee_rate(volatility_bps, volume_24h))
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

    /// Validate fee configuration
    pub fn validate_fee_configuration(fee_config: &FeeConfig) -> Result<()> {
        require!(
            fee_config.base_rate <= 10000,
            FeelsProtocolError::InvalidFeeRate
        );
        require!(
            fee_config.protocol_share <= 10000,
            FeelsProtocolError::InvalidProtocolFeeRate
        );
        Ok(())
    }

    /// Update dynamic fee configuration
    pub fn update_dynamic_fee_config(
        fee_config: &mut FeeConfig,
        config: crate::state::fee::DynamicFeeConfig,
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

        // Update dynamic fee config on FeeConfig account
        fee_config.dynamic_config = config;
        fee_config.last_update = Clock::get()?.unix_timestamp;

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
        fee_config: &FeeConfig,
        amount_in: u64,
    ) -> Result<u64> {
        let fee_rate = fee_config.base_rate;
        let total_fee = mul_div_u64(amount_in, fee_rate as u64, BASIS_POINTS_DENOMINATOR)?;
        amount_in
            .checked_sub(total_fee)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow.into())
    }

    /// Get the appropriate fee calculation method based on pool state
    /// This is a convenience method that calculates fees without full context
    pub fn get_fee_breakdown(
        _pool: &Pool,
        amount_in: u64,
        volatility_bps: Option<u64>,
        volume_24h: Option<u128>,
        average_leverage: Option<u64>,
        _duration: Option<crate::state::duration::Duration>,
        fee_config: &Account<FeeConfig>,
    ) -> Result<FeeBreakdown> {
        // Calculate base fee rate
        let base_rate = fee_config.base_rate as u64;
        
        // Apply position multipliers if applicable
        let mut multiplier = 10000u64; // 100% base
        
        if let Some(leverage) = average_leverage {
            if leverage > RiskProfile::LEVERAGE_SCALE {
                // Add 10% for each 1x of leverage above 1x
                let leverage_multiplier = 10000 + ((leverage - RiskProfile::LEVERAGE_SCALE) * 1000 / RiskProfile::LEVERAGE_SCALE);
                multiplier = multiplier.saturating_mul(leverage_multiplier) / 10000;
            }
        }
        
        // Apply volatility adjustment
        if let Some(vol_bps) = volatility_bps {
            if vol_bps > 500 {
                // High volatility: increase fee by 50%
                multiplier = multiplier.saturating_mul(15000) / 10000;
            } else if vol_bps > 300 {
                // Medium volatility: increase fee by 20%
                multiplier = multiplier.saturating_mul(12000) / 10000;
            }
        }
        
        // Apply volume discount
        if let Some(vol_24h) = volume_24h {
            if vol_24h > 10_000_000_000_000 { // > $10M
                // High volume: 10% discount
                multiplier = multiplier.saturating_mul(9000) / 10000;
            }
        }
        
        // Calculate final fee rate
        let fee_rate = base_rate.saturating_mul(multiplier) / 10000;
        let fee_rate = fee_rate.clamp(1, 10000); // 0.01% to 100%
        
        // Calculate fee amounts
        let total_fee = amount_in.saturating_mul(fee_rate) / 10000;
        let protocol_fee = total_fee.saturating_mul(fee_config.protocol_share as u64) / 10000;
        let lp_fee = total_fee.saturating_sub(protocol_fee);
        
        Ok(FeeBreakdown {
            total_fee,
            lp_fee,
            protocol_fee,
            liquidity_fee: lp_fee,
        })
    }
}
