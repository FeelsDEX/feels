//! # Order Types
//! 
//! Types for unified order system.

use crate::types::{Duration, RiskProfile};

/// Position type for time/leverage positions
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionType {
    /// Standard position (no time/leverage)
    Standard,
    /// Time-locked position
    Time { duration: Duration },
    /// Leveraged position
    Leverage { risk_profile: RiskProfile },
}

/// Order modification types
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum OrderModification {
    AdjustLeverage { new_leverage: u64 },
    ChangeDuration { new_duration: Duration },
    AddLiquidity { additional_amount: u64 },
    RemoveLiquidity { amount_to_remove: u64 },
    UpdateLimit { new_rate_limit: u128 },
}

/// Swap result information
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct SwapResult {
    /// Amount of tokens swapped in
    pub amount_in: u64,
    /// Amount of tokens received
    pub amount_out: u64,
    /// Fee amount paid
    pub fee_amount: u64,
    /// Rebate amount received
    pub rebate_amount: u64,
    /// Final sqrt price after swap
    pub sqrt_price_after: u128,
    /// Average execution price
    pub avg_price: u128,
    /// Price impact (basis points)
    pub price_impact_bps: u32,
}

/// Position flow result
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct PositionFlowResult {
    /// Amount of tokens input
    pub amount_in: u64,
    /// Amount of tokens output
    pub tokens_out: u64,
    /// Exchange rate used (Q32)
    pub exchange_rate: u64,
    /// Fee charged
    pub fee_amount: u64,
    /// Rebate paid (if any)
    pub rebate_amount: u64,
    /// Work performed
    pub work: u64,
    /// Whether this was entry (true) or exit (false)
    pub is_entry: bool,
}

/// Liquidity result
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct LiquidityResult {
    /// Position ID for liquidity
    pub position_id: u64,
    /// Amount of liquidity added/removed
    pub liquidity: u128,
    /// Amount of token0
    pub amount0: u64,
    /// Amount of token1
    pub amount1: u64,
    /// Lower tick of range
    pub tick_lower: i32,
    /// Upper tick of range
    pub tick_upper: i32,
}