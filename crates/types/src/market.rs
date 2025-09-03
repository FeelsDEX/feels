/// Market state and related types for SDK and keeper components

use anchor_lang::prelude::*;
use serde::{Deserialize, Serialize};
use crate::{constants::*, field::FieldCommitmentData};

// ============================================================================
// Market State
// ============================================================================

/// Complete market state data used for field computation and client operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketState {
    /// Market identifier
    pub market_pubkey: Pubkey,
    
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
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    
    /// Current field commitment (if available)
    pub field_commitment: Option<FieldCommitmentData>,
    
    /// Current base fee in basis points (optional, set by hysteresis controller)
    pub base_fee_bps: Option<u64>,
}

/// Simplified market field data for work calculations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// ============================================================================
// Pool Information
// ============================================================================

/// Pool configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolInfo {
    /// Pool identifier
    pub pool_pubkey: Pubkey,
    
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
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    
    /// Statistics
    pub volume_24h_0: u128,
    pub volume_24h_1: u128,
    pub fees_24h: u64,
    pub liquidity_providers: u32,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub logo_uri: Option<String>,
}

// ============================================================================
// Position Information
// ============================================================================

/// Liquidity position information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionInfo {
    /// Position identifier
    pub position_pubkey: Pubkey,
    pub owner: Pubkey,
    pub pool: Pubkey,
    
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
    pub token_amount_0: u64,
    pub token_amount_1: u64,
    
    /// Position metadata
    pub created_at: i64,
    pub last_update: i64,
    
    /// 3D extension (if applicable)
    pub duration_lock: Option<DurationInfo>,
    pub leverage: Option<LeverageInfo>,
}

/// Duration lock information for time dimension
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DurationInfo {
    pub duration_type: DurationType,
    pub lock_start: i64,
    pub lock_end: i64,
    pub yield_multiplier: u32,  // basis points
}

/// Types of duration locks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DurationType {
    None,
    Short,      // < 1 day
    Medium,     // 1-7 days  
    Long,       // 7-30 days
    Extended,   // > 30 days
}

/// Leverage information for leverage dimension
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeverageInfo {
    pub leverage_ratio: u64,      // e.g., 2_000 = 2.0x
    pub margin_requirement: u64,   // basis points
    pub liquidation_threshold: u128,
    pub funding_rate: i64,         // Can be negative (paid) or positive (received)
}

// ============================================================================
// Trading Types
// ============================================================================

/// Swap operation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapResult {
    pub signature: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_impact: u64,      // basis points
    pub sqrt_price_after: u128,
    pub tick_after: i32,
    pub work_computed: i128,    // Work value for the swap
}

/// Add liquidity operation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddLiquidityResult {
    pub signature: String,
    pub position: Pubkey,
    pub liquidity_amount: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower: i32,
    pub tick_upper: i32,
}

/// Remove liquidity operation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoveLiquidityResult {
    pub signature: String,
    pub amount_0: u64,
    pub amount_1: u64,
    pub fees_0: u64,
    pub fees_1: u64,
}

/// Create pool operation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreatePoolResult {
    pub signature: String,
    pub pool: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub initial_sqrt_price: u128,
    pub initial_tick: i32,
}

// ============================================================================
// Route and Path Types
// ============================================================================

/// Trading route through hub-constrained pools (max 2 hops)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradingRoute {
    /// Pools in the route (1 or 2, always including FeelsSOL)
    pub pools: Vec<Pubkey>,
    /// Token being traded in
    pub token_in: Pubkey,
    /// Token being traded out
    pub token_out: Pubkey,
    /// Input amount
    pub amount_in: u64,
    /// Expected output amount
    pub amount_out: u64,
    /// Total fees across all hops
    pub total_fees: u64,
}

/// Single hop in a hub-constrained route
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteStep {
    /// Pool for this hop (must include FeelsSOL as one side)
    pub pool: Pubkey,
    /// Tokens for this hop
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    /// Amounts for this hop
    pub amount_in: u64,
    pub amount_out: u64,
}

// ============================================================================
// Account Data Parsing
// ============================================================================

/// Raw account data parsers for different account types
pub struct AccountParser;

impl AccountParser {
    /// Parse MarketField account data
    pub fn parse_market_field(data: &[u8]) -> std::result::Result<MarketFieldData, String> {
        if data.len() < DISCRIMINATOR_SIZE + 200 {
            return Err("Invalid market field account data length".to_string());
        }
        
        let mut offset = DISCRIMINATOR_SIZE;
        
        // Parse scalars (3 * 16 bytes)
        let S = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid S scalar")?
        );
        offset += 16;
        
