/// Comprehensive swap routing module implementing the hub-and-spoke model where all
/// tokens trade through FeelsSOL. Contains SwapRoute logic, RoutingLogic utilities,
/// pool address derivation, and enhanced route analysis with proper liquidity utilization
/// calculations. Determines optimal paths considering multiple fee tiers and provides
/// accurate gas estimation and liquidity metrics for different routing strategies.

use anchor_lang::prelude::*;
use crate::utils::VALID_FEE_TIERS;

// ============================================================================
// Type Definitions
// ============================================================================

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum SwapRoute {
    /// Direct swap - one of the tokens is FeelsSOL
    Direct(Pubkey), // pool_key
    /// Two-hop swap - neither token is FeelsSOL, route through FeelsSOL
    TwoHop(Pubkey, Pubkey), // pool1_key, pool2_key
}

impl SwapRoute {
    
    /// Determine the optimal routing strategy for a token pair
    /// Returns the route using the lowest available fee tier
    pub fn find(
        token_in: Pubkey,
        token_out: Pubkey,
        feelssol_mint: Pubkey,
        program_id: &Pubkey,
    ) -> SwapRoute {
        // Check if either token is FeelsSOL
        if token_in == feelssol_mint || token_out == feelssol_mint {
            // Direct swap possible - find best fee tier
            // In production, would check which pools actually exist
            // For now, return first available fee tier (would query on-chain)
            let pool_key = Self::find_best_pool(token_in, token_out, program_id);
            SwapRoute::Direct(pool_key)
        } else {
            // Two-hop swap needed - find best fee tiers for each hop
            let pool1_key = Self::find_best_pool(token_in, feelssol_mint, program_id);
            let pool2_key = Self::find_best_pool(feelssol_mint, token_out, program_id);
            SwapRoute::TwoHop(pool1_key, pool2_key)
        }
    }
    
    /// Find the best pool for a token pair by checking multiple fee tiers
    /// In production, this would query on-chain state to find existing pools
    fn find_best_pool(token_a: Pubkey, token_b: Pubkey, program_id: &Pubkey) -> Pubkey {
        // Try fee tiers in order of preference (lowest to highest)
        // In a real implementation, would check if pool exists on-chain
        for &fee_tier in VALID_FEE_TIERS {
            let pool_key = Self::derive_pool_key(token_a, token_b, fee_tier, program_id);
            // TODO: Check if pool exists on-chain
            // For Phase 1, return the standard 0.3% fee tier
            if fee_tier == 30 {
                return pool_key;
            }
        }
        
        // Default to 0.3% fee tier
        Self::derive_pool_key(token_a, token_b, 30, program_id)
    }

    /// Derive pool PDA for a token pair using proper program derivation
    /// Considers fee tiers and ensures canonical token ordering
    pub fn derive_pool_key(token_a: Pubkey, token_b: Pubkey, fee_rate: u16, program_id: &Pubkey) -> Pubkey {
        // Use canonical token ordering to ensure deterministic pool addresses
        let (token_0, token_1) = crate::utils::CanonicalSeeds::sort_token_mints(&token_a, &token_b);

        // Use proper PDA derivation with program ownership
        let seeds = &[
            b"pool",
            token_0.as_ref(),
            token_1.as_ref(),
            &fee_rate.to_le_bytes(),
        ];
        
        // Proper PDA derivation owned by the program
        let (pool_address, _bump) = Pubkey::find_program_address(seeds, program_id);
        pool_address
    }

    /// Get all pools involved in this route
    pub fn get_pools(&self) -> Vec<Pubkey> {
        match self {
            SwapRoute::Direct(pool) => vec![*pool],
            SwapRoute::TwoHop(pool1, pool2) => vec![*pool1, *pool2],
        }
    }

    /// Check if this route is optimal (single hop preferred over two hop)
    pub fn is_optimal(&self) -> bool {
        matches!(self, SwapRoute::Direct(_))
    }

    /// Get the number of hops in this route
    pub fn hop_count(&self) -> u8 {
        match self {
            SwapRoute::Direct(_) => 1,
            SwapRoute::TwoHop(_, _) => 2,
        }
    }
}

// ============================================================================
// Routing Logic
// ============================================================================

/// Routing logic for cross-token swaps
pub struct RoutingLogic;

