/// Unified order parameter structures that consolidate all order-related operations.
/// This module provides a simplified API for all trading operations through the 3D order system.
use anchor_lang::prelude::*;
use crate::state::duration::Duration;

// ============================================================================
// Unified Order Parameters
// ============================================================================

/// Unified parameters for all order operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UnifiedOrderParams {
    /// Base amount for the order
    pub amount: u64,
    
    /// Order configuration
    pub config: OrderConfig,
    
    /// Optional advanced parameters
    pub advanced: Option<AdvancedOrderParams>,
}

/// Order configuration specifying the type and parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderConfig {
    /// Spot swap (immediate execution)
    Swap {
        /// Token to swap from (true = token A to B, false = token B to A)
        is_token_a_to_b: bool,
        /// Minimum output amount (slippage protection)
        min_amount_out: u64,
        /// Optional rate limit
        sqrt_rate_limit: Option<u128>,
    },
    
    /// Add liquidity to a pool
    AddLiquidity {
        /// Lower tick of the position
        tick_lower: i32,
        /// Upper tick of the position
        tick_upper: i32,
        /// Optional: specific amounts for each token
        token_amounts: Option<(u64, u64)>,
    },
    
    /// Remove liquidity from a pool
    RemoveLiquidity {
        /// Position to remove liquidity from
        position: Pubkey,
        /// Percentage to remove (basis points, 10000 = 100%)
        liquidity_percentage: u16,
        /// Minimum amounts to receive
        min_amounts: Option<(u64, u64)>,
    },
    
    /// Limit order (executes when conditions are met)
    LimitOrder {
        /// Direction of the order
        is_buy: bool,
        /// Target execution rate
        target_sqrt_rate: u128,
        /// Expiry time (0 for no expiry)
        expiry: i64,
    },
    
    /// Flash loan
    FlashLoan {
        /// Token to borrow (true = token A, false = token B)
        borrow_token_a: bool,
        /// Callback program to execute
        callback_program: Pubkey,
        /// Callback data
        callback_data: Vec<u8>,
    },
}

/// Advanced order parameters for sophisticated trading
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AdvancedOrderParams {
    /// Duration commitment (defaults to Swap for immediate orders)
    pub duration: Duration,
    
    /// Leverage multiplier (6 decimals, 1e6 = 1.0x)
    pub leverage: u64,
    
    /// MEV protection
    pub mev_protection: Option<MevProtection>,
    
    /// Hook data for custom logic
    pub hook_data: Option<Vec<u8>>,
}

/// MEV protection parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MevProtection {
    /// Maximum allowed slippage in basis points
    pub max_slippage_bps: u16,
    /// Minimum blocks before execution
    pub min_blocks_delay: u8,
    /// Required validator signature
    pub validator_signature: Option<[u8; 64]>,
}

// ============================================================================
// Order Modification Parameters
// ============================================================================

/// Unified parameters for modifying existing orders
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UnifiedModifyParams {
    /// Order or position to modify
    pub target: ModifyTarget,
    
    /// Modification to apply
    pub modification: OrderModification,
}

/// Target of the modification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum ModifyTarget {
    /// Modify a specific order by ID
    Order(Pubkey),
    /// Modify a liquidity position
    Position(Pubkey),
    /// Modify all orders matching criteria
    Batch(BatchCriteria),
}

/// Batch modification criteria
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BatchCriteria {
    /// Pool to target
    pub pool: Pubkey,
    /// Order type filter
    pub order_type: Option<OrderTypeFilter>,
    /// Rate range filter
    pub rate_range: Option<(i32, i32)>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderTypeFilter {
    Liquidity,
    Limit,
    All,
}

/// Modification to apply to orders
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderModification {
    /// Cancel the order
    Cancel,
    
    /// Update order parameters
    Update {
        /// New amount (if changing)
        amount: Option<u64>,
        /// New rate parameters (if changing)
        rate: Option<RateUpdate>,
        /// New leverage (if changing)
        leverage: Option<u64>,
        /// New duration (if changing)
        duration: Option<Duration>,
    },
    
    /// Partially fill a limit order
    PartialFill {
        /// Amount to fill
        fill_amount: u64,
    },
}

/// Rate update parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RateUpdate {
    /// Update target rate for limit orders
    TargetRate(u128),
    /// Update tick range for liquidity
    TickRange { tick_lower: i32, tick_upper: i32 },
}

