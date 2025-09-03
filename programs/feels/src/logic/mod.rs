/// Core business logic - Unified order execution with thermodynamic physics
/// The OrderManager uses StateContext for state abstraction while delegating
/// physics calculations to specialized modules

// Unified order and state management with physics integration
pub mod order_manager;  // Unified order manager with thermodynamic physics
pub mod state_access;

// Thermodynamic physics modules (CRITICAL - DO NOT DELETE)
pub mod work_calculation;      // Work-based fee calculations
pub mod instantaneous_fee;     // Fee/rebate determination
pub mod conservation_check;    // Conservation law enforcement
pub mod field_update;          // Market field calculations (S,T,L)
pub mod field_verification;    // Keeper update verification
pub mod leverage_safety;       // Leverage bounds and safety
pub mod fallback_mode;        // Degraded operation handling

// Core AMM logic
pub mod concentrated_liquidity; // Concentrated liquidity math
pub mod tick;                  // Tick management
pub mod position_manager;      // Position lifecycle

// Supporting modules
pub mod event;                 // Event emission
pub mod hook;                  // Hook system
pub mod pool_discovery;        // Pool finding
pub mod order;                 // Legacy order types

// Re-export unified components
pub use order_manager::{
    OrderManager, SwapResult, PositionResult, LiquidityResult, LimitOrderResult,
};

// PhysicsOrderManager functionality now merged into OrderManager

pub use state_access::{
    StateContext, MarketStateAccess, TickStateAccess, PositionStateAccess, BufferStateAccess,
};

// Re-export thermodynamic components
pub use work_calculation::{
    WorkResult, calculate_path_work, work_to_fee,
};

pub use instantaneous_fee::{
    calculate_instantaneous_fee, calculate_rebate_amount,
    InstantaneousFeeParams, InstantaneousFeeResult,
};

pub use conservation_check::{
    ConservationProof, verify_conservation,
};

pub use field_update::{
    update_market_field_data, FieldUpdateContext,
};

pub use field_verification::{
    verify_field_commitment, FieldCommitment, FieldUpdateMode,
};

pub use leverage_safety::{
    LeverageLimits, check_leverage_safety,
};

pub use fallback_mode::{
    FallbackModeManager, OperationalMode,
};

// Re-export other commonly used items
pub use event::*;
pub use hook::*;