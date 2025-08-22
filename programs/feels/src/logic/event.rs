/// Provides comprehensive event emission for all protocol operations enabling off-chain
/// indexing, analytics, and monitoring. Events follow a consistent structure with pool,
/// timestamp, and actor data. Essential for MEV analysis, volume tracking, and building
/// responsive UIs that react to on-chain state changes in real-time.

use anchor_lang::prelude::*;
use crate::logic::swap::SwapRoute;

// ============================================================================
// Type Definitions
// ============================================================================

pub trait EventBase {
    fn pool(&self) -> Pubkey;
    fn timestamp(&self) -> i64;
    fn actor(&self) -> Pubkey;
}

// ============================================================================
// Routing Logic
// ============================================================================

// SwapRoute implementation moved to swap_manager.rs

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

// ============================================================================
// Helper Functions
// ============================================================================

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

// ============================================================================
// Event Aggregation
// ============================================================================

/// Event aggregation utilities
pub struct EventAggregator;

impl EventAggregator {
    /// Aggregate volume data from multiple events
    pub fn aggregate_volume(events: &[impl EventBase]) -> (u128, u128) {
        // Simplified aggregation - in production would have more sophisticated logic
        let event_count = events.len() as u128;
        (event_count * 1000, event_count * 500) // Mock volume data
    }
    
    /// Calculate time-weighted average price from events
    pub fn calculate_twap(events: &[impl EventBase], window_seconds: i64) -> Option<u128> {
        if events.is_empty() {
            return None;
        }
        
        // Simplified TWAP calculation - in production would use proper price data
        let recent_timestamp = events.last()?.timestamp();
        let cutoff_time = recent_timestamp - window_seconds;
        
        let recent_events: Vec<_> = events.iter()
            .filter(|e| e.timestamp() >= cutoff_time)
            .collect();
        
        if recent_events.is_empty() {
            None
        } else {
            // Mock TWAP calculation
            Some(1_000_000u128) // Placeholder price
        }
    }
}