// ============================================================================
// Order Computation Parameters
// ============================================================================

/// Simplified parameters for order computation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UnifiedComputeParams {
    /// Order configuration to compute
    pub order_config: OrderConfig,
    
    /// Optional: specific route preference
    pub route_preference: Option<RoutePreference>,
}

/// Route preference for order execution
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RoutePreference {
    /// Use the most liquid route
    MostLiquid,
    /// Use the shortest route (fewest hops)
    Shortest,
    /// Use a specific route
    Specific(Vec<Pubkey>),
}

// ============================================================================
// Result Types
// ============================================================================

/// Unified result for all order operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UnifiedOrderResult {
    /// Order ID (for trackable orders)
    pub order_id: Option<Pubkey>,
    
    /// Execution summary
    pub execution: ExecutionSummary,
    
    /// Gas used estimate
    pub gas_used: u64,
}

/// Summary of order execution
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum ExecutionSummary {
    /// Swap executed
    Swap {
        amount_in: u64,
        amount_out: u64,
        rate_achieved: u128,
    },
    
    /// Liquidity added
    LiquidityAdded {
        position_id: Pubkey,
        liquidity: u128,
        amounts: (u64, u64),
    },
    
    /// Liquidity removed
    LiquidityRemoved {
        amounts_received: (u64, u64),
        fees_collected: (u64, u64),
    },
    
    /// Limit order created
    LimitCreated {
        order_id: Pubkey,
        expires_at: Option<i64>,
    },
    
    /// Flash loan executed
    FlashLoanExecuted {
        amount_borrowed: u64,
        fee_paid: u64,
    },
}

// ============================================================================
// Conversion Helpers
// ============================================================================

impl UnifiedOrderParams {
    /// Convert to the internal OrderParams structure
    pub fn to_internal_params(self) -> crate::instructions::order::OrderParams {
        use crate::instructions::order::{OrderParams, OrderType, RateParams};
        
        let (rate_params, order_type, limit_value) = match self.config {
            OrderConfig::Swap { is_token_a_to_b, min_amount_out, sqrt_rate_limit } => {
                let rate_params = RateParams::TargetRate {
                    sqrt_rate_limit: sqrt_rate_limit.unwrap_or(if is_token_a_to_b { 
                        u128::MIN 
                    } else { 
                        u128::MAX 
                    }),
                    is_token_a_to_b,
                };
                (rate_params, OrderType::Immediate, min_amount_out)
            }
            
            OrderConfig::AddLiquidity { tick_lower, tick_upper, .. } => {
                let rate_params = RateParams::RateRange { tick_lower, tick_upper };
                (rate_params, OrderType::Liquidity, 0)
            }
            
            OrderConfig::LimitOrder { is_buy, target_sqrt_rate, .. } => {
                let rate_params = RateParams::TargetRate {
                    sqrt_rate_limit: target_sqrt_rate,
                    is_token_a_to_b: !is_buy, // Buy = B to A
                };
                (rate_params, OrderType::Limit, 0)
            }
            
            OrderConfig::FlashLoan { .. } => {
                // Flash loans use immediate execution with special duration
                let rate_params = RateParams::TargetRate {
                    sqrt_rate_limit: u128::MAX,
                    is_token_a_to_b: true,
                };
                (rate_params, OrderType::Immediate, 0)
            }
            
            OrderConfig::RemoveLiquidity { position, liquidity_percentage, min_amounts } => {
                // For remove liquidity, we don't need rate params in the same way
                // Use a placeholder that won't affect the operation
                let rate_params = RateParams::Market;
                let order_type = OrderType::Immediate;
                let limit_value = 0; // Not used for liquidity removal
                (rate_params, order_type, limit_value)
            }
        };
        
        let duration = self.advanced
            .as_ref()
            .map(|a| a.duration)
            .unwrap_or(match self.config {
                OrderConfig::FlashLoan { .. } => Duration::Flash,
                _ => Duration::Swap,
            });
        
        let leverage = self.advanced
            .as_ref()
            .map(|a| a.leverage)
            .unwrap_or(1_000_000); // 1x default
        
        OrderParams {
            amount: self.amount,
            rate_params,
            duration,
            leverage,
            order_type,
            limit_value,
        }
    }
}

