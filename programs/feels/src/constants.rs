//! Global constants for the Feels protocol
//! 
//! Centralized constants for PDA seeds and protocol parameters

// PDA seed constants
pub const MARKET_AUTHORITY_SEED: &[u8] = b"authority"; // Unified market authority
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority"; // Deprecated - use MARKET_AUTHORITY_SEED
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
pub const BUFFER_AUTHORITY_SEED: &[u8] = b"buffer_authority"; // Deprecated - use MARKET_AUTHORITY_SEED
pub const JITOSOL_VAULT_SEED: &[u8] = b"jitosol_vault";
pub const BUFFER_SEED: &[u8] = b"buffer"; // For market fee buffer (τ)
pub const ESCROW_SEED: &[u8] = b"escrow"; // For pre-launch token escrow
pub const ESCROW_AUTHORITY_SEED: &[u8] = b"escrow_authority"; // Authority for pre-launch escrow
pub const MARKET_SEED: &[u8] = b"market";
pub const VAULT_SEED: &[u8] = b"vault";
pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
pub const POSITION_SEED: &[u8] = b"position";
pub const METADATA_SEED: &[u8] = b"metadata";

// Token constants
pub const TOKEN_DECIMALS: u8 = 6;
pub const TOTAL_SUPPLY: u64 = 1_000_000_000 * 1_000_000; // 1B tokens with 6 decimals
pub const MIN_LAUNCH_AMOUNT: u64 = 250_000_000 * 1_000_000; // 250M tokens with 6 decimals

// Fee constants
pub const MAX_FEE_BPS: u16 = 1000; // 10%
pub const MAX_TICK_SPACING: u16 = 1000;

// Bonding curve constants
pub const NUM_TRANCHES: usize = 10;
pub const TICK_RANGE_PER_TRANCHE: i32 = 1000;

// Epoch
pub const EPOCH_PARAMS_SEED: &[u8] = b"epoch_params";

// Math constants
pub const Q64: u128 = 1u128 << 64;
pub const MIN_TICK: i32 = -443636;
pub const MAX_TICK: i32 = 443636;

// Liquidity constants
/// Minimum liquidity threshold to prevent dust positions
/// This prevents creation of positions that are economically insignificant
/// but still consume on-chain resources. Set to 1000 units.
pub const MIN_LIQUIDITY: u128 = 1000;

// Swap constants
/// Maximum number of ticks that can be crossed in a single swap
/// This prevents griefing attacks where attackers create many empty ticks
pub const MAX_TICKS_CROSSED: u8 = 200;

// Protocol token registry
pub const PROTOCOL_TOKEN_SEED: &[u8] = b"protocol_token";

// Floor liquidity constants
/// Minimum threshold for floor liquidity placement (100 tokens with 6 decimals)
/// This prevents griefing by requiring economically significant amounts
pub const MIN_FLOOR_PLACEMENT_THRESHOLD: u64 = 100_000_000;