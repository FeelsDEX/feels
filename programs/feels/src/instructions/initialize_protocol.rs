/// Initializes the Feels Protocol's global state and configuration.
/// This is a one-time setup instruction that establishes the protocol's authority,
/// fee configuration, and other protocol-wide parameters that govern all pools.
/// Must be called before any pools can be created or other operations performed.

use anchor_lang::prelude::*;
use crate::utils::MAX_FEE_RATE;

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize the Feels Protocol
pub fn handler(ctx: Context<crate::InitializeFeels>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    let clock = Clock::get()?;
    
    // Set protocol authority
    protocol_state.authority = ctx.accounts.authority.key();
    protocol_state.treasury = ctx.accounts.treasury.key();
    
    // Set default fee parameters
    protocol_state.default_protocol_fee_rate = 2000; // 20% of pool fees (2000/10000)
    protocol_state.max_pool_fee_rate = MAX_FEE_RATE; // 100% max (10000 basis points)
    
    // Enable protocol operations
    protocol_state.paused = false;
    protocol_state.pool_creation_allowed = true;
    
    // Initialize counters
    protocol_state.total_pools = 0;
    protocol_state.total_fees_collected = 0;
    
    // Set timestamp
    protocol_state.initialized_at = clock.unix_timestamp;
    
    // Clear reserved space
    protocol_state._reserved = [0u8; 128];
    
    Ok(())
}