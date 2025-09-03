/// Instruction builders for the Feels Protocol
pub mod order_unified; // New unified order system matching on-chain
pub mod pool;
pub mod protocol;
pub mod token;

// Legacy modules (to be removed after migration)
pub mod order; // Old order interface
pub mod liquidity;
pub mod swap;
pub mod unified;
pub mod entry_exit;

// Re-export new unified order interface as the primary API
pub use order_unified::*;

// Re-export specific functions needed for pool initialization
pub use pool::initialize_pool;
pub use protocol::initialize_protocol;
pub use token::{create_token, initialize_feelssol};