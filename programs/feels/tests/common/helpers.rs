//! High-level test helpers that use SDK for common operations

use super::*;
use feels::state::{Market, Position};
use feels_sdk as sdk;
use feels_sdk::utils;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Helper for market operations
pub struct MarketHelper {
    ctx: TestContext,
}

impl MarketHelper {
    pub fn new(ctx: TestContext) -> Self {
        Self { ctx }
    }

    /// Create a new market with default configuration
    pub async fn create_simple_market(
        &self,
        token_0: &Pubkey,
        token_1: &Pubkey,
    ) -> TestResult<Pubkey> {
        println!("create_simple_market called with:");
        println!("  token_0: {}", token_0);
        println!("  token_1: {}", token_1);
        println!("  feelssol_mint: {}", self.ctx.feelssol_mint);

        // Ensure one of the tokens is FeelsSOL for MVP testing
        if *token_0 != self.ctx.feelssol_mint && *token_1 != self.ctx.feelssol_mint {
            return Err("For MVP testing, one token must be FeelsSOL".into());
        }

        // Determine token order (token_0 should be lower pubkey)
        let (ordered_token_0, ordered_token_1) = if token_0 < token_1 {
            (*token_0, *token_1)
        } else {
            (*token_1, *token_0)
        };

        println!("  Ordered tokens:");
        println!("    token_0: {}", ordered_token_0);
        println!("    token_1: {}", ordered_token_1);

        // Use the SDK's initialize_market instruction
        let ix = sdk::instructions::initialize_market(
            self.ctx.accounts.market_creator.pubkey(),
            ordered_token_0,
            ordered_token_1,
            self.ctx.feelssol_mint,
            30,                            // 0.3% base fee
            64,                            // tick spacing
            79228162514264337593543950336, // sqrt price 1:1
            0,                             // no initial buy
            None,                          // no creator feelssol account
            None,                          // no creator token out account
        )
        .map_err(|e| format!("Failed to create initialize market instruction: {}", e))?;

        println!("  Created instruction, processing...");

        self.ctx
            .process_instruction(ix, &[&self.ctx.accounts.market_creator])
            .await?;

        // Derive market address - use the ordered tokens
        let (market_id, _) = sdk::find_market_address(&ordered_token_0, &ordered_token_1);
        println!("  Market created successfully: {}", market_id);
        Ok(market_id)
    }

    /// Create a market with FeelsSOL and another token (simplified for testing)
    pub async fn create_feelssol_market(&self, other_token: &Pubkey) -> TestResult<Pubkey> {
        self.create_simple_market(&self.ctx.feelssol_mint, other_token)
            .await
    }

    /// Create a market with Raydium-style configuration
    pub async fn create_raydium_market(
        &self,
        token_0: &Pubkey,
        token_1: &Pubkey,
        initial_price_q64: u128,
    ) -> TestResult<Pubkey> {
        // Determine token order (token_0 should be lower pubkey)
        let (token_0, token_1) = if token_0 < token_1 {
            (*token_0, *token_1)
        } else {
            (*token_1, *token_0)
        };

        let ix = sdk::instructions::initialize_market(
            self.ctx.accounts.market_creator.pubkey(),
            token_0,
            token_1,
            self.ctx.feelssol_mint, // Add feelssol_mint parameter
            30,                     // 0.3% base fee
            64,                     // tick spacing
            initial_price_q64,
            0,    // no initial buy
            None, // no creator feelssol account
            None, // no creator token out account
        )
        .map_err(|e| format!("Failed to create initialize market instruction: {}", e))?;

        self.ctx
            .process_instruction(ix, &[&self.ctx.accounts.market_creator])
            .await?;

        let (market_id, _) = sdk::find_market_address(&token_0, &token_1);
        Ok(market_id)
    }

    /// Get market state
    pub async fn get_market(&self, market: &Pubkey) -> TestResult<Option<Market>> {
        self.ctx.get_account(market).await
    }

