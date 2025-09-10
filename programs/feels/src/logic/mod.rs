//! Protocol logic modules
//! 
//! Core business logic separated from instruction handlers

pub mod pomm;
pub mod engine;
pub mod liquidity_math;
pub mod position_fees;
pub mod bonding_curve;

pub use pomm::*;
pub use engine::*;
pub use liquidity_math::*;
pub use position_fees::*;
pub use bonding_curve::*;

// Re-export specific types that might not be caught by glob exports
pub use engine::{SwapDirection, StepOutcome, SwapContext};

// Re-export commonly used math functions
pub use crate::utils::math::{tick_from_sqrt_price, sqrt_price_from_tick};