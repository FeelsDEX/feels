//! Test context that provides a unified interface for testing

use super::*;
use crate::common::client::{InMemoryClient, DevnetClient};
use crate::common::helpers::TestMarketSetup;
use std::sync::Arc;
use tokio::sync::Mutex;
use solana_program::program_pack::Pack;

/// Pre-configured test accounts
pub struct TestAccounts {
    pub alice: Keypair,
    pub bob: Keypair,
    pub charlie: Keypair,
    pub market_creator: Keypair,
    pub fee_collector: Keypair,
}

impl TestAccounts {
    pub fn new() -> Self {
        Self {
            alice: Keypair::new(),
            bob: Keypair::new(),
            charlie: Keypair::new(),
            market_creator: Keypair::new(),
            fee_collector: Keypair::new(),
        }
    }
}

/// Main test context that provides all test functionality
pub struct TestContext {
    pub client: Arc<Mutex<TestClient>>,
    pub accounts: TestAccounts,
    pub environment: TestEnvironment,
    pub feelssol_mint: Pubkey,
    pub jitosol_mint: Pubkey,
    pub feelssol_authority: Keypair,
    pub jitosol_authority: Keypair,
}

impl TestContext {
    /// Create a new test context for the given environment
    pub async fn new(environment: TestEnvironment) -> TestResult<Self> {
        let client = match &environment {
            TestEnvironment::InMemory => {
                TestClient::InMemory(InMemoryClient::new().await?)
            }
            TestEnvironment::Devnet { url, payer_path } => {
                TestClient::Devnet(DevnetClient::new(url, payer_path.as_deref()).await?)
            }
            TestEnvironment::Localnet { url, payer_path } => {
                TestClient::Devnet(DevnetClient::new(url, payer_path.as_deref()).await?)
            }
        };

        // Create test token mints
        let jitosol_authority = Keypair::new();
        let feelssol_authority = Keypair::new();
        let jitosol_mint = constants::JITOSOL_MINT;
        let feelssol_mint = Keypair::new(); // Will be created as a mint
        
        let mut ctx = Self {
            client: Arc::new(Mutex::new(client)),
            accounts: TestAccounts::new(),
            environment,
            feelssol_mint: feelssol_mint.pubkey(),
            jitosol_mint,
            feelssol_authority,
            jitosol_authority,
        };

        // Fund test accounts
        ctx.setup_test_accounts().await?;
        
        // Create FeelsSOL mint
        ctx.create_feelssol_mint(&feelssol_mint).await?;

        Ok(ctx)
    }

    /// Setup and fund test accounts
    async fn setup_test_accounts(&mut self) -> TestResult<()> {
        let accounts_to_fund = vec![
            &self.accounts.alice,
            &self.accounts.bob,
            &self.accounts.charlie,
            &self.accounts.market_creator,
            &self.accounts.fee_collector,
        ];

        for account in accounts_to_fund {
            self.airdrop(&account.pubkey(), constants::DEFAULT_AIRDROP).await?;
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    
    /// Create FeelsSOL mint
    async fn create_feelssol_mint(&self, feelssol_mint: &Keypair) -> TestResult<()> {
        let payer_pubkey = self.payer().await;
        
        // Calculate rent for mint account
        let rent = solana_program::sysvar::rent::Rent::default();
        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

        let instructions = vec![
            solana_sdk::system_instruction::create_account(
                &payer_pubkey,
                &feelssol_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &feelssol_mint.pubkey(),
                &self.feelssol_authority.pubkey(),
                None,
                constants::FEELSSOL_DECIMALS,
            )?,
        ];

        // Get the payer keypair from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => {
                client.payer.insecure_clone()
            }
            TestClient::Devnet(client) => {
                client.payer.insecure_clone()
            }
        };
        
        self.process_transaction(&instructions, &[&payer, feelssol_mint]).await?;
        
        Ok(())
    }
    
    /// Mint a protocol token using the mint_token instruction
    pub async fn mint_protocol_token(
        &self,
        token_name: &str,
        _decimals: u8,
        _initial_supply: u64, // No longer used - all tokens go to buffer
    ) -> TestResult<Keypair> {
        use feels_sdk as sdk;
        use feels::instructions::MintTokenParams;
        
        // Create a fresh creator account (unfunded)
        let creator = Keypair::new();
        
        // Fund the creator account with minimum SOL for transaction fees
        self.airdrop(&creator.pubkey(), 10_000_000).await?; // 0.01 SOL
        
        // Create the token mint keypair
        let token_mint = Keypair::new();
        println!("Minting protocol token {} at {}", token_name, token_mint.pubkey());
        
        // Create parameters for mint_token instruction
        let params = MintTokenParams {
            ticker: token_name.to_string(),
            name: token_name.to_string(),
            uri: "https://test.com".to_string(),
        };
        
        // Create the mint_token instruction
        let ix = sdk::mint_token(
            creator.pubkey(),
            token_mint.pubkey(),
            self.feelssol_mint,
            params,
        )?;
        
        // Process the instruction
        self.process_instruction(ix, &[&creator, &token_mint]).await?;
        
        Ok(token_mint)
    }

