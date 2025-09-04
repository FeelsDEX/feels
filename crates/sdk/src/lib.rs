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
pub mod math;
pub mod field_commitment;
pub mod proof_builder;
pub mod rebate_calculator;
pub mod work_calculator;
pub mod instructions;
pub mod router;
pub mod router_advanced;

pub use client::*;
pub use config::*;
pub use utils::*;
pub use errors::*;

// Re-export types from instructions modules
pub use instructions::{
    // Pool types
    PoolCreationResult, CreatePoolResult, PoolInfo,
    // Position/liquidity types
    LiquidityResult, AddLiquidityResult, PositionInfo,
    // Swap types
    SwapResult,
    // Token types
    TokenAccountInfo,
};

pub use router::{HubRouter, Route, PoolInfo as RouterPoolInfo};
pub use router_advanced::{AdvancedRouter, RouteQuote, PoolReserves};

// Re-export shared types and math from feels-core
pub use feels_core::constants::*;
pub use feels_core::errors::*;
pub use feels_core::math::*;
pub use feels_core::types::*;

// Re-export the program ID (use a placeholder for now)
use solana_sdk::pubkey;
pub const FEELS_PROGRAM_ID: solana_sdk::pubkey::Pubkey = pubkey!("Fee1sProtoco11111111111111111111111111111111");