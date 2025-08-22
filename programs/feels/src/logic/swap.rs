/// Implements the hub-and-spoke routing model where all tokens trade through FeelsSOL.
/// Determines optimal swap paths: direct (when one token is FeelsSOL) or two-hop
/// (routing through FeelsSOL for cross-token swaps). This design simplifies liquidity
/// aggregation and ensures all tokens have a common price reference.

use anchor_lang::prelude::*;

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
    pub fn find(
        token_in: Pubkey,
        token_out: Pubkey,
        feelssol_mint: Pubkey,
    ) -> SwapRoute {
        // Check if either token is FeelsSOL
        if token_in == feelssol_mint || token_out == feelssol_mint {
            // Direct swap possible - one token is FeelsSOL
            let pool_key = Self::derive_pool_key(token_in, token_out, 30); // Default 0.3% fee
            SwapRoute::Direct(pool_key)
        } else {
            // Two-hop swap needed - neither token is FeelsSOL
            let pool1_key = Self::derive_pool_key(token_in, feelssol_mint, 30);
            let pool2_key = Self::derive_pool_key(feelssol_mint, token_out, 30);
            SwapRoute::TwoHop(pool1_key, pool2_key)
        }
    }

    /// Derive pool PDA for a token pair
    /// In production, this would consider multiple fee tiers and find the best liquidity
    fn derive_pool_key(token_a: Pubkey, token_b: Pubkey, fee_rate: u16) -> Pubkey {
        // Sort tokens into canonical order
        let (token_0, token_1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        // This is a simplified derivation - in production would use actual program ID
        let _seeds = &[
            b"pool",
            token_0.as_ref(),
            token_1.as_ref(),
            &fee_rate.to_le_bytes(),
        ];
        
        // For Phase 1, return a placeholder. In production, use:
        // Pubkey::find_program_address(seeds, &program_id).0
        Pubkey::new_unique()
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

/// Route analysis for client-side optimization
pub struct RouteAnalysis {
    pub route: SwapRoute,
    pub estimated_gas: u64,
    pub estimated_slippage: u16, // basis points
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
            liquidity_utilization: 85, // Placeholder - would be calculated from actual pool data
        }
    }
}