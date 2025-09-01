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
pub mod errors;
pub mod instructions;
pub mod types;
pub mod utils;
pub mod work_calculator;

pub use client::*;
pub use config::*;
pub use errors::*;
pub use types::*;
pub use work_calculator::{calculate_path_work, PathWorkParams, WorkResult};

// Re-export commonly used types from the program
pub use feels::state::{Pool, ProtocolState};

// Re-export the program ID
pub const FEELS_PROGRAM_ID: solana_sdk::pubkey::Pubkey = feels::ID;
