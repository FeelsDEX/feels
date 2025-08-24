use anchor_lang::prelude::*;
use solana_program_test::{BanksClient, ProgramTest, ProgramTestContext};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use feels_sdk::{FeelsClient, SdkConfig};
use std::sync::Arc;

/// Test environment for simulating Feels Protocol operations
pub struct TestEnvironment {
    pub context: ProgramTestContext,
    pub program_id: Pubkey,
    pub client: Arc<FeelsClient>,
    pub payer: Keypair,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Result<Self> {
        let program_id = feels::ID;
        let mut program_test = ProgramTest::new(
            "feels",
            program_id,
            None, // BPF program will be loaded from target/deploy
        );
        
        // Add SPL Token 2022 program
        program_test.add_program(
            "spl_token_2022",
            spl_token_2022::ID,
            None,
        );
        
        let mut context = program_test.start_with_context().await;
        let payer = context.payer.insecure_clone();
        
        let config = SdkConfig::localnet(payer.insecure_clone());
        let client = Arc::new(FeelsClient::new(config));
        
        Ok(Self {
            context,
            program_id,
            client,
            payer,
        })
    }
    
    /// Initialize the protocol
    pub async fn initialize_protocol(&mut self) -> Result<()> {
        let protocol_authority = self.payer.pubkey();
        let emergency_authority = self.payer.pubkey();
        
        self.client
            .initialize_protocol(&protocol_authority, &emergency_authority)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize protocol: {:?}", e))?;
        
        Ok(())
    }
    
    /// Initialize FeelsSOL
    pub async fn initialize_feelssol(
        &mut self,
        feelssol_mint: &Pubkey,
        underlying_mint: &Pubkey,
    ) -> Result<()> {
        self.client
            .initialize_feelssol(feelssol_mint, underlying_mint)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize FeelsSOL: {:?}", e))?;
        
        Ok(())
    }
    
    /// Fund an account with SOL
    pub async fn fund_account(&mut self, account: &Pubkey, lamports: u64) -> Result<()> {
        let ix = system_instruction::transfer(
            &self.payer.pubkey(),
            account,
            lamports,
        );
        
        let recent_blockhash = self.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );
        
        self.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        Ok(())
    }
    
    /// Advance the clock by a number of slots
    pub async fn advance_clock(&mut self, slots: u64) -> Result<()> {
        let clock = self.context.banks_client.get_sysvar::<Clock>().await?;
        self.context.warp_to_slot(clock.slot + slots)?;
        Ok(())
    }
    
    /// Get the current slot
    pub async fn get_slot(&mut self) -> Result<u64> {
        let clock = self.context.banks_client.get_sysvar::<Clock>().await?;
        Ok(clock.slot)
    }
    
    /// Get banks client
    pub fn banks_client(&self) -> &BanksClient {
        &self.context.banks_client
    }
    
    /// Get mutable banks client
    pub fn banks_client_mut(&mut self) -> &mut BanksClient {
        &mut self.context.banks_client
    }
}