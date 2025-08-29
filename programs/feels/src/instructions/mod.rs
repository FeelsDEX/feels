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

// Fee management
pub mod fee;

// Protocol configuration
pub mod config;

// Oracle operations
pub mod oracle_update;

// Asset management (separate concerns)
pub mod token;
pub mod vault;

// Maintenance operations
pub mod cleanup;

// Re-export functions
pub use pool::{initialize_protocol, initialize_feelssol, initialize_pool};
pub use token::create_token;
pub use fee::{collect_pool_fees, collect_protocol_fees, update_dynamic_fees};
pub use cleanup::{cleanup_tick_array, CleanupTickArrayParams};
pub use order::{handler as order_handler, OrderParams, OrderResult};
pub use order_compute::{handler as order_compute_handler, OrderComputeParams, Tick3DArrayInfo};
pub use order_modify::{handler as order_modify_handler, OrderModifyParams};
pub use order_redenominate::{handler as order_redenominate_handler, RedenominateParams, RedenominationResult};

