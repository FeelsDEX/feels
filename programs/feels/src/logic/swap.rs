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
        program_id: &Pubkey,
    ) -> SwapRoute {
        // Check if either token is FeelsSOL
        if token_in == feelssol_mint || token_out == feelssol_mint {
            // Direct swap possible - one token is FeelsSOL
            let pool_key = Self::derive_pool_key(token_in, token_out, 30, program_id); // Default 0.3% fee
            SwapRoute::Direct(pool_key)
        } else {
            // Two-hop swap needed - neither token is FeelsSOL
            let pool1_key = Self::derive_pool_key(token_in, feelssol_mint, 30, program_id);
            let pool2_key = Self::derive_pool_key(feelssol_mint, token_out, 30, program_id);
            SwapRoute::TwoHop(pool1_key, pool2_key)
        }
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
            liquidity_utilization: 85, // TODO: Placeholder - would be calculated from actual pool data
        }
    }
}