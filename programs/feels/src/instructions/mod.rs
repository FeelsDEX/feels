//! Instruction handlers
//!
//! Re-export everything from instructions to satisfy Anchor program macro

#![allow(ambiguous_glob_reexports)]

pub mod enter_feelssol;
pub use enter_feelssol::*;

pub mod exit_feelssol;
pub use exit_feelssol::*;

pub mod swap;
pub use swap::*;

pub mod open_position;
pub use open_position::*;

pub mod close_position;
pub use close_position::*;

pub mod collect_fees;
pub use collect_fees::*;

pub mod mint_token;
pub use mint_token::*;

pub mod deploy_initial_liquidity;
pub use deploy_initial_liquidity::*;

pub mod open_position_with_metadata;
pub use open_position_with_metadata::*;

pub mod close_position_with_metadata;
pub use close_position_with_metadata::*;

pub mod update_position_fee_lower;
pub use update_position_fee_lower::*;

pub mod update_position_fee_upper;
pub use update_position_fee_upper::*;

pub mod initialize_market;
pub use initialize_market::*;

pub mod initialize_protocol;
pub use initialize_protocol::*;

pub mod destroy_expired_token;
pub use destroy_expired_token::*;

pub mod graduate_pool;
pub use graduate_pool::*;

pub mod initialize_tranche_ticks;
pub use initialize_tranche_ticks::*;

pub mod cleanup_bonding_curve;
pub use cleanup_bonding_curve::*;

pub mod update_floor;
pub use update_floor::*;

pub mod update_protocol_oracle;
pub use update_protocol_oracle::*;

pub mod initialize_hub;
pub use initialize_hub::*;

// Additional specific exports for Anchor
pub use update_protocol_oracle::{
    update_dex_twap, update_native_rate, UpdateDexTwap, UpdateDexTwapParams, UpdateNativeRate,
    UpdateNativeRateParams,
};
