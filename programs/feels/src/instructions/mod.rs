/// Instruction module organizing all protocol operations into logical groups.
/// Initialization instructions set up protocol and pool infrastructure,
/// liquidity instructions manage LP positions, trading instructions handle swaps,
/// and administrative instructions manage protocol configuration and maintenance.
/// Instructions are consolidated into coherent functional groups for better organization.

// Pool operations (initialization and setup)
pub mod pool;

// Unified Order System
pub mod order;              // Order execution (swaps, liquidity, limits)
pub mod order_compute;      // Tick computation and routing
pub mod order_modify;       // Modify existing orders
pub mod order_redenominate; // Redenomination handling
pub mod unified_order;      // Simplified unified order API

// Fee management removed - handled through unified order system

// Virtual rebasing
pub mod rebase_initialize;
pub mod rebase_update;
pub mod weight_rebase;

// Swap removed - use unified order system

// Protocol configuration
pub mod configure_pool;  // Unified configuration system

// Market field updates
pub mod update_market_field;

// Asset management
pub mod token;

// Maintenance operations
pub mod cleanup;

// Security examples (for documentation)
#[cfg(feature = "security-examples")]
pub mod security_example;

// Re-export functions
pub use pool::{initialize_protocol, initialize_feelssol, initialize_pool};
pub use token::create_token;
pub use rebase_initialize::handler as initialize_rebase;
pub use rebase_update::{handler as update_rebase, RebaseUpdateParams};
pub use cleanup::{cleanup_tick_array, CleanupTickArrayParams};
pub use order::{handler as order_handler, OrderParams, OrderResult, OrderType, RateParams};
pub use order_compute::{handler as order_compute_handler, OrderComputeParams, Tick3DArrayInfo, RateComputeParams};
pub use order_modify::{handler as order_modify_handler, OrderModifyParams};
pub use order_redenominate::{handler as order_redenominate_handler, RedenominateParams, RedenominationResult};
pub use configure_pool::{handler as configure_pool_handler, PoolConfigParams};
pub use unified_order::{UnifiedOrderParams, UnifiedOrderResult, UnifiedModifyParams, UnifiedComputeParams};
pub use weight_rebase::{execute_weight_rebase, ExecuteWeightRebase, emergency_weight_update, EmergencyWeightUpdate};
pub use update_market_field::{handler as update_market_field, UpdateMarketField, UpdateMarketFieldParams,
    initialize_handler as initialize_market_field, InitializeMarketField, InitializeMarketFieldParams};

