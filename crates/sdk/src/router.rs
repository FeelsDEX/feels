/// Hub-constrained router for deterministic route building
/// All routes must go through FeelsSOL as the hub token
use solana_program::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};

use crate::errors::SdkError;

/// Route type for the hub-and-spoke model
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The pools in the route (max 2)
    pub pools: Vec<PoolInfo>,
    /// Total hops (1 or 2)
    pub hops: usize,
    /// Input token
    pub token_in: Pubkey,
    /// Output token  
    pub token_out: Pubkey,
    /// Whether this route uses FeelsSOL as intermediate
    pub uses_hub: bool,
}

/// Pool information for routing
#[derive(Debug, Clone, PartialEq)]
pub struct PoolInfo {
    /// Pool address
    pub address: Pubkey,
    /// Token A mint
    pub token_a: Pubkey,
    /// Token B mint
    pub token_b: Pubkey,
    /// Fee rate (basis points)
    pub fee_rate: u16,
}

/// Hub-constrained router
pub struct HubRouter {
    /// FeelsSOL mint address (the hub)
    feelssol_mint: Pubkey,
    /// All known pools
    pools: HashMap<Pubkey, PoolInfo>,
    /// Pools indexed by token pair (sorted)
    pools_by_pair: HashMap<(Pubkey, Pubkey), Vec<PoolInfo>>,
    /// Pools that include FeelsSOL
    hub_pools: HashSet<Pubkey>,
}

impl HubRouter {
    /// Create a new hub router
    pub fn new(feelssol_mint: Pubkey) -> Self {
        Self {
            feelssol_mint,
            pools: HashMap::new(),
            pools_by_pair: HashMap::new(),
            hub_pools: HashSet::new(),
        }
    }

    /// Add a pool to the router
    pub fn add_pool(&mut self, pool: PoolInfo) -> Result<(), SdkError> {
        // Verify pool includes FeelsSOL
        if pool.token_a != self.feelssol_mint && pool.token_b != self.feelssol_mint {
            return Err(SdkError::InvalidPool(
                "Pool must include FeelsSOL as one side".to_string()
            ));
        }

        // Add to hub pools
        self.hub_pools.insert(pool.address);

        // Add to pools map
        self.pools.insert(pool.address, pool.clone());

        // Add to pools by pair (sorted keys)
        let pair = if pool.token_a < pool.token_b {
            (pool.token_a, pool.token_b)
        } else {
            (pool.token_b, pool.token_a)
        };
        
        self.pools_by_pair
            .entry(pair)
            .or_insert_with(Vec::new)
            .push(pool);

        Ok(())
    }

    /// Find the best route between two tokens
    pub fn find_route(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
    ) -> Result<Route, SdkError> {
        // Same token - no route needed
        if token_in == token_out {
            return Err(SdkError::InvalidRoute(
                "Input and output tokens are the same".to_string()
            ));
        }

        // Check for direct route (1 hop)
        if let Some(route) = self.find_direct_route(token_in, token_out) {
            return Ok(route);
        }

        // Check for 2-hop route through FeelsSOL
        if let Some(route) = self.find_hub_route(token_in, token_out) {
            return Ok(route);
        }

        Err(SdkError::NoRouteFound)
    }

    /// Find a direct route (1 hop)
    fn find_direct_route(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
    ) -> Option<Route> {
        // Get sorted pair
        let pair = if token_in < token_out {
            (*token_in, *token_out)
        } else {
            (*token_out, *token_in)
        };

        // Find pools for this pair
        if let Some(pools) = self.pools_by_pair.get(&pair) {
            // Must include FeelsSOL
            let valid_pools: Vec<_> = pools.iter()
                .filter(|p| self.hub_pools.contains(&p.address))
                .cloned()
                .collect();

            if !valid_pools.is_empty() {
                // Return pool with lowest fee
                let best_pool = valid_pools.iter()
                    .min_by_key(|p| p.fee_rate)
                    .cloned()
                    .unwrap();

                return Some(Route {
                    pools: vec![best_pool],
                    hops: 1,
                    token_in: *token_in,
                    token_out: *token_out,
                    uses_hub: *token_in == self.feelssol_mint || *token_out == self.feelssol_mint,
                });
            }
        }

        None
    }

    /// Find a 2-hop route through FeelsSOL hub
    fn find_hub_route(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
    ) -> Option<Route> {
        // Already checked direct route, so neither token is FeelsSOL
        // Find token_in -> FeelsSOL pool
        let in_to_hub = self.find_pool_for_pair(token_in, &self.feelssol_mint)?;
        
        // Find FeelsSOL -> token_out pool
        let hub_to_out = self.find_pool_for_pair(&self.feelssol_mint, token_out)?;

        Some(Route {
            pools: vec![in_to_hub, hub_to_out],
            hops: 2,
            token_in: *token_in,
            token_out: *token_out,
            uses_hub: true,
        })
    }