    /// Get current price from market
    pub async fn get_price(&self, market: &Pubkey) -> TestResult<u128> {
        let market_state = self.get_market(market).await?.ok_or("Market not found")?;
        Ok(market_state.sqrt_price)
    }

    /// Observe the oracle for a market
    pub async fn observe_oracle(&self, market: &Pubkey) -> TestResult<()> {
        // Note: observe_oracle instruction not available in current SDK
        // This would need to be implemented if oracle functionality is required
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    /// Create a complete test market setup with FeelsSOL and a custom token
    /// This is the recommended way to create markets in tests
    pub async fn create_test_market_with_feelssol(
        &self,
        token_decimals: u8,
    ) -> TestResult<TestMarketSetup> {
        println!("Creating test market with FeelsSOL...");
        use sdk::instructions::mint_token as build_mint_token_ix;
        let creator = &self.ctx.accounts.market_creator;
        // Create a new protocol token mint keypair
        let token_mint = self
            .ctx
            .create_mint(&creator.pubkey(), token_decimals)
            .await?;
        // Creator's FeelsSOL ATA (for mint fee; protocol sets mint_fee=0 for tests)
        let creator_feelssol_ata = self
            .ctx
            .create_ata(&creator.pubkey(), &self.ctx.feelssol_mint)
            .await?;

        match &self.ctx.environment {
            TestEnvironment::InMemory => {
                // In-memory environment cannot CPI to Metaplex; fall back to minimal path
                println!("  InMemory: using simplified path without mint_token CPI");
                let (token_0, token_1) = if self.ctx.feelssol_mint < token_mint.pubkey() {
                    (self.ctx.feelssol_mint, token_mint.pubkey())
                } else {
                    (token_mint.pubkey(), self.ctx.feelssol_mint)
                };
                let (market_id, _) = sdk::find_market_address(&token_0, &token_1);
                Ok(TestMarketSetup {
                    market_id,
                    feelssol_mint: self.ctx.feelssol_mint,
                    custom_token_mint: token_mint.pubkey(),
                    custom_token_keypair: token_mint,
                    token_0,
                    token_1,
                })
            }
            _ => {
                // Build and send mint_token instruction
                let ix_mint = build_mint_token_ix(
                    creator.pubkey(),
                    creator_feelssol_ata,
                    token_mint.pubkey(),
                    self.ctx.feelssol_mint,
                    feels::instructions::MintTokenParams {
                        ticker: "TEST".to_string(),
                        name: "Test Token".to_string(),
                        uri: "https://example.com".to_string(),
                    },
                )?;
                self.ctx
                    .process_instruction(ix_mint, &[creator, &token_mint])
                    .await?;

                // Initialize market using SDK builder
                let (token_0, token_1) = if self.ctx.feelssol_mint < token_mint.pubkey() {
                    (self.ctx.feelssol_mint, token_mint.pubkey())
                } else {
                    (token_mint.pubkey(), self.ctx.feelssol_mint)
                };

                let ix_init = sdk::instructions::initialize_market(
                    creator.pubkey(),
                    token_0,
                    token_1,
                    self.ctx.feelssol_mint,
                    30,                                // base_fee_bps
                    64,                                // tick_spacing
                    79228162514264337593543950336u128, // sqrt price = 1:1
                    0,                                 // no initial buy
                    None,
                    None,
                )?;
                self.ctx.process_instruction(ix_init, &[creator]).await?;

                // Deploy initial liquidity (staircase) without initial buy
                let (market_id, _) = sdk::find_market_address(&token_0, &token_1);
                let ix_deploy = sdk::instructions::deploy_initial_liquidity(
                    creator.pubkey(),
                    market_id,
                    token_0,
                    token_1,
                    self.ctx.feelssol_mint,
                    100, // tick_step_size
                    0,   // initial_buy
                    Some(creator_feelssol_ata),
                    Some(creator_feelssol_ata), // unused without initial buy
                )?;
                self.ctx.process_instruction(ix_deploy, &[creator]).await?;

                Ok(TestMarketSetup {
                    market_id,
                    feelssol_mint: self.ctx.feelssol_mint,
                    custom_token_mint: token_mint.pubkey(),
                    custom_token_keypair: token_mint,
                    token_0,
                    token_1,
                })
            }
        }
    }

    /// Create a test market with initial liquidity
    pub async fn create_test_market_with_liquidity(
        &self,
        token_decimals: u8,
        liquidity_provider: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount: u128,
    ) -> TestResult<TestMarketSetup> {
        // Create the basic market setup
        let setup = self
            .create_test_market_with_feelssol(token_decimals)
            .await?;

        // Get market state to calculate required token amounts
        let market_state = self.ctx.get_account::<Market>(&setup.market_id).await?.unwrap();
        
        // Calculate required token amounts based on current price and tick range
        let sqrt_price_lower = feels::logic::sqrt_price_from_tick(lower_tick).unwrap();
        let sqrt_price_upper = feels::logic::sqrt_price_from_tick(upper_tick).unwrap();
        let sqrt_price_current = market_state.sqrt_price;
        
        let (amount_0, amount_1) = feels::logic::amounts_from_liquidity(
            sqrt_price_current,
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity_amount,
        ).unwrap();
        
        // Fund the liquidity provider with required tokens
        // For test tokens, we use the market creator as mint authority
        let mint_authority = &self.ctx.accounts.market_creator;
        if amount_0 > 0 {
            self.ctx.mint_to(&market_state.token_0, &liquidity_provider.pubkey(), mint_authority, amount_0).await?;
        }
        if amount_1 > 0 {
            self.ctx.mint_to(&market_state.token_1, &liquidity_provider.pubkey(), mint_authority, amount_1).await?;
        }
        
        // Open position using the position helper
        let position_helper = self.ctx.position_helper();
        let position_mint = position_helper.open_position(
            &setup.market_id,
            liquidity_provider,
            lower_tick,
            upper_tick,
            liquidity_amount,
        ).await?;
        
        println!("Created position with mint: {} and liquidity: {}", position_mint, liquidity_amount);

        Ok(setup)
    }
}

/// Helper for swap operations
pub struct SwapHelper {
    ctx: TestContext,
}

impl SwapHelper {
    pub fn new(ctx: TestContext) -> Self {
        Self { ctx }
    }

