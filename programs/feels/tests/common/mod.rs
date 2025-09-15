//! Unified test infrastructure for Feels Protocol
//!
//! Supports both in-memory (ProgramTest) and devnet testing with identical APIs

pub mod assertions;
pub mod builders;
pub mod client;
pub mod context;
pub mod environment;
pub mod fixtures;
pub mod helpers;
pub mod jito;
pub mod time;
pub mod tracing;
#[macro_use]
pub mod macros;

// Type alias for test results
pub type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Import Error trait for macro usage
pub use std::error::Error;

// Re-export core types
pub use builders::MarketBuilder;
pub use client::TestClient;
pub use context::TestContext;
pub use environment::{should_run_devnet_tests, should_run_localnet_tests, TestEnvironment};
pub use helpers::SwapResult;

// Re-export assertion utilities
pub use assertions::{
    MarketAssertions, MarketTestData, PositionAssertions, ProtocolInvariants, SwapAssertions,
    SwapResult as AssertionSwapResult, TickArrayAssertions,
};

// Note: Macros assert_tx_success, assert_error, assert_balance_change are available
// at crate root due to #[macro_export]

// Re-export test macros
pub use crate::{test_all_environments, test_devnet, test_in_memory};

// Common imports for all tests
pub use anchor_lang::prelude::*;
pub use feels::{instructions::*, state::*, ID as PROGRAM_ID};
pub use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
pub use spl_token::state::Account as TokenAccount;

// Test types
pub struct MarketInfo {
    pub address: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee_tier: u16,
    pub tick_spacing: u16,
}

// SDK imports
use feels_sdk as sdk;

// Constants
pub mod constants {
    use super::*;

    // Token constants
    pub const JITOSOL_MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
    // Use system program ID (all zeros) as a base for FeelsSOL in tests
    // This ensures FeelsSOL will have a very low pubkey value and can be token_0
    pub const FEELSSOL_TEST_MINT: Pubkey = pubkey!("11111111111111111111111111111112");
    pub const FEELSSOL_DECIMALS: u8 = 9;
    pub const TEST_TOKEN_DECIMALS: u8 = 6;

    // Amounts
    pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
    pub const DEFAULT_AIRDROP: u64 = 10 * LAMPORTS_PER_SOL;
    pub const DUST_AMOUNT: u64 = 1_000;

    // Common swap amounts
    pub const SMALL_SWAP: u64 = 100_000; // 0.1 token (6 decimals)
    pub const MEDIUM_SWAP: u64 = 1_000_000; // 1 token
    pub const LARGE_SWAP: u64 = 100_000_000; // 100 tokens

    // Tick constants
    pub const MIN_TICK: i32 = -443636;
    pub const MAX_TICK: i32 = 443636;

    // Price constants (Q64.64)
    pub const PRICE_1_TO_1: u128 = 79228162514264337593543950336;

    // Sqrt price bounds (Q64)
    pub const MIN_SQRT_PRICE: u128 = 4295128739; // tick -443636
    pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055; // tick 443636

    // Fee tiers
    pub const STABLE_FEE_TIER: u16 = 5; // 0.05%
    pub const LOW_FEE_TIER: u16 = 30; // 0.3%
    pub const MEDIUM_FEE_TIER: u16 = 100; // 1%
    pub const HIGH_FEE_TIER: u16 = 300; // 3%
}

// Utils module for test helpers
pub mod utils {

    // Re-export PDA derivation functions
    pub use feels::utils::seeds::{
        derive_buffer, derive_epoch_params, derive_market_authority, derive_oracle,
        derive_position, derive_tick_array, derive_vault,
    };

    // Re-export additional PDA derivation functions
    pub use feels::utils::seeds::{
        derive_jitosol_vault, derive_mint_authority, derive_vault_authority,
    };
}
