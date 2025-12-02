//! Market creation and management helper

use super::super::*;
use crate::common::sdk_compat;
use feels::state::Market;
use solana_sdk::instruction::{AccountMeta, Instruction};

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

        // Hub-and-spoke model: FeelsSOL must ALWAYS be token_0
        let (ordered_token_0, ordered_token_1) = if *token_0 == self.ctx.feelssol_mint {
            (*token_0, *token_1)
        } else if *token_1 == self.ctx.feelssol_mint {
            (*token_1, *token_0)
        } else {
            // This should never happen given the check above
            return Err("One token must be FeelsSOL".into());
        };

        println!("  Ordered tokens:");
        println!("    token_0: {}", ordered_token_0);
        println!("    token_1: {}", ordered_token_1);

        // Use the SDK's initialize_market instruction
        let params = feels::instructions::InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: 64,
            initial_sqrt_price: 79228162514264337593543950336,
            initial_buy_feelssol_amount: 0,
        };
        let (market_id, _) = sdk_compat::find_market_address(&ordered_token_0, &ordered_token_1);
        let ix = sdk_compat::initialize_market(
            self.ctx.accounts.market_creator.pubkey(),
            market_id,
            ordered_token_0,
            ordered_token_1,
            params,
        );

        println!("  Created instruction, processing...");

        self.ctx
            .process_instruction(ix, &[&self.ctx.accounts.market_creator])
            .await?;

        // Derive market address - use the ordered tokens
        let (market_id, _) = sdk_compat::find_market_address(&ordered_token_0, &ordered_token_1);
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

        let params = feels::instructions::InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: 64,
            initial_sqrt_price: initial_price_q64,
            initial_buy_feelssol_amount: 0,
        };
        let (market_id, _) = sdk_compat::find_market_address(&token_0, &token_1);
        let ix = sdk_compat::initialize_market(
            self.ctx.accounts.market_creator.pubkey(),
            market_id,
            token_0,
            token_1,
            params,
        );

        self.ctx
            .process_instruction(ix, &[&self.ctx.accounts.market_creator])
            .await?;

        let (market_id, _) = sdk_compat::find_market_address(&token_0, &token_1);
        Ok(market_id)
    }

    /// Create a market for a specific token paired with FeelsSOL
    pub async fn create_market_for_token(
        &self,
        token_mint: &Pubkey,
        _token_decimals: u8, // decimals parameter for future use
    ) -> TestResult<TestMarketSetup> {
        // Create market with token paired against FeelsSOL
        let feelssol_mint = self.ctx.feelssol_mint;

        // Determine token order (lower pubkey is token_0)
        let (token_0, token_1) = if token_mint < &feelssol_mint {
            (*token_mint, feelssol_mint)
        } else {
            (feelssol_mint, *token_mint)
        };

        let market_id = self.create_simple_market(&token_0, &token_1).await?;

        // Calculate all the derived addresses
        let (oracle_id, _) = sdk_compat::find_oracle_address(&market_id);
        let (vault_0, _) = sdk_compat::find_vault_address(&market_id, &token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(&market_id, &token_1);
        let (market_authority, _) = sdk_compat::find_market_authority_address(&market_id);
        let (buffer_id, _) = sdk_compat::find_buffer_address(&market_id);
        let (protocol_config, _) = sdk_compat::find_protocol_config_address();
        let protocol_treasury = self.ctx.accounts.protocol_treasury.pubkey();

        Ok(TestMarketSetup {
            market_id,
            market: market_id,
            oracle_id,
            vault_0,
            vault_1,
            market_authority,
            buffer_id,
            protocol_config,
            protocol_treasury,
            feelssol_mint,
            custom_token_mint: *token_mint,
            custom_token_keypair: Keypair::new(), // Placeholder - this might need to be passed in
            token_0,
            token_1,
            token_mint: *token_mint,
        })
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
    pub async fn observe_oracle(&self, _market: &Pubkey) -> TestResult<()> {
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
        let creator = &self.ctx.accounts.market_creator;

        match &self.ctx.environment {
            TestEnvironment::InMemory => {
                // In-memory environment - create simple token without protocol token
                println!("  InMemory: Creating simple test token...");

                // Create a simple token mint with ordering constraint
                // This bypasses the need for Metaplex and ProtocolToken
                let token_mint = self
                    .ctx
                    .create_mint_with_ordering_constraint(
                        &creator.pubkey(),
                        token_decimals,
                        &self.ctx.feelssol_mint,
                    )
                    .await?;

                println!("[OK] Created test token: {}", token_mint.pubkey());

                // For tests, we'll create the market with a simplified approach
                // The initialize_market will get the system program as protocol_token accounts
                let market_id = self.create_feelssol_market(&token_mint.pubkey()).await?;

                // Determine token ordering
                let (token_0, token_1) = if self.ctx.feelssol_mint < token_mint.pubkey() {
                    (self.ctx.feelssol_mint, token_mint.pubkey())
                } else {
                    (token_mint.pubkey(), self.ctx.feelssol_mint)
                };

                // Calculate all the derived addresses
                let (oracle_id, _) = sdk_compat::find_oracle_address(&market_id);
                let (vault_0, _) = sdk_compat::find_vault_address(&market_id, &token_0);
                let (vault_1, _) = sdk_compat::find_vault_address(&market_id, &token_1);
                let (market_authority, _) = sdk_compat::find_market_authority_address(&market_id);
                let (buffer_id, _) = sdk_compat::find_buffer_address(&market_id);
                let (protocol_config, _) = sdk_compat::find_protocol_config_address();
                let protocol_treasury = self.ctx.accounts.protocol_treasury.pubkey();
                let token_mint_pubkey = token_mint.pubkey(); // Get pubkey before move

                Ok(TestMarketSetup {
                    market_id,
                    market: market_id,
                    oracle_id,
                    vault_0,
                    vault_1,
                    market_authority,
                    buffer_id,
                    protocol_config,
                    protocol_treasury,
                    feelssol_mint: self.ctx.feelssol_mint,
                    custom_token_mint: token_mint_pubkey,
                    custom_token_keypair: token_mint,
                    token_0,
                    token_1,
                    token_mint: token_mint_pubkey,
                })
            }
            _ => {
                // For non-in-memory environments, use the same simplified approach
                println!("  Non-InMemory: Creating simple test token...");

                // Create a simple token mint with ordering constraint
                let token_mint = self
                    .ctx
                    .create_mint_with_ordering_constraint(
                        &creator.pubkey(),
                        token_decimals,
                        &self.ctx.feelssol_mint,
                    )
                    .await?;

                println!("[OK] Created test token: {}", token_mint.pubkey());

                // Initialize market using SDK builder
                let (token_0, token_1) = if self.ctx.feelssol_mint < token_mint.pubkey() {
                    (self.ctx.feelssol_mint, token_mint.pubkey())
                } else {
                    (token_mint.pubkey(), self.ctx.feelssol_mint)
                };

                let params = feels::instructions::InitializeMarketParams {
                    base_fee_bps: 30,
                    tick_spacing: 64,
                    initial_sqrt_price: 79228162514264337593543950336u128,
                    initial_buy_feelssol_amount: 0,
                };
                let ix_init = sdk_compat::instructions::initialize_market(
                    creator.pubkey(),
                    token_0,
                    token_1,
                    params,
                )?;
                self.ctx.process_instruction(ix_init, &[creator]).await?;

                // Deploy initial liquidity (staircase) without initial buy
                let (market_id, _) = sdk_compat::find_market_address(&token_0, &token_1);
                let deploy_params = feels::instructions::DeployInitialLiquidityParams {
                    initial_buy_feelssol_amount: 0,
                    tick_step_size: 100,
                };
                let ix_deploy = sdk_compat::instructions::deploy_initial_liquidity(
                    creator.pubkey(),
                    market_id,
                    deploy_params,
                )?;
                self.ctx.process_instruction(ix_deploy, &[creator]).await?;

                // Calculate all the derived addresses
                let (oracle_id, _) = sdk_compat::find_oracle_address(&market_id);
                let (vault_0, _) = sdk_compat::find_vault_address(&market_id, &token_0);
                let (vault_1, _) = sdk_compat::find_vault_address(&market_id, &token_1);
                let (market_authority, _) = sdk_compat::find_market_authority_address(&market_id);
                let (buffer_id, _) = sdk_compat::find_buffer_address(&market_id);
                let (protocol_config, _) = sdk_compat::find_protocol_config_address();
                let protocol_treasury = self.ctx.accounts.protocol_treasury.pubkey();
                let token_mint_pubkey = token_mint.pubkey(); // Get pubkey before move

                Ok(TestMarketSetup {
                    market_id,
                    market: market_id,
                    oracle_id,
                    vault_0,
                    vault_1,
                    market_authority,
                    buffer_id,
                    protocol_config,
                    protocol_treasury,
                    feelssol_mint: self.ctx.feelssol_mint,
                    custom_token_mint: token_mint_pubkey,
                    custom_token_keypair: token_mint,
                    token_0,
                    token_1,
                    token_mint: token_mint_pubkey,
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
        let market_state = self
            .ctx
            .get_account::<Market>(&setup.market_id)
            .await?
            .unwrap();

        // Calculate required token amounts based on current price and tick range
        let sqrt_price_lower = feels::logic::sqrt_price_from_tick(lower_tick).unwrap();
        let sqrt_price_upper = feels::logic::sqrt_price_from_tick(upper_tick).unwrap();
        let sqrt_price_current = market_state.sqrt_price;

        let (amount_0, amount_1) = feels::logic::amounts_from_liquidity(
            sqrt_price_current,
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity_amount,
        )
        .unwrap();

        // Fund the liquidity provider with required tokens
        // For test tokens, we use the market creator as mint authority
        let mint_authority = &self.ctx.accounts.market_creator;
        if amount_0 > 0 {
            self.ctx
                .mint_to(
                    &market_state.token_0,
                    &liquidity_provider.pubkey(),
                    mint_authority,
                    amount_0,
                )
                .await?;
        }
        if amount_1 > 0 {
            self.ctx
                .mint_to(
                    &market_state.token_1,
                    &liquidity_provider.pubkey(),
                    mint_authority,
                    amount_1,
                )
                .await?;
        }

        // Open position using the position helper
        let position_helper = self.ctx.position_helper();
        let position_mint = position_helper
            .open_position(
                &setup.market_id,
                liquidity_provider,
                lower_tick,
                upper_tick,
                liquidity_amount,
            )
            .await?;

        println!(
            "Created position with mint: {} and liquidity: {}",
            position_mint, liquidity_amount
        );

        Ok(setup)
    }

    /// Create a test market with an existing protocol token
    pub async fn create_test_market_with_protocol_token(
        &self,
        creator: &Keypair,
        token_mint: &Keypair,
    ) -> TestResult<TestMarketSetup> {
        println!("Creating test market with protocol token...");

        // Determine token order (FeelsSOL must be token_0)
        let (token_0, token_1) = if self.ctx.feelssol_mint < token_mint.pubkey() {
            (self.ctx.feelssol_mint, token_mint.pubkey())
        } else {
            (token_mint.pubkey(), self.ctx.feelssol_mint)
        };

        // Initialize market
        let params = feels::instructions::InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: 64,
            initial_sqrt_price: 79228162514264337593543950336u128,
            initial_buy_feelssol_amount: 0,
        };
        let (market_id, _) = sdk_compat::find_market_address(&token_0, &token_1);
        let ix_init =
            sdk_compat::initialize_market(creator.pubkey(), market_id, token_0, token_1, params);

        self.ctx.process_instruction(ix_init, &[creator]).await?;

        // Deploy initial liquidity (staircase)
        let (market_id, _) = sdk_compat::find_market_address(&token_0, &token_1);
        let deploy_params = feels::instructions::DeployInitialLiquidityParams {
            tick_step_size: 100,
            initial_buy_feelssol_amount: 0,
        };
        let ix_deploy =
            sdk_compat::deploy_initial_liquidity(creator.pubkey(), market_id, deploy_params);

        self.ctx.process_instruction(ix_deploy, &[creator]).await?;

        // Calculate all the derived addresses
        let (oracle_id, _) = sdk_compat::find_oracle_address(&market_id);
        let (vault_0, _) = sdk_compat::find_vault_address(&market_id, &token_0);
        let (vault_1, _) = sdk_compat::find_vault_address(&market_id, &token_1);
        let (market_authority, _) = sdk_compat::find_market_authority_address(&market_id);
        let (buffer_id, _) = sdk_compat::find_buffer_address(&market_id);
        let (protocol_config, _) = sdk_compat::find_protocol_config_address();
        let protocol_treasury = self.ctx.accounts.protocol_treasury.pubkey();

        Ok(TestMarketSetup {
            market_id,
            market: market_id,
            oracle_id,
            vault_0,
            vault_1,
            market_authority,
            buffer_id,
            protocol_config,
            protocol_treasury,
            feelssol_mint: self.ctx.feelssol_mint,
            custom_token_mint: token_mint.pubkey(),
            custom_token_keypair: Keypair::from_bytes(&token_mint.to_bytes()).unwrap(),
            token_0,
            token_1,
            token_mint: token_mint.pubkey(),
        })
    }
}