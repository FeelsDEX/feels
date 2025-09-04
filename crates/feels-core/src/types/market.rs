//! # Market Types
//! 
//! Core market structure definitions for the 3D thermodynamic AMM.

use crate::types::field::FieldCommitmentData;

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Duration options for time dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub enum Duration {
    Flash,
    Swap,
    Weekly,
    Monthly,
    Quarterly,
    Annual,
}

impl Duration {
    /// Convert to seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            Duration::Flash => 1,
            Duration::Swap => 60,
            Duration::Weekly => 7 * 24 * 60 * 60,
            Duration::Monthly => 30 * 24 * 60 * 60,
            Duration::Quarterly => 90 * 24 * 60 * 60,
            Duration::Annual => 365 * 24 * 60 * 60,
        }
    }
    
    /// Convert to slots (assuming 400ms slots)
    pub fn to_slots(&self) -> u64 {
        self.to_seconds() * 1000 / 400
    }
}

/// Risk profile for leverage dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct RiskProfile {
    /// Maximum leverage (6 decimals, 1_000_000 = 1x)
    pub max_leverage: u64,
    /// Protection factor (basis points)
    pub protection_factor: u32,
}

impl RiskProfile {
    /// Leverage scale factor
    pub const LEVERAGE_SCALE: u64 = 1_000_000;
    
    /// Maximum leverage (10x)
    pub const MAX_LEVERAGE_SCALE: u64 = 10_000_000;
    
    /// Create conservative profile (1x)
    pub fn conservative() -> Self {
        Self {
            max_leverage: Self::LEVERAGE_SCALE,
            protection_factor: 10000, // 100%
        }
    }
    
    /// Create moderate profile (3x)
    pub fn moderate() -> Self {
        Self {
            max_leverage: 3 * Self::LEVERAGE_SCALE,
            protection_factor: 5000, // 50%
        }
    }
    
    /// Create aggressive profile (5x)
    pub fn aggressive() -> Self {
        Self {
            max_leverage: 5 * Self::LEVERAGE_SCALE,
            protection_factor: 2500, // 25%
        }
    }
}

// ============================================================================
// Extended Market Types (for off-chain use)
// ============================================================================

#[cfg(feature = "client")]
pub mod extended {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    /// Complete market state data used for field computation and client operations
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MarketState {
        /// Market identifier (Pubkey as [u8; 32])
        pub market_pubkey: [u8; 32],
        
        /// Current market prices and liquidity
        pub current_sqrt_price: u128,
        pub liquidity: u128,
        pub tick_current: i32,
        
        /// Fee growth tracking  
        pub fee_growth_global_0: u128,
        pub fee_growth_global_1: u128,
        
        /// Protocol fees accumulated
        pub protocol_fees_0: u64,
        pub protocol_fees_1: u64,
        
        /// Time-weighted average prices
        pub twap_0: u128,
        pub twap_1: u128,
        
        /// Market metadata
        pub last_update_ts: i64,
        pub total_volume_0: u128,
        pub total_volume_1: u128,
        pub swap_count: u64,
        
        /// Token information
        pub token_0_mint: [u8; 32],
        pub token_1_mint: [u8; 32],
        pub token_0_decimals: u8,
        pub token_1_decimals: u8,
        
        /// Current field commitment (if available)
        pub field_commitment: Option<FieldCommitmentData>,
        
        /// Current base fee in basis points (optional, set by hysteresis controller)
        pub base_fee_bps: Option<u64>,
    }
    
    /// Simplified market field data for work calculations
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[allow(non_snake_case)]
    pub struct MarketFieldData {
        /// Market scalars
        pub S: u128,
        pub T: u128,
        pub L: u128,
        
        /// Domain weights (basis points)
        pub w_s: u32,
        pub w_t: u32,
        pub w_l: u32,
        pub w_tau: u32,
        
        /// Risk scalers (basis points)
        pub sigma_price: u64,
        pub sigma_rate: u64,
        pub sigma_leverage: u64,
        