    /// Find best pool for a token pair
    fn find_pool_for_pair(&self, token_a: &Pubkey, token_b: &Pubkey) -> Option<PoolInfo> {
        let pair = if token_a < token_b {
            (*token_a, *token_b)
        } else {
            (*token_b, *token_a)
        };

        self.pools_by_pair.get(&pair)?
            .iter()
            .min_by_key(|p| p.fee_rate)
            .cloned()
    }

    /// Get all pools that include a specific token
    pub fn get_pools_for_token(&self, token: &Pubkey) -> Vec<&PoolInfo> {
        self.pools.values()
            .filter(|p| p.token_a == *token || p.token_b == *token)
            .collect()
    }

    /// Validate an existing route
    pub fn validate_route(&self, route: &Route) -> Result<(), SdkError> {
        // Check hop count
        if route.hops > 2 {
            return Err(SdkError::InvalidRoute(
                "Route exceeds maximum 2 hops".to_string()
            ));
        }

        // Check pool count matches hops
        if route.pools.len() != route.hops {
            return Err(SdkError::InvalidRoute(
                "Pool count doesn't match hop count".to_string()
            ));
        }

        // Validate each pool includes FeelsSOL
        for pool in &route.pools {
            if !self.hub_pools.contains(&pool.address) {
                return Err(SdkError::InvalidRoute(
                    format!("Pool {} doesn't include FeelsSOL", pool.address)
                ));
            }
        }

        // For 2-hop routes, validate intermediate token is FeelsSOL
        if route.hops == 2 {
            let pool1 = &route.pools[0];
            let pool2 = &route.pools[1];

            // Find intermediate token
            let intermediate = if pool1.token_a == route.token_in {
                pool1.token_b
            } else {
                pool1.token_a
            };

            if intermediate != self.feelssol_mint {
                return Err(SdkError::InvalidRoute(
                    "2-hop routes must use FeelsSOL as intermediate".to_string()
                ));
            }

            // Verify continuity
            if pool2.token_a != intermediate && pool2.token_b != intermediate {
                return Err(SdkError::InvalidRoute(
                    "Route pools are not connected".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Get route summary for display
    pub fn get_route_summary(&self, route: &Route) -> String {
        if route.hops == 1 {
            format!("{} -> {} (direct)", 
                route.token_in.to_string()[..8].to_string(),
                route.token_out.to_string()[..8].to_string()
            )
        } else {
            format!("{} -> FeelsSOL -> {} (2 hops)",
                route.token_in.to_string()[..8].to_string(),
                route.token_out.to_string()[..8].to_string()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_router_basic() {
        let feelssol = Pubkey::new_unique();
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let mut router = HubRouter::new(feelssol);

        // Add valid pool (includes FeelsSOL)
        let pool1 = PoolInfo {
            address: Pubkey::new_unique(),
            token_a: feelssol,
            token_b: token_a,
            fee_rate: 30,
        };
        assert!(router.add_pool(pool1.clone()).is_ok());

        // Try to add invalid pool (no FeelsSOL)
        let invalid_pool = PoolInfo {
            address: Pubkey::new_unique(),
            token_a,
            token_b,
            fee_rate: 30,
        };
        assert!(router.add_pool(invalid_pool).is_err());

        // Find direct route
        let route = router.find_route(&feelssol, &token_a).unwrap();
        assert_eq!(route.hops, 1);
        assert_eq!(route.pools.len(), 1);
    }

    #[test]
    fn test_two_hop_routing() {
        let feelssol = Pubkey::new_unique();
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let mut router = HubRouter::new(feelssol);

        // Add two pools connected by FeelsSOL
        let pool1 = PoolInfo {
            address: Pubkey::new_unique(),
            token_a: token_a,
            token_b: feelssol,
            fee_rate: 30,
        };
        let pool2 = PoolInfo {
            address: Pubkey::new_unique(),
            token_a: feelssol,
            token_b: token_b,
            fee_rate: 30,
        };

        router.add_pool(pool1).unwrap();
        router.add_pool(pool2).unwrap();

        // Find 2-hop route
        let route = router.find_route(&token_a, &token_b).unwrap();
        assert_eq!(route.hops, 2);
        assert_eq!(route.pools.len(), 2);
        assert!(route.uses_hub);
    }
}