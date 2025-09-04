//! # State Architecture for 3D Thermodynamic AMM
//! 
//! Organized by functional groups:
//! 1. **Market**: MarketField (S,T,L), MarketManager, physics calculations
//! 2. **Oracles**: Unified price oracle, volatility, volume, data sources
//! 3. **Fees**: BufferAccount (τ), fee policies, rebate management
//! 4. **Positions**: NFT positions with leverage support
//! 5. **Infrastructure**: Protocol state, security, PDAs
//! 6. **AMM Foundation**: Ticks, tokens, rebasing, hooks

// ============================================================================
// Core Type Re-exports (moved from utils/types.rs)
// ============================================================================

// Re-export Position3D and TradeDimension from feels-core for backward compatibility
pub use feels_core::types::{Position3D, TradeDimension};

// ============================================================================
// 1. CORE MARKET STATE
// ============================================================================

/// Unified market account
pub mod unified_market;

// Re-export unified market
pub use unified_market::{Market, DomainWeights};

// ============================================================================
// 2. ORACLE INFRASTRUCTURE
// ============================================================================

/// Consolidated oracle structures
pub mod oracles;

// Re-export oracle types
pub use oracles::{
    // Core oracle types
    OracleStatus, OracleConfig, VolatilityObservation, PriceObservation,
    // Unified price oracle
    UnifiedOracle,
    // Market data sources
    MarketDataSource, DataSourceType, DataSourceConfig, UnifiedMarketUpdate,
    OraclePriceData,
    // Volatility tracking
    VolatilityOracle,
    // Volume tracking
    VolumeTracker,
    // TWAP submission
    TwapResult, TwapSubmission, TwapProof,
    // Helper functions
    is_stale, calculate_sma, calculate_twap, annualized_to_daily_volatility,
};

// ============================================================================
// 3. FEE MANAGEMENT
// ============================================================================

/// Consolidated fee structures
pub mod fees;

// Re-export fee types
pub use fees::{
    // Buffer account (τ dimension)
    BufferAccount,
    // Fee policy
    FeesPolicy,
    // Constants
    MAX_REBATE_PER_TX_BPS, MAX_REBATE_PER_EPOCH_BPS,
    FEE_EWMA_HALF_LIFE, REBATE_EPOCH_DURATION, BPS_DENOMINATOR,
    // Helper functions
    calculate_rebate, collect_fee, pay_rebate,
    calculate_buffer_growth, verify_buffer_conservation,
};

// ============================================================================
// 4. POSITION MANAGEMENT
// ============================================================================

/// Consolidated position and leverage structures
pub mod position;

// Re-export position types
pub use position::{
    // Position NFT
    TickPositionMetadata,
    // Position manager
    PositionManager,
    // Leverage system
    LeverageParameters, ProtectionCurveType, ProtectionCurveData,
    // Duration tracking
    Duration as PositionDuration, DurationType,
    // Helper functions
    calculate_fee_growth_inside, validate_position,
};

// ============================================================================
// 5. INFRASTRUCTURE MODULES
// ============================================================================

/// PDA derivation functions
pub mod pda; 

/// Protocol configuration
pub mod protocol;

/// Security and reentrancy
pub mod security;

/// Field commitments
pub mod field_commitment; 

/// Keeper registry
pub mod keeper_registry;

/// Numeraire system
pub mod numeraire;

// Re-export infrastructure types
pub use pda::*;
pub use protocol::ProtocolState;
pub use security::{
    ReentrancyStatus, ReentrancyGuard, ScopedReentrancyGuard,
    EmergencyFlags, EmergencyModeParams,
    InitializeEmergencyFlags, ToggleEmergencyMode,
    initialize_emergency_flags, toggle_emergency_mode,
};
pub use field_commitment::*;
pub use keeper_registry::KeeperRegistry;
pub use numeraire::{
    ConversionRate, ProtocolNumeraire, NumeraireCache,
    FallbackRate,
    InitializeNumeraire, UpdateNumeraireCache,
    MIN_TWAP_OBSERVATIONS, RATE_PRECISION,
};

// ============================================================================
// 6. AMM FOUNDATION LAYER
// ============================================================================

/// Tick-based price levels
pub mod tick;

/// Token metadata
pub mod token;

/// Rebase system
pub mod rebase;

/// Hook framework
pub mod hook;

// Re-export AMM types
pub use tick::{
    Tick, TickArray, TickArrayRouter, Tick3D,
    RouterConfig, InitializeRouter, UpdateRouter,
};
pub use token::{TokenMetadata, PositionToken};
pub use rebase::{
    RebaseAccumulator, RebaseCheckpoint,
    apply_position_rebase, create_checkpoint,
    calculate_supply_rate, calculate_borrow_rate, calculate_funding_rate,
    REBASE_INDEX_SCALE, SECONDS_PER_YEAR,
};
pub use hook::{
    HookConfig, HookRegistry, HookContext, HookMessage, HookMessageQueue,
    HookType, HookPermission, EventData, MessageData,
    // Event flags
    EVENT_POOL_INITIALIZED, EVENT_RATE_UPDATED, EVENT_LIQUIDITY_CHANGED,
    EVENT_TICK_CHANGED, EVENT_ORDER_CREATED, EVENT_ORDER_FILLED,
    EVENT_REBASE_APPLIED, EVENT_LEVERAGE_ADJUSTED,
    // Stage flags
    STAGE_VALIDATE, STAGE_PRE_EXECUTE, STAGE_POST_EXECUTE, STAGE_ASYNC,
};

// ============================================================================
// ERROR TYPES
// ============================================================================

// Re-export from main error module
pub use crate::error::{FeelsError, FeelsProtocolError};

// ============================================================================
// DEPRECATED/REMOVED MODULES
// ============================================================================

// The following modules have been consolidated:
// - market_state.rs -> market.rs
// - market_manager.rs -> market.rs
// - market_physics.rs -> market.rs
// - oracle_types.rs -> oracles.rs
// - oracle_field.rs -> oracles.rs
// - oracle_price.rs -> oracles.rs
// - oracle_submission.rs -> oracles.rs
// - oracle_volatility.rs -> oracles.rs
// - oracle_volume.rs -> oracles.rs
// - fees_policy.rs -> fees.rs
// - buffer.rs -> fees.rs
// - leverage.rs -> position.rs
// - position.rs -> position.rs (updated)