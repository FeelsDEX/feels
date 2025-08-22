/// State module organizing all on-chain account structures and data types.
/// Includes pool state, position NFTs, tick arrays for liquidity, error definitions,
/// and infrastructure for future features like hooks and tick routing optimization.
/// All accounts use efficient serialization for optimal Solana performance.
pub mod error;
pub mod pool;
pub mod tick;          // Now includes TickArrayRouter
pub mod tick_position;
pub mod hook;
pub mod protocol;

// Re-exports
pub use error::*;
pub use pool::*;
pub use tick::*;      // Exports both TickArray and TickArrayRouter
pub use tick_position::*;
pub use hook::*;
pub use protocol::*;