        /// TWAPs
        pub twap_0: u128,
        pub twap_1: u128,
    }
    
    /// Pool configuration and metadata
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PoolInfo {
        /// Pool identifier
        pub pool_pubkey: [u8; 32],
        
        /// Token information
        pub token_0: TokenInfo,
        pub token_1: TokenInfo,
        
        /// Pool configuration
        pub fee_rate: u16,
        pub tick_spacing: i16,
        pub max_liquidity_per_tick: u128,
        
        /// Current state
        pub sqrt_price: u128,
        pub tick: i32,
        pub liquidity: u128,
        
        /// Fee tracking
        pub fee_growth_global_0: u128,
        pub fee_growth_global_1: u128,
        pub protocol_fee_rate: u16,
        
        /// Vault accounts
        pub token_vault_0: [u8; 32],
        pub token_vault_1: [u8; 32],
        
        /// Statistics
        pub volume_24h_0: u128,
        pub volume_24h_1: u128,
        pub fees_24h: u64,
        pub liquidity_providers: u32,
    }
    
    /// Token information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TokenInfo {
        pub mint: [u8; 32],
        pub name: String,
        pub symbol: String,
        pub decimals: u8,
        pub logo_uri: Option<String>,
    }
    
    /// Liquidity position information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PositionInfo {
        /// Position identifier
        pub position_pubkey: [u8; 32],
        pub owner: [u8; 32],
        pub pool: [u8; 32],
        
        /// Position range
        pub tick_lower: i32,
        pub tick_upper: i32,
        
        /// Liquidity amount
        pub liquidity: u128,
        
        /// Fee tracking
        pub fee_growth_inside_0: u128,
        pub fee_growth_inside_1: u128,
        pub fees_owed_0: u64,
        pub fees_owed_1: u64,
        
        /// Token amounts
        pub tokens_owed_0: u64,
        pub tokens_owed_1: u64,
        
        /// 3D extension fields for Feels protocol
        pub duration_lock: Option<i64>,
        pub leverage: Option<u64>,
        pub field_position: Option<FieldPositionData>,
        
        /// Position metadata
        pub opened_at: i64,
        pub last_update: i64,
    }
    
    /// 3D field position data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FieldPositionData {
        /// Position in 3D field space
        pub s_position: u128,
        pub t_position: u128,
        pub l_position: u128,
        
        /// Accumulated work
        pub accumulated_work: i128,
        
        /// Last field commitment sequence
        pub last_field_sequence: u64,
    }
    
    /// Trading result types
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SwapResult {
        /// Input/output amounts
        pub amount_in: u64,
        pub amount_out: u64,
        pub is_token_0_to_1: bool,
        
        /// Price impact
        pub price_before: u128,
        pub price_after: u128,
        pub price_impact_bps: u64,
        
        /// Fees
        pub fee_amount: u64,
        pub protocol_fee: u64,
        pub rebate_amount: u64,
        
        /// Work calculation
        pub work_done: i128,
        pub efficiency_score: u8,
        
        /// Route taken
        pub route_segments: u8,
    }
    
    /// Liquidity operation result
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AddLiquidityResult {
        /// Liquidity minted
        pub liquidity: u128,
        
        /// Token amounts
        pub amount_0: u64,
        pub amount_1: u64,
        
        /// Position details
        pub position_id: [u8; 32],
        pub tick_lower: i32,
        pub tick_upper: i32,
        
        /// Fee growth inside
        pub fee_growth_inside_0: u128,
        pub fee_growth_inside_1: u128,
    }
    
    /// Order types for limit orders
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
    pub enum OrderType {
        Limit,
        StopLoss,
        TakeProfit,
        RangeOrder,
    }
    
    /// Order status
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
    pub enum OrderStatus {
        Open,
        PartiallyFilled,
        Filled,
        Cancelled,
        Expired,
    }
}

// Re-export extended types for convenience
#[cfg(feature = "client")]
pub use extended::*;