impl UnifiedComputeParams {
    /// Convert to the internal OrderComputeParams structure
    pub fn to_internal_compute_params(self) -> crate::instructions::order_compute::OrderComputeParams {
        use crate::instructions::order_compute::{OrderComputeParams, RateComputeParams};
        use crate::state::duration::Duration;
        
        let (rate_params, leverage, duration) = match self.order_config {
            OrderConfig::Swap { is_token_a_to_b, sqrt_rate_limit, .. } => {
                let rate_params = RateComputeParams::SwapPath {
                    sqrt_rate_limit: sqrt_rate_limit.unwrap_or(if is_token_a_to_b { 
                        u128::MIN 
                    } else { 
                        u128::MAX 
                    }),
                    is_token_a_to_b,
                };
                (rate_params, 1_000_000, Duration::Swap)
            }
            
            OrderConfig::AddLiquidity { tick_lower, tick_upper, .. } => {
                let rate_params = RateComputeParams::LiquidityRange { 
                    tick_lower, 
                    tick_upper 
                };
                (rate_params, 1_000_000, Duration::Swap)
            }
            
            OrderConfig::LimitOrder { is_buy, target_sqrt_rate, .. } => {
                let rate_params = RateComputeParams::SwapPath {
                    sqrt_rate_limit: target_sqrt_rate,
                    is_token_a_to_b: !is_buy,
                };
                (rate_params, 1_000_000, Duration::Swap)
            }
            
            OrderConfig::FlashLoan { .. } => {
                let rate_params = RateComputeParams::SwapPath {
                    sqrt_rate_limit: u128::MAX,
                    is_token_a_to_b: true,
                };
                (rate_params, 1_000_000, Duration::Flash)
            }
            
            OrderConfig::RemoveLiquidity { .. } => {
                // Compute doesn't need special handling for remove liquidity
                let rate_params = RateComputeParams::SwapPath {
                    sqrt_rate_limit: u128::MAX,
                    is_token_a_to_b: true,
                };
                (rate_params, 1_000_000, Duration::Swap)
            }
        };
        
        OrderComputeParams {
            amount: 0, // Compute doesn't use amount
            rate_params,
            leverage,
            duration,
        }
    }
}

impl UnifiedModifyParams {
    /// Convert to the internal OrderModifyParams structure
    pub fn to_internal_params(self) -> crate::instructions::order_modify::OrderModifyParams {
        use crate::instructions::order_modify::{OrderModifyParams, OrderModification as InternalModification, ModificationParams};
        
        // Extract order ID from target
        let order_id = match self.target {
            ModifyTarget::Order(id) => id,
            ModifyTarget::Position(id) => id,
            ModifyTarget::Batch(_) => {
                // For batch modifications, we'd need a different approach
                // For now, use a default pubkey as this would be handled differently
                Pubkey::default()
            }
        };
        
        // Convert modification type
        let modification = match self.modification {
            OrderModification::Cancel => {
                // Cancel maps to removing all liquidity
                InternalModification::RemoveLiquidity { amount_to_remove: u64::MAX }
            }
            OrderModification::Update { amount, rate, leverage, duration } => {
                // Map to the most appropriate internal modification
                if let Some(new_leverage) = leverage {
                    InternalModification::AdjustLeverage { new_leverage }
                } else if let Some(new_duration) = duration {
                    InternalModification::ChangeDuration { new_duration }
                } else if let Some(new_amount) = amount {
                    InternalModification::AddLiquidity { additional_amount: new_amount }
                } else if let Some(rate_update) = rate {
                    match rate_update {
                        RateUpdate::TargetRate(new_rate) => {
                            InternalModification::UpdateLimit { new_rate_limit: new_rate }
                        }
                        RateUpdate::TickRange { .. } => {
                            // Tick range updates would require position migration
                            // For now, default to no-op
                            InternalModification::AddLiquidity { additional_amount: 0 }
                        }
                    }
                } else {
                    // No modification specified
                    InternalModification::AddLiquidity { additional_amount: 0 }
                }
            }
            OrderModification::PartialFill { fill_amount } => {
                // Partial fills map to removing liquidity
                InternalModification::RemoveLiquidity { amount_to_remove: fill_amount }
            }
        };
        
        OrderModifyParams {
            order_id,
            modification,
            new_params: ModificationParams {
                max_slippage_bps: 100, // 1% default
                immediate: true,
            },
        }
    }
}