        let T = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid T scalar")?
        );
        offset += 16;
        
        let L = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid L scalar")?
        );
        offset += 16;
        
        // Parse domain weights (4 * 4 bytes)
        let w_s = u32::from_le_bytes(
            data[offset..offset+4].try_into()
                .map_err(|_| "Invalid w_s")?
        );
        offset += 4;
        
        let w_t = u32::from_le_bytes(
            data[offset..offset+4].try_into()
                .map_err(|_| "Invalid w_t")?
        );
        offset += 4;
        
        let w_l = u32::from_le_bytes(
            data[offset..offset+4].try_into()
                .map_err(|_| "Invalid w_l")?
        );
        offset += 4;
        
        let w_tau = u32::from_le_bytes(
            data[offset..offset+4].try_into()
                .map_err(|_| "Invalid w_tau")?
        );
        offset += 4;
        
        // Parse risk parameters (3 * 8 bytes)
        let sigma_price = u64::from_le_bytes(
            data[offset..offset+8].try_into()
                .map_err(|_| "Invalid sigma_price")?
        );
        offset += 8;
        
        let sigma_rate = u64::from_le_bytes(
            data[offset..offset+8].try_into()
                .map_err(|_| "Invalid sigma_rate")?
        );
        offset += 8;
        
        let sigma_leverage = u64::from_le_bytes(
            data[offset..offset+8].try_into()
                .map_err(|_| "Invalid sigma_leverage")?
        );
        offset += 8;
        
        // Parse TWAPs (2 * 16 bytes)
        let twap_0 = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid twap_0")?
        );
        offset += 16;
        
        let twap_1 = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid twap_1")?
        );
        
        Ok(MarketFieldData {
            S, T, L,
            w_s, w_t, w_l, w_tau,
            sigma_price, sigma_rate, sigma_leverage,
            twap_0, twap_1,
        })
    }
    
    /// Parse basic pool information from account data
    pub fn parse_pool_basic(data: &[u8]) -> std::result::Result<(u128, i32, u128), String> {
        if data.len() < DISCRIMINATOR_SIZE + 48 {
            return Err("Invalid pool account data length".to_string());
        }
        
        let mut offset = DISCRIMINATOR_SIZE;
        
        // Parse sqrt_price (16 bytes)
        let sqrt_price = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid sqrt_price")?
        );
        offset += 16;
        
        // Parse current_tick (4 bytes)  
        let current_tick = i32::from_le_bytes(
            data[offset..offset+4].try_into()
                .map_err(|_| "Invalid current_tick")?
        );
        offset += 4;
        
        // Skip some fields, parse liquidity (16 bytes)
        offset += 12; // Skip padding/other fields
        let liquidity = u128::from_le_bytes(
            data[offset..offset+16].try_into()
                .map_err(|_| "Invalid liquidity")?
        );
        
        Ok((sqrt_price, current_tick, liquidity))
    }
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl FieldCommitmentData {
    /// Convert to MarketFieldData for work calculations
    pub fn to_market_field_data(&self) -> MarketFieldData {
        MarketFieldData {
            S: self.S,
            T: self.T,
            L: self.L,
            w_s: self.w_s,
            w_t: self.w_t,
            w_l: self.w_l,
            w_tau: self.w_tau,
            sigma_price: self.sigma_price,
            sigma_rate: self.sigma_rate,
            sigma_leverage: self.sigma_leverage,
            twap_0: self.twap_0,
            twap_1: self.twap_1,
        }
    }
}

impl MarketState {
    /// Check if market state is fresh enough for calculations
    pub fn is_fresh(&self, max_age_seconds: i64) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        current_time - self.last_update_ts <= max_age_seconds
    }
    
    /// Calculate approximate market cap in token 0 units
    pub fn market_cap_0(&self) -> u128 {
        // Simplified calculation: sqrt_price^2 * total_liquidity
        let price_0_per_1 = (self.current_sqrt_price * self.current_sqrt_price) / Q64;
        self.liquidity * price_0_per_1 / Q64
    }
    
    /// Get 24h volume in USD equivalent (requires external price data)
    pub fn volume_24h_usd(&self, token_0_price_usd: f64, token_1_price_usd: f64) -> f64 {
        let volume_0_usd = (self.total_volume_0 as f64 / 10_f64.powi(self.token_0_decimals as i32)) * token_0_price_usd;
        let volume_1_usd = (self.total_volume_1 as f64 / 10_f64.powi(self.token_1_decimals as i32)) * token_1_price_usd;
        volume_0_usd + volume_1_usd
    }
}

impl Default for DurationType {
    fn default() -> Self {
        DurationType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_field_data_conversion() {
        let commitment = FieldCommitmentData::new(
            Q64, Q64, Q64,
            3333, 3333, 3334, 0,
            5000, 5000,
            1000, 500, 1500,
            Q64, Q64,
        ).unwrap();
        
        let field_data = commitment.to_market_field_data();
        assert_eq!(field_data.S, Q64);
        assert_eq!(field_data.w_s, 3333);
    }

    #[test]
    fn test_market_state_freshness() {
        let mut state = MarketState {
            market_pubkey: Pubkey::default(),
            current_sqrt_price: Q64,
            liquidity: 1000000,
            tick_current: 0,
            fee_growth_global_0: 0,
            fee_growth_global_1: 0,
            protocol_fees_0: 0,
            protocol_fees_1: 0,
            twap_0: Q64,
            twap_1: Q64,
            last_update_ts: 0,
            total_volume_0: 0,
            total_volume_1: 0,
            swap_count: 0,
            token_0_mint: Pubkey::default(),
            token_1_mint: Pubkey::default(),
            token_0_decimals: 6,
            token_1_decimals: 6,
            field_commitment: None,
            base_fee_bps: None,
        };
        
        // Fresh state (just updated)
        state.last_update_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(state.is_fresh(300)); // 5 minutes
        
        // Stale state
        state.last_update_ts = 0;
        assert!(!state.is_fresh(300));
    }

    #[test]
    fn test_account_parser() {
        // Test with minimal valid data
        let mut data = vec![0u8; 200];
        
        // Set discriminator
        data[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        
        // Set S scalar to Q64
        data[8..24].copy_from_slice(&Q64.to_le_bytes());
        
        // Set other required fields...
        let result = AccountParser::parse_market_field(&data);
        assert!(result.is_ok());
        
        let field_data = result.unwrap();
        assert_eq!(field_data.S, Q64);
    }
}