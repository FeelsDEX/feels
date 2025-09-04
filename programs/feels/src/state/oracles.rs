//! # Oracle State - Consolidated Oracle-Related Structures
//! 
//! This module consolidates all oracle-related state structures:
//! - Unified price oracle with internal TWAP and external feeds
//! - Market data source management (keeper-based and oracle-based)
//! - Volatility tracking and calculations
//! - Volume tracking for lending/borrowing
//! - Off-chain TWAP submission handling
//! - Shared oracle types and utilities

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::utils::bitmap::u8_bitmap;

// ============================================================================
// Shared Oracle Types and Constants
// ============================================================================

/// Unified oracle status enum used by all oracle types
#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum OracleStatus {
    /// Oracle is not initialized or disabled
    Inactive = 0,
    /// Oracle is active and receiving updates
    Active = 1,
    /// Oracle data is stale (no recent updates)
    Stale = 2,
    /// Oracle is temporarily offline
    Offline = 3,
}

impl Default for OracleStatus {
    fn default() -> Self {
        OracleStatus::Inactive
    }
}

/// A single volatility observation containing log return data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VolatilityObservation {
    /// Unix timestamp when this observation was recorded
    pub timestamp: i64,
    /// Squared log return scaled by 10^6 for precision
    pub log_return_squared: u32,
    /// Padding to align to 8-byte boundary
    pub _padding: u32,
}

impl VolatilityObservation {
    pub const SIZE: usize = 8 + 4 + 4; // timestamp (i64) + log_return_squared (u32) + padding (u32)
}

/// A single price observation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct PriceObservation {
    /// Block timestamp
    pub timestamp: i64,
    /// Price at this observation (Q64)
    pub price: u128,
    /// Cumulative price for TWAP calculation
    pub cumulative: u128,
}

/// Common oracle configuration parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct OracleConfig {
    /// Maximum allowed staleness in seconds
    pub max_staleness: u32,
    /// Minimum observations required for valid TWAP
    pub min_observations: u16,
    /// Update interval in seconds
    pub update_interval: u16,
}

// ============================================================================
// Unified Price Oracle
// ============================================================================

/// Number of price observations to store
pub const PRICE_OBSERVATION_COUNT: usize = 720; // 12 hours at 1-minute intervals

/// **UnifiedOracle - Comprehensive Price Oracle System**
/// 
/// This oracle combines:
/// 1. **Internal TWAP**: Based on actual pool trades with circular buffer storage
/// 2. **External Feeds**: USD price data from Pyth/Switchboard with confidence intervals
/// 3. **Fallback Logic**: Automatic switching between sources based on staleness
/// 
/// The oracle provides multiple TWAP windows (1min, 5min, 15min, 1hr) calculated
/// from on-chain observations, plus external USD pricing for stablecoin references.
#[account(zero_copy)]
#[repr(C)]
pub struct UnifiedOracle {
    /// Pool address this oracle is tracking
    pub pool: Pubkey,
    
    // ========== Internal TWAP Data ==========
    
    /// Circular buffer of price observations
    pub observations: [PriceObservation; PRICE_OBSERVATION_COUNT],
    
    /// Current index in circular buffer
    pub observation_index: u16,
    
    /// Total observations written (for TWAP calculation)
    pub observation_count: u16,
    
    /// Cumulative price for token A (Q128.128)
    pub cumulative_price_a: u128,
    
    /// Cumulative price for token B (Q128.128)
    pub cumulative_price_b: u128,
    
    // ========== External Price Feed Data ==========
    
    /// Token A USD price from external oracle (Q64)
    pub token_a_usd_price: u128,
    
    /// Token A price confidence interval (Q64)
    pub token_a_confidence: u64,
    
    /// Token A price last update
    pub token_a_last_update: i64,
    
    /// Token B USD price from external oracle (Q64)
    pub token_b_usd_price: u128,
    
    /// Token B price confidence interval (Q64)
    pub token_b_confidence: u64,
    
    /// Token B price last update
    pub token_b_last_update: i64,
    
    // ========== TWAP Calculations ==========
    
    /// 1-minute TWAP for token A
    pub twap_1min_a: u128,
    
    /// 5-minute TWAP for token A
    pub twap_5min_a: u128,
    
    /// 15-minute TWAP for token A
    pub twap_15min_a: u128,
    
    /// 1-hour TWAP for token A
    pub twap_1hr_a: u128,
    
    /// 1-minute TWAP for token B
    pub twap_1min_b: u128,
    
    /// 5-minute TWAP for token B
    pub twap_5min_b: u128,
    
    /// 15-minute TWAP for token B
    pub twap_15min_b: u128,
    
    /// 1-hour TWAP for token B
    pub twap_1hr_b: u128,
    
    // ========== Metadata ==========
    
    /// Last observation timestamp
    pub last_observation_time: i64,
    
    /// Oracle status
    pub status: OracleStatus,
    
    /// Configuration flags
    pub flags: u8,
    
    /// Reserved padding
    pub _padding: [u8; 6],
}

