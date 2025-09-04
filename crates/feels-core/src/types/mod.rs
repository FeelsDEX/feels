//! # Core Type Definitions
//! 
//! Shared type definitions that work in both on-chain and off-chain environments.

pub mod fees;
pub mod field;
pub mod market;
pub mod math;
pub mod orders;
pub mod routes;
pub mod work;

// Re-export all types
pub use fees::*;
pub use field::*;
pub use market::*;
pub use math::*;
pub use orders::*;
pub use routes::*;
pub use work::*;