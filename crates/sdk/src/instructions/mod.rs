/// Instruction builders for the Feels Protocol
pub mod liquidity;
pub mod pool;
pub mod protocol;
pub mod swap;
pub mod token;
pub mod unified;

// Re-export everything from unified as the primary interface
pub use unified::*;

// Re-export specific functions needed for pool initialization
pub use pool::initialize_pool;
pub use protocol::initialize_protocol;
pub use token::{create_token, initialize_feelssol};
pub use liquidity::{add_liquidity, remove_liquidity};
pub use swap::{swap, compute_swap_route};