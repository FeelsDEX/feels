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

// Keeper operations
pub mod keeper_register;
pub mod keeper_slash;
pub mod verify_market_update;

// Simplified oracle system
pub mod oracle_update;

// Asset management
pub mod token;

// Maintenance operations
pub mod cleanup;


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
pub use keeper_register::{register_keeper, RegisterKeeper, add_keeper_stake, AddKeeperStake, 
    remove_keeper_stake, RemoveKeeperStake, exit_keeper, ExitKeeper, withdraw_keeper_stake, WithdrawKeeperStake};
pub use keeper_slash::{slash_keeper, SlashKeeper, automated_slash_batch, AutomatedSlash, appeal_slash, AppealSlash};
pub use verify_market_update::{verify_market_update, VerifyMarketUpdate};
pub use oracle_update::{update_oracle, UpdateOracle, initialize_oracle_config, InitializeOracleConfig,
    update_oracle_config, UpdateOracleConfig, emergency_oracle_override, EmergencyOracleOverride};

