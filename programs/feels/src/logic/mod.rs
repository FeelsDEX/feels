//! Protocol logic modules
//!
//! Core business logic separated from instruction handlers

pub mod engine;
pub mod fees;
pub mod floor;
pub mod jit_core;
pub mod jit_safety;
pub mod jit_swap_integration;
pub mod liquidity_math;
pub mod pomm;
pub mod position_fees;
pub mod swap_common;
pub mod swap_execution;
pub mod swap_fees;

pub use engine::*;
pub use fees::*;
pub use floor::*;
pub use jit_core::*;
pub use jit_safety::*;
pub use jit_swap_integration::*;
pub use liquidity_math::*;
pub use pomm::*;
pub use position_fees::*;
// Import from swap_common (SwapResult conflict resolved by renaming execution one)
pub use swap_common::{
    validate_swap_params, execute_swap_transfers, distribute_swap_fees, 
    update_market_state, update_oracle_state, emit_swap_event, 
    validate_slippage_exact_out, validate_slippage, validate_fee_cap, 
    get_swap_accounts, SwapAccounts, SwapResult
};
// Export specific items from swap_execution to avoid conflicts
pub use swap_execution::{
    SwapExecutionResult, SwapState, SwapParams, execute_swap_steps, initialize_jit_liquidity
};
// Export specific items from swap_fees
pub use swap_fees::{
    FeeSplit, calculate_dynamic_fees, split_and_apply_fees, 
    finalize_fee_state
};

// Re-export specific types that might not be caught by glob exports
pub use engine::{StepOutcome, SwapContext, SwapDirection};

// Re-export commonly used math functions
pub use crate::utils::math::{sqrt_price_from_tick, tick_from_sqrt_price};
