/// Segment validation for route and segmentation caps
use anchor_lang::prelude::*;
use crate::constant::{MAX_SEGMENTS_PER_HOP, MAX_SEGMENTS_PER_TRADE};
use crate::error::FeelsError;

/// Segment info for validation
#[derive(Debug, Clone, Copy)]
pub struct SegmentInfo {
    /// Number of segments in this hop
    pub segments: u8,
    /// Size of each segment
    pub segment_size: u64,
}

/// Validate segment counts for a route
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