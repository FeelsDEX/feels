//! Hub-constrained router for Feels protocol
//! 
//! All routes must go through FeelsSOL (hub token)

use crate::error::{SdkError, SdkResult};
use crate::types::Route;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// Pool information for routing
#[derive(Clone, Debug, PartialEq)]
pub struct PoolInfo {
    /// Pool address
    pub address: Pubkey,
    /// First token mint
    pub token_0: Pubkey,
    /// Second token mint
    pub token_1: Pubkey,
    /// Fee rate in basis points
    pub fee_rate: u16,
}

/// Hub-constrained router
pub struct HubRouter {
    /// Hub token mint (FeelsSOL)
    hub_mint: Pubkey,
    /// Pools indexed by token pair
    pools: HashMap<(Pubkey, Pubkey), PoolInfo>,
}

impl HubRouter {
    /// Create a new router with the given hub token
    pub fn new(hub_mint: Pubkey) -> Self {
        Self {
            hub_mint,
            pools: HashMap::new(),
        }
    }
    
    /// Add a pool to the router
    pub fn add_pool(&mut self, pool: PoolInfo) -> SdkResult<()> {
        // Validate that pool includes hub token
        if pool.token_0 != self.hub_mint && pool.token_1 != self.hub_mint {
            return Err(SdkError::InvalidRoute(
                "Pool must include hub token (FeelsSOL)".to_string()
            ));
        }
        
        // Add pool for both directions
        let key1 = Self::order_tokens(pool.token_0, pool.token_1);
        let key2 = Self::order_tokens(pool.token_1, pool.token_0);
        
        self.pools.insert(key1, pool.clone());
        self.pools.insert(key2, pool);
        
        Ok(())
    }
    
    /// Find route between two tokens
    pub fn find_route(&self, from: &Pubkey, to: &Pubkey) -> SdkResult<Route> {
        // Same token - no route needed
        if from == to {
            return Err(SdkError::InvalidRoute("Cannot route to same token".to_string()));
        }
        
        // Direct route if one token is hub
        if from == &self.hub_mint || to == &self.hub_mint {
            // Check if pool exists
            let key = Self::order_tokens(*from, *to);
            if self.pools.contains_key(&key) {
                return Ok(Route::Direct { from: *from, to: *to });
            }
        }
        
        // Two-hop route through hub
        let key1 = Self::order_tokens(*from, self.hub_mint);
        let key2 = Self::order_tokens(self.hub_mint, *to);
        
        if self.pools.contains_key(&key1) && self.pools.contains_key(&key2) {
            return Ok(Route::TwoHop {
                from: *from,
                intermediate: self.hub_mint,
                to: *to,
            });
        }
        
        Err(SdkError::InvalidRoute(format!(
            "No route found from {} to {}",
            from, to
        )))
    }
    
    /// Get pools for a route
    pub fn get_route_pools(&self, route: &Route) -> Vec<&PoolInfo> {
        match route {
            Route::Direct { from, to } => {
                let key = Self::order_tokens(*from, *to);
                self.pools.get(&key).into_iter().collect()
            }
            Route::TwoHop { from, intermediate, to } => {
                let key1 = Self::order_tokens(*from, *intermediate);
                let key2 = Self::order_tokens(*intermediate, *to);
                
                let mut pools = Vec::new();
                if let Some(pool1) = self.pools.get(&key1) {
                    pools.push(pool1);
                }
                if let Some(pool2) = self.pools.get(&key2) {
                    pools.push(pool2);
                }
                pools
            }
        }
    }
    
    /// Get human-readable route summary
    pub fn get_route_summary(&self, route: &Route) -> String {
        match route {
            Route::Direct { from, to } => {
                format!("{} -> {}", from, to)
            }
            Route::TwoHop { from, intermediate, to } => {
                format!("{} -> {} -> {}", from, intermediate, to)
            }
        }
    }
    
    /// Calculate total fee for a route
    pub fn calculate_route_fee(&self, route: &Route) -> u16 {
        let pools = self.get_route_pools(route);
        pools.iter().map(|p| p.fee_rate).sum()
    }
    
    /// Order tokens consistently
    fn order_tokens(a: Pubkey, b: Pubkey) -> (Pubkey, Pubkey) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hub_router_validation() {
        let hub = Pubkey::new_unique();
        let token_0 = Pubkey::new_unique();
        let token_1 = Pubkey::new_unique();
        
        let mut router = HubRouter::new(hub);
        
        // Should fail - no hub token
        let invalid_pool = PoolInfo {
            address: Pubkey::new_unique(),
            token_0,
            token_1,
            fee_rate: 30,
        };
        assert!(router.add_pool(invalid_pool).is_err());
        
        // Should succeed - includes hub
        let valid_pool = PoolInfo {
            address: Pubkey::new_unique(),
            token_0,
            token_1: hub,
            fee_rate: 30,
        };
        assert!(router.add_pool(valid_pool).is_ok());
    }
    
    #[test]
    fn test_route_finding() {
        let hub = Pubkey::new_unique();
        let usdc = Pubkey::new_unique();
        let sol = Pubkey::new_unique();
        
        let mut router = HubRouter::new(hub);
        
        // Add pools
        router.add_pool(PoolInfo {
            address: Pubkey::new_unique(),
            token_0: usdc,
            token_1: hub,
            fee_rate: 30,
        }).unwrap();
        
        router.add_pool(PoolInfo {
            address: Pubkey::new_unique(),
            token_0: sol,
            token_1: hub,
            fee_rate: 25,
        }).unwrap();
        
        // Direct route
        let route1 = router.find_route(&usdc, &hub).unwrap();
        assert_eq!(route1.hop_count(), 1);
        
        // Two-hop route
        let route2 = router.find_route(&usdc, &sol).unwrap();
        assert_eq!(route2.hop_count(), 2);
    }
}