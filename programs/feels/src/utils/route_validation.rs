/// Route validation for hub-and-spoke architecture
/// 
/// This module extends feels-core validation with on-chain specific constraints.
use anchor_lang::prelude::*;
use crate::error::FeelsError;

// Re-export validation functions from feels-core
pub use feels_core::types::routes::{
    SegmentInfo,
    RouteValidation,
    validate_hop_count,
    validate_segments_per_hop,
    validate_total_segments,
    calculate_segments,
    validate_route as validate_route_segments,
};

// ============================================================================
// Route Validation
// ============================================================================

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

/// Helper to determine if a trade requires two hops
pub fn requires_two_hops(
    token_in: &Pubkey,
    token_out: &Pubkey,
    feelssol_mint: &Pubkey,
) -> bool {
    *token_in != *feelssol_mint && *token_out != *feelssol_mint
}

// ============================================================================
// Segment Validation
// ============================================================================

/// Segment info for validation
#[derive(Debug, Clone, Copy)]
pub struct SegmentInfo {
    /// Number of segments in this hop
    pub segments: u8,
    /// Size of each segment
    pub segment_size: u64,
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

/// Validate segment caps for a route
pub fn validate_segment_caps(hop_segments: &[SegmentInfo]) -> Result<()> {
    // Check hop count (already validated by route validation)
    require!(
        hop_segments.len() <= 2,
        FeelsError::RouteTooLong
    );
    
    let mut total_segments = 0u8;
    
    // Validate each hop
    for (hop_idx, segment_info) in hop_segments.iter().enumerate() {
        // Check segments per hop
        require!(
            segment_info.segments as usize <= MAX_SEGMENTS_PER_HOP,
            FeelsError::TooManySegments
        );
        
        // Accumulate total
        total_segments = total_segments
            .checked_add(segment_info.segments)
            .ok_or(FeelsError::MathOverflow)?;
        
        msg!(
            "Hop {}: {} segments of size {}",
            hop_idx,
            segment_info.segments,
            segment_info.segment_size
        );
    }
    
    // Check total segments
    require!(
        total_segments as usize <= MAX_SEGMENTS_PER_TRADE,
        FeelsError::TooManySegments
    );
    
    msg!("Total segments: {} (max: {})", total_segments, MAX_SEGMENTS_PER_TRADE);
    
    Ok(())
}

/// Calculate segments needed for a trade size
pub fn calculate_segments(
    amount: u64,
    max_segment_size: u64,
) -> Result<SegmentInfo> {
    // Avoid division by zero
    require!(max_segment_size > 0, FeelsError::InvalidAmount);
    
    // Calculate number of segments needed
    let segments = amount
        .checked_div(max_segment_size)
        .ok_or(FeelsError::MathOverflow)?
        .checked_add(if amount % max_segment_size > 0 { 1 } else { 0 })
        .ok_or(FeelsError::MathOverflow)?;
    
    // Cap at maximum
    let segments = segments.min(MAX_SEGMENTS_PER_HOP as u64) as u8;
    
    // Calculate actual segment size
    let segment_size = amount
        .checked_div(segments as u64)
        .ok_or(FeelsError::MathOverflow)?
        .checked_add(if amount % (segments as u64) > 0 { 1 } else { 0 })
        .ok_or(FeelsError::MathOverflow)?;
    
    Ok(SegmentInfo {
        segments,
        segment_size,
    })
}

/// Validate and optimize segments for a two-hop route
pub fn validate_two_hop_segments(
    hop1_amount: u64,
    hop2_amount: u64,
    hop1_max_segment: u64,
    hop2_max_segment: u64,
) -> Result<(SegmentInfo, SegmentInfo)> {
    // Calculate segments for each hop
    let hop1_segments = calculate_segments(hop1_amount, hop1_max_segment)?;
    let hop2_segments = calculate_segments(hop2_amount, hop2_max_segment)?;
    
    // Validate total doesn't exceed limit
    let total = hop1_segments.segments
        .checked_add(hop2_segments.segments)
        .ok_or(FeelsError::MathOverflow)?;
        
    require!(
        total as usize <= MAX_SEGMENTS_PER_TRADE,
        FeelsError::TooManySegments
    );
    
    Ok((hop1_segments, hop2_segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_segment_validation() {
        // Valid single hop
        let segments = vec![SegmentInfo { segments: 5, segment_size: 1000 }];
        assert!(validate_segment_caps(&segments).is_ok());
        
        // Valid two hop
        let segments = vec![
            SegmentInfo { segments: 8, segment_size: 1000 },
            SegmentInfo { segments: 10, segment_size: 500 },
        ];
        assert!(validate_segment_caps(&segments).is_ok());
        
        // Too many in one hop
        let segments = vec![SegmentInfo { segments: 11, segment_size: 1000 }];
        assert!(validate_segment_caps(&segments).is_err());
        
        // Too many total
        let segments = vec![
            SegmentInfo { segments: 10, segment_size: 1000 },
            SegmentInfo { segments: 11, segment_size: 500 },
        ];
        assert!(validate_segment_caps(&segments).is_err());
    }
    
    #[test]
    fn test_segment_calculation() {
        // Exact division
        let seg = calculate_segments(10000, 1000).unwrap();
        assert_eq!(seg.segments, 10);
        assert_eq!(seg.segment_size, 1000);
        
        // With remainder
        let seg = calculate_segments(10500, 1000).unwrap();
        assert_eq!(seg.segments, 10); // Capped at MAX_SEGMENTS_PER_HOP
        assert_eq!(seg.segment_size, 1050);
    }
}