/// State module organizing all on-chain account structures and data types.
/// Includes pool state, position NFTs, tick arrays for liquidity, token metadata,
/// protocol configuration, error definitions, and infrastructure for future features
/// like hooks and tick routing optimization. All accounts use efficient serialization for optimal Solana performance.
pub mod error;
pub mod hook;
pub mod pool;
pub mod pool_metrics;
pub mod pool_hooks;
pub mod pool_rebase;
// New simplified structures
pub mod pool_simplified;
pub mod pool_metrics_consolidated;
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

// Metrics
pub mod metrics_price; // Price oracle and TWAP
pub mod metrics_volume; // Trading volume tracking
pub mod metrics_volatility; // High-frequency volatility tracking
pub mod metrics_lending; // Lending metrics (flash loans + utilization)

// ============================================================================
// Re-exports
// ============================================================================

// Error types
pub use error::FeelsProtocolError;

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

// Core pool types
pub use pool::{Pool, PoolKey};
pub use pool_metrics::PoolMetrics;
pub use pool_hooks::PoolHooks;
pub use pool_rebase::PoolRebase;

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

// Price metrics
pub use metrics_price::{
    PriceOracle, PriceTransition, update_price_oracle,
    calculate_price_twap,
};

// Volume metrics
pub use metrics_volume::{
    VolumeTracker, VolumeWindow, update_volume_tracker,
    get_current_volume,
};

// Volatility metrics
pub use metrics_volatility::{
    VolatilityCalculator, VolatilityWindow, update_volatility,
    calculate_realized_volatility,
};

// Lending metrics
pub use metrics_lending::{
    LendingMetrics, FlashLoanTracker, update_lending_metrics,
    calculate_utilization_rate,
};

// ============================================================================
// Market Physics
// ============================================================================

pub mod buffer;
pub mod market_field;
pub mod numeraire;
pub mod twap_oracle;

// Buffer account
pub use buffer::{
    BufferAccount, calculate_rebate,
    InitializeBuffer, UpdateBufferParams,
    MAX_REBATE_PER_TX_BPS, MAX_REBATE_PER_EPOCH_BPS,
    FEE_EWMA_HALF_LIFE, BPS_DENOMINATOR as BUFFER_BPS_DENOMINATOR,
};

// Market field commitment
pub use market_field::{
    MarketField, WorkCalculationParams, FieldUpdateParams,
    calculate_work_closed_form,
};

// TWAP Oracle
pub use twap_oracle::{
    TwapOracle, PriceObservation, InitializeTwapParams,
    OBSERVATION_BUFFER_SIZE, DEFAULT_TWAP_WINDOW,
};

// Numeraire system
pub use numeraire::{
    ConversionRate, ProtocolNumeraire, NumeraireCache,
    PriceObservation, FallbackRate, calculate_geometric_twap,
    InitializeNumeraire, UpdateNumeraireCache,
    DEFAULT_TWAP_WINDOW, MIN_TWAP_OBSERVATIONS, RATE_PRECISION,
};
