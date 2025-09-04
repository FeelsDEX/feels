/// Instruction builders for the Feels Protocol
pub mod order_unified; // New unified order system matching on-chain
pub mod pool;
pub mod protocol;
pub mod token;
pub mod entry_exit;
pub mod position;

// Re-export new unified order interface as the primary API
pub use order_unified::*;

// Re-export entry/exit builders
pub use entry_exit::{EnterProtocolBuilder, ExitProtocolBuilder, calculate_min_output};

// Re-export position builders and types
pub use position::{EnterPositionBuilder, ExitPositionBuilder, LiquidityResult, AddLiquidityResult, PositionInfo};

// Re-export pool functions and types
pub use pool::{initialize_pool, PoolCreationResult, CreatePoolResult, PoolInfo};

// Re-export protocol functions
pub use protocol::initialize_protocol;

// Re-export token functions and types
pub use token::{create_token, initialize_feelssol, TokenAccountInfo};