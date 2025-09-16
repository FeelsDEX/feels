//! Jupiter AMM adapter for the Feels Protocol concentrated liquidity AMM
//!
//! This adapter enables Feels markets to be discovered and used by Jupiter's
//! routing engine, providing seamless integration with the broader Solana DeFi
//! ecosystem. The adapter implements Jupiter's AMM interface to:
//!
//! - Quote swap prices using Feels concentrated liquidity math
//! - Generate transaction instructions for Jupiter-routed swaps
//! - Maintain tick array state for accurate cross-tick calculations
//! - Handle hub-and-spoke routing through FeelsSOL

use anyhow::{ensure, Result};
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, 
    Swap, SwapAndAccountMetas, SwapParams, AmmProgramIdToLabel, AmmLabel,
    try_get_account_data,
};
use solana_program::{
    pubkey::Pubkey,
    instruction::AccountMeta,
};
use anchor_lang::prelude::*;
use orca_whirlpools_core::{
    U128,
    try_get_next_sqrt_price_from_a, try_get_next_sqrt_price_from_b,
    try_get_amount_delta_a, try_get_amount_delta_b,
};
use spl_token::state::Account as TokenAccount;
use solana_program::program_pack::Pack;

use feels::state::Market;
use ahash::AHashMap;

// =============================================================================
// CONSTANTS & CONFIGURATION
// =============================================================================

/// Number of ticks per tick array, matching Feels Protocol configuration
const TICK_ARRAY_SIZE: i32 = 64;

// =============================================================================
// DATA STRUCTURES & TYPES
// =============================================================================

/// Cached view of a tick array for efficient liquidity calculations
///
/// This structure maintains a sparse representation of initialized ticks
/// within a tick array, allowing for fast lookups during quote calculations.
#[derive(Clone, Default)]
struct TickArrayView {
    /// Starting tick index for this array (aligned to array boundaries)
    start_tick_index: i32,
    /// Map of tick_index -> liquidity_net for initialized ticks only
    /// Sparse representation saves memory and improves lookup performance
    inits: AHashMap<i32, i128>,
}

/// Jupiter AMM adapter for Feels Protocol markets
///
/// This struct implements the Jupiter AMM interface, enabling Feels markets
/// to participate in Jupiter's routing and aggregation. It maintains cached
/// state for efficient quote calculations and generates proper transaction
/// instructions for swap execution.
pub struct FeelsAmm {
    /// Market account public key
    key: Pubkey,
    /// Deserialized market state from on-chain account
    market: Market,
    /// Market authority PDA that controls vault operations
    authority: Pubkey,
    /// Feels program ID
    program_id: Pubkey,
    /// Token mints for the trading pair [token_0, token_1]
    reserve_mints: [Pubkey; 2],
    /// Current token reserves in vaults [vault_0_amount, vault_1_amount]
    reserves: [u64; 2],
    /// Token vault addresses
    vault_0: Pubkey,
    vault_1: Pubkey,
    /// Tick spacing for this market (determines price granularity)
    tick_spacing: u16,
    /// Cached tick array views for liquidity calculations
    tick_arrays: AHashMap<i32, TickArrayView>, // start_index -> view
    /// Public keys of tick arrays to monitor for updates
    tick_array_keys: Vec<Pubkey>,
}

// =============================================================================
// TRAIT IMPLEMENTATIONS
// =============================================================================

impl AmmProgramIdToLabel for FeelsAmm {
    const PROGRAM_ID_TO_LABELS: &'static [(Pubkey, AmmLabel)] = &[
        (feels::ID, "Feels"),
    ];
}

impl Clone for FeelsAmm {
    fn clone(&self) -> Self {
        FeelsAmm {
            key: self.key,
            market: self.market.clone(),
            authority: self.authority,
            program_id: self.program_id,
            reserve_mints: self.reserve_mints,
            reserves: self.reserves,
            vault_0: self.vault_0,
            vault_1: self.vault_1,
            tick_spacing: self.tick_spacing,
            tick_arrays: self.tick_arrays.clone(),
            tick_array_keys: self.tick_array_keys.clone(),
        }
    }
}

// =============================================================================
// JUPITER AMM INTERFACE IMPLEMENTATION
// =============================================================================

