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
//!
//! IMPORTANT: This adapter uses the SDK's SwapSimulator to ensure quotes
//! exactly match on-chain execution, preventing discrepancies.

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

// TickArrayView is now provided by the SDK
use feels_sdk::TickArrayView;

/// Jupiter AMM adapter for Feels Protocol markets
///
/// This struct implements the Jupiter AMM interface, enabling Feels markets
/// to participate in Jupiter's routing and aggregation. It maintains cached
/// state for efficient quote calculations and generates proper transaction
/// instructions for swap execution.
///
/// IMPORTANT: Fee Account Handling
/// The adapter must correctly handle protocol and creator fee accounts:
/// 1. Protocol treasury must be fetched from the protocol_config account
/// 2. Creator fees are only applicable for protocol-minted tokens
/// 3. Jupiter may provide fee accounts via quote_mint_to_referrer
/// 4. Fallback treasury derivation should only be used when config unavailable
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

impl FeelsAmm {
    /// Calculate swap output using the SDK's SwapSimulator
    /// 
    /// This method ensures quotes exactly match on-chain execution by using
    /// the SDK's SwapSimulator, which contains the authoritative swap logic.
    fn calculate_swap_with_sdk(
        &self,
        amount_in: u64,
        is_token_0_to_1: bool,
        _estimated_ticks: i32,
    ) -> Result<(u64, u64)> {
        // Convert Jupiter adapter state to SDK format
        let market_state = self.to_market_state()?;
        let tick_arrays = self.to_tick_array_loader()?;
        
        // Use SDK's SwapSimulator for authoritative calculation
        let simulator = feels_sdk::SwapSimulator::new(&market_state, &tick_arrays);
        let result = simulator.simulate_swap(amount_in, is_token_0_to_1)
            .map_err(|e| anyhow::anyhow!("Swap simulation failed: {}", e))?;
        
        Ok((result.amount_out, result.fee_amount))
    }
    
    /// Convert adapter state to SDK MarketState format
    fn to_market_state(&self) -> Result<feels_sdk::MarketState> {
        Ok(feels_sdk::MarketState {
            market_key: self.key,
            token_0: self.market.token_0,
            token_1: self.market.token_1,
            sqrt_price: self.market.sqrt_price,
            current_tick: self.market.current_tick,
            liquidity: self.market.liquidity,
            fee_bps: self.market.base_fee_bps,
            tick_spacing: self.market.tick_spacing,
            global_lower_tick: self.market.global_lower_tick,
            global_upper_tick: self.market.global_upper_tick,
            fee_growth_global_0: self.market.fee_growth_global_0_x64,
            fee_growth_global_1: self.market.fee_growth_global_1_x64,
        })
    }
    
