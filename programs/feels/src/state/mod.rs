/// State module organizing all on-chain account structures and data types.
/// Includes pool state, position NFTs, tick arrays for liquidity, token metadata,
/// protocol configuration, error definitions, and infrastructure for future features
/// like hooks and tick routing optimization. All accounts use efficient serialization for optimal Solana performance.
pub mod error;
pub mod hook;
pub mod pool;
pub mod protocol;
pub mod tick; // Now includes TickArrayRouter
pub mod position;
pub mod token;

// Core Types
pub mod duration; // Duration dimension for 3D model
pub mod fee; // Fee system (static and dynamic)
pub mod leverage; // Continuous leverage system
pub mod vault; // Automated position management
pub mod reentrancy; // Reentrancy protection

// Metrics
pub mod metrics_price; // Price oracle and TWAP
pub mod metrics_volume; // Trading volume tracking
pub mod metrics_volatility; // High-frequency volatility tracking
pub mod metrics_lending; // Lending metrics (flash loans + utilization)

// Re-exports
pub use error::*;
pub use hook::*;
pub use pool::*;
pub use protocol::*;
pub use reentrancy::*;
pub use tick::*; // Exports both TickArray and TickArrayRouter
pub use position::*;
pub use token::*;

// Core type exports
pub use duration::*;
pub use fee::*;
pub use leverage::*;
pub use vault::*;

// Metrics exports
pub use metrics_price::*;
pub use metrics_volume::*;
pub use metrics_volatility::*;
pub use metrics_lending::*;
