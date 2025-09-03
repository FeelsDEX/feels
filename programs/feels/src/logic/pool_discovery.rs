use anchor_lang::prelude::*;
use crate::state::MarketManager;

/// Pool discovery and existence checking
pub struct PoolDiscovery;

/// Result of pool discovery
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_key: Pubkey,
    pub fee_tier: u16,
    pub exists: bool,
    pub liquidity: u128,
    pub is_active: bool,
}

impl PoolDiscovery {
    /// Standard fee tiers in basis points
    pub const FEE_TIERS: [u16; 4] = [
        1,    // 0.01% - stablecoin pairs
        5,    // 0.05% - blue chip pairs
        30,   // 0.30% - standard pairs
        100,  // 1.00% - exotic pairs
    ];
    
    /// Check existence of pools for a token pair across all fee tiers
    pub fn discover_pools<'info>(
        token_0: Pubkey,
        token_1: Pubkey,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<Vec<PoolInfo>> {
        // Ensure canonical ordering
        let (token_0, token_1) = if token_0 < token_1 {
            (token_0, token_1)
        } else {
            (token_1, token_0)
        };
        
        let mut pool_infos = Vec::new();
        
        // Check each fee tier
        for &fee_tier in &Self::FEE_TIERS {
            let pool_key = Self::derive_pool_key(token_0, token_1, fee_tier, program_id);
            
            // Look for this pool in remaining accounts
            let pool_info = Self::check_pool_existence(
                &pool_key,
                fee_tier,
                remaining_accounts,
            )?;
            
            pool_infos.push(pool_info);
        }
        
        Ok(pool_infos)
    }
    
    /// Find the best pool with sufficient liquidity
    pub fn find_best_pool(
        token_0: Pubkey,
        token_1: Pubkey,
        min_liquidity: u128,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Option<PoolInfo>> {
        let pools = Self::discover_pools(token_0, token_1, program_id, remaining_accounts)?;
        
        // Filter for active pools with sufficient liquidity
        let viable_pools: Vec<_> = pools
            .into_iter()
            .filter(|p| p.exists && p.is_active && p.liquidity >= min_liquidity)
            .collect();
        
        // Return pool with lowest fee tier
        Ok(viable_pools.into_iter().min_by_key(|p| p.fee_tier))
    }
    
    /// Check if a specific pool exists in the provided accounts
    fn check_pool_existence<'info>(
        pool_key: &Pubkey,
        fee_tier: u16,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<PoolInfo> {
        // Look for the pool account in remaining accounts
        for account in remaining_accounts {
            if account.key() == *pool_key {
                // Try to deserialize as MarketManager
                if account.owner == &crate::id() && account.data_len() >= 8 + MarketManager::SIZE {
                    match MarketManager::try_deserialize(&mut &account.data.borrow()[8..]) {
                        Ok(market) => {
                            return Ok(PoolInfo {
                                pool_key: *pool_key,
                                fee_tier,
                                exists: true,
                                liquidity: market.liquidity,
                                is_active: market.is_enabled,
                            });
                        }
                        Err(_) => {
                            // Account exists but failed to deserialize
                            return Ok(PoolInfo {
                                pool_key: *pool_key,
                                fee_tier,
                                exists: false,
                                liquidity: 0,
                                is_active: false,
                            });
                        }
                    }
                }
            }
        }
        
        // Pool not found in remaining accounts
        Ok(PoolInfo {
            pool_key: *pool_key,
            fee_tier,
            exists: false,
            liquidity: 0,
            is_active: false,
        })
    }
    
    /// Derive pool PDA for given parameters
    fn derive_pool_key(
        token_0: Pubkey,
        token_1: Pubkey,
        fee_tier: u16,
        program_id: &Pubkey,
    ) -> Pubkey {
        let seeds = &[
            b"market",
            token_0.as_ref(),
            token_1.as_ref(),
            &fee_tier.to_le_bytes(),
        ];
        
        let (pool_key, _) = Pubkey::find_program_address(seeds, program_id);
        pool_key
    }
}