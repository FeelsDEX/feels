/// Core business logic for markets, orders, ticks, and positions.
/// These modules handle the fundamental operations of the AMM.

pub mod order;
pub mod tick;
pub mod position_manager;
pub mod event;
pub mod hook;
pub mod fallback_mode;
pub mod field_update;
pub mod field_verification;
pub mod concentrated_liquidity;
pub mod instantaneous_fee;
pub mod leverage_safety;
pub mod conservation_check;
pub mod work_calculation;
pub mod pool_discovery;

// Re-export commonly used items
pub use order::{
    OrderManager, OrderState, OrderStep, OrderRoute, RoutingLogic,
    SecureOrderManager, OracleTwapWindow, get_oracle_from_remaining,
    get_oracle_data_from_remaining,
    // 3D Order components
    OrderManager3D, DimensionWeights, Liquidity3D, PriceImpact3D, OrderType,
};

pub use tick::{
    TickManager,
};

pub use position_manager::{
    create_position_with_rebase,
};

pub use fallback_mode::{
    FallbackModeManager, FallbackContext, FallbackEvaluation,
    OperationalMode, EmergencyActions,
    should_use_fallback_mode, get_fee_parameters, log_fallback_status,
};

pub use work_calculation::{
    WorkResult, Position3D, PathSegment,
    calculate_path_work, calculate_work_from_field,
    work_to_fee, work_qualifies_for_rebate,
};

pub use conservation_check::{
    ConservationProof, ConservationCheckResult,
    BufferConservationContext, BufferConservationProof,
    RebaseOperationType, DomainActivity,
    verify_conservation, verify_conservation_with_buffer,
    calculate_buffer_fee_share, build_buffer_conservation_proof,
};

// Path integration removed - now using keeper analytics

pub use pool_discovery::{
    PoolDiscovery, PoolInfo,
};