impl UnifiedOracle {
    /// Check if oracle has stale data
    pub fn is_stale(&self, current_time: i64, max_staleness: i64) -> bool {
        current_time - self.last_observation_time > max_staleness
    }
    
    /// Get the safest TWAP (1hr for stability)
    pub fn get_safe_twap_a(&self) -> u128 {
        self.twap_1hr_a
    }
    
    /// Get the safest TWAP (1hr for stability)
    pub fn get_safe_twap_b(&self) -> u128 {
        self.twap_1hr_b
    }
}

// ============================================================================
// Market Data Source Management
// ============================================================================

/// Type of data source for market updates
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum DataSourceType {
    /// Keeper-submitted field commitments (Mode B)
    Keeper,
    /// Oracle price feeds
    Oracle,
    /// Hybrid approach using both
    Hybrid,
}

/// Configuration for data source
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DataSourceConfig {
    /// Primary data source type
    pub source_type: DataSourceType,
    
    /// Keeper registry for Mode B
    pub keeper_registry: Option<Pubkey>,
    
    /// Oracle accounts for price feeds
    pub oracle_accounts: Vec<Pubkey>,
    
    /// Maximum staleness allowed (seconds)
    pub max_staleness: u32,
    
    /// Minimum confirmations required
    pub min_confirmations: u8,
}

/// **MarketDataSource - Unified Market Data Management**
/// 
/// This account manages all data sources for market field updates, supporting:
/// - Mode B keeper field commitments with local approximations
/// - Direct oracle price feeds
/// - Hybrid approaches with fallback logic
#[account]
#[derive(Default)]
pub struct MarketDataSource {
    /// Market this data source belongs to
    pub market: Pubkey,
    
    /// Data source configuration
    pub config: DataSourceConfig,
    
    /// Latest keeper field commitment
    pub latest_commitment: Option<crate::state::field_commitment::FieldCommitment>,
    
    /// Last oracle update
    pub last_oracle_update: i64,
    
    /// Update sequence number
    pub sequence: u64,
    
    /// Authority that can modify config
    pub authority: Pubkey,
    
    /// Reserved space
    pub _reserved: [u8; 128],
}

impl MarketDataSource {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        200 + // config (estimated)
        300 + // latest_commitment (estimated)
        8 + 8 + // timestamps
        32 + // authority
        128; // reserved
}

/// Unified market update that can come from different sources
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UnifiedMarketUpdate {
    /// Update source type
    pub source: DataSourceType,
    
    /// Keeper field commitment (if from keeper)
    pub field_commitment: Option<crate::state::field_commitment::FieldCommitment>,
    
    /// Oracle price data (if from oracle)
    pub oracle_data: Option<OraclePriceData>,
    
    /// Update timestamp
    pub timestamp: i64,
    
    /// Signature for verification
    pub signature: Option<[u8; 64]>,
}

/// Oracle price data for market updates
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct OraclePriceData {
    /// Spot price
    pub spot_price: u128,
    
    /// Implied volatility
    pub implied_volatility: u64,
    
    /// Confidence interval
    pub confidence: u64,
}

// ============================================================================
// Volatility Oracle
// ============================================================================

/// Maximum number of volatility observations to store
pub const MAX_VOLATILITY_OBSERVATIONS: usize = 24; // 24 hours of hourly data

/// **VolatilityOracle - External Volatility Data Integration**
/// 
/// Integrates with external oracle providers (Pyth, Switchboard) to track:
/// - Current implied volatility
/// - Historical volatility over different timeframes
/// - Volatility term structure for risk management
#[account]
pub struct VolatilityOracle {
    /// Market this volatility data is for
    pub market: Pubkey,
    
    /// Oracle provider (Pyth, Switchboard, etc)
    pub provider: Pubkey,
    
    /// Current volatility (basis points)
    pub current_volatility: u64,
    
    /// 24h volatility (basis points)
    pub volatility_24h: u64,
    
    /// 7d volatility (basis points)
    pub volatility_7d: u64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Recent observations for validation
    pub observations: Vec<VolatilityObservation>,
    
    /// Oracle status
    pub status: OracleStatus,
    
    /// Reserved space
    pub _reserved: [u8; 64],
}

impl VolatilityOracle {
    pub const LEN: usize = 8 + // discriminator
        32 + 32 + // pubkeys
        8 + 8 + 8 + // volatilities
        8 + // timestamp
        4 + (MAX_VOLATILITY_OBSERVATIONS * VolatilityObservation::SIZE) + // observations
        1 + // status
        64; // reserved
    
    /// Convert to VolatilityObservation for compatibility
    pub fn to_observation(&self) -> VolatilityObservation {
        VolatilityObservation {
            timestamp: self.last_update,
            log_return_squared: (self.current_volatility * self.current_volatility / 100) as u32,
            _padding: 0,
        }
    }
    
    /// Check if volatility is within acceptable range
    pub fn is_valid(&self) -> bool {
        self.status == OracleStatus::Active &&
        self.current_volatility > 0 &&
        self.current_volatility < 50000 // Max 500% volatility
    }
}

