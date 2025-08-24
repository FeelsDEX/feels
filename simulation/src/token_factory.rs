use anchor_lang::prelude::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{
    extension::{ExtensionType, BaseStateWithExtensions},
    instruction as token_instruction,
};
use crate::{TestEnvironment, TestAccount};

/// Factory for creating test tokens
pub struct TokenFactory<'a> {
    env: &'a mut TestEnvironment,
}

impl<'a> TokenFactory<'a> {
    /// Create a new token factory
    pub fn new(env: &'a mut TestEnvironment) -> Self {
        Self { env }
    }
    
    /// Create a new SPL Token 2022 mint
    pub async fn create_token(
        &mut self,
        decimals: u8,
        mint_authority: &Pubkey,
    ) -> Result<TestToken> {
        let mint_keypair = Keypair::new();
        let mint_pubkey = mint_keypair.pubkey();
        
        // Calculate space needed for mint account
        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[])
            .map_err(|e| anyhow::anyhow!("Failed to calculate mint size: {:?}", e))?;
        
        // Create mint account
        let rent = self.env.context.banks_client.get_rent().await?;
        let lamports = rent.minimum_balance(space);
        
        let create_account_ix = system_instruction::create_account(
            &self.env.payer.pubkey(),
            &mint_pubkey,
            lamports,
            space as u64,
            &spl_token_2022::ID,
        );
        
        let init_mint_ix = token_instruction::initialize_mint2(
            &spl_token_2022::ID,
            &mint_pubkey,
            mint_authority,
            None,
            decimals,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create init mint instruction: {:?}", e))?;
        
        let recent_blockhash = self.env.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, init_mint_ix],
            Some(&self.env.payer.pubkey()),
            &[&self.env.payer, &mint_keypair],
            recent_blockhash,
        );
        
        self.env.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        Ok(TestToken {
            mint: mint_pubkey,
            decimals,
            mint_authority: *mint_authority,
        })
    }
    
    /// Create a Feels token
    pub async fn create_feels_token(
        &mut self,
        name: String,
        ticker: String,
        decimals: u8,
        token_authority: &TestAccount,
    ) -> Result<TestToken> {
        let mint_keypair = Keypair::new();
        let mint_pubkey = mint_keypair.pubkey();
        
        // First create the SPL token
        let token = self.create_token(decimals, &token_authority.pubkey()).await?;
        
        // Then register it as a Feels token
        let metadata = feels::state::TokenMetadata {
            name: name.clone(),
            ticker: ticker.clone(),
            uri: format!("https://example.com/{}.json", ticker),
            decimals,
        };
        
        let ix = feels_sdk::instructions::create_feels_token(
            &self.env.program_id,
            &token.mint,
            &token_authority.pubkey(),
            metadata,
            &self.env.payer.pubkey(),
        );
        
        let recent_blockhash = self.env.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.env.payer.pubkey()),
            &[&self.env.payer, &token_authority.keypair],
            recent_blockhash,
        );
        
        self.env.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        Ok(token)
    }
    
    /// Mint tokens to an account
    pub async fn mint_to(
        &mut self,
        token: &TestToken,
        recipient: &Pubkey,
        amount: u64,
        mint_authority: &Keypair,
    ) -> Result<()> {
        // First create associated token account
        let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            recipient,
            &token.mint,
            &spl_token_2022::ID,
        );
        
        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &self.env.payer.pubkey(),
            recipient,
            &token.mint,
            &spl_token_2022::ID,
        );
        
        let mint_ix = token_instruction::mint_to(
            &spl_token_2022::ID,
            &token.mint,
            &ata,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create mint instruction: {:?}", e))?;
        
        let recent_blockhash = self.env.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[create_ata_ix, mint_ix],
            Some(&self.env.payer.pubkey()),
            &[&self.env.payer, mint_authority],
            recent_blockhash,
        );
        
        self.env.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        Ok(())
    }
    
    /// Issue FeelsSOL to an account
    pub async fn issue_feelssol(
        &mut self,
        feelssol_mint: &Pubkey,
        recipient: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        // In production, this would wrap SOL into FeelsSOL
        // For testing, we'll just mint FeelsSOL directly
        let mint_authority = &self.env.payer;
        
        let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            recipient,
            feelssol_mint,
            &spl_token_2022::ID,
        );
        
        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &self.env.payer.pubkey(),
            recipient,
            feelssol_mint,
            &spl_token_2022::ID,
        );
        
        let mint_ix = token_instruction::mint_to(
            &spl_token_2022::ID,
            feelssol_mint,
            &ata,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create mint instruction: {:?}", e))?;
        
        let recent_blockhash = self.env.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[create_ata_ix, mint_ix],
            Some(&self.env.payer.pubkey()),
            &[&self.env.payer, mint_authority],
            recent_blockhash,
        );
        
        self.env.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        Ok(())
    }
}

/// Represents a test token
#[derive(Clone, Debug)]
pub struct TestToken {
    pub mint: Pubkey,
    pub decimals: u8,
    pub mint_authority: Pubkey,
}