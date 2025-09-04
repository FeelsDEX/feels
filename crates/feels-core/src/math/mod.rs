//! # Mathematical Functions
//! 
//! Pure mathematical functions for thermodynamic calculations.

pub mod big_int;
pub mod fee_math;
pub mod fixed_point;
pub mod liquidity_math;
pub mod safe_math;
pub mod tick_math;
pub mod work_calc;

#[cfg(feature = "advanced")]
pub mod geometry;

// Re-export commonly used functions
pub use big_int::*;
pub use fee_math::*;
pub use fixed_point::*;
pub use liquidity_math::*;
pub use safe_math::*;
pub use tick_math::*;
pub use work_calc::*;

#[cfg(feature = "advanced")]
pub use geometry::*;