impl Amm for FeelsAmm {
    /// Initialize FeelsAmm from a Jupiter KeyedAccount
    ///
    /// This function deserializes a Feels market account and sets up the adapter
    /// with all necessary state for quote calculations and swap instruction generation.
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        // Validate account ownership
        ensure!(
            keyed_account.account.owner == feels::ID,
            "Invalid program owner for Feels market"
        );
        
        // Deserialize market account data
        let data = &keyed_account.account.data;
        let market = Market::try_deserialize_unchecked(&mut &data[..])?;
        
        // Validate market state
        ensure!(market.is_initialized, "Market not initialized");
        ensure!(!market.is_paused, "Market is paused");
        
        // Derive protocol PDAs
        let program_id = feels::ID;
        let market_key = keyed_account.key;
        let (authority, _) = market.derive_market_authority_with_key(&market_key, &program_id);
        let ((vault_0, _), (vault_1, _)) = market.get_vault_addresses(&market_key, &program_id);
        
        // Cache token mints for routing validation
        let reserve_mints = [market.token_0, market.token_1];
        
        // Pre-compute tick array addresses for liquidity calculations
        // Start with a conservative window that can be expanded if needed
        let tick_spacing = market.tick_spacing;
        let arrays = derive_tick_arrays_for_quote(&market_key, market.current_tick, tick_spacing, 3);

