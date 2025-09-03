/// Unit tests for hub-constrained routing validation
use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use feels::{
    constant::{MAX_ROUTE_HOPS, MAX_SEGMENTS_PER_HOP, MAX_SEGMENTS_PER_TRADE},
    error::FeelsError,
    utils::routing::{
        validate_route, validate_pool_includes_feelssol, 
        validate_entry_exit_pairing, build_route
    },
    utils::segment_validation::{
        validate_segment_caps, calculate_segments, SegmentInfo
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_feelssol() -> Pubkey {
        Pubkey::new_from_array([1u8; 32])
    }

    fn mock_token_a() -> Pubkey {
        Pubkey::new_from_array([2u8; 32])
    }

    fn mock_token_b() -> Pubkey {
        Pubkey::new_from_array([3u8; 32])
    }

    fn mock_jitosol() -> Pubkey {
        Pubkey::new_from_array([4u8; 32])
    }

    #[test]
    fn test_pool_validation() {
        let feelssol = mock_feelssol();
        let token_a = mock_token_a();
        let token_b = mock_token_b();

        // Valid pool with FeelsSOL
        assert!(validate_pool_includes_feelssol(&feelssol, &token_a, &feelssol).is_ok());
        assert!(validate_pool_includes_feelssol(&token_a, &feelssol, &feelssol).is_ok());

        // Invalid pool without FeelsSOL
        assert!(validate_pool_includes_feelssol(&token_a, &token_b, &feelssol).is_err());
    }

    #[test]
    fn test_route_validation() {
        let feelssol = mock_feelssol();
        let token_a = mock_token_a();
        let token_b = mock_token_b();

        // Valid 1-hop route
        let route1 = vec![feelssol];
        assert!(validate_route(&route1, 2).is_ok());

        // Valid 2-hop route
        let route2 = vec![feelssol, feelssol];
        assert!(validate_route(&route2, 2).is_ok());

        // Invalid 3-hop route
        let route3 = vec![feelssol, feelssol, feelssol];
        assert!(validate_route(&route3, 2).is_err());

        // Empty route
        let route_empty = vec![];
        assert!(validate_route(&route_empty, 2).is_err());
    }

    #[test]
    fn test_entry_exit_pairing() {
        let feelssol = mock_feelssol();
        let jitosol = mock_jitosol();
        let token_a = mock_token_a();

        // Valid entry pairing (JitoSOL -> FeelsSOL)
        assert!(validate_entry_exit_pairing(&jitosol, &feelssol, &jitosol, &feelssol).is_ok());

        // Valid exit pairing (FeelsSOL -> JitoSOL)
        assert!(validate_entry_exit_pairing(&feelssol, &jitosol, &jitosol, &feelssol).is_ok());

        // Invalid pairing (wrong token)
        assert!(validate_entry_exit_pairing(&token_a, &feelssol, &jitosol, &feelssol).is_err());
        assert!(validate_entry_exit_pairing(&feelssol, &token_a, &jitosol, &feelssol).is_err());
    }

    #[test]
    fn test_route_building() {
        let feelssol = mock_feelssol();
        let token_a = mock_token_a();
        let token_b = mock_token_b();

        // Direct route (one token is FeelsSOL)
        let route1 = build_route(&token_a, &feelssol, &feelssol);
        assert_eq!(route1.len(), 1);

        let route2 = build_route(&feelssol, &token_b, &feelssol);
        assert_eq!(route2.len(), 1);

        // 2-hop route through FeelsSOL
        let route3 = build_route(&token_a, &token_b, &feelssol);
        assert_eq!(route3.len(), 2);

        // Same token (should return single element)
        let route4 = build_route(&token_a, &token_a, &feelssol);
        assert_eq!(route4.len(), 1);
    }

    #[test]
    fn test_segment_validation() {
        // Valid single hop
        let segments1 = vec![SegmentInfo { segments: 5, segment_size: 1000 }];
        assert!(validate_segment_caps(&segments1).is_ok());

        // Maximum segments per hop
        let segments2 = vec![SegmentInfo { segments: MAX_SEGMENTS_PER_HOP as u8, segment_size: 1000 }];
        assert!(validate_segment_caps(&segments2).is_ok());

        // Exceeds per-hop limit
        let segments3 = vec![SegmentInfo { segments: (MAX_SEGMENTS_PER_HOP + 1) as u8, segment_size: 1000 }];
        assert!(validate_segment_caps(&segments3).is_err());

        // Valid two-hop
        let segments4 = vec![
            SegmentInfo { segments: 5, segment_size: 1000 },
            SegmentInfo { segments: 5, segment_size: 1000 },
        ];
        assert!(validate_segment_caps(&segments4).is_ok());

        // Two-hop exceeds total limit
        let segments5 = vec![
            SegmentInfo { segments: MAX_SEGMENTS_PER_HOP as u8, segment_size: 1000 },
            SegmentInfo { segments: (MAX_SEGMENTS_PER_HOP + 1) as u8, segment_size: 1000 },
        ];
        assert!(validate_segment_caps(&segments5).is_err());
    }

    #[test]
    fn test_segment_calculation() {
        // Exact division
        let seg1 = calculate_segments(10_000, 1_000).unwrap();
        assert_eq!(seg1.segments, 10);
        assert_eq!(seg1.segment_size, 1_000);

        // Needs rounding up
        let seg2 = calculate_segments(10_100, 1_000).unwrap();
        assert_eq!(seg2.segments, MAX_SEGMENTS_PER_HOP as u8); // Capped
        assert_eq!(seg2.segment_size, 1_010); // Adjusted size

        // Small amount
        let seg3 = calculate_segments(500, 1_000).unwrap();
        assert_eq!(seg3.segments, 1);
        assert_eq!(seg3.segment_size, 500);

        // Zero segment size (should error)
        assert!(calculate_segments(1_000, 0).is_err());
    }

    #[test]
    fn test_route_constants() {
        // Ensure constants are reasonable
        assert!(MAX_ROUTE_HOPS > 0 && MAX_ROUTE_HOPS <= 3);
        assert!(MAX_SEGMENTS_PER_HOP > 0 && MAX_SEGMENTS_PER_HOP <= 100);
        assert!(MAX_SEGMENTS_PER_TRADE >= MAX_SEGMENTS_PER_HOP);
        assert!(MAX_SEGMENTS_PER_TRADE <= MAX_SEGMENTS_PER_HOP * MAX_ROUTE_HOPS);
    }
}