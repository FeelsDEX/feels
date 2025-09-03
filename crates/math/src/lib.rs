/// Mathematical utilities for the Feels Protocol
/// 
/// This crate provides safe mathematical operations, fixed-point arithmetic,
/// 3D geometry functions, and work calculation algorithms used across
/// the SDK, keeper, and other protocol components.

pub mod safe;
pub mod fixed_point;
pub mod work_calc;
pub mod geometry;

#[cfg(feature = "advanced")]
pub mod eigenvalues;

// Re-export commonly used functions
pub use safe::*;
pub use fixed_point::*;
pub use work_calc::*;
pub use geometry::*;

#[cfg(feature = "advanced")]
pub use eigenvalues::*;