    /// Convert adapter tick arrays to SDK TickArrayLoader format
    fn to_tick_array_loader(&self) -> Result<feels_sdk::TickArrayLoader> {
        let mut loader = feels_sdk::TickArrayLoader::new();
        
        // Convert cached tick arrays to SDK format using the shared parser
        for (start_index, view) in &self.tick_arrays {
            // Create ParsedTickArray from our cached data
            let parsed = feels_sdk::ParsedTickArray {
                format: feels_sdk::TickArrayFormat::V1,
                market: self.key,
                start_tick_index: *start_index,
                initialized_ticks: view.inits.clone(),
                initialized_count: Some(view.inits.len() as u16),
            };
            
            loader.add_parsed_array(parsed)
                .map_err(|e| anyhow::anyhow!("Failed to add parsed array: {}", e))?;
        }
        
        Ok(loader)
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
                if let Ok(parsed) = feels_sdk::parse_tick_array_auto(bytes, self.tick_spacing) {
                    let view = feels_sdk::TickArrayView::from(parsed);
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
        
        // Calculate swap output using SDK's swap simulator
        // This ensures consistency with on-chain execution
        let (amount_out, fee_amount) = self.calculate_swap_with_sdk(
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
    /// 
    /// IMPORTANT: Fee Account Requirements:
    /// 1. protocol_treasury must be an ATA owned by protocol_config.treasury
    /// 2. If a protocol token exists, creator_token_account must match protocol_token.creator
    /// 3. Jupiter should provide fee accounts via quote_mint_to_referrer when available
    /// 4. All fee accounts must exist and have correct ownership or swaps will fail
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let SwapParams {
            source_mint,
            destination_mint,
            source_token_account,
            destination_token_account,
            token_transfer_authority,
            // open_order_address field removed in jupiter-amm-interface 0.6
            quote_mint_to_referrer,
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
        
        // Derive protocol config PDA
        let (protocol_config, _) = Pubkey::find_program_address(
            &[b"protocol_config"], 
            &feels::id()
        );
        
        // CRITICAL: Protocol treasury must be the correct ATA
        // The treasury account MUST be an ATA owned by protocol_config.treasury
        // Jupiter can override via quote_mint_to_referrer, but we need a safe default
        
        // Check if Jupiter provided treasury account for the output token
        let protocol_treasury = if let Some(referrer_map) = quote_mint_to_referrer {
            if let Some(treasury) = referrer_map.get(destination_mint) {
                // Jupiter provided treasury - use it (assumed to be validated)
                *treasury
            } else {
                // Jupiter didn't provide treasury for output mint
                // Derive the correct ATA using the known treasury pubkey
                derive_protocol_treasury_ata(destination_mint)
            }
        } else {
            // No referrer map provided - use default derivation
            derive_protocol_treasury_ata(destination_mint)
        };
        
        // Determine if the input token is protocol-minted (eligible for creator fees)
        // For protocol tokens, we need to check both token_0 and token_1
        let is_token_0_protocol = is_protocol_minted_token(&self.market.token_0);
        let is_token_1_protocol = is_protocol_minted_token(&self.market.token_1);
        
        // Determine which token might have creator fees based on swap direction
        let protocol_token_mint = if *source_mint == self.market.token_0 && is_token_0_protocol {
            Some(self.market.token_0)
        } else if *source_mint == self.market.token_1 && is_token_1_protocol {
            Some(self.market.token_1)
        } else {
            None
        };
        
        let (protocol_token, creator_token_account) = if let Some(mint) = protocol_token_mint {
            // Derive the protocol token PDA
            let (protocol_token_pda, _) = Pubkey::find_program_address(
                &[b"protocol_token", mint.as_ref()],
                &feels::id()
            );
            
            // Creator account for output token (where creator fees go)
            // This should ideally be fetched from the protocol_token account
            let creator_account = if let Some(referrer_map) = quote_mint_to_referrer {
                // Check if Jupiter provided creator account for output mint
                referrer_map.get(destination_mint)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Fallback: derive creator ATA (requires knowing creator pubkey)
                        derive_creator_token_account(destination_mint)
                    })
            } else {
                // Use default derivation
                derive_creator_token_account(destination_mint)
            };
            
            (protocol_token_pda, creator_account)
        } else {
            // Not a protocol token - no creator fees
            (Pubkey::default(), Pubkey::default())
        };

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
            
            // Fee distribution accounts
            AccountMeta::new_readonly(protocol_config, false), // protocol_config
            AccountMeta::new(protocol_treasury, false), // protocol_treasury (must be correct ATA)
            
            // Optional protocol token accounts
            if protocol_token != Pubkey::default() {
                AccountMeta::new_readonly(protocol_token, false) // protocol_token
            } else {
                AccountMeta::new_readonly(Pubkey::default(), false) // None placeholder
            },
            if creator_token_account != Pubkey::default() {
                AccountMeta::new(creator_token_account, false) // creator_token_account  
            } else {
                AccountMeta::new_readonly(Pubkey::default(), false) // None placeholder
            },
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
// SWAP CALCULATION FUNCTIONS (REMOVED - NOW USING SDK)
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

// =============================================================================
// TICK ARRAY UTILITIES
// =============================================================================

/// Derive the protocol treasury ATA for a given mint
///
/// The treasury ATA must be owned by protocol_config.treasury
fn derive_protocol_treasury_ata(mint: &Pubkey) -> Pubkey {
    crate::config::get_treasury_ata(mint)
}

/// Derive creator token account for fees
///
/// For protocol tokens, creator fees go to the creator's ATA for the output token
fn derive_creator_token_account(_mint: &Pubkey) -> Pubkey {
    // In production, this requires fetching the ProtocolToken account to get the creator
    // then deriving their ATA for the output mint
    // For now, return a safe default (no creator fees)
    Pubkey::default()
}

/// Check if a token is protocol-minted (eligible for creator fees)
fn is_protocol_minted_token(mint: &Pubkey) -> bool {
    crate::config::is_protocol_token(mint)
}

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

// Tick array parsing is now handled by the SDK

// Removed unused functions - tick array operations are now handled by the SDK

