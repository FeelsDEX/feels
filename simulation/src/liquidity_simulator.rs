use anchor_lang::prelude::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use feels_sdk::types::AddLiquidityResult;
use crate::{TestEnvironment, TestAccount, TestPool, TestToken};

/// Simulator for liquidity operations
pub struct LiquiditySimulator<'a> {
    env: &'a mut TestEnvironment,
}

impl<'a> LiquiditySimulator<'a> {
    /// Create a new liquidity simulator
    pub fn new(env: &'a mut TestEnvironment) -> Self {
        Self { env }
    }
    
    /// Add liquidity to a pool
    pub async fn add_liquidity(
        &mut self,
        pool: &TestPool,
        provider: &TestAccount,
        amount_a: u64,
        amount_b: u64,
        price_lower: f64,
        price_upper: f64,
    ) -> Result<LiquidityPosition> {
        // Convert prices to ticks
        let tick_lower = self.price_to_tick(price_lower);
        let tick_upper = self.price_to_tick(price_upper);
        
        // Create position NFT
        let position_mint = Keypair::new();
        let position_mint_pubkey = position_mint.pubkey();
        
        // Create position NFT instruction
        let create_position_ix = feels_sdk::instructions::create_position_nft(
            &self.env.program_id,
            &pool.address,
            &position_mint_pubkey,
            &provider.pubkey(),
            &self.env.payer.pubkey(),
        );
        
        let recent_blockhash = self.env.context.banks_client
            .get_latest_blockhash()
            .await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[create_position_ix],
            Some(&self.env.payer.pubkey()),
            &[&self.env.payer, &position_mint, &provider.keypair],
            recent_blockhash,
        );
        
        self.env.context.banks_client
            .process_transaction(transaction)
            .await?;
        
        // Add liquidity
        let result = self.env.client
            .add_liquidity(
                &pool.address,
                &position_mint_pubkey,
                amount_a,
                amount_b,
                (amount_a * 95) / 100, // 5% slippage
                (amount_b * 95) / 100,
                tick_lower,
                tick_upper,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add liquidity: {:?}", e))?;
        
        let (position_address, _) = feels_sdk::utils::derive_position(
            &pool.address,
            &position_mint_pubkey,
            &self.env.program_id,
        );
        
        Ok(LiquidityPosition {
            address: position_address,
            mint: position_mint_pubkey,
            pool: pool.address,
            owner: provider.pubkey(),
            liquidity: result.liquidity_minted,
            tick_lower,
            tick_upper,
            amount_0: result.amount_0,
            amount_1: result.amount_1,
        })
    }
    
    /// Add liquidity in a range around current price
    pub async fn add_liquidity_around_price(
        &mut self,
        pool: &TestPool,
        provider: &TestAccount,
        amount_a: u64,
        amount_b: u64,
        range_percentage: f64,
    ) -> Result<LiquidityPosition> {
        let current_price = pool.get_price();
        let price_lower = current_price * (1.0 - range_percentage / 100.0);
        let price_upper = current_price * (1.0 + range_percentage / 100.0);
        
        self.add_liquidity(
            pool,
            provider,
            amount_a,
            amount_b,
            price_lower,
            price_upper,
        ).await
    }
    
    /// Add full range liquidity
    pub async fn add_full_range_liquidity(
        &mut self,
        pool: &TestPool,
        provider: &TestAccount,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<LiquidityPosition> {
        // Use very wide price range
        let price_lower = 0.00001;
        let price_upper = 100000.0;
        
        self.add_liquidity(
            pool,
            provider,
            amount_a,
            amount_b,
            price_lower,
            price_upper,
        ).await
    }
    
    /// Remove liquidity from a position
    pub async fn remove_liquidity(
        &mut self,
        position: &LiquidityPosition,
        liquidity_amount: u128,
    ) -> Result<()> {
        self.env.client
            .remove_liquidity(
                &position.pool,
                &position.mint,
                &position.owner,
                liquidity_amount,
                0, // No minimum amounts for testing
                0,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove liquidity: {:?}", e))?;
        
        Ok(())
    }
    
    /// Collect fees from a position
    pub async fn collect_fees(
        &mut self,
        position: &LiquidityPosition,
    ) -> Result<(u64, u64)> {
        // For now, return dummy values
        // In production, this would collect actual fees
        Ok((0, 0))
    }
    
    /// Convert price to tick
    fn price_to_tick(&self, price: f64) -> i32 {
        (price.ln() / 1.0001_f64.ln()).round() as i32
    }
}

/// Represents a liquidity position
#[derive(Clone, Debug)]
pub struct LiquidityPosition {
    pub address: Pubkey,
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub owner: Pubkey,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount_0: u64,
    pub amount_1: u64,
}