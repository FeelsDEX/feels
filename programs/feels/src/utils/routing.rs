/// Routing validation and utilities for hub-and-spoke constraint enforcement
use anchor_lang::prelude::*;
use crate::error::FeelsError;
use crate::state::FeelsSOL;
use crate::constant::{MAX_ROUTE_HOPS, MAX_SEGMENTS_PER_HOP, MAX_SEGMENTS_PER_TRADE};

/// Validates that a pool includes FeelsSOL as one side
pub fn validate_pool_includes_feelssol(
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
    feelssol_mint: &Pubkey,
) -> Result<()> {
    if token_0_mint != feelssol_mint && token_1_mint != feelssol_mint {
        return Err(FeelsError::invalid_route_pool(&format!(
            "{} <-> {}",
            token_0_mint,
            token_1_mint
        )).into());
    }
    Ok(())
}

/// Validates that a route complies with hub constraints
pub fn validate_route(
    route: &[Pubkey],
    feelssol_mint: &Pubkey,
    pools: &[(Pubkey, Pubkey)], // (token0, token1) for each pool
) -> Result<()> {
    // Check hop count
    if route.len() > MAX_ROUTE_HOPS {
        return Err(FeelsError::route_too_long(route.len(), MAX_ROUTE_HOPS).into());
    }
    
    // Validate each pool includes FeelsSOL
    for (i, pool_key) in route.iter().enumerate() {
        if let Some((token_0, token_1)) = pools.get(i) {
            validate_pool_includes_feelssol(token_0, token_1, feelssol_mint)?;
        }
    }
    
    Ok(())
}

/// Validates entry/exit flows use JitoSOL <-> FeelsSOL
pub fn validate_entry_exit_pairing(
    token_in: &Pubkey,
    token_out: &Pubkey,
    jitosol_mint: &Pubkey,
    feelssol_mint: &Pubkey,
) -> Result<()> {
    let is_entry = *token_in == *jitosol_mint && *token_out == *feelssol_mint;
    let is_exit = *token_in == *feelssol_mint && *token_out == *jitosol_mint;
    
    if !is_entry && !is_exit {
        return Err(FeelsError::InvalidEntryExitPairing.into());
    }
    
    Ok(())
}

/// Validates segment count within policy limits
pub fn validate_segment_count(
    segments_per_hop: &[usize],
) -> Result<()> {
    let total_segments: usize = segments_per_hop.iter().sum();
    
    // Check per-hop limits
    for (i, &segments) in segments_per_hop.iter().enumerate() {
        if segments > MAX_SEGMENTS_PER_HOP {
            msg!("Hop {} has {} segments, exceeds limit {}", i, segments, MAX_SEGMENTS_PER_HOP);
            return Err(FeelsError::too_many_segments(segments, MAX_SEGMENTS_PER_HOP).into());
        }
    }
    
    // Check total limit
    if total_segments > MAX_SEGMENTS_PER_TRADE {
        return Err(FeelsError::too_many_segments(total_segments, MAX_SEGMENTS_PER_TRADE).into());
    }
    
    Ok(())
}

/// Helper to determine if a trade requires two hops
pub fn requires_two_hops(
    token_in: &Pubkey,
    token_out: &Pubkey,
    feelssol_mint: &Pubkey,
) -> bool {
    *token_in != *feelssol_mint && *token_out != *feelssol_mint
}

/// Builds the pool route for a token pair
pub fn build_route(
    token_in: &Pubkey,
    token_out: &Pubkey,
    feelssol_mint: &Pubkey,
) -> Vec<(Pubkey, Pubkey)> {
    if requires_two_hops(token_in, token_out, feelssol_mint) {
        // Two hop: TokenA -> FeelsSOL -> TokenB
        vec![
            (token_in.clone(), feelssol_mint.clone()),
            (feelssol_mint.clone(), token_out.clone()),
        ]
    } else {
        // Single hop: one token is already FeelsSOL
        vec![(token_in.clone(), token_out.clone())]
    }
}