        Ok(Self {
            key: keyed_account.key,
            market,
            authority,
            program_id,
            reserve_mints,
            reserves: [0, 0], // Updated in update() call
            vault_0,
            vault_1,
            tick_spacing,
            tick_arrays: AHashMap::new(),
            tick_array_keys: arrays,
        })
    }

    /// Return human-readable label for this AMM
    fn label(&self) -> String {
        "Feels".to_string()
    }

    /// Return the Feels program ID
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    /// Return the market account public key
    fn key(&self) -> Pubkey {
        self.key
    }

    /// Return the token mints for this trading pair
    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.reserve_mints.to_vec()
    }

    /// Return accounts that need to be monitored for state changes
    ///
    /// Jupiter will fetch these accounts and call update() when they change.
    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let mut accounts = vec![self.vault_0, self.vault_1];
        accounts.extend(self.tick_array_keys.iter().copied());
        accounts
    }

    /// Update cached state from fresh account data
    ///
    /// Jupiter calls this method when monitored accounts change, allowing
    /// the adapter to refresh its cached state for accurate quotes.
    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        // Update vault reserve amounts
        let vault_0_account = try_get_account_data(account_map, &self.vault_0)?;
        let vault_0_token_account = TokenAccount::unpack(vault_0_account)?;
        
        let vault_1_account = try_get_account_data(account_map, &self.vault_1)?;
        let vault_1_token_account = TokenAccount::unpack(vault_1_account)?;
        
        self.reserves = [
            vault_0_token_account.amount,
            vault_1_token_account.amount,
        ];

        // Parse and cache tick array data for liquidity calculations
        for key in &self.tick_array_keys {
            if let Ok(bytes) = try_get_account_data(account_map, key) {
                if let Ok(view) = parse_tick_array(bytes, self.tick_spacing) {
                    self.tick_arrays.insert(view.start_tick_index, view);
                }
            }
        }
        
        Ok(())
    }

    /// Generate a quote for a potential swap
    ///
    /// Uses Feels concentrated liquidity math to calculate the expected output
    /// amount and fees for a given input amount and token pair.
    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        // Determine swap direction based on input mint
        let (is_token_0_to_1, amount_in) = if quote_params.input_mint == self.reserve_mints[0] {
            (true, quote_params.amount)
        } else if quote_params.input_mint == self.reserve_mints[1] {
            (false, quote_params.amount)
        } else {
            anyhow::bail!("Invalid input mint for this market");
        };
        
        // Estimate required tick coverage based on input size and current liquidity
        let estimated_ticks = estimate_ticks_to_cross(
            &self.market,
            amount_in,
            is_token_0_to_1,
        );
        
        // Calculate swap output using Feels concentrated liquidity math
        let (amount_out, fee_amount) = calculate_swap_output_with_coverage_check(
            &self.market,
            &self.tick_arrays,
            amount_in,
            is_token_0_to_1,
            estimated_ticks,
        )?;
        
        // Calculate fee percentage for Jupiter interface
        use rust_decimal::Decimal;
        use std::str::FromStr;
        
        let fee_pct = if amount_in > 0 {
            let fee_ratio = fee_amount as f64 / amount_in as f64;
            Decimal::from_str(&format!("{}", fee_ratio * 100.0)).unwrap_or_default()
        } else {
            Decimal::ZERO
        };
        
        Ok(Quote {
            in_amount: amount_in,
            out_amount: amount_out,
            fee_amount,
            fee_mint: quote_params.input_mint,
            fee_pct,
        })
    }

    /// Return the number of accounts required for a Feels swap instruction
    fn get_accounts_len(&self) -> usize {
        17 // Updated for fee distribution accounts: protocol_config, treasury, protocol_token, creator_account
    }

    /// Generate swap instruction and account metas for Jupiter routing
    ///
    /// This method constructs the account list required for a Feels swap instruction,
    /// ensuring the accounts are in the correct order to match the Swap struct.
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let SwapParams {
            source_mint,
            destination_mint,
            source_token_account,
            destination_token_account,
            token_transfer_authority,
            ..
        } = swap_params;
        
        // Validate token pair and determine swap direction
        let _is_token_0_to_1 = if *source_mint == self.market.token_0 && *destination_mint == self.market.token_1 {
            true
        } else if *source_mint == self.market.token_1 && *destination_mint == self.market.token_0 {
            false
        } else {
            anyhow::bail!("Invalid mint pair for this market");
        };
        
        // Derive required PDAs for fee distribution
        let (protocol_config, _) = Pubkey::find_program_address(
            &[b"protocol_config"], 
            &feels::id()
        );
        
        // Derive protocol treasury token account for the output token
        // Note: In a full implementation, we would need to fetch the protocol config
        // to get the treasury pubkey. For MVP, we'll use a derived treasury address.
        let protocol_treasury = derive_protocol_treasury_token_account(destination_mint);

        // Build account metas matching the Swap struct from swap.rs
        // Order is critical - must match exactly for instruction parsing
        let account_metas = vec![
            // Core swap accounts
            AccountMeta::new(*token_transfer_authority, true), // user (signer)
            AccountMeta::new(self.key, false), // market
            AccountMeta::new(self.vault_0, false), // vault_0
            AccountMeta::new(self.vault_1, false), // vault_1
            AccountMeta::new_readonly(self.authority, false), // market_authority
            AccountMeta::new(self.market.buffer, false), // buffer
            
            // Oracle account for TWAP updates
            AccountMeta::new(self.market.oracle, false), // oracle
            
            // User token accounts
            AccountMeta::new(*source_token_account, false), // user_token_in
            AccountMeta::new(*destination_token_account, false), // user_token_out
            
            // System programs
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false), // clock
            
            // Fee distribution accounts (required for v2+ fee system)
            AccountMeta::new_readonly(protocol_config, false), // protocol_config
            AccountMeta::new(protocol_treasury, false), // protocol_treasury
            
            // Optional accounts for protocol tokens - Jupiter adapter defaults to None
            AccountMeta::new_readonly(Pubkey::default(), false), // protocol_token (None placeholder)
            AccountMeta::new_readonly(Pubkey::default(), false), // creator_token_account (None placeholder)
        ];
        
        Ok(SwapAndAccountMetas {
            swap: Swap::Saber, // Jupiter uses Saber variant for generic AMM swaps
            account_metas,
        })
    }

    /// Clone this AMM instance for thread-safe usage
    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    /// Indicates whether this AMM requires dynamic account resolution
    ///
    /// Feels markets have static account structures, so this returns false.
    fn has_dynamic_accounts(&self) -> bool {
        false
    }

    /// Indicates whether this AMM supports exact-out swaps
    ///
    /// Feels concentrated liquidity math supports both exact-in and exact-out.
    fn supports_exact_out(&self) -> bool {
        true
    }
}

// =============================================================================
// SWAP CALCULATION FUNCTIONS
// =============================================================================

