/// Core business logic for pools, orders, ticks, and positions.
/// These modules handle the fundamental operations of the AMM.

pub mod pool;
pub mod order;
pub mod path_integration;
pub mod tick;
pub mod position_manager;

// Re-export commonly used items
pub use pool::{
    PoolManager, calculate_sqrt_rate_from_tick, calculate_tick_from_sqrt_rate,
    update_pool_state, verify_pool_initialized,
};

pub use order::{
    OrderManager, OrderState, OrderStep, OrderRoute, RoutingLogic,
    SecureOrderManager, OracleTwapWindow, get_oracle_from_remaining,
    get_oracle_data_from_remaining,
    // 3D Order components
    OrderManager3D, DimensionWeights, Liquidity3D, PriceImpact3D, OrderType,
};

pub use tick::{
    TickManager, TickUpdate, get_next_initialized_tick,
    update_tick, flip_tick,
};

pub use position_manager::{
    PositionManager, PositionUpdate, calculate_position_fees,
};

pub use path_integration::{
    PathSegment, Position3D, CellIndex3D, TradeDimension,
    plan_path_by_cells, integrate_path_work,
};