/// Business logic module containing core AMM functionality separated from instruction handlers.
/// Organizes complex calculations and state management into focused modules by domain.
/// Tick logic has been consolidated into tick.rs. Each module handles a specific
/// domain: pools, tick positions, events, concentrated liquidity, tick management (consolidated), swaps, hooks.

// ============================================================================
// Module Declarations  
// ============================================================================

pub mod concentrated_liquidity;
pub mod event;
pub mod fee_manager;
pub mod hook;
pub mod order;
pub mod pool;
pub mod tick;
pub mod tick_position;
pub mod volatility_manager;

// ============================================================================
// Re-exports
// ============================================================================

pub use concentrated_liquidity::*;
pub use event::*;
pub use fee_manager::*;
pub use hook::*;
pub use order::*;
pub use tick::*;
pub use volatility_manager::*;

// Export order components explicitly
pub use order::{
    OrderManager, OrderState, OrderStep, OrderRoute, RoutingLogic,
    SecureOrderManager, OracleTwapWindow, get_oracle_from_remaining, 
    get_oracle_data_from_remaining,
    // 3D Order components
    OrderManager3D, DimensionWeights, Liquidity3D, PriceImpact3D, OrderType
};