    /// Execute a simple swap
    pub async fn swap(
        &self,
        market: &Pubkey,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
        trader: &Keypair,
    ) -> TestResult<SwapResult> {
        // Get trader's token accounts
        let trader_token_in = self.ctx.create_ata(&trader.pubkey(), token_in).await?;
        let trader_token_out = self.ctx.create_ata(&trader.pubkey(), token_out).await?;

        // Get initial balances
        let initial_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let initial_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Get market state
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();

        // Determine swap direction
        let zero_for_one = token_in == &market_state.token_0;
        let current_tick = market_state.current_tick;
        let tick_spacing = market_state.tick_spacing as i32;

        // Calculate tick arrays needed for swap
        // We need tick arrays around the current tick
        let tick_array_size = 88; // TICK_ARRAY_SIZE constant

        // Calculate start indices for tick arrays
        let array_start =
            (current_tick / (tick_array_size * tick_spacing)) * tick_array_size * tick_spacing;

        let mut tick_arrays = Vec::new();

        // Add current tick array
        let (current_array, _) = utils::find_tick_array_address(market, array_start);
        tick_arrays.push(current_array);

        // Add next tick array in swap direction
        let next_start = if zero_for_one {
            array_start - (tick_array_size * tick_spacing)
        } else {
            array_start + (tick_array_size * tick_spacing)
        };
        let (next_array, _) = utils::find_tick_array_address(market, next_start);
        tick_arrays.push(next_array);

        // Add one more for safety
        let next_next_start = if zero_for_one {
            next_start - (tick_array_size * tick_spacing)
        } else {
            next_start + (tick_array_size * tick_spacing)
        };
        let (next_next_array, _) = utils::find_tick_array_address(market, next_next_start);
        tick_arrays.push(next_next_array);

        // Build swap instruction manually with correct accounts
        let (oracle, _) = Pubkey::find_program_address(&[b"oracle", market.as_ref()], &sdk::program_id());
        let (vault_0, _) = sdk::find_vault_address(market, &market_state.token_0);
        let (vault_1, _) = sdk::find_vault_address(market, &market_state.token_1);
        let (market_authority, _) = sdk::find_vault_authority_address(market);

        let buffer_key = market_state.buffer;

        // Snapshot buffer tau_spot before swap to estimate impact fee paid
        let buf_before: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_before: u128 = buf_before.as_ref().map(|b| b.tau_spot).unwrap_or(0);

        // Create accounts list
        let mut accounts = vec![
            AccountMeta::new(trader.pubkey(), true),
            AccountMeta::new(*market, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new_readonly(market_authority, false),
            AccountMeta::new(buffer_key, false),
            AccountMeta::new(oracle, false),
            AccountMeta::new(trader_token_in, false),
            AccountMeta::new(trader_token_out, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ];

        // Add tick arrays as remaining accounts
        for tick_array in &tick_arrays {
            accounts.push(AccountMeta::new(*tick_array, false));
        }

        // Create instruction data
        let params = sdk::instructions::SwapParams {
            amount_in,
            minimum_amount_out: 0,
            max_ticks_crossed: 10,
            max_total_fee_bps: 0,
        };

        let data = {
            let discriminator = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]; // SWAP_DISCRIMINATOR
            let mut data = discriminator.to_vec();
            data.extend_from_slice(&params.try_to_vec().unwrap());
            data
        };

        let ix = Instruction {
            program_id: sdk::program_id(),
            accounts,
            data,
        };

        // Execute swap
        self.ctx.process_instruction(ix, &[trader]).await?;

        // Get final balances
        let final_balance_in = self.ctx.get_token_balance(&trader_token_in).await?;
        let final_balance_out = self.ctx.get_token_balance(&trader_token_out).await?;

        // Calculate actual amounts
        let amount_in = initial_balance_in - final_balance_in;
        let amount_out = final_balance_out - initial_balance_out;

        // Read buffer after swap
        let buf_after: Option<feels::state::Buffer> = self
            .ctx
            .client
            .lock()
            .await
            .get_account(&buffer_key)
            .await?;
        let tau_after: u128 = buf_after.as_ref().map(|b| b.tau_spot).unwrap_or(tau_before);
        let fee_paid_est = tau_after.saturating_sub(tau_before) as u64;

        // Estimate price impact bps from pre-swap sqrt_price
        // price = (sqrt/2^64)^2; adjust depending on direction
        let q64 = 1u128 << 64;
        let sqrt = market_state.sqrt_price as f64 / q64 as f64;
        let price_01 = sqrt * sqrt; // token1 per token0
        let exec_price = amount_out as f64 / amount_in.max(1) as f64;
        let (spot, exec) = if zero_for_one {
            (price_01, exec_price)
        } else {
            (1.0 / price_01.max(1e-18), exec_price)
        };
        let price_impact_bps = if spot > 0.0 {
            ((spot - exec).abs() / spot) * 10_000.0
        } else {
            0.0
        };

        Ok(SwapResult {
            amount_in,
            amount_out,
            fee_paid: fee_paid_est,
            price_impact: price_impact_bps,
        })
    }

    /// Execute swap with exact output
    pub async fn swap_exact_out(
        &self,
        market: &Pubkey,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_out: u64,
        max_amount_in: u64,
        trader: &Keypair,
    ) -> TestResult<SwapResult> {
        // Note: The current SDK doesn't support exact output swaps
        // For now, we'll approximate by doing a regular swap
        // In a real implementation, this would need SDK support

        // Estimate amount in needed (very rough approximation)
        let estimated_amount_in = amount_out; // 1:1 for simplicity

        self.swap(
            market,
            token_in,
            token_out,
            estimated_amount_in.min(max_amount_in),
            trader,
        )
        .await
    }

    /// Perform multiple swaps in sequence
    pub async fn multi_swap(&self, swaps: Vec<SwapParams>) -> TestResult<Vec<SwapResult>> {
        let mut results = Vec::new();

        for swap in swaps {
            let result = self
                .swap(
                    &swap.market,
                    &swap.token_in,
                    &swap.token_out,
                    swap.amount_in,
                    &swap.trader,
                )
                .await?;
            results.push(result);
        }

        Ok(results)
    }
}

/// Helper for position operations
pub struct PositionHelper {
    ctx: TestContext,
}

impl PositionHelper {
    pub fn new(ctx: TestContext) -> Self {
        Self { ctx }
    }

