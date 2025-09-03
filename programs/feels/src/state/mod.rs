/// State module organizing all on-chain account structures for the market physics model.
/// The protocol is built around the unified physics engine with MarketField, BufferAccount,
/// and gradient-based fee calculations. All legacy Pool-based architecture has been removed.
pub mod hook;
pub mod protocol;
pub mod tick; // Now includes TickArrayRouter
pub mod position;
pub mod token;

// Core Types
pub mod duration; // Duration dimension for 3D model
// Fee system removed - now handled through GradientCache and BufferAccount
pub mod leverage; // Continuous leverage system
pub mod rebase; // Virtual rebasing with lazy evaluation
// Vault removed - position management handled through unified order system
pub mod reentrancy; // Reentrancy protection
pub mod volatility; // Volatility observation types

// ============================================================================
// Re-exports
// ============================================================================

// Error types - re-export from main error module
pub use crate::error::{FeelsError, FeelsProtocolError};

// Hook system
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

// Market physics types are re-exported below

// Protocol configuration
pub use protocol::ProtocolState;

// Reentrancy protection
pub use reentrancy::{ReentrancyStatus, ReentrancyGuard, ScopedReentrancyGuard};

// Tick management
pub use tick::{
    Tick, TickArray, TickArrayRouter, Tick3D,
    RouterConfig, InitializeRouter, UpdateRouter,
};

// Position management
pub use position::TickPositionMetadata;

// Token metadata
pub use token::TokenMetadata;

// Duration types
pub use duration::{Duration, DurationConfig};

// Leverage system
pub use leverage::{
    ProtectionCurveType, ProtectionCurveData, LeverageParameters,
    RiskProfile, LeverageStatistics,
};

// Rebase system
pub use rebase::{
    RebaseAccumulator, RebaseCheckpoint,
    apply_position_rebase, create_checkpoint,
    calculate_supply_rate, calculate_borrow_rate, calculate_funding_rate,
    REBASE_INDEX_SCALE, SECONDS_PER_YEAR, BPS_DENOMINATOR,
};

// Metrics are now consolidated in PoolMetricsConsolidated
// Use the consolidated metrics account for all metric operations

// ============================================================================
// Market Physics
// ============================================================================

pub mod buffer;
pub mod field_commitment;
pub mod fees_policy;
pub mod market_data_source;
pub mod market_field;
pub mod market_field_work;
pub mod market_manager;
pub mod market_state;
pub mod numeraire;
pub mod numeraire_twap;
pub mod twap_oracle;
pub mod keeper_registry;
pub mod volatility_oracle;
pub mod token_price_oracle;
pub mod volume_tracker;
pub mod emergency_flags;

// Buffer account
pub use buffer::{
    BufferAccount, calculate_rebate,
    InitializeBuffer, UpdateBufferParams,
    MAX_REBATE_PER_TX_BPS, MAX_REBATE_PER_EPOCH_BPS,
    FEE_EWMA_HALF_LIFE, BPS_DENOMINATOR as BUFFER_BPS_DENOMINATOR,
};

// Fees policy
pub use fees_policy::{
    FeesPolicy, PoolStatus,
    calculate_combined_stress, qualifies_for_rebate,
};

// Market data source (unified keeper + oracle interface)
pub use market_data_source::{
    MarketDataSource, DataSourceConfig, KeeperConfig, OracleConfig,
    UnifiedMarketUpdate, FieldCommitmentData, OraclePriceData,
    verify_market_update,
    // Constants for data source types
    DATA_SOURCE_TYPE_KEEPER, DATA_SOURCE_TYPE_ORACLE, DATA_SOURCE_TYPE_HYBRID,
    // Constants for price status
    PRICE_STATUS_VALID, PRICE_STATUS_STALE, PRICE_STATUS_LOW_CONFIDENCE, PRICE_STATUS_OFFLINE,
};

// Field commitment (keeper-provided compact market state)
// Re-export all items from field_commitment module
pub use field_commitment::*;

// Market field commitment
pub use market_field::{
    MarketField, WorkCalculationParams, FieldUpdateParams,
    // Deprecated: use market_field_work module instead
    calculate_work_closed_form,
};

// Market field work calculation
pub use market_field_work::{
    WorkCalculationMethod, calculate_work_for_market,
    FieldCommitmentWithMethod, LocalQuadraticCoeffs,
};

// Market manager (AMM compatibility layer)
pub use market_manager::{
    MarketManager, MarketView,
    // position_to_tick, tick_to_position, // Commented out until implemented
};

// TWAP Oracle
pub use twap_oracle::{
    TwapOracle, PriceObservation, InitializeTwapParams,
    OBSERVATION_BUFFER_SIZE, DEFAULT_TWAP_WINDOW,
};

// Volatility tracking
pub use volatility::VolatilityObservation;

// Keeper registry
pub use keeper_registry::KeeperRegistry;

// Volatility oracle
pub use volatility_oracle::{
    VolatilityOracle, OracleStatus, VolatilityTimeframe,
    InitializeVolatilityOracle, initialize_volatility_oracle,
};

// Token price oracle
pub use token_price_oracle::{
    TokenPriceOracle,
    InitializeTokenPriceOracle, initialize_token_price_oracle,
};

// Volume tracker
pub use volume_tracker::{
    VolumeTracker,
    InitializeVolumeTracker, initialize_volume_tracker,
};

// Emergency flags
pub use emergency_flags::{
    EmergencyFlags, EmergencyModeParams,
    InitializeEmergencyFlags, ToggleEmergencyMode,
    initialize_emergency_flags, toggle_emergency_mode,
};

// Market state structures
pub use market_state::{
    MarketState, DomainWeights, PoolSimplified,
};


// Numeraire system
pub use numeraire::{
    ConversionRate, ProtocolNumeraire, NumeraireCache,
    FallbackRate,
    InitializeNumeraire, UpdateNumeraireCache,
    MIN_TWAP_OBSERVATIONS, RATE_PRECISION,
    // Note: calculate_geometric_twap is off-chain only
};

// Numeraire TWAP submission
pub use numeraire_twap::{
    TwapResult, TwapSubmission, TwapProof, MethodParams,
    SubmitTwap, submit_twap_handler, validate_twap_submission,
    get_fallback_rate,
};
