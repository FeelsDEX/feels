/// Business logic module containing core AMM functionality separated from instruction handlers.
/// Organizes complex calculations and state management into focused modules by domain.
/// Tick logic has been consolidated into tick.rs. Each module handles a specific
/// domain: pools, tick positions, events, concentrated liquidity, tick management (consolidated), swaps, hooks.

// ============================================================================
// Module Declarations  
// ============================================================================

// Organized submodules
pub mod core;
pub mod market_physics;
pub mod rebase;

// Standalone modules
pub mod concentrated_liquidity;
pub mod event;
pub mod hook;
pub mod lazy_router;
pub mod optimal_path;
pub mod oracle;
pub mod volatility_manager;

// ============================================================================
// Re-exports
// ============================================================================

// Concentrated liquidity calculations
pub use concentrated_liquidity::{
    calculate_liquidity_from_amounts, get_amounts_from_liquidity,
    LiquidityCalculator, ConcentratedLiquidityManager,
};


// Events
pub use event::{
    OrderEvent, LiquidityEvent, SwapEvent, PositionEvent,
    RebaseEvent, TokenCreated, PoolCreated,
};

// Market physics
pub use market_physics::{
    // Conservation laws
    verify_conservation, solve_conservation_factor,
    // Gradients
    calculate_gradient_3d, Gradient3D, GradientCalculator,
    // Hessians
    calculate_hessian_3x3, Hessian3x3, HessianCalculator,
    // Potential
    calculate_potential, FixedPoint, ln_fixed, exp_fixed,
    // Work
    calculate_work, calculate_fee_and_rebate, WorkCalculator,
};

// Hook system
pub use hook::{
    HookContext, HookContextBuilder, HookType, execute_hooks,
    execute_post_hooks, EVENT_ORDER_CREATED, EVENT_ORDER_FILLED,
};


// Oracle system
pub use oracle::{
    OracleUpdate, MarketParameters, OracleConfig, OracleProvider,
    validate_oracle_update, apply_oracle_update, calculate_simple_gradients,
    MAX_ORACLE_STALENESS, MAX_PARAMETER_CHANGE_BPS,
};

// Routing
pub use lazy_router::{
    LazyRouter, RouteStep, RouteResult,
};
pub use optimal_path::{
    optimize_path_gradient_descent, find_optimal_path_astar,
    PathConstraints,
};

// Core AMM functionality
pub use core::{
    // Pool management
    PoolManager, calculate_sqrt_rate_from_tick, calculate_tick_from_sqrt_rate,
    update_pool_state, verify_pool_initialized,
    // Order management
    OrderManager, OrderState, OrderStep, OrderRoute, RoutingLogic,
    SecureOrderManager, OracleTwapWindow, get_oracle_from_remaining, 
    get_oracle_data_from_remaining,
    OrderManager3D, DimensionWeights, Liquidity3D, PriceImpact3D, OrderType,
    // Position management
    PositionManager, PositionUpdate, calculate_position_fees,
    // Tick management
    TickManager, TickUpdate, get_next_initialized_tick,
    update_tick, flip_tick,
    // Path integration
    PathSegment, Position3D, CellIndex3D, TradeDimension,
    plan_path_by_cells, integrate_path_work,
};

// Rebasing system
pub use rebase::{
    // Core framework
    RebaseStrategy, RebaseFactors, RebaseState, RebaseParams, 
    DomainParams, DomainWeights, RebaseExecutor,
    // Funding rebase
    FundingRebaseFactors, FundingRebaseStrategy, calculate_funding_rebase,
    // Lending rebase
    LendingRebaseFactors, LendingRebaseStrategy, calculate_lending_rebase,
    // Leverage rebase
    LeverageRebaseFactors, LeverageRebaseStrategy, calculate_leverage_rebase,
    // Weight rebase
    WeightRebaseFactors, WeightRebaseStrategy, calculate_weight_rebase,
};

// Volatility management
pub use volatility_manager::{
    VolatilityManager, calculate_realized_volatility,
    update_volatility_metrics,
};