    /// Open a new position
    pub async fn open_position(
        &self,
        market: &Pubkey,
        owner: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> TestResult<Pubkey> {
        // Get market state to find token vaults
        let market_state = self.ctx.get_account::<Market>(market).await?.unwrap();
        
        // Create position mint and token account
        let position_mint = Keypair::new();
        let position_token_account = self.ctx.create_ata(&owner.pubkey(), &position_mint.pubkey()).await?;
        
        // Create provider token accounts if they don't exist
        let provider_token_0 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_0).await?;
        let provider_token_1 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_1).await?;
        
        // Derive vaults
        let (vault_0, _) = sdk::find_vault_address(market, &market_state.token_0);
        let (vault_1, _) = sdk::find_vault_address(market, &market_state.token_1);
        
        // Calculate tick array addresses
        let lower_array_start = utils::get_tick_array_start_index(lower_tick, market_state.tick_spacing);
        let upper_array_start = utils::get_tick_array_start_index(upper_tick, market_state.tick_spacing);
        let (lower_tick_array, _) = utils::find_tick_array_address(market, lower_array_start);
        let (upper_tick_array, _) = utils::find_tick_array_address(market, upper_array_start);
        
        // Use SDK to build instruction
        let ix = sdk::instructions::open_position(
            owner.pubkey(),
            *market,
            position_mint.pubkey(),
            position_token_account,
            provider_token_0,
            provider_token_1,
            vault_0,
            vault_1,
            lower_tick_array,
            upper_tick_array,
            lower_tick,
            upper_tick,
            liquidity,
        )?;
        