/// Estimate the number of ticks likely to be crossed in a swap
///
/// This provides a rough estimate based on current liquidity and input size
/// to help determine if we have sufficient tick array coverage.
fn estimate_ticks_to_cross(
    market: &Market,
    amount_in: u64,
    _is_token_0_to_1: bool,
) -> i32 {
    // Calculate net input after base fee
    let fee_bps = market.base_fee_bps as u64;
    let fee_amount = (amount_in as u128 * fee_bps as u128 / 10_000) as u64;
    let amount_after_fee = amount_in.saturating_sub(fee_amount);
    
    // Rough estimate: assume average liquidity and calculate price impact
    // This is intentionally conservative to avoid underestimating
    if market.liquidity == 0 {
        return 0;
    }
    
    // Estimate price movement as a percentage
    let price_impact_estimate = (amount_after_fee as f64) / (market.liquidity as f64);
    
    // Convert to ticks (very rough - assumes ~0.01% per tick for common tick spacings)
    let estimated_ticks = (price_impact_estimate * 10000.0 / (market.tick_spacing as f64)) as i32;
    
    // Add safety margin
    estimated_ticks.saturating_mul(2).max(10)
}

/// Calculate swap output with coverage validation
///
/// This enhanced version checks if we have sufficient tick array coverage
/// and returns an error if the swap would exceed our cached data.
fn calculate_swap_output_with_coverage_check(
    market: &Market,
    tick_arrays: &AHashMap<i32, TickArrayView>,
    amount_in: u64,
    is_token_0_to_1: bool,
    estimated_ticks: i32,
) -> Result<(u64, u64)> {
    // Check if we have sufficient coverage
    let arrays_needed = (estimated_ticks / TICK_ARRAY_SIZE).max(1) + 1;
    let arrays_loaded = tick_arrays.len() as i32;
    
    if arrays_needed > arrays_loaded {
        anyhow::bail!(
            "Insufficient tick array coverage for large quote. Need {} arrays but only have {}. \
             Consider reducing quote size or updating more tick arrays.",
            arrays_needed,
            arrays_loaded
        );
    }
    
    // Delegate to original calculation with coverage tracking
    calculate_swap_output_with_tracking(market, tick_arrays, amount_in, is_token_0_to_1)
}