    /// Get the payer pubkey
    pub async fn payer(&self) -> Pubkey {
        self.client.lock().await.payer()
    }

    /// Process an instruction using SDK
    pub async fn process_instruction(
        &self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> TestResult<()> {
        self.client.lock().await.process_instruction(instruction, signers).await
    }

    /// Process multiple instructions
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        self.client.lock().await.process_transaction(instructions, signers).await
    }

    /// Get account data
    pub async fn get_account<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> TestResult<Option<T>> {
        self.client.lock().await.get_account(address).await
    }

    /// Get token balance
    pub async fn get_token_balance(&self, address: &Pubkey) -> TestResult<u64> {
        self.client.lock().await.get_token_balance(address).await
    }

    /// Airdrop SOL
    pub async fn airdrop(&self, to: &Pubkey, lamports: u64) -> TestResult<()> {
        self.client.lock().await.airdrop(to, lamports).await
    }

    /// Advance time
    pub async fn advance_time(&self, seconds: i64) -> TestResult<()> {
        self.client.lock().await.advance_time(seconds).await
    }

    /// Get current slot
    pub async fn get_slot(&self) -> TestResult<u64> {
        self.client.lock().await.get_slot().await
    }

    /// Create a new SPL token mint
    pub async fn create_mint(&self, authority: &Pubkey, decimals: u8) -> TestResult<Keypair> {
        let mint = Keypair::new();
        let payer_pubkey = self.payer().await;
        
        // Calculate rent for mint account
        let rent = solana_program::sysvar::rent::Rent::default();
        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);


        let instructions = vec![
            solana_sdk::system_instruction::create_account(
                &payer_pubkey,
                &mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                authority,
                None,
                decimals,
            )?,
        ];

