//! Feels Protocol SDK
//!
//! A modern, service-based SDK for interacting with the Feels Protocol
//! concentrated liquidity AMM on Solana.
//!
//! # Architecture
//!
//! The SDK is organized into four main modules:
//! - `core`: Core types, constants, and errors
//! - `protocol`: Protocol math, PDA derivation, and fee calculations
//! - `instructions`: Type-safe instruction builders
//! - `client`: Service-based API for protocol interaction
//!
//! # Example
//!
//! ```no_run
//! use feels_sdk::FeelsClient;
//! use solana_sdk::{pubkey::Pubkey, signature::Keypair};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client
//!     let client = FeelsClient::new("https://api.mainnet-beta.solana.com").await?;
//!     
//!     // Example token addresses
//!     let token_0: Pubkey = "TokenA11111111111111111111111111111111111".parse()?;
//!     let token_1: Pubkey = "TokenB22222222222222222222222222222222222".parse()?;
//!     let signer = Keypair::new();
//!     let user_token_in: Pubkey = "UserTokenIn333333333333333333333333333".parse()?;
//!     let user_token_out: Pubkey = "UserTokenOut44444444444444444444444444".parse()?;
//!     
//!     // Get market info
//!     let market = client.market.get_market_by_tokens(&token_0, &token_1).await?;
//!     
//!     // Execute swap
//!     let result = client.swap.swap_exact_in(
//!         &signer,
//!         market.address,
//!         user_token_in,
//!         user_token_out,
//!         1_000_000,
//!         950_000,
//!         Some(100), // 1% slippage
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod core;
pub mod instructions;
pub mod protocol;
pub mod jupiter;

// Re-export main types and functions
pub use client::FeelsClient;
pub use core::{
    constants::program_id, FeeEstimate, MarketInfo, PositionInfo, Route, SdkError, SdkResult,
    SwapDirection, SwapSimulation,
};
pub use protocol::{
    align_tick, calculate_fee_amount, calculate_price_impact_bps, calculate_swap_fees,
    find_market_address, is_full_range_only, sqrt_price_to_price, sqrt_price_to_tick,
    tick_to_sqrt_price,
};

// Re-export Jupiter integration types
pub use jupiter::{
    MarketState, ParsedTickArray, SwapSimulator, TickArrayFormat, TickArrayLoader, TickArrayView,
    TickData, parse_tick_array_auto,
};

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");