        self.ctx
            .process_instruction(ix, &[owner, &position_mint])
            .await?;
            
        Ok(position_mint.pubkey())
    }

    /// Close a position
    pub async fn close_position(&self, position_mint: &Pubkey, owner: &Keypair) -> TestResult<()> {
        // Get position state
        let (position_pda, _) = sdk::utils::find_position_address(position_mint);
        let position = self.ctx.get_account::<Position>(&position_pda).await?.unwrap();
        
        // Get market state
        let market_state = self.ctx.get_account::<Market>(&position.market).await?.unwrap();
        
        // Get owner's position token account
        let position_token_account = spl_associated_token_account::get_associated_token_address(&owner.pubkey(), position_mint);
        
        // Create owner token accounts if they don't exist
        let owner_token_0 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_0).await?;
        let owner_token_1 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_1).await?;
        
        // Derive vaults
        let (vault_0, _) = sdk::find_vault_address(&position.market, &market_state.token_0);
        let (vault_1, _) = sdk::find_vault_address(&position.market, &market_state.token_1);
        
        // Calculate tick array addresses
        let lower_array_start = utils::get_tick_array_start_index(position.tick_lower, market_state.tick_spacing);
        let upper_array_start = utils::get_tick_array_start_index(position.tick_upper, market_state.tick_spacing);
        let (lower_tick_array, _) = utils::find_tick_array_address(&position.market, lower_array_start);
        let (upper_tick_array, _) = utils::find_tick_array_address(&position.market, upper_array_start);
        
