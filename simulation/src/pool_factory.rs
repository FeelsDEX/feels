use anchor_lang::prelude::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};
use feels_sdk::types::CreatePoolResult;
use crate::{TestEnvironment, TestToken};

/// Factory for creating test pools
pub struct PoolFactory<'a> {
    env: &'a mut TestEnvironment,
}

impl<'a> PoolFactory<'a> {
    /// Create a new pool factory
    pub fn new(env: &'a mut TestEnvironment) -> Self {
        Self { env }
    }
    
    /// Create a new pool
    pub async fn create_pool(
        &mut self,
        token_a: &TestToken,
        token_b: &TestToken,
        fee_rate: u16,
        initial_price: f64,
    ) -> Result<TestPool> {
        // Convert price to sqrt price X64
        let sqrt_price = self.price_to_sqrt_price_x64(initial_price);
        
        // Sort tokens to ensure consistent pool derivation
        let (token_0, token_1) = if token_a.mint < token_b.mint {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        
        let result = self.env.client
            .create_pool(
                &token_0.mint,
                &token_1.mint,
                fee_rate,
                sqrt_price,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create pool: {:?}", e))?;
        
        Ok(TestPool {
            address: result.pool_address,
            token_a: token_0.clone(),
            token_b: token_1.clone(),
            fee_rate,
            sqrt_price,
            liquidity: 0,
            tick: self.sqrt_price_x64_to_tick(sqrt_price),
        })
    }
    
    /// Create a standard pool with default fee tier (0.3%)
    pub async fn create_standard_pool(
        &mut self,
        token_a: &TestToken,
        token_b: &TestToken,
        initial_price: f64,
    ) -> Result<TestPool> {
        self.create_pool(token_a, token_b, 30, initial_price).await
    }
    
    /// Create multiple pools with different fee tiers
    pub async fn create_pools_with_fee_tiers(
        &mut self,
        token_a: &TestToken,
        token_b: &TestToken,
        initial_price: f64,
        fee_tiers: &[u16],
    ) -> Result<Vec<TestPool>> {
        let mut pools = Vec::new();
        
        for &fee_rate in fee_tiers {
            pools.push(
                self.create_pool(token_a, token_b, fee_rate, initial_price).await?
            );
        }
        
        Ok(pools)
    }
    
    /// Convert a decimal price to sqrt price X64 format
    fn price_to_sqrt_price_x64(&self, price: f64) -> u128 {
        let sqrt_price = price.sqrt();
        (sqrt_price * (1u128 << 64) as f64) as u128
    }
    
    /// Convert sqrt price X64 to tick
    fn sqrt_price_x64_to_tick(&self, sqrt_price_x64: u128) -> i32 {
        // This is a simplified conversion
        // In production, use the proper TickMath utilities
        let price = (sqrt_price_x64 as f64 / (1u128 << 64) as f64).powi(2);
        (price.ln() / 1.0001_f64.ln()).round() as i32
    }
}

/// Represents a test pool
#[derive(Clone, Debug)]
pub struct TestPool {
    pub address: Pubkey,
    pub token_a: TestToken,
    pub token_b: TestToken,
    pub fee_rate: u16,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub tick: i32,
}

impl TestPool {
    /// Get the current price (token_b per token_a)
    pub fn get_price(&self) -> f64 {
        let sqrt_price_float = self.sqrt_price as f64 / (1u128 << 64) as f64;
        sqrt_price_float * sqrt_price_float
    }
    
    /// Get fee rate as basis points
    pub fn fee_rate_bps(&self) -> u16 {
        self.fee_rate
    }
    
    /// Get fee rate as decimal
    pub fn fee_rate_decimal(&self) -> f64 {
        self.fee_rate as f64 / 10000.0
    }
}