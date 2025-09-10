use super::*;
use anchor_lang::AnchorSerialize;
use feels::state::Market;
use solana_program_test::{BanksClient, ProgramTest};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signer},
    system_instruction,
};

/// Test result type alias
pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Core test suite for all Feels protocol tests
pub struct TestSuite {
    pub program_test: ProgramTest,
    pub program_id: Pubkey,
    banks_client: BanksClient,
    pub payer: Keypair,
    rent: Rent,
}

impl TestSuite {
    /// Create a new test suite instance
    pub async fn new() -> TestResult<Self> {
        // Create program test without processor (for BPF testing)
        let mut program_test = ProgramTest::new(
            "feels",
            FEELS_PROGRAM_ID,
            None,
        );
        
        // Add more compute units for complex operations
        program_test.set_compute_max_units(1_000_000);
        
        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        let rent = banks_client.get_rent().await?;
        
        Ok(Self {
            program_test: ProgramTest::default(), // Placeholder
            program_id: FEELS_PROGRAM_ID,
            banks_client,
            payer,
            rent,
        })
    }
    
    /// Get a recent blockhash
    pub async fn get_recent_blockhash(&mut self) -> TestResult<solana_sdk::hash::Hash> {
        Ok(self.banks_client.get_latest_blockhash().await?)
    }
    