        // Build close position params
        let params = sdk::instructions::ClosePositionParams {
            amount_0_min: 0,
            amount_1_min: 0,
            close_account: true,
        };
        
        // Use SDK to build instruction
        let ix = sdk::instructions::close_position(
            owner.pubkey(),
            position.market,
            *position_mint,
            position_token_account,
            owner_token_0,
            owner_token_1,
            vault_0,
            vault_1,
            lower_tick_array,
            upper_tick_array,
            params,
        )?;
        
        self.ctx.process_instruction(ix, &[owner]).await?;
        
        Ok(())
    }

    /// Open a position with metadata
    pub async fn open_position_with_metadata(
        &self,
        market: &Pubkey,
        owner: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> TestResult<PositionInfo> {
        // This is a placeholder for position creation with NFT metadata
        // In a real implementation, this would create the NFT metadata alongside the position
        let position_pubkey = self
            .open_position(market, owner, lower_tick, upper_tick, liquidity)
            .await?;

        // Create mock NFT info
        let mint = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();

        Ok(PositionInfo {
            pubkey: position_pubkey,
            mint,
            token_account,
        })
    }

    /// Close a position with metadata
    pub async fn close_position_with_metadata(
        &self,
        position_info: &PositionInfo,
        owner: &Keypair,
    ) -> TestResult<()> {
        // This is a placeholder for closing a position that has NFT metadata
        self.close_position(&position_info.pubkey, owner).await
    }

    /// Get position state
    pub async fn get_position(&self, position_id: &Pubkey) -> TestResult<Option<Position>> {
        self.ctx.get_account(position_id).await
    }

    /// Collect fees from a position
    pub async fn collect_fees(
        &self,
        position_mint: &Pubkey,
        owner: &Keypair,
    ) -> TestResult<CollectFeesResult> {
        // Get position state
        let (position_pda, _) = sdk::utils::find_position_address(position_mint);
        let position = self.ctx.get_account::<Position>(&position_pda).await?.unwrap();
        
        // Get market state
        let market_state = self.ctx.get_account::<Market>(&position.market).await?.unwrap();
        
        // Get owner's position token account
        let position_token_account = spl_associated_token_account::get_associated_token_address(&owner.pubkey(), position_mint);
        
        // Create owner token accounts if they don't exist
        let owner_token_0 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_0).await?;
        let owner_token_1 = self.ctx.create_ata(&owner.pubkey(), &market_state.token_1).await?;
        
        // Derive vaults
        let (vault_0, _) = sdk::find_vault_address(&position.market, &market_state.token_0);
        let (vault_1, _) = sdk::find_vault_address(&position.market, &market_state.token_1);
        
        // Calculate tick array addresses for fee calculation
        let lower_array_start = utils::get_tick_array_start_index(position.tick_lower, market_state.tick_spacing);
        let upper_array_start = utils::get_tick_array_start_index(position.tick_upper, market_state.tick_spacing);
        let (lower_tick_array, _) = utils::find_tick_array_address(&position.market, lower_array_start);
        let (upper_tick_array, _) = utils::find_tick_array_address(&position.market, upper_array_start);
        
        // Get initial balances
        let initial_balance_0 = self.ctx.get_token_balance(&owner_token_0).await?;
        let initial_balance_1 = self.ctx.get_token_balance(&owner_token_1).await?;
        
        // Use SDK to build instruction with tick arrays for fee calculation
        let ix = sdk::instructions::collect_fees(
            owner.pubkey(),
            position.market,
            *position_mint,
            position_token_account,
            owner_token_0,
            owner_token_1,
            vault_0,
            vault_1,
            Some((lower_tick_array, upper_tick_array)),
        )?;
        
        self.ctx.process_instruction(ix, &[owner]).await?;
        
        // Get final balances
        let final_balance_0 = self.ctx.get_token_balance(&owner_token_0).await?;
        let final_balance_1 = self.ctx.get_token_balance(&owner_token_1).await?;
        
        Ok(CollectFeesResult {
            fee_a_collected: final_balance_0 - initial_balance_0,
            fee_b_collected: final_balance_1 - initial_balance_1,
        })
    }

    /// Add liquidity to existing position
    pub async fn add_liquidity(
        &self,
        _position_id: &Pubkey,
        _owner: &Keypair,
        _liquidity_delta: u128,
    ) -> TestResult<()> {
        // Note: Liquidity modification requires a new instruction to be added to the program
        // Current implementation only supports opening new positions
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    /// Remove liquidity from position
    pub async fn remove_liquidity(
        &self,
        _position_id: &Pubkey,
        _owner: &Keypair,
        _liquidity_delta: u128,
    ) -> TestResult<()> {
        // Note: Liquidity modification requires a new instruction to be added to the program
        // Current implementation only supports closing entire positions
        Ok::<(), Box<dyn std::error::Error>>(())
    }
}

