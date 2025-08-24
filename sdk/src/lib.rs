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
pub mod instructions;
pub mod types;
pub mod utils;
pub mod errors;

pub use client::*;
pub use config::*;
pub use types::*;
pub use errors::*;

// Re-export commonly used types from the program
pub use feels::{
    state::{Pool, ProtocolState},
};

// Re-export the program ID
pub const FEELS_PROGRAM_ID: solana_sdk::pubkey::Pubkey = feels::ID;