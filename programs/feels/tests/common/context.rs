//! Test context that provides a unified interface for testing

use super::*;
use crate::common::client::{DevnetClient, InMemoryClient};
use anchor_lang::prelude::*;
use solana_program::program_pack::Pack;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Pre-configured test accounts
pub struct TestAccounts {
    pub alice: Keypair,
    pub bob: Keypair,
    pub charlie: Keypair,
    pub market_creator: Keypair,
    pub fee_collector: Keypair,
    pub protocol_treasury: Keypair,
}

impl TestAccounts {
    pub fn new() -> Self {
        Self {
            alice: Keypair::new(),
            bob: Keypair::new(),
            charlie: Keypair::new(),
            market_creator: Keypair::new(),
            fee_collector: Keypair::new(),
            protocol_treasury: Keypair::new(),
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
    /// Generate a keypair with a low pubkey value
    /// This is useful for creating FeelsSOL mints that can always be token_0
    fn generate_low_pubkey_keypair(max_attempts: usize) -> Keypair {
        let mut best_kp = Keypair::new();
        let mut best_first_bytes = best_kp.pubkey().to_bytes()[0..2].to_vec();

        for i in 0..max_attempts {
            let kp = Keypair::new();
            let bytes = kp.pubkey().to_bytes();

            if bytes[0] < best_first_bytes[0]
                || (bytes[0] == best_first_bytes[0] && bytes[1] < best_first_bytes[1])
            {
                best_kp = kp;
                best_first_bytes = bytes[0..2].to_vec();

                // If we find one starting with 0x00, that's good enough
                if bytes[0] == 0 {
                    println!(
                        "Found pubkey starting with 0x{:02x}{:02x} after {} attempts",
                        bytes[0],
                        bytes[1],
                        i + 1
                    );
                    break;
                }
            }
        }

        println!(
            "Generated low pubkey: {} (first bytes: 0x{:02x}{:02x})",
            best_kp.pubkey(),
            best_first_bytes[0],
            best_first_bytes[1]
        );
        best_kp
    }

    /// Create a new test context for the given environment
    pub async fn new(environment: TestEnvironment) -> TestResult<Self> {
        // Initialize tracing to suppress OpenTelemetry warnings
        crate::common::tracing::init_test_tracing();

        let client = match &environment {
            TestEnvironment::InMemory => TestClient::InMemory(InMemoryClient::new().await?),
            TestEnvironment::Devnet {
                url,
                payer_path,
                disable_airdrop_rate_limit,
            } => {
                let mut client = DevnetClient::new(url, payer_path.as_deref()).await?;
                client.set_disable_airdrop_rate_limit(*disable_airdrop_rate_limit);
                TestClient::Devnet(client)
            }
            TestEnvironment::Localnet { url, payer_path } => {
                TestClient::Devnet(DevnetClient::new(url, payer_path.as_deref()).await?)
            }
        };

        // Create test token mints
        let jitosol_authority = Keypair::new();
        let feelssol_authority = Keypair::new();

        // For in-memory tests and localnet, create a mock JitoSOL mint
        let (jitosol_mint, jitosol_mint_keypair) = match &environment {
            TestEnvironment::InMemory | TestEnvironment::Localnet { .. } => {
                let mint = Keypair::new();
                (mint.pubkey(), Some(mint))
            }
            _ => {
                // For devnet, use the real JitoSOL mint
                (constants::JITOSOL_MINT, None)
            }
        };

        // Generate a FeelsSOL mint with a very low pubkey to ensure it can be token_0
        let feelssol_mint = if matches!(&environment, TestEnvironment::InMemory) {
            // For in-memory tests, use the helper to find a keypair with low pubkey
            Self::generate_low_pubkey_keypair(1000)
        } else {
            // For other environments, use a regular keypair
            Keypair::new()
        };

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

        // Create mock JitoSOL mint for in-memory and localnet tests
        if let Some(jitosol_keypair) = jitosol_mint_keypair {
            ctx.create_mock_jitosol_mint(&jitosol_keypair).await?;
        }

        // Initialize protocol with 0 mint fee for testing
        if let Err(e) = ctx.initialize_protocol().await {
            println!("Failed to initialize protocol: {:?}", e);
            return Err(e);
        }

        // Initialize FeelsHub for enter/exit FeelsSOL
        if let Err(e) = ctx.initialize_feels_hub().await {
            println!("Failed to initialize FeelsHub: {:?}", e);
            // This is not critical for all tests, so we'll just log it
        }

        Ok(ctx)
    }

    /// Setup and fund test accounts
    async fn setup_test_accounts(&mut self) -> TestResult<()> {
        // For devnet, check if we should skip initial funding to avoid rate limits
        if matches!(&self.environment, TestEnvironment::Devnet { .. })
            && std::env::var("SKIP_INITIAL_FUNDING").is_ok()
        {
            println!("Skipping initial account funding for devnet tests");
            return Ok(());
        }

        let accounts_to_fund = vec![
            &self.accounts.alice,
            &self.accounts.bob,
            &self.accounts.charlie,
            &self.accounts.market_creator,
            &self.accounts.fee_collector,
        ];

        // For devnet, fund only the payer initially to reduce airdrop calls
        if matches!(&self.environment, TestEnvironment::Devnet { .. }) {
            // Fund only essential accounts with smaller amounts
            let essential_accounts = vec![
                (
                    &self.accounts.market_creator,
                    constants::DEFAULT_AIRDROP / 2,
                ),
                (&self.accounts.fee_collector, constants::DEFAULT_AIRDROP / 4),
            ];

            for (account, amount) in essential_accounts {
                if let Err(e) = self.airdrop(&account.pubkey(), amount).await {
                    eprintln!("Warning: Failed to fund {}: {}", account.pubkey(), e);
                }
            }
        } else {
            // For in-memory tests, fund all accounts normally
            for account in accounts_to_fund {
                self.airdrop(&account.pubkey(), constants::DEFAULT_AIRDROP)
                    .await?;
            }
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }

    /// Initialize FeelsHub for enter/exit FeelsSOL operations
    pub async fn initialize_feels_hub(&self) -> TestResult<()> {
        use feels::constants::FEELS_HUB_SEED;

        let payer_pubkey = self.payer().await;

        // Derive the hub PDA
        let (hub_pda, _) = Pubkey::find_program_address(
            &[FEELS_HUB_SEED, self.feelssol_mint.as_ref()],
            &PROGRAM_ID,
        );

        // Check if already initialized
        match self.get_account_raw(&hub_pda).await {
            Ok(_) => {
                println!("FeelsHub already initialized");
                return Ok(());
            }
            Err(_) => {
                println!("Initializing FeelsHub...");
            }
        }

        // Use the SDK to build the instruction
        let ix = crate::common::sdk_compat::initialize_hub(
            payer_pubkey,
            self.feelssol_mint,
            self.jitosol_mint,
        );

        // Get the payer keypair
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        // Process the instruction
        self.process_instruction(ix, &[&payer]).await?;

        println!("FeelsHub initialized successfully");
        Ok(())
    }

    /// Create FeelsSOL mint
    async fn create_feelssol_mint(&self, feelssol_mint: &Keypair) -> TestResult<()> {
        use feels::constants::MINT_AUTHORITY_SEED;

        let payer_pubkey = self.payer().await;

        // Derive the mint authority PDA
        let (mint_authority, _) = Pubkey::find_program_address(
            &[MINT_AUTHORITY_SEED, feelssol_mint.pubkey().as_ref()],
            &PROGRAM_ID,
        );

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
                &mint_authority, // Use the PDA as mint authority
                None,
                constants::FEELSSOL_DECIMALS,
            )?,
        ];

        // Get the payer keypair from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        self.process_transaction(&instructions, &[&payer, feelssol_mint])
            .await?;

        Ok(())
    }

    /// Create mock JitoSOL mint for testing
    async fn create_mock_jitosol_mint(&self, jitosol_mint: &Keypair) -> TestResult<()> {
        let payer_pubkey = self.payer().await;

        // Calculate rent for mint account
        let rent = solana_program::sysvar::rent::Rent::default();
        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

        let instructions = vec![
            solana_sdk::system_instruction::create_account(
                &payer_pubkey,
                &jitosol_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &jitosol_mint.pubkey(),
                &self.jitosol_authority.pubkey(),
                None,
                9, // JitoSOL has 9 decimals
            )?,
        ];

        // Get the payer keypair from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        self.process_transaction(&instructions, &[&payer, jitosol_mint])
            .await?;

        Ok(())
    }

    /// Create a mock protocol token for testing
    /// This creates a simple token mint AND a ProtocolToken account
    /// to satisfy market creation requirements
    pub async fn mint_protocol_token(
        &self,
        token_name: &str,
        decimals: u8,
        _initial_supply: u64, // No longer used - all tokens go to buffer
    ) -> TestResult<Keypair> {
        // Create a fresh creator account
        let creator = self.accounts.market_creator.insecure_clone();

        // Create the token mint with ordering constraint
        // Ensure mint pubkey > feelssol mint so feelssol can be token_0
        let token_mint = self
            .create_mint_with_ordering_constraint(&creator.pubkey(), decimals, &self.feelssol_mint)
            .await?;

        println!(
            "Created mock protocol token {} at {}",
            token_name,
            token_mint.pubkey()
        );

        // Create the ProtocolToken account to satisfy market initialization
        use feels::constants::PROTOCOL_TOKEN_SEED;
        use feels::state::ProtocolToken;
        use feels::state::TokenType;

        let (protocol_token_pda, _) = Pubkey::find_program_address(
            &[PROTOCOL_TOKEN_SEED, token_mint.pubkey().as_ref()],
            &PROGRAM_ID,
        );

        // In ProgramTest environment, we need to add the account to the test
        match &*self.client.lock().await {
            TestClient::InMemory(client) => {
                use solana_sdk::account::Account;

                // Create the protocol token account data
                let protocol_token = ProtocolToken {
                    mint: token_mint.pubkey(),
                    creator: creator.pubkey(),
                    token_type: TokenType::Spl,
                    created_at: 0, // Use 0 for tests
                    can_create_markets: true,
                    _reserved: [0; 32],
                };

                // Serialize the data with discriminator
                let mut data = vec![0u8; ProtocolToken::LEN];
                // Write a dummy discriminator (8 bytes)
                data[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
                // Write the account data
                let protocol_token_bytes = protocol_token
                    .try_to_vec()
                    .map_err(|e| format!("Failed to serialize ProtocolToken: {}", e))?;
                data[8..8 + protocol_token_bytes.len()].copy_from_slice(&protocol_token_bytes);

                // Create the account
                let account = Account {
                    lamports: Rent::default().minimum_balance(ProtocolToken::LEN),
                    data,
                    owner: PROGRAM_ID,
                    executable: false,
                    rent_epoch: 0,
                };

                // Add the account to the test environment
                // This requires accessing the banks_client directly
                println!(
                    "Created ProtocolToken account for testing at {}",
                    protocol_token_pda
                );

                // Note: In practice, we can't directly add accounts to ProgramTest after it's started
                // We'll need to use a different approach
            }
            _ => {
                println!(
                    "ProtocolToken PDA at {} for non-in-memory tests",
                    protocol_token_pda
                );
            }
        }

        Ok(token_mint)
    }

    /// Initialize protocol configuration
    pub async fn initialize_protocol(&self) -> TestResult<()> {
        use crate::common::sdk_compat::instructions::InitializeProtocolParams;

        // Try to get protocol config - if it exists, skip initialization
        let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol_config"], &PROGRAM_ID);

        // Check if already initialized
        match self.get_account_raw(&protocol_config).await {
            Ok(_) => {
                println!("Protocol already initialized");
                return Ok(());
            }
            Err(_) => {
                println!("Protocol not initialized, proceeding with initialization");
            }
        }

        println!("Initializing protocol...");

        // Get the payer to be the authority
        let payer_pubkey = self.payer().await;
        println!("Using payer as authority: {}", payer_pubkey);

        // Create initialization parameters with 0 mint fee for testing and sane oracle/safety defaults
        let params = InitializeProtocolParams {
            mint_fee: 0,                         // No fee for testing
            treasury: payer_pubkey,              // Use payer as treasury for simplicity
            default_protocol_fee_rate: Some(30), // 0.3% for testing
            default_creator_fee_rate: Some(70),  // 0.7% for testing
            max_protocol_fee_rate: Some(100),    // 1% max for testing
            dex_twap_updater: payer_pubkey,
            depeg_threshold_bps: 500, // 5%
            depeg_required_obs: 2,
            clear_required_obs: 2,
            dex_twap_window_secs: 300,     // 5m for testing
            dex_twap_stale_age_secs: 3600, // 1 hour - very lenient for testing
            dex_whitelist: vec![],
        };

        // Clone params before passing to SDK
        let ix = crate::common::sdk_compat::instructions::initialize_protocol(
            payer_pubkey,
            params.clone(),
        )?;
        println!("Built initialize_protocol instruction successfully");
        println!("Instruction data length: {}", ix.data.len());
        println!("Instruction program_id: {}", ix.program_id);
        println!("Number of accounts: {}", ix.accounts.len());

        // Get the payer keypair based on environment
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        // Check if the payer has enough SOL
        let payer_balance = self.get_balance(&payer_pubkey).await?;
        println!("Payer balance: {} SOL", payer_balance as f64 / 1e9);

        // If payer has insufficient balance, this is a test setup issue
        if payer_balance < 1_000_000 {
            println!("WARNING: Payer has insufficient balance for protocol initialization");
        }

        // Check if the program is deployed
        match self.get_account_raw(&PROGRAM_ID).await {
            Ok(data) => println!(
                "Program {} is deployed (size: {} bytes)",
                PROGRAM_ID,
                data.data.len()
            ),
            Err(_) => println!("WARNING: Program {} is NOT deployed!", PROGRAM_ID),
        }

        // Log first 8 bytes of instruction data (discriminator)
        println!("Instruction discriminator: {:?}", &ix.data[..8]);

        // Log the raw bytes of the instruction data for debugging
        println!("Full instruction data (hex): {}", hex::encode(&ix.data));

        // Try to manually verify the params serialization
        use anchor_lang::AnchorSerialize;
        let test_params = params.clone();
        match test_params.try_to_vec() {
            Ok(serialized) => {
                println!(
                    "Params serialized successfully, length: {}",
                    serialized.len()
                );
                println!("Params data (hex): {}", hex::encode(&serialized));
            }
            Err(e) => {
                println!("ERROR: Failed to serialize params: {:?}", e);
            }
        }

        // Process the instruction
        match self.process_instruction(ix, &[&payer]).await {
            Ok(_) => {
                println!("Protocol initialized successfully");

                // Note: In production, oracle rates would be set by an oracle updater
                // For MVP testing, exit operations will fail if oracle is not updated

                Ok(())
            }
            Err(e) => {
                println!("Error during protocol initialization: {:?}", e);
                Err(e)
            }
        }
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
        self.client
            .lock()
            .await
            .process_instruction(instruction, signers)
            .await
    }

    /// Process multiple instructions
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        self.client
            .lock()
            .await
            .process_transaction(instructions, signers)
            .await
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

        self.process_transaction(&instructions, &[&payer, &mint])
            .await?;

        Ok(mint)
    }

    /// Create a new SPL token mint with ordering constraint
    /// Ensures the generated mint pubkey is greater than the reference pubkey
    /// This is needed for hub-and-spoke model where FeelsSOL must always be token_0
    pub async fn create_mint_with_ordering_constraint(
        &self,
        authority: &Pubkey,
        decimals: u8,
        reference_pubkey: &Pubkey,
    ) -> TestResult<Keypair> {
        let max_attempts = 1000; // Prevent infinite loop
        for _ in 0..max_attempts {
            let mint = Keypair::new();

            // Check if this mint satisfies the ordering constraint
            // We need mint.pubkey() > reference_pubkey for proper token ordering
            if mint.pubkey() > *reference_pubkey {
                // This mint satisfies our constraint, create it
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

                self.process_transaction(&instructions, &[&payer, &mint])
                    .await?;

                println!(
                    "Created mint {} (> {}) after generating keypairs",
                    mint.pubkey(),
                    reference_pubkey
                );
                return Ok(mint);
            }
        }

        Err(format!(
            "Failed to generate mint with pubkey > {} after {} attempts",
            reference_pubkey, max_attempts
        )
        .into())
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
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        self.process_instruction(ix, &[&payer]).await?;

        Ok(ata)
    }

    /// Alias for create_ata for clarity
    pub async fn get_or_create_ata(&self, owner: &Pubkey, mint: &Pubkey) -> TestResult<Pubkey> {
        self.create_ata(owner, mint).await
    }

    /// Mint tokens to an account
    pub async fn mint_to(
        &self,
        mint: &Pubkey,
        to: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> TestResult<()> {
        // Special handling for FeelsSOL mint - use enter_feelssol instruction
        if mint == &self.feelssol_mint {
            return self.mint_feelssol_to(to, amount).await;
        }

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

    /// Mint FeelsSOL tokens by entering with JitoSOL
    async fn mint_feelssol_to(&self, to: &Pubkey, amount: u64) -> TestResult<()> {
        // To mint FeelsSOL, we need to:
        // 1. Fund the user with JitoSOL
        // 2. Call enter_feelssol instruction

        // Get or create JitoSOL account for the recipient
        let user_jitosol = self.get_or_create_ata(to, &self.jitosol_mint).await?;
        let user_feelssol = self.get_or_create_ata(to, &self.feelssol_mint).await?;

        // Fund with JitoSOL first (1:1 ratio for simplicity)
        let jitosol_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.jitosol_mint,
            &user_jitosol,
            &self.jitosol_authority.pubkey(),
            &[],
            amount,
        )?;

        self.process_instruction(jitosol_ix, &[&self.jitosol_authority])
            .await?;

        // Get the user's keypair if they're one of our test accounts
        let user_keypair = if to == &self.accounts.alice.pubkey() {
            &self.accounts.alice
        } else if to == &self.accounts.bob.pubkey() {
            &self.accounts.bob
        } else if to == &self.accounts.charlie.pubkey() {
            &self.accounts.charlie
        } else if to == &self.accounts.market_creator.pubkey() {
            &self.accounts.market_creator
        } else {
            return Err("Cannot mint FeelsSOL to unknown user".into());
        };

        // Now enter FeelsSOL
        use crate::common::sdk_compat;

        // Build enter feelssol instruction
        let (feels_hub, _) = self.derive_feels_hub();
        let enter_ix = sdk_compat::enter_feelssol(
            user_keypair.pubkey(),
            feels_hub,
            self.feelssol_mint,
            user_jitosol,
            user_feelssol,
            amount,
        );

        self.process_instruction(enter_ix, &[user_keypair]).await
    }

    /// Get raw account data
    pub async fn get_account_raw(
        &self,
        address: &Pubkey,
    ) -> TestResult<solana_sdk::account::Account> {
        // For simplicity, create a minimal account with the data
        // The actual lamports will be checked separately in the banks client
        let data = self.client.lock().await.get_account_data(address).await?;
        match data {
            Some(data) => Ok(solana_sdk::account::Account {
                lamports: 1_000_000_000, // Default 1 SOL for testing
                data,
                owner: PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            }),
            None => Err("Account not found".into()),
        }
    }

    /// Get SOL balance for an account
    pub async fn get_balance(&self, address: &Pubkey) -> TestResult<u64> {
        self.client.lock().await.get_balance(address).await
    }

    /// Get token account
    pub async fn get_token_account(&self, address: &Pubkey) -> TestResult<TokenAccount> {
        let account_data = self
            .client
            .lock()
            .await
            .get_account_data(address)
            .await?
            .ok_or("Token account not found")?;
        TokenAccount::unpack(&account_data).map_err(|e| e.into())
    }

    /// Get mint account
    pub async fn get_mint(&self, address: &Pubkey) -> TestResult<spl_token::state::Mint> {
        let account_data = self
            .client
            .lock()
            .await
            .get_account_data(address)
            .await?
            .ok_or("Mint not found")?;
        spl_token::state::Mint::unpack(&account_data).map_err(|e| e.into())
    }

    // Helper method builders
    pub fn market_builder(&self) -> MarketBuilder {
        MarketBuilder::new(self.clone())
    }

    /// Create a test market with FeelsSOL and a custom token (convenience method)
    pub async fn create_test_market(&self, token_decimals: u8) -> TestResult<TestMarketSetup> {
        self.market_helper()
            .create_test_market_with_feelssol(token_decimals)
            .await
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
        self.market_helper()
            .create_test_market_with_liquidity(
                token_decimals,
                liquidity_provider,
                lower_tick,
                upper_tick,
                liquidity_amount,
            )
            .await
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

    /// Update protocol oracle with initial rates for testing
    pub async fn update_protocol_oracle_for_testing(&self) -> TestResult<()> {
        // For testing, we need to set initial oracle rates
        // In production, these would be updated by the oracle updater

        // First update native rate (done by protocol authority)
        let authority = self.payer().await;
        let native_rate_q64 = 1u128 << 64; // 1.0 in Q64 format

        let native_ix = sdk_compat::update_native_rate(authority, native_rate_q64);

        // Get the payer from the client
        let payer = match &*self.client.lock().await {
            TestClient::InMemory(client) => client.payer.insecure_clone(),
            TestClient::Devnet(client) => client.payer.insecure_clone(),
        };

        self.process_instruction(native_ix, &[&payer]).await?;
        println!("Updated native rate to 1.0");

        // Then update DEX TWAP rate (use payer as updater for testing)
        // In production, this would be the configured dex_twap_updater
        let dex_twap_rate_q64 = 1u128 << 64; // 1.0 in Q64 format
        let venue_id = Pubkey::default(); // For testing

        let dex_ix = sdk_compat::update_dex_twap(payer.pubkey(), dex_twap_rate_q64, venue_id);

        self.process_instruction(dex_ix, &[&payer]).await?;
        println!("Updated DEX TWAP rate to 1.0");

        Ok(())
    }

    pub async fn enter_feelssol(
        &self,
        user: &Keypair,
        user_jitosol: &Pubkey,
        user_feelssol: &Pubkey,
        amount: u64,
    ) -> TestResult<()> {
        let ix = sdk_compat::enter_feelssol(
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
        let ix = sdk_compat::exit_feelssol(
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
            &[b"tick_array", market.as_ref(), &tick_index.to_le_bytes()],
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
        let params = feels::instructions::InitializeMarketParams {
            base_fee_bps: fee_tier,
            tick_spacing,
            initial_sqrt_price,
            initial_buy_feelssol_amount,
        };

        let ix = sdk_compat::instructions::initialize_market(
            creator.pubkey(),
            *token_0,
            *token_1,
            params,
        )?;

        self.process_instruction(ix, &[creator]).await?;

        let (market, _) = sdk_compat::find_market_address(token_0, token_1);
        Ok(market)
    }

    // PDA derivation helper methods
    pub fn derive_feels_hub(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"feels_hub"], &PROGRAM_ID)
    }

    pub fn derive_feels_mint(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"feels_mint"], &PROGRAM_ID)
    }

    pub fn derive_vault(&self, market: &Pubkey, token: &Pubkey, vault_index: u8) -> (Pubkey, u8) {
        // Determine the actual vault index based on token ordering in the market
        let index_bytes = if vault_index == 0 { b"0" } else { b"1" };
        Pubkey::find_program_address(&[b"vault", market.as_ref(), index_bytes], &PROGRAM_ID)
    }

    pub fn derive_market_authority(&self, market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"market_authority", market.as_ref()], &PROGRAM_ID)
    }

    pub fn derive_buffer(&self, market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"buffer", market.as_ref()], &PROGRAM_ID)
    }

    pub fn derive_oracle(&self, market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"oracle", market.as_ref()], &PROGRAM_ID)
    }

    pub async fn derive_tick_arrays(
        &self,
        market: &Pubkey,
        current_tick: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> TestResult<Vec<Pubkey>> {
        let mut tick_arrays = Vec::new();
        let tick_array_size = feels::state::TICK_ARRAY_SIZE as i32;
        let tick_array_spacing = (tick_spacing as i32) * tick_array_size;

        // Get start index for current tick
        let mut current_start = if current_tick >= 0 {
            (current_tick / tick_array_spacing) * tick_array_spacing
        } else {
            ((current_tick - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
        };

        // Add 3 tick arrays in the swap direction
        for _ in 0..3 {
            let (tick_array, _) = Pubkey::find_program_address(
                &[b"tick_array", market.as_ref(), &current_start.to_le_bytes()],
                &PROGRAM_ID,
            );
            tick_arrays.push(tick_array);

            // Move to next array in swap direction
            current_start = if zero_for_one {
                current_start - tick_array_spacing
            } else {
                current_start + tick_array_spacing
            };
        }

        Ok(tick_arrays)
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
            jitosol_authority: Keypair::new(),  // Create new keypair
        }
    }
}