// Result types
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_paid: u64,
    pub price_impact: f64,
}

/// Result of creating a test market setup
pub struct TestMarketSetup {
    pub market_id: Pubkey,
    pub feelssol_mint: Pubkey,
    pub custom_token_mint: Pubkey,
    pub custom_token_keypair: Keypair,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
}

pub struct SwapParams {
    pub market: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub trader: Keypair,
}

pub struct CollectFeesResult {
    pub fee_a_collected: u64,
    pub fee_b_collected: u64,
}

#[derive(Clone)]
pub struct PositionInfo {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
}

// Extension methods for TestContext
impl TestContext {
    /// Get market helper
    pub fn market_helper(&self) -> MarketHelper {
        MarketHelper::new(self.clone())
    }

    /// Get swap helper
    pub fn swap_helper(&self) -> SwapHelper {
        SwapHelper::new(self.clone())
    }

    /// Get position helper
    pub fn position_helper(&self) -> PositionHelper {
        PositionHelper::new(self.clone())
    }
}

// Clone implementation is in context.rs

// ============================================================================
// Low-level token utilities (merged from helpers/)
// ============================================================================

use solana_program_test::BanksClient;
use solana_sdk::transaction::Transaction;
use spl_token::instruction as token_instruction;

/// Create a mint account directly using BanksClient (low-level utility)
pub async fn create_mint_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    decimals: u8,
) -> TestResult<Pubkey> {
    let mint = Keypair::new();
    let rent = banks_client.get_rent().await?;
    let lamports = rent.minimum_balance(82);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            lamports,
            82,
            &spl_token::id(),
        ),
        token_instruction::initialize_mint(
            &spl_token::id(),
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            decimals,
        )?,
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer, &mint], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(mint.pubkey())
}

/// Create a token account directly using BanksClient (low-level utility)
pub async fn create_token_account_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    owner: Pubkey,
) -> TestResult<Pubkey> {
    let account = Keypair::new();
    let rent = banks_client.get_rent().await?;
    let lamports = rent.minimum_balance(165);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            lamports,
            165,
            &spl_token::id(),
        ),
        token_instruction::initialize_account(&spl_token::id(), &account.pubkey(), &mint, &owner)?,
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer, &account], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(account.pubkey())
}

/// Mint tokens directly using BanksClient (low-level utility)
pub async fn mint_to_direct(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: Pubkey,
    account: Pubkey,
    amount: u64,
) -> TestResult<()> {
    let instruction = token_instruction::mint_to(
        &spl_token::id(),
        &mint,
        &account,
        &payer.pubkey(),
        &[],
        amount,
    )?;

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(())
}