/// Calculate swap output using Feels concentrated liquidity math
///
/// This function simulates a swap through the concentrated liquidity engine,
/// crossing ticks and applying liquidity changes as needed to determine the
/// final output amount and total fees.
fn calculate_swap_output_with_tracking(
    market: &Market,
    tick_arrays: &AHashMap<i32, TickArrayView>,
    amount_in: u64,
    is_token_0_to_1: bool,
) -> Result<(u64, u64)> {
    // Calculate base fee from input amount
    let fee_bps = market.base_fee_bps as u64;
    let fee_amount = (amount_in as u128 * fee_bps as u128 / 10_000) as u64;
    let amount_after_fee = amount_in.saturating_sub(fee_amount);
    
    // Initialize swap state variables
    let mut liquidity = market.liquidity;
    let mut sqrt_price = market.sqrt_price;
    let mut current_tick = market.current_tick;
    let tick_spacing = market.tick_spacing as i32;
    let mut remaining_input = amount_after_fee;
    let mut total_output: u128 = 0;

    // Validate market has sufficient liquidity and valid price
    ensure!(liquidity > 0 && sqrt_price > 0, "Insufficient liquidity or invalid price");

    // Track arrays searched to detect coverage limits
    let mut arrays_searched = 0;
    
    // Execute swap simulation across ticks
    while remaining_input > 0 {
        // Find next initialized tick in the swap direction
        let (next_tick, searched) = next_initialized_tick_with_coverage(
            tick_arrays, 
            current_tick, 
            tick_spacing, 
            is_token_0_to_1
        )?;
        arrays_searched += searched;
        
        // Check if we've hit coverage limits
        if next_tick.is_none() && searched >= 3 {
            anyhow::bail!(
                "Reached tick array coverage limit. Quote may be incomplete. \
                 Arrays searched: {}, Current tick: {}, Direction: {}",
                arrays_searched,
                current_tick,
                if is_token_0_to_1 { "0->1" } else { "1->0" }
            );
        }
        
        let target_tick = next_tick.unwrap_or(if is_token_0_to_1 { i32::MIN / 2 } else { i32::MAX / 2 });

        // Calculate target sqrt price for the tick boundary
        let target_sqrt_price = if let Some(tick) = next_tick {
            U128::from(orca_whirlpools_core::tick_index_to_sqrt_price(tick))
        } else {
            // No more initialized ticks - consume remaining input within current range
            let new_sqrt_price = if is_token_0_to_1 {
                try_get_next_sqrt_price_from_a(U128::from(sqrt_price), U128::from(liquidity), remaining_input, true)
            } else {
                try_get_next_sqrt_price_from_b(U128::from(sqrt_price), U128::from(liquidity), remaining_input, true)
            }.map_err(|e| anyhow::anyhow!("Price calculation error: {:?}", e))?;
            
            let segment_output = if is_token_0_to_1 {
                try_get_amount_delta_b(new_sqrt_price, U128::from(sqrt_price), U128::from(liquidity), false)
            } else {
                try_get_amount_delta_a(U128::from(sqrt_price), new_sqrt_price, U128::from(liquidity), false)
            }.map_err(|e| anyhow::anyhow!("Output calculation error: {:?}", e))?;
            
            total_output = total_output.saturating_add(segment_output as u128);
            break;
        };

        // Calculate input needed to reach the target tick
        let input_to_target = if is_token_0_to_1 {
            try_get_amount_delta_a(target_sqrt_price, U128::from(sqrt_price), U128::from(liquidity), false)
        } else {
            try_get_amount_delta_b(U128::from(sqrt_price), target_sqrt_price, U128::from(liquidity), false)
        }.map_err(|e| anyhow::anyhow!("Input calculation error: {:?}", e))?;

        if remaining_input as u128 >= input_to_target as u128 {
            // We can cross the tick - calculate output for this segment
            let segment_output = if is_token_0_to_1 {
                try_get_amount_delta_b(target_sqrt_price, U128::from(sqrt_price), U128::from(liquidity), false)
            } else {
                try_get_amount_delta_a(U128::from(sqrt_price), target_sqrt_price, U128::from(liquidity), false)
            }.map_err(|e| anyhow::anyhow!("Output calculation error: {:?}", e))?;
            
            total_output = total_output.saturating_add(segment_output as u128);
            remaining_input = remaining_input.saturating_sub(u64::try_from(input_to_target).unwrap_or(u64::MAX));
            sqrt_price = target_sqrt_price.into();
            current_tick = target_tick;
            
            // Apply liquidity change at crossed tick (Uniswap V3 convention)
            let liquidity_net = liquidity_net_at(tick_arrays, current_tick).unwrap_or(0);
            if is_token_0_to_1 {
                // Crossing down: L_new = L - liquidity_net
                if liquidity_net >= 0 {
                    liquidity = liquidity.saturating_sub(liquidity_net as u128);
                } else {
                    liquidity = liquidity.saturating_add((-liquidity_net) as u128);
                }
            } else {
                // Crossing up: L_new = L + liquidity_net
                if liquidity_net >= 0 {
                    liquidity = liquidity.saturating_add(liquidity_net as u128);
                } else {
                    liquidity = liquidity.saturating_sub((-liquidity_net) as u128);
                }
            }
            
            // Stop if no liquidity remains
            if liquidity == 0 { break; }
        } else {
            // Partial fill within current tick range
            let new_sqrt_price = if is_token_0_to_1 {
                try_get_next_sqrt_price_from_a(U128::from(sqrt_price), U128::from(liquidity), remaining_input, true)
            } else {
                try_get_next_sqrt_price_from_b(U128::from(sqrt_price), U128::from(liquidity), remaining_input, true)
            }.map_err(|e| anyhow::anyhow!("Price calculation error: {:?}", e))?;
            
            let segment_output = if is_token_0_to_1 {
                try_get_amount_delta_b(new_sqrt_price, U128::from(sqrt_price), U128::from(liquidity), false)
            } else {
                try_get_amount_delta_a(U128::from(sqrt_price), new_sqrt_price, U128::from(liquidity), false)
            }.map_err(|e| anyhow::anyhow!("Output calculation error: {:?}", e))?;
            
            total_output = total_output.saturating_add(segment_output as u128);
            break;
        }
    }
    // Calculate impact fee based on price movement (ticks crossed)
    let ticks_moved = (current_tick - market.current_tick).abs() as u16;
    let impact_bps: u16 = ticks_moved.min(2500); // Cap at 25% impact fee
    let impact_fee = (total_output.saturating_mul(impact_bps as u128) / 10_000u128) as u64;
    
    // Calculate final net output after impact fee
    let net_output = u64::try_from(total_output).unwrap_or(u64::MAX).saturating_sub(impact_fee);
    let total_fees = fee_amount.saturating_add(impact_fee);
    
    Ok((net_output, total_fees))
}