    /// Airdrop SOL to an account
    pub async fn airdrop(&mut self, to: &Pubkey, amount: u64) -> TestResult<()> {
        let ix = system_instruction::transfer(
            &self.payer.pubkey(),
            to,
            amount,
        );
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer]).await
    }
    
    /// Process a transaction with given instructions and signers
    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> TestResult<()> {
        let recent_blockhash = self.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
            signers,
            recent_blockhash,
        );
        
        self.banks_client.process_transaction(tx).await?;
        Ok(())
    }
    
    /// Get account data
    pub async fn get_account(&mut self, address: &Pubkey) -> TestResult<Option<Account>> {
        Ok(self.banks_client.get_account(*address).await?)
    }
    
    /// Get account data and deserialize it
    pub async fn get_account_data<T: anchor_lang::AccountDeserialize>(
        &mut self,
        address: &Pubkey,
    ) -> TestResult<T> {
        let account = self.get_account(address).await?
            .ok_or("Account not found")?;
        
        let data = account.data.as_slice();
        let mut data_slice = &data[8..]; // Skip discriminator
        T::try_deserialize(&mut data_slice)
            .map_err(|e| format!("Deserialization error: {}", e).into())
    }
    
    /// Create a new mint
    pub async fn create_mint(
        &mut self,
        authority: &Pubkey,
        decimals: u8,
    ) -> TestResult<Keypair> {
        let mint = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Mint::LEN);
        
        let instructions = vec![
            system_instruction::create_account(
                &self.payer.pubkey(),
                &mint.pubkey(),
                rent,
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
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&instructions, &[&payer, &mint]).await?;
        Ok(mint)
    }
    
    /// Create a token account
    pub async fn create_token_account(
        &mut self,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> TestResult<Keypair> {
        let token_account = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);
        
        let instructions = vec![
            system_instruction::create_account(
                &self.payer.pubkey(),
                &token_account.pubkey(),
                rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &token_account.pubkey(),
                mint,
                owner,
            )?,
        ];
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&instructions, &[&payer, &token_account]).await?;
        Ok(token_account)
    }
    
    /// Mint tokens to an account
    pub async fn mint_to(
        &mut self,
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
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer, authority]).await
    }
    
    /// Get token balance
    pub async fn get_token_balance(&mut self, token_account: &Pubkey) -> TestResult<u64> {
        let account = self.get_account(token_account).await?
            .ok_or_else(|| "Token account not found")?;
        
        let token_account = spl_token::state::Account::unpack(&account.data)?;
        Ok(token_account.amount)
    }
    
    /// Mint tokens to an account (for testing)
    pub async fn mint_tokens(
        &mut self,
        mint: &Pubkey,
        recipient: &Pubkey,
        amount: u64,
    ) -> TestResult<()> {
        // For testing, we'll use the payer as the mint authority
        let ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            recipient,
            &self.payer.pubkey(),
            &[&self.payer.pubkey()],
            amount,
        )?;
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer]).await
    }
    
    /// Create associated token account if it doesn't exist
    pub async fn create_ata_if_needed(
        &mut self,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> TestResult<Pubkey> {
        let ata = spl_associated_token_account::get_associated_token_address(owner, mint);
        
        // Check if it already exists
        if self.get_account(&ata).await?.is_none() {
            let ix = spl_associated_token_account::instruction::create_associated_token_account(
                &self.payer.pubkey(),
                owner,
                mint,
                &spl_token::id(),
            );
            
            let payer = self.payer.insecure_clone();
            self.process_transaction(&[ix], &[&payer]).await?;
        }
        
        Ok(ata)
    }
    
    /// Ensure tick array is initialized
    pub async fn ensure_tick_array_initialized(
        &mut self,
        market: &Pubkey,
        start_tick_index: i32,
    ) -> TestResult<()> {
        // Derive tick array PDA
        let (tick_array, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                market.as_ref(),
                &start_tick_index.to_le_bytes(),
            ],
            &self.program_id,
        );
        
        // Check if it already exists
        if self.get_account(&tick_array).await?.is_some() {
            return Ok(());
        }
        
        // For testing, we'll just create a dummy account
        // In production, you'd call the actual initialize_tick_array instruction
        let rent = self.banks_client.get_rent().await?;
        let space = 8 + std::mem::size_of::<feels::state::TickArray>();
        let lamports = rent.minimum_balance(space);
        
        // Create the account
        let create_ix = solana_sdk::system_instruction::create_account(
            &self.payer.pubkey(),
            &tick_array,
            lamports,
            space as u64,
            &self.program_id,
        );
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[create_ix], &[&payer]).await?;
        
        Ok(())
    }
    
    /// Execute a swap with tick arrays
    pub async fn swap_with_arrays(
        &mut self,
        market: Pubkey,
        user: &Keypair,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        tick_arrays: Vec<Pubkey>,
    ) -> TestResult<SwapResult> {
        use feels::accounts::Swap;
        
        
        // Get market data to determine the oracle account
        let market_data = self.get_account_data::<Market>(&market).await?;
        
        // Get token balances before
        let balance_in_before = self.get_token_balance(&user_token_in).await?;
        let balance_out_before = self.get_token_balance(&user_token_out).await?;
        
        // Derive vault addresses
        let (vault_0, _) = Market::derive_vault_address(&market, &market_data.token_0, &self.program_id);
        let (vault_1, _) = Market::derive_vault_address(&market, &market_data.token_1, &self.program_id);
        let (market_authority, _) = Market::derive_market_authority(&market, &self.program_id);
        
        // Derive buffer
        let (buffer, _) = feels::utils::derive_buffer(&market_data.feelssol_mint, &self.program_id);
        
        // Derive oracle
        let (oracle, _) = Pubkey::find_program_address(
            &[b"oracle", market.as_ref()],
            &self.program_id,
        );
        
        // Build accounts
        let accounts = Swap {
            user: user.pubkey(),
            market,
            vault_0,
            vault_1,
            market_authority,
            buffer,
            oracle,
            user_token_in,
            user_token_out,
            token_program: spl_token::id(),
            clock: solana_sdk::sysvar::clock::id(),
        };
        
        // Build the swap instruction
        // Compute discriminator for swap instruction using Anchor's method
        use anchor_lang::solana_program::hash::hash;
        let discriminator = &hash(b"global:swap").to_bytes()[..8];
        let mut data = discriminator.to_vec();
        
        // Serialize params
        let params = feels::instructions::SwapParams {
            amount_in,
            minimum_amount_out,
            max_ticks_crossed: 0,
        };
        params.serialize(&mut data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize swap params: {}", e))?;
        
        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data,
        };
        
        // Add tick arrays as remaining accounts
        let mut all_accounts = ix.accounts.clone();
        for tick_array in &tick_arrays {
            all_accounts.push(AccountMeta::new(*tick_array, false));
        }
        
        let swap_ix = Instruction {
            program_id: ix.program_id,
            accounts: all_accounts,
            data: ix.data,
        };
        
        // Execute the swap
        self.process_transaction(&[swap_ix], &[user]).await?;
        
        // Get token balances after
        let balance_in_after = self.get_token_balance(&user_token_in).await?;
        let balance_out_after = self.get_token_balance(&user_token_out).await?;
        
        // Get market data after
        let market_data_after = self.get_account_data::<Market>(&market).await?;
        
        // Calculate actual amounts
        let actual_in = balance_in_before.saturating_sub(balance_in_after);
        let actual_out = balance_out_after.saturating_sub(balance_out_before);
        let fee_amount = actual_in.saturating_sub(
            (actual_in as u128 * 10_000 / (10_000 + market_data.base_fee_bps as u128)) as u64
        );
        
        Ok(SwapResult {
            amount_in: actual_in,
            amount_out: actual_out,
            fee_amount,
            end_sqrt_price: market_data_after.sqrt_price,
            end_tick: market_data_after.current_tick,
        })
    }
    
    /// Initialize test utils for FeelsSOL environment
    #[cfg(feature = "test-utils")]
    pub async fn init_feelssol_test_env(
        &mut self,
        jitosol_mint: Pubkey,
        feelssol_mint: Pubkey,
        authority: &Keypair,
    ) -> TestResult<()> {
        use feels::accounts::test_utils::TestInitFeelsEnv;
        use feels::instruction::TestInitFeelsEnv as TestInitFeelsEnvIx;
        
        let (jitosol_vault, _) = crate::utils::derive_jitosol_vault(&feelssol_mint, &self.program_id);
        let (vault_authority, _) = crate::utils::derive_vault_authority(&feelssol_mint, &self.program_id);
        let (mint_authority, _) = crate::utils::derive_mint_authority(&feelssol_mint, &self.program_id);
        
        let accounts = TestInitFeelsEnv {
            payer: self.payer.pubkey(),
            jitosol_mint,
            feelssol_mint,
            jitosol_vault,
            vault_authority,
            mint_authority,
            current_feelssol_authority: authority.pubkey(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
        };
        
        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: TestInitFeelsEnvIx {}.data(),
        };
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer, authority]).await
    }
    
    /// Mint FeelsSOL in test environment
    #[cfg(feature = "test-utils")]
    pub async fn mint_feelssol_test(
        &mut self,
        feelssol_mint: Pubkey,
        to: Pubkey,
        amount: u64,
    ) -> TestResult<()> {
        use feels::accounts::test_utils::TestMintFeelsSOL;
        use feels::instruction::TestMintFeelssol as TestMintFeelssolIx;
        
        let (mint_authority, _) = crate::utils::derive_mint_authority(&feelssol_mint, &self.program_id);
        
        let accounts = TestMintFeelsSOL {
            requester: self.payer.pubkey(),
            feelssol_mint,
            to,
            mint_authority,
            token_program: spl_token::id(),
        };
        
        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: TestMintFeelssolIx { amount }.data(),
        };
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer]).await
    }
    
    /// Initialize a tick array for testing
    #[cfg(feature = "test-utils")]
    pub async fn init_tick_array_test(
        &mut self,
        market: Pubkey,
        start_tick_index: i32,
    ) -> TestResult<Pubkey> {
        use feels::accounts::test_utils::TestInitTickArray;
        use feels::instruction::TestInitTickArray as TestInitTickArrayIx;
        
        let (tick_array, _) = crate::utils::derive_tick_array(&market, start_tick_index, &self.program_id);
        
        let accounts = TestInitTickArray {
            payer: self.payer.pubkey(),
            market,
            tick_array,
            system_program: system_program::id(),
        };
        
        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: TestInitTickArrayIx {
                params: feels::instructions::test_utils::TestInitTickArrayParams {
                    start_tick_index,
                },
            }.data(),
        };
        
        let payer = self.payer.insecure_clone();
        self.process_transaction(&[ix], &[&payer]).await?;
        Ok(tick_array)
    }
}

