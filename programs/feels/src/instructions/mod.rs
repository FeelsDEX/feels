/// Instruction module organizing all protocol operations into logical groups.
/// Initialization instructions set up protocol and pool infrastructure,
/// liquidity instructions manage LP positions, trading instructions handle swaps,
/// and administrative instructions manage protocol configuration and maintenance.
/// Instructions are consolidated into coherent functional groups for better organization.

// Market operations
pub mod market_initialize;  // Initialize new markets
pub mod market_update;      // Unified market management (config, keeper updates, field updates)

// Order System (using keeper-provided field commitments)
pub mod order;              // Unified order execution and modification

// Fee enforcement
pub mod enforce_fees;       // Fee policy enforcement and pool status management

// Asset management
pub mod token;

// Rebase operations (temporarily disabled due to compilation issues)
// pub mod apply_rebase;

// Maintenance operations
pub mod cleanup;

// Keeper management
pub mod keeper_registry;

// Security examples (for documentation) - removed feature gate to prevent warning
// #[cfg(feature = "security-examples")]
// pub mod security_example;

// Re-export functions and types
pub use market_initialize::{initialize_market, InitializeMarketParams, InitializeMarketResult};
pub use market_update::{
    handler as market_update_handler, 
    MarketOperation, MarketConfigParams, FieldCommitmentUpdate, PoolUpdateParams,
    WeightConfig, RiskConfig, BufferConfig, FreshnessConfig,
    MarketUpdate,
};
pub use token::{handler as token_create_handler, TokenCreateParams, TokenCreateResult};
pub use cleanup::{cleanup_tick_array, CleanupTickArrayParams, CleanupTickArrayResult};
pub use order::{
    handler as order_handler, 
    OrderParams, CreateOrderParams, ModifyOrderParams,
    OrderResult, CreateOrderResult, ModifyOrderResult, 
    OrderType, RateParams, OrderModification,
};
pub use enforce_fees::{
    handler as enforce_fees_handler, 
    initialize_pool_status,
    EnforceFeesParams, EnforceFeesResult,
    EnforceFees, InitializePoolStatus,
};
// pub use apply_rebase::{handler as apply_rebase_handler, RebaseOperation, ApplyRebase};
pub use keeper_registry::{
    initialize_keeper_registry, add_keeper, remove_keeper,
    InitializeKeeperRegistry, AddKeeper, RemoveKeeper,
    AddKeeperParams, RemoveKeeperParams,
};