// =============================================================================
// TICK ARRAY UTILITIES
// =============================================================================

/// Derive the PDA address for a tick array account
///
/// Tick arrays are PDAs derived from the market key and starting tick index.
pub(crate) fn derive_tick_array(market: &Pubkey, start_tick_index: i32) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"tick_array",
            market.as_ref(),
            &start_tick_index.to_le_bytes(),
        ],
        &feels::ID,
    ).0
}

/// Calculate the number of ticks covered by one tick array
pub(crate) fn ticks_per_array(tick_spacing: u16) -> i32 { 
    (TICK_ARRAY_SIZE as i32) * tick_spacing as i32 
}

/// Calculate the starting tick index for the array containing the given tick
pub(crate) fn array_start_for_tick(tick: i32, tick_spacing: u16) -> i32 {
    let ticks_per_array = ticks_per_array(tick_spacing);
    tick.div_euclid(ticks_per_array) * ticks_per_array
}

/// Derive tick array addresses around the current tick for quote calculations
///
/// This generates addresses for the current array plus `range` arrays in each direction.
pub(crate) fn derive_tick_arrays_for_quote(market: &Pubkey, current_tick: i32, tick_spacing: u16, range: i32) -> Vec<Pubkey> {
    let mut arrays = Vec::new();
    let start_tick = array_start_for_tick(current_tick, tick_spacing);
    let ticks_per_array = ticks_per_array(tick_spacing);
    
    // Add the current array
    arrays.push(derive_tick_array(market, start_tick));
    
    // Add arrays in both directions
    for i in 1..=range {
        arrays.push(derive_tick_array(market, start_tick + i * ticks_per_array));
        arrays.push(derive_tick_array(market, start_tick - i * ticks_per_array));
    }
    
    arrays
}

/// Parse tick array account data into a TickArrayView
///
/// This function deserializes the on-chain tick array data and extracts
/// only the initialized ticks for efficient lookup during quotes.
pub(crate) fn parse_tick_array(data: &[u8], tick_spacing: u16) -> Result<TickArrayView> {
    // Calculate expected size: discriminator(8) + market(32) + start_tick_index(4) + pad0(12) + 
    // ticks(80*64) + initialized_tick_count(2) + pad1(14) + reserved(32)
    const EXPECTED_LEN: usize = 8 + 32 + 4 + 12 + (80 * TICK_ARRAY_SIZE as usize) + 2 + 14 + 32;
    
    // Validate exact size match
    anyhow::ensure!(
        data.len() == EXPECTED_LEN, 
        "Invalid tick array data length: expected {}, got {}", 
        EXPECTED_LEN, 
        data.len()
    );
    
    // Validate discriminator (Anchor uses 8-byte discriminator)
    const TICK_ARRAY_DISCRIMINATOR: [u8; 8] = [249, 200, 168, 102, 16, 153, 250, 241];
    let discriminator = &data[..8];
    anyhow::ensure!(
        discriminator == TICK_ARRAY_DISCRIMINATOR,
        "Invalid tick array discriminator"
    );
    
    // Skip Anchor discriminator and parse header
    let mut cursor = &data[8..];
    let _market = Pubkey::new_from_array(<[u8;32]>::try_from(&cursor[..32]).unwrap());
    cursor = &cursor[32..];
    let start_tick_index = i32::from_le_bytes(cursor[..4].try_into().unwrap());
    
    // Calculate offset to tick data (skip padding)
    let ticks_offset = 32 + 4 + 12; // market + start_tick + padding
    let ticks_bytes = &data[8 + ticks_offset..];
    
    // Parse individual tick entries
    let mut initialized_ticks = AHashMap::new();
    let tick_entry_size = 80usize; // Size of each tick entry in bytes
    
    // Validate we have enough data for all ticks
    let expected_ticks_size = (TICK_ARRAY_SIZE as usize) * tick_entry_size;
    anyhow::ensure!(
        ticks_bytes.len() >= expected_ticks_size,
        "Insufficient tick data: expected at least {} bytes, got {}",
        expected_ticks_size,
        ticks_bytes.len()
    );
    
    for i in 0..TICK_ARRAY_SIZE {
        let offset = (i as usize) * tick_entry_size;
        let tick_bytes = &ticks_bytes[offset..offset + tick_entry_size];
        
        // Extract liquidity_net and initialized flag
        let liquidity_net = i128::from_le_bytes(tick_bytes[0..16].try_into().unwrap());
        let initialized = tick_bytes[64]; // Initialized flag at byte 64
        
        if initialized != 0 {
            let tick_index = start_tick_index + (i as i32) * tick_spacing as i32;
            initialized_ticks.insert(tick_index, liquidity_net);
        }
    }
    
    Ok(TickArrayView { 
        start_tick_index, 
        inits: initialized_ticks 
    })
}

