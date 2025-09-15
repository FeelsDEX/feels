//! MVP state structures
//! 
//! Minimal state for Phase 1 implementation

pub mod market;
pub mod buffer;
pub mod escrow;
pub mod epoch_params;
pub mod protocol_oracle;
pub mod safety_controller;
pub mod feels_hub;
pub mod tick;
pub mod position;
pub mod oracle;
pub mod token_metadata;
pub mod liquidity_commitment;
pub mod protocol_config;
pub mod tranche_plan;

pub use market::*;
pub use buffer::*;
pub use escrow::*;
pub use epoch_params::*;
pub use protocol_oracle::*;
pub use safety_controller::*;
pub use feels_hub::*;
pub use tick::*;
pub use position::*;
pub use oracle::*;
pub use token_metadata::*;
pub use liquidity_commitment::*;
pub use protocol_config::*;
pub use tranche_plan::*;

// Compile-time assertions for zero_copy struct sizes
// These ensure our structs maintain expected memory layout
// Disabled for now due to size calculation issues
#[cfg(feature = "size-checks")]
mod _size_assertions {
    // TODO: Fix size calculations and re-enable
    // use super::*;
    // use static_assertions::const_assert_eq;
}
