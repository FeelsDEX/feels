/// Feels Protocol SDK
///
/// A comprehensive SDK for interacting with the Feels Protocol on-chain.
/// Provides high-level abstractions for:
/// - Protocol initialization
/// - Token operations
/// - Pool management
/// - Liquidity operations
/// - Trading

pub mod client;
pub mod config;
pub mod utils;
pub mod errors;
pub mod types;
pub mod math;
pub mod field_commitment;
pub mod proof_builder;
pub mod rebate_calculator;
pub mod work_calculator;
pub mod instructions;

pub use client::*;
pub use config::*;
pub use utils::*;
pub use errors::*;
pub use types::*;

// Re-export shared types and math
pub use feels_types::*;
pub use feels_math::*;

// Re-export the program ID (use a placeholder for now)
use solana_sdk::pubkey;
pub const FEELS_PROGRAM_ID: solana_sdk::pubkey::Pubkey = pubkey!("Fee1sProtoco11111111111111111111111111111111");