        // Get the payer keypair from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => {
                // For in-memory tests, we need to use the payer from the client
                client.payer.insecure_clone()
            }
            TestClient::Devnet(_) => {
                // For devnet tests, use the market creator as payer
                self.accounts.market_creator.insecure_clone()
            }
        };
        
        self.process_transaction(&instructions, &[&payer, &mint]).await?;
        
        Ok(mint)
    }

    /// Create associated token account
    pub async fn create_ata(&self, owner: &Pubkey, mint: &Pubkey) -> TestResult<Pubkey> {
        let ata = spl_associated_token_account::get_associated_token_address(owner, mint);
        
        // Check if already exists by trying to get the raw account data
        if let Some(data) = self.client.lock().await.get_account_data(&ata).await? {
            if data.len() == TokenAccount::LEN {
                return Ok(ata);
            }
        }

        let payer_pubkey = self.payer().await;
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer_pubkey,
            owner,
            mint,
            &spl_token::id(),
        );

        // Get the payer keypair from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => {
                client.payer.insecure_clone()
            }
            TestClient::Devnet(client) => {
                client.payer.insecure_clone()
            }
        };
        
        self.process_instruction(ix, &[&payer]).await?;
        
        Ok(ata)
    }

    /// Mint tokens to an account
    pub async fn mint_to(
        &self,
        mint: &Pubkey,
        to: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> TestResult<()> {
        let ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            to,
            &authority.pubkey(),
            &[],
            amount,
        )?;

        self.process_instruction(ix, &[authority]).await
    }

    /// Get raw account data
    pub async fn get_account_raw(&self, address: &Pubkey) -> TestResult<solana_sdk::account::Account> {
        let data = self.client.lock().await.get_account_data(address).await?;
        match data {
            Some(data) => Ok(solana_sdk::account::Account {
                lamports: 0, // Placeholder
                data,
                owner: PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            }),
            None => Err("Account not found".into()),
        }
    }

    /// Get token account
    pub async fn get_token_account(&self, address: &Pubkey) -> TestResult<TokenAccount> {
        let account_data = self.client.lock().await.get_account_data(address).await?
            .ok_or("Token account not found")?;
        TokenAccount::unpack(&account_data).map_err(|e| e.into())
    }

    /// Get mint account
    pub async fn get_mint(&self, address: &Pubkey) -> TestResult<spl_token::state::Mint> {
        let account_data = self.client.lock().await.get_account_data(address).await?
            .ok_or("Mint not found")?;
        spl_token::state::Mint::unpack(&account_data).map_err(|e| e.into())
    }

    // Helper method builders
    pub fn market_builder(&self) -> MarketBuilder {
        MarketBuilder::new(self.clone())
    }
    
    /// Create a test market with FeelsSOL and a custom token (convenience method)
    pub async fn create_test_market(&self, token_decimals: u8) -> TestResult<TestMarketSetup> {
        self.market_helper().create_test_market_with_feelssol(token_decimals).await
    }
    
    /// Create a test market with initial liquidity (convenience method)
    pub async fn create_test_market_with_liquidity(
        &self,
        token_decimals: u8,
        liquidity_provider: &Keypair,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount: u128,
    ) -> TestResult<TestMarketSetup> {
        self.market_helper().create_test_market_with_liquidity(
            token_decimals,
            liquidity_provider,
            lower_tick,
            upper_tick,
            liquidity_amount,
        ).await
    }

    // Instruction wrapper methods
    // pub async fn mint_token(
    //     &self,
    //     token_mint: &Keypair,
    //     creator_token: &Pubkey,
    //     buffer_pda: &Pubkey,
    //     buffer_token_vault: &Pubkey,
    //     buffer_feelssol_vault: &Pubkey,
    //     buffer_authority: &Pubkey,
    //     params: feels::instructions::MintTokenParams,
    // ) -> TestResult<()> {
    //     // This instruction is not yet available in the SDK
    //     todo!("mint_token instruction not implemented")
    // }

    // pub async fn deploy_initial_liquidity(
    //     &self,
    //     market: &MarketInfo,
    //     deployer: &Keypair,
    //     buffer_pda: &Pubkey,
    //     buffer_authority: &Pubkey,
    //     params: feels::instructions::DeployInitialLiquidityParams,
    //     tick_array_lower: &Pubkey,
    //     tick_array_upper: &Pubkey,
    // ) -> TestResult<()> {
    //     // For testing purposes, just simulate success
    //     Ok(())
    // }

    pub async fn enter_feelssol(
        &self,
        user: &Keypair,
        user_jitosol: &Pubkey,
        user_feelssol: &Pubkey,
        amount: u64,
    ) -> TestResult<()> {
        let ix = sdk::enter_feelssol(
            user.pubkey(),
            *user_jitosol,
            *user_feelssol,
            self.feelssol_mint,
            self.jitosol_mint,
            amount,
        );
        
        self.process_instruction(ix, &[user]).await
    }

    pub async fn exit_feelssol(
        &self,
        user: &Keypair,
        user_feelssol: &Pubkey,
        user_jitosol: &Pubkey,
        amount: u64,
    ) -> TestResult<()> {
        let ix = sdk::exit_feelssol(
            user.pubkey(),
            *user_feelssol,
            *user_jitosol,
            self.feelssol_mint,
            self.jitosol_mint,
            amount,
        );
        
        self.process_instruction(ix, &[user]).await
    }

    pub async fn get_or_create_tick_array(
        &self,
        market: &Pubkey,
        tick_index: i32,
    ) -> TestResult<Pubkey> {
        // Return a deterministic PDA for the tick array
        let (tick_array, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                market.as_ref(),
                &tick_index.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        Ok(tick_array)
    }
    
    /// Initialize a market
    pub async fn initialize_market(
        &self,
        creator: &Keypair,
        token_0: &Pubkey,
        token_1: &Pubkey,
        fee_tier: u16,
        tick_spacing: u16,
        initial_sqrt_price: u128,
        initial_buy_feelssol_amount: u64,
    ) -> TestResult<Pubkey> {
        // Get creator's token accounts if doing initial buy
        let (creator_feelssol, creator_token_out) = if initial_buy_feelssol_amount > 0 {
            let is_token_0_feelssol = token_0 == &self.feelssol_mint;
            let feelssol_account = self.create_ata(&creator.pubkey(), &self.feelssol_mint).await?;
            let token_out_account = if is_token_0_feelssol {
                self.create_ata(&creator.pubkey(), token_1).await?
            } else {
                self.create_ata(&creator.pubkey(), token_0).await?
            };
            (Some(feelssol_account), Some(token_out_account))
        } else {
            (None, None)
        };
        
        let ix = sdk::initialize_market(
            creator.pubkey(),
            *token_0,
            *token_1,
            self.feelssol_mint,
            fee_tier,
            tick_spacing,
            initial_sqrt_price,
            initial_buy_feelssol_amount,
            creator_feelssol,
            creator_token_out,
        )?;
        
        self.process_instruction(ix, &[creator]).await?;
        
        let (market, _) = sdk::find_market_address(token_0, token_1);
        Ok(market)
    }
}

// Make TestContext cloneable for helper usage
impl Clone for TestContext {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            accounts: TestAccounts::new(), // Create new accounts (not ideal but works for testing)
            environment: self.environment.clone(),
            feelssol_mint: self.feelssol_mint,
            jitosol_mint: self.jitosol_mint,
            feelssol_authority: Keypair::new(), // Create new keypair
            jitosol_authority: Keypair::new(), // Create new keypair
        }
    }
}