// ============================================================================
// Volume Tracking
// ============================================================================

/// **VolumeTracker - Protocol Volume Statistics**
/// 
/// Tracks lending and borrowing volumes for:
/// - Fee tier optimization
/// - Utilization rate calculations
/// - Market depth analysis
#[account]
pub struct VolumeTracker {
    /// Pool being tracked
    pub pool: Pubkey,
    
    /// Token being tracked
    pub token_mint: Pubkey,
    
    // ========== Current Epoch ==========
    
    /// Current epoch start time
    pub epoch_start: i64,
    
    /// Lending volume this epoch
    pub epoch_lending_volume: u128,
    
    /// Borrowing volume this epoch
    pub epoch_borrowing_volume: u128,
    
    // ========== All Time ==========
    
    /// Total lending volume
    pub total_lending_volume: u128,
    
    /// Total borrowing volume
    pub total_borrowing_volume: u128,
    
    // ========== Moving Averages ==========
    
    /// 24h lending volume
    pub lending_volume_24h: u128,
    
    /// 24h borrowing volume
    pub borrowing_volume_24h: u128,
    
    /// Last update time
    pub last_update: i64,
    
    /// Reserved space
    pub _reserved: [u8; 64],
}

impl VolumeTracker {
    pub const LEN: usize = 8 + // discriminator
        32 + 32 + // pubkeys
        8 + 16 + 16 + // epoch data
        16 + 16 + // totals
        16 + 16 + // 24h volumes
        8 + // timestamp
        64; // reserved
    
    /// Calculate current utilization rate
    pub fn utilization_rate(&self) -> u64 {
        if self.total_lending_volume == 0 {
            return 0;
        }
        
        let utilization = (self.total_borrowing_volume * 10000) / self.total_lending_volume;
        utilization.min(10000) as u64
    }
}

// ============================================================================
// Off-chain TWAP Submission
// ============================================================================

/// Result of off-chain TWAP calculation
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TwapResult {
    /// Start timestamp of TWAP window
    pub start_time: i64,
    
    /// End timestamp of TWAP window
    pub end_time: i64,
    
    /// Calculated TWAP value (Q64 fixed-point)
    pub twap_value: u128,
    
    /// Number of observations used
    pub observation_count: u32,
    
    /// Standard deviation (for confidence)
    pub std_deviation: u64,
    
    /// Computation timestamp
    pub computed_at: i64,
}

/// Complete TWAP submission from keeper
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TwapSubmission {
    /// Pool identifier
    pub pool: Pubkey,
    
    /// Token pair (0 = token0/token1, 1 = token1/token0)
    pub token_pair: u8,
    
    /// TWAP calculation result
    pub twap_result: TwapResult,
    
    /// Optional proof data
    pub proof: Option<TwapProof>,
}

/// Proof data for TWAP verification
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TwapProof {
    /// Merkle root of observations
    pub observation_root: [u8; 32],
    
    /// Aggregated signature
    pub signature: [u8; 64],
    
    /// Additional metadata
    pub metadata: Vec<u8>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a timestamp is stale
pub fn is_stale(timestamp: i64, current_time: i64, max_staleness: i64) -> bool {
    current_time - timestamp > max_staleness
}

/// Calculate simple moving average
pub fn calculate_sma(observations: &[u64], window: usize) -> u64 {
    if observations.is_empty() || window == 0 {
        return 0;
    }
    
    let actual_window = window.min(observations.len());
    let sum: u64 = observations[observations.len() - actual_window..]
        .iter()
        .sum();
    
    sum / actual_window as u64
}

/// Calculate time-weighted average price
pub fn calculate_twap(
    observations: &[PriceObservation],
    start_time: i64,
    end_time: i64,
) -> Result<u128> {
    if observations.is_empty() {
        return Err(FeelsProtocolError::InsufficientData.into());
    }
    
    let mut weighted_sum = 0u128;
    let mut total_weight = 0u64;
    
    for window in observations.windows(2) {
        let obs1 = &window[0];
        let obs2 = &window[1];
        
        // Skip if outside time range
        if obs2.timestamp < start_time || obs1.timestamp > end_time {
            continue;
        }
        
        // Calculate time weight
        let weight = (obs2.timestamp - obs1.timestamp).max(0) as u64;
        
        // Add weighted price
        weighted_sum = weighted_sum.saturating_add(obs1.price.saturating_mul(weight as u128));
        total_weight = total_weight.saturating_add(weight);
    }
    
    if total_weight == 0 {
        return Err(FeelsProtocolError::InsufficientData.into());
    }
    
    Ok(weighted_sum / total_weight as u128)
}

/// Convert volatility from annual to daily
pub fn annualized_to_daily_volatility(annual_vol_bps: u64) -> u64 {
    // Daily vol = Annual vol / sqrt(365)
    // sqrt(365) ≈ 19.1
    annual_vol_bps * 10 / 191
}