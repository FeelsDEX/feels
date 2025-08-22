/// Business logic module containing core AMM functionality separated from instruction handlers.
/// Organizes complex calculations and state management into focused modules by domain.
/// Each module handles a specific domain: pools, tick positions, ticks, events, liquidity, tick arrays, fees, swaps, hooks.

pub mod pool;
pub mod tick_position;
pub mod tick;
pub mod event;
pub mod liquidity;
pub mod tick_array;
pub mod fee;
pub mod swap;
pub mod hook;

// pub use pool::*;
// pub use tick_position::*;
// pub use tick::*;
pub use event::*;
pub use liquidity::*;
pub use tick_array::*;
pub use fee::*;
pub use swap::*;
pub use hook::*;