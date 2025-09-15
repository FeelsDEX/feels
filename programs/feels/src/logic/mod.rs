//! Protocol logic modules
//!
//! Core business logic separated from instruction handlers

pub mod engine;
pub mod fees;
pub mod floor;
pub mod jit;
pub mod liquidity_math;
pub mod pomm;
pub mod position_fees;

pub use engine::*;
pub use fees::*;
pub use floor::*;
pub use jit::*;
pub use liquidity_math::*;
pub use pomm::*;
pub use position_fees::*;

// Re-export specific types that might not be caught by glob exports
pub use engine::{StepOutcome, SwapContext, SwapDirection};

// Re-export commonly used math functions
pub use crate::utils::math::{sqrt_price_from_tick, tick_from_sqrt_price};