/// Find the next initialized tick with coverage tracking
///
/// Returns the next initialized tick and the number of arrays searched.
/// This allows callers to detect when coverage limits are reached.
pub(crate) fn next_initialized_tick_with_coverage(
    arrays: &AHashMap<i32, TickArrayView>,
    current_tick: i32,
    tick_spacing: i32,
    zero_for_one: bool,
) -> Result<(Option<i32>, i32)> {
    let ticks_per_array = TICK_ARRAY_SIZE * tick_spacing;
    let start_array_index = current_tick.div_euclid(ticks_per_array) * ticks_per_array;
    let mut array_index = start_array_index;
    let mut arrays_searched = 0;
    
    // Search through available arrays
    while arrays_searched < arrays.len() as i32 {
        if let Some(view) = arrays.get(&array_index) {
            arrays_searched += 1;
            
            if zero_for_one {
                // Search downward from current_tick - tick_spacing
                let mut tick = current_tick - tick_spacing;
                while tick >= view.start_tick_index {
                    if view.inits.contains_key(&tick) {
                        return Ok((Some(tick), arrays_searched));
                    }
                    tick -= tick_spacing;
                }
            } else {
                // Search upward from current_tick + tick_spacing
                let mut tick = current_tick + tick_spacing;
                let array_end = view.start_tick_index + (TICK_ARRAY_SIZE * tick_spacing);
                while tick < array_end {
                    if view.inits.contains_key(&tick) {
                        return Ok((Some(tick), arrays_searched));
                    }
                    tick += tick_spacing;
                }
            }
        } else {
            // Array not loaded - we've hit coverage limit
            break;
        }
        
        // Move to next array in the search direction
        array_index += if zero_for_one { -ticks_per_array } else { ticks_per_array };
    }
    
    Ok((None, arrays_searched))
}

/// Get the liquidity_net value for a specific tick
///
/// Searches through cached tick arrays to find the liquidity change
/// at the specified tick index.
pub(crate) fn liquidity_net_at(arrays: &AHashMap<i32, TickArrayView>, tick_index: i32) -> Option<i128> {
    for view in arrays.values() {
        if let Some(&liquidity_net) = view.inits.get(&tick_index) {
            return Some(liquidity_net);
        }
    }
    None
}

/// Derive the protocol treasury token account for a given mint
///
/// This function derives the associated token account for the protocol treasury.
/// In a production implementation, this would fetch the treasury pubkey from
/// the protocol config account. For now, we use a deterministic derivation.
pub(crate) fn derive_protocol_treasury_token_account(mint: &Pubkey) -> Pubkey {
    // Derive the protocol treasury authority (would come from protocol config)
    let (treasury_authority, _) = Pubkey::find_program_address(
        &[b"treasury"],
        &feels::ID,
    );
    
    // Derive the associated token account for this mint
    spl_associated_token_account::get_associated_token_address(
        &treasury_authority,
        mint
    )
}
