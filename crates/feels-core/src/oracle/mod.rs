//! # Oracle Module
//! 
//! Price and volatility oracle implementations for the Feels protocol.
//! Provides TWAP calculations and volatility tracking used by both on-chain
//! and off-chain components.

pub mod twap;
pub mod volatility;

pub use twap::*;
pub use volatility::*;