/// Business logic module containing core AMM functionality separated from instruction handlers.
/// Organizes complex calculations and state management into focused modules by domain.
/// Tick logic has been consolidated into tick.rs. Each module handles a specific
/// domain: pools, tick positions, events, concentrated liquidity, tick management (consolidated), swaps, hooks.
pub mod pool;
pub mod tick_position;
pub mod event;
pub mod concentrated_liquidity;
pub mod tick;
pub mod swap;
pub mod hook;

pub use event::*;
pub use concentrated_liquidity::*;
pub use tick::*;
pub use swap::*;
pub use hook::*;