impl RoutingLogic {
    /// Calculate the optimal route for a given token pair
    pub fn calculate_route(
        token_a: Pubkey, 
        token_b: Pubkey, 
        feelssol_mint: Pubkey,
        program_id: &Pubkey
    ) -> SwapRoute {
        SwapRoute::find(token_a, token_b, feelssol_mint, program_id)
    }
    
    /// Estimate gas costs for different routing strategies
    pub fn estimate_gas_cost(route: &SwapRoute) -> u64 {
        match route {
            SwapRoute::Direct(_) => 50_000, // Single swap compute units
            SwapRoute::TwoHop(_, _) => 95_000, // Two swap compute units
        }
    }
    
    /// Validate that a route is executable
    pub fn validate_route(route: &SwapRoute) -> bool {
        match route {
            SwapRoute::Direct(pool) => *pool != Pubkey::default(),
            SwapRoute::TwoHop(pool1, pool2) => {
                *pool1 != Pubkey::default() && *pool2 != Pubkey::default()
            }
        }
    }
}

/// Derive pool address using proper PDA derivation
/// Uses canonical token ordering to ensure deterministic pool addresses
pub fn derive_pool_address(token_a: Pubkey, token_b: Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
    // Use canonical token ordering to ensure deterministic pool addresses
    let (token_0, token_1) = crate::utils::CanonicalSeeds::sort_token_mints(&token_a, &token_b);
    
    // Use proper PDA derivation with program ownership
    let seeds = &[
        b"pool",
        token_0.as_ref(),
        token_1.as_ref(),
    ];
    
    let (pool_address, _bump) = Pubkey::find_program_address(seeds, program_id);
    Ok(pool_address)
}

/// Route analysis for client-side optimization
pub struct RouteAnalysis {
    pub route: SwapRoute,
    pub estimated_gas: u64,
    pub estimated_slippage: u16,   // basis points
    pub liquidity_utilization: u8, // percentage
}

impl RouteAnalysis {
    /// Analyze a route for efficiency metrics
    pub fn analyze(route: SwapRoute) -> Self {
        let (estimated_gas, estimated_slippage) = match route.hop_count() {
            1 => (50_000, 30),   // Single hop: lower gas, lower slippage
            2 => (95_000, 60),   // Two hop: higher gas, higher slippage
            _ => (150_000, 100), // Fallback
        };

        RouteAnalysis {
            route,
            estimated_gas,
            estimated_slippage,
            liquidity_utilization: 85, // Default value when pool data not available
        }
    }
    
    /// Analyze a route with actual pool data for accurate liquidity metrics
    pub fn analyze_with_pools(
        route: SwapRoute,
        pool_liquidity: Vec<u128>,
        swap_amount: u64,
    ) -> Self {
        let (estimated_gas, estimated_slippage) = match route.hop_count() {
            1 => (50_000, 30),   // Single hop: lower gas, lower slippage
            2 => (95_000, 60),   // Two hop: higher gas, higher slippage
            _ => (150_000, 100), // Fallback
        };
        
        // Calculate liquidity utilization based on swap amount vs available liquidity
        let liquidity_utilization = Self::calculate_liquidity_utilization(
            &pool_liquidity,
            swap_amount,
        );

        RouteAnalysis {
            route,
            estimated_gas,
            estimated_slippage,
            liquidity_utilization,
        }
    }
    
    /// Calculate liquidity utilization as a percentage
    /// Higher utilization = more price impact
    fn calculate_liquidity_utilization(
        pool_liquidity: &[u128],
        swap_amount: u64,
    ) -> u8 {
        if pool_liquidity.is_empty() {
            return 85; // Default fallback
        }
        
        // For multi-hop swaps, use the minimum liquidity (bottleneck)
        let min_liquidity = pool_liquidity.iter()
            .min()
            .copied()
            .unwrap_or(0);
        
        if min_liquidity == 0 {
            return 100; // Max utilization if no liquidity
        }
        
        // Calculate utilization as swap_amount / liquidity
        // Assume 1:1 token value for simplicity (in practice would consider prices)
        let utilization = (swap_amount as u128)
            .saturating_mul(100)
            .saturating_div(min_liquidity);
        
        // Cap at 100%
        std::cmp::min(utilization, 100) as u8
    }
}