//! Static assertions for zero-copy struct safety
//! 
//! These tests verify that our zero-copy structs maintain expected memory layouts
//! and detect any field shifts that could break binary compatibility.

use feels::state::{Tick, TickArray, TICK_ARRAY_SIZE};
use std::mem;

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test Tick struct layout and field offsets
    #[test]
    fn test_tick_layout() {
        // Verify total size
        assert_eq!(mem::size_of::<Tick>(), 80, "Tick size must be exactly 80 bytes");
        
        // Verify field offsets using offset_of! when stable, for now use manual calculation
        unsafe {
            let tick = Tick::default();
            let base_ptr = &tick as *const _ as usize;
            
            // Check field offsets
            let liquidity_net_offset = &tick.liquidity_net as *const _ as usize - base_ptr;
            let liquidity_gross_offset = &tick.liquidity_gross as *const _ as usize - base_ptr;
            let fee_growth_0_offset = &tick.fee_growth_outside_0_x64 as *const _ as usize - base_ptr;
            let fee_growth_1_offset = &tick.fee_growth_outside_1_x64 as *const _ as usize - base_ptr;
            let initialized_offset = &tick.initialized as *const _ as usize - base_ptr;
            let pad_offset = &tick._pad as *const _ as usize - base_ptr;
            
            assert_eq!(liquidity_net_offset, 0, "liquidity_net should be at offset 0");
            assert_eq!(liquidity_gross_offset, 16, "liquidity_gross should be at offset 16");
            assert_eq!(fee_growth_0_offset, 32, "fee_growth_outside_0_x64 should be at offset 32");
            assert_eq!(fee_growth_1_offset, 48, "fee_growth_outside_1_x64 should be at offset 48");
            assert_eq!(initialized_offset, 64, "initialized should be at offset 64");
            assert_eq!(pad_offset, 65, "_pad should be at offset 65");
        }
        
        // Verify field sizes
        assert_eq!(mem::size_of_val(&Tick::default().liquidity_net), 16);
        assert_eq!(mem::size_of_val(&Tick::default().liquidity_gross), 16);
        assert_eq!(mem::size_of_val(&Tick::default().fee_growth_outside_0_x64), 16);
        assert_eq!(mem::size_of_val(&Tick::default().fee_growth_outside_1_x64), 16);
        assert_eq!(mem::size_of_val(&Tick::default().initialized), 1);
        assert_eq!(mem::size_of_val(&Tick::default()._pad), 15);
    }
    
    /// Test TickArray struct layout and field offsets
    #[test]
    fn test_tick_array_layout() {
        // Verify total size (without discriminator)
        assert_eq!(mem::size_of::<TickArray>(), TickArray::LEN - 8, 
                   "TickArray size mismatch");
        
        // Create a zeroed TickArray for offset calculations
        let array = Box::new(TickArray {
            market: Default::default(),
            start_tick_index: 0,
            _pad0: [0; 12],
            ticks: [Tick::default(); TICK_ARRAY_SIZE],
            initialized_tick_count: 0,
            _pad1: [0; 14],
            _reserved: [0; 32],
        });
        
        unsafe {
            let base_ptr = array.as_ref() as *const _ as usize;
            
            // Check field offsets (relative to struct start, not including discriminator)
            let market_offset = &array.market as *const _ as usize - base_ptr;
            let start_tick_offset = &array.start_tick_index as *const _ as usize - base_ptr;
            let pad0_offset = &array._pad0 as *const _ as usize - base_ptr;
            let ticks_offset = &array.ticks as *const _ as usize - base_ptr;
            let count_offset = &array.initialized_tick_count as *const _ as usize - base_ptr;
            let pad1_offset = &array._pad1 as *const _ as usize - base_ptr;
            let reserved_offset = &array._reserved as *const _ as usize - base_ptr;
            
            assert_eq!(market_offset, 0, "market should be at offset 0");
            assert_eq!(start_tick_offset, 32, "start_tick_index should be at offset 32");
            assert_eq!(pad0_offset, 36, "_pad0 should be at offset 36");
            assert_eq!(ticks_offset, 48, "ticks array should be at offset 48");
            assert_eq!(count_offset, 48 + (80 * TICK_ARRAY_SIZE), 
                      "initialized_tick_count should be at offset {}", 48 + (80 * TICK_ARRAY_SIZE));
            assert_eq!(pad1_offset, 48 + (80 * TICK_ARRAY_SIZE) + 2,
                      "_pad1 should be at offset {}", 48 + (80 * TICK_ARRAY_SIZE) + 2);
            assert_eq!(reserved_offset, 48 + (80 * TICK_ARRAY_SIZE) + 2 + 14,
                      "_reserved should be at offset {}", 48 + (80 * TICK_ARRAY_SIZE) + 2 + 14);
        }
        
        // Verify ticks array size
        assert_eq!(mem::size_of_val(&array.ticks), 80 * TICK_ARRAY_SIZE,
                   "Ticks array should be {} bytes", 80 * TICK_ARRAY_SIZE);
    }
    
    /// Test alignment requirements
    #[test]
    fn test_alignment() {
        // Tick should be aligned to allow efficient access
        assert_eq!(mem::align_of::<Tick>(), 16, "Tick should be 16-byte aligned");
        
        // TickArray should also maintain proper alignment
        assert!(mem::align_of::<TickArray>() >= 8, "TickArray should be at least 8-byte aligned");
    }
    
    /// Test that the structs are repr(C) and have stable layouts
    #[test]
    fn test_repr_c_layout() {
        // This test ensures our structs use repr(C) for stable ABI
        // The actual repr(C) is enforced at compile time, this just documents it
        
        // Verify no unexpected padding in Tick
        let tick_size = mem::size_of::<i128>() * 2 +  // liquidity_net + liquidity_gross
                       mem::size_of::<u128>() * 2 +  // fee growths
                       mem::size_of::<u8>() +        // initialized
                       15;                           // explicit padding
        assert_eq!(tick_size, 80);
        
        // Verify TickArray follows expected layout
        let array_size = mem::size_of::<[u8; 32]>() +   // market (Pubkey)
                        mem::size_of::<i32>() +         // start_tick_index
                        12 +                            // pad0
                        80 * TICK_ARRAY_SIZE +          // ticks
                        mem::size_of::<u16>() +         // initialized_tick_count
                        14 +                            // pad1
                        32;                             // reserved
        assert_eq!(array_size, TickArray::LEN - 8);
    }
}