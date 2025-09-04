//! # Route Types
//! 
//! Types for hub-and-spoke routing validation.

use crate::constants::{MAX_ROUTE_HOPS, MAX_SEGMENTS_PER_HOP, MAX_SEGMENTS_PER_TRADE};
use crate::errors::{CoreResult, FeelsCoreError};

/// Segment info for validation
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct SegmentInfo {
    /// Number of segments in this hop
    pub segments: u8,
    /// Size of each segment
    pub segment_size: u64,
}

impl SegmentInfo {
    /// Create new segment info
    pub fn new(segments: u8, segment_size: u64) -> Self {
        Self { segments, segment_size }
    }
    
    /// Total amount for all segments
    pub fn total_amount(&self) -> u64 {
        self.segments as u64 * self.segment_size
    }
}

/// Route validation result
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RouteValidation {
    /// Number of hops in route
    pub hop_count: usize,
    /// Segments per hop
    pub segments: Vec<SegmentInfo>,
    /// Total segments across all hops
    pub total_segments: usize,
    /// Whether route is valid
    pub is_valid: bool,
}

/// Validate route hop count
pub fn validate_hop_count(hops: usize) -> CoreResult<()> {
    if hops > MAX_ROUTE_HOPS {
        return Err(FeelsCoreError::route_too_long(hops, MAX_ROUTE_HOPS));
    }
    Ok(())
}

/// Validate segment count for a hop
pub fn validate_segments_per_hop(segments: usize) -> CoreResult<()> {
    if segments > MAX_SEGMENTS_PER_HOP {
        return Err(FeelsCoreError::too_many_segments(segments, MAX_SEGMENTS_PER_HOP));
    }
    Ok(())
}

/// Validate total segments across all hops
pub fn validate_total_segments(total: usize) -> CoreResult<()> {
    if total > MAX_SEGMENTS_PER_TRADE {
        return Err(FeelsCoreError::too_many_segments(total, MAX_SEGMENTS_PER_TRADE));
    }
    Ok(())
}

/// Calculate segments needed for a trade size
pub fn calculate_segments(amount: u64, max_segment_size: u64) -> CoreResult<SegmentInfo> {
    if max_segment_size == 0 {
        return Err(FeelsCoreError::InvalidAmount);
    }
    
    // Calculate number of segments needed
    let segments = (amount + max_segment_size - 1) / max_segment_size;
    let segments = segments.min(MAX_SEGMENTS_PER_HOP as u64) as u8;
    
    // Calculate actual segment size
    let segment_size = (amount + segments as u64 - 1) / segments as u64;
    
    Ok(SegmentInfo::new(segments, segment_size))
}

/// Validate complete route with all constraints
pub fn validate_route(hop_segments: &[SegmentInfo]) -> CoreResult<RouteValidation> {
    // Validate hop count
    validate_hop_count(hop_segments.len())?;
    
    // Calculate total segments
    let total_segments = hop_segments.iter()
        .map(|s| s.segments as usize)
        .sum();
    
    // Validate per-hop segments
    for (_i, segment) in hop_segments.iter().enumerate() {
        validate_segments_per_hop(segment.segments as usize)
            .map_err(|_| FeelsCoreError::too_many_segments(segment.segments as usize, MAX_SEGMENTS_PER_HOP))?;
    }
    
    // Validate total segments
    validate_total_segments(total_segments)?;
    
    Ok(RouteValidation {
        hop_count: hop_segments.len(),
        segments: hop_segments.to_vec(),
        total_segments,
        is_valid: true,
    })
}