// Extension traits for specific test functionality
pub trait MarketTestExt {
    async fn create_market(
        &mut self,
        token_mint_0: Pubkey,
        token_mint_1: Pubkey,
        buffer: Pubkey,
        tick_spacing: u16,
        initial_sqrt_price: u128,
        fee_tier: u16,
    ) -> TestResult<Pubkey>;
    
    async fn get_market(&mut self, market: &Pubkey) -> TestResult<Market>;
}

pub trait SwapTestExt {
    async fn execute_swap(
        &mut self,
        market: Pubkey,
        user: &Keypair,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        tick_arrays: Vec<Pubkey>,
    ) -> TestResult<SwapResult>;
}

impl SwapTestExt for TestSuite {
    async fn execute_swap(
        &mut self,
        market: Pubkey,
        user: &Keypair,
        user_token_in: Pubkey,
        user_token_out: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        tick_arrays: Vec<Pubkey>,
    ) -> TestResult<SwapResult> {
        // Call the swap_with_arrays helper
        self.swap_with_arrays(
            market,
            user,
            user_token_in,
            user_token_out,
            amount_in,
            minimum_amount_out,
            tick_arrays,
        ).await
    }
}

pub trait PositionTestExt {
    async fn open_position(
        &mut self,
        market: Pubkey,
        owner: &Keypair,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> TestResult<Pubkey>;
    
    async fn close_position(
        &mut self,
        position: Pubkey,
        owner: &Keypair,
    ) -> TestResult<()>;
    
    async fn collect_fees(
        &mut self,
        position: Pubkey,
        owner: &Keypair,
    ) -> TestResult<(u64, u64)>;
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub end_sqrt_price: u128,
    pub end_tick: i32,
}

// Re-export common constants
pub mod constants {
    
}
