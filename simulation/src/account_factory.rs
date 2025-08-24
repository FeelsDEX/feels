use anchor_lang::prelude::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use crate::TestEnvironment;

/// Factory for creating test accounts
pub struct AccountFactory<'a> {
    env: &'a mut TestEnvironment,
}

impl<'a> AccountFactory<'a> {
    /// Create a new account factory
    pub fn new(env: &'a mut TestEnvironment) -> Self {
        Self { env }
    }
    
    /// Create a new funded test account
    pub async fn create_account(&mut self, lamports: u64) -> Result<TestAccount> {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        
        // Fund the account
        self.env.fund_account(&pubkey, lamports).await?;
        
        Ok(TestAccount {
            keypair,
            pubkey,
        })
    }
    
    /// Create multiple funded test accounts
    pub async fn create_accounts(&mut self, count: usize, lamports: u64) -> Result<Vec<TestAccount>> {
        let mut accounts = Vec::with_capacity(count);
        
        for _ in 0..count {
            accounts.push(self.create_account(lamports).await?);
        }
        
        Ok(accounts)
    }
    
    /// Create a trader account with sufficient SOL for operations
    pub async fn create_trader(&mut self) -> Result<TestAccount> {
        // 10 SOL should be enough for most test operations
        self.create_account(10_000_000_000).await
    }
    
    /// Create a liquidity provider account
    pub async fn create_liquidity_provider(&mut self) -> Result<TestAccount> {
        // 100 SOL for providing liquidity
        self.create_account(100_000_000_000).await
    }
}

/// Represents a test account
#[derive(Clone)]
pub struct TestAccount {
    pub keypair: Keypair,
    pub pubkey: Pubkey,
}

impl TestAccount {
    /// Get the public key
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
    
    /// Get a reference to the keypair
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }
}