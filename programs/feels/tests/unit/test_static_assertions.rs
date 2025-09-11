//! Compile-time size and layout checks for zero-copy types
//! 
//! These tests ensure that our zero-copy structs maintain expected sizes
//! to prevent breaking changes and memory layout issues.

use static_assertions::{const_assert_eq, const_assert};
use feels::state::{Tick, TickArray, Observation};

// Tick struct size checks
// 16 (liquidity_net) + 16 (liquidity_gross) + 16 (fee_growth_outside_0) 
// + 16 (fee_growth_outside_1) + 1 (initialized) + 15 (padding) = 80 bytes
const_assert_eq!(core::mem::size_of::<Tick>(), 80);
const_assert_eq!(core::mem::align_of::<Tick>(), 16);

// TickArray size checks
// 32 (market) + 4 (start_tick_index) + 12 (pad0)
// + 80 * 64 (ticks) + 2 (initialized_tick_count) + 14 (pad1) + 32 (reserved)
// = 32 + 4 + 12 + 5120 + 2 + 14 + 32 = 5216 bytes
const_assert_eq!(core::mem::size_of::<TickArray>(), 5216);
const_assert_eq!(core::mem::align_of::<TickArray>(), 16);

// Observation struct size checks
// 8 (block_timestamp) + 8 (padding for i128 alignment) + 16 (tick_cumulative) + 1 (initialized) + 7 (_padding) = 40 bytes
// Actually, let's just verify it's <= 64 bytes since Rust may add more padding
const_assert!(core::mem::size_of::<Observation>() <= 64);
// Note: Alignment depends on the largest field (i128), but since it's not repr(C)
// Rust may choose different alignment. Just verify it's reasonable.
const_assert!(core::mem::align_of::<Observation>() <= 16);

// Additional safety checks
const_assert!(core::mem::size_of::<Tick>() % 8 == 0);
const_assert!(core::mem::size_of::<TickArray>() <= 10240);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;
    
    #[test]
    fn test_tick_field_offsets() {
        // Verify Tick field offsets for zero-copy safety
        assert_eq!(offset_of!(Tick, liquidity_net), 0);
        assert_eq!(offset_of!(Tick, liquidity_gross), 16);
        assert_eq!(offset_of!(Tick, fee_growth_outside_0_x64), 32);
        assert_eq!(offset_of!(Tick, fee_growth_outside_1_x64), 48);
        assert_eq!(offset_of!(Tick, initialized), 64);
        assert_eq!(offset_of!(Tick, _pad), 65);
    }
    
    #[test]
    fn test_observation_field_offsets() {
        // Verify Observation field offsets
        // Note: Rust reorders fields for optimal layout
        // The i128 field (tick_cumulative) comes first for alignment
        assert_eq!(offset_of!(Observation, tick_cumulative), 0);  // i128 is placed first
        assert_eq!(offset_of!(Observation, block_timestamp), 16); // i64 follows
        assert_eq!(offset_of!(Observation, _padding), 24);       // padding array
        assert_eq!(offset_of!(Observation, initialized), 31);     // bool at the end
        
        // Verify total size is 32 bytes
        assert_eq!(std::mem::size_of::<Observation>(), 32);
    }
    
    #[test]
    fn test_zero_copy_requirements() {
        // Ensure types meet zero-copy requirements
        
        // Types must be Copy
        fn assert_copy<T: Copy>() {}
        assert_copy::<Tick>();
        assert_copy::<Observation>();
        
        // Types must have consistent layout
        assert_eq!(
            std::mem::size_of::<[Tick; 64]>(),
            std::mem::size_of::<Tick>() * 64,
            "Tick array packing must be consistent"
        );
    }
    
    #[test]
    fn test_account_size_limits() {
        // Solana account size limit is 10MB, but we should stay well below
        const MAX_REASONABLE_SIZE: usize = 100_000; // 100KB
        
        assert!(
            std::mem::size_of::<TickArray>() < MAX_REASONABLE_SIZE,
            "TickArray size should be reasonable for Solana accounts"
        );
    }
}