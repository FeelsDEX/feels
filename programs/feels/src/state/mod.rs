//! MVP state structures
//! 
//! Minimal state for Phase 1 implementation

pub mod market;
pub mod buffer;
pub mod epoch_params;
pub mod tick;
pub mod position;
pub mod oracle;
pub mod token_metadata;
pub mod liquidity_commitment;

pub use market::*;
pub use buffer::*;
pub use epoch_params::*;
pub use tick::*;
pub use position::*;
pub use oracle::*;
pub use token_metadata::*;
pub use liquidity_commitment::*;

// Compile-time assertions for zero_copy struct sizes
// These ensure our structs maintain expected memory layout
#[cfg(feature = "size-checks")]
mod size_assertions {
    use super::*;
    use static_assertions::const_assert_eq;
    
    // Tick struct must be exactly 80 bytes for Pod safety
    const_assert_eq!(core::mem::size_of::<Tick>(), 80);
    
    // TickArray size calculation:
    // 8 (discriminator) + 32 (market) + 4 (start_tick_index) + 12 (pad0)
    // + (80 * 64) (ticks) + 2 (initialized_tick_count) + 14 (pad1) + 32 (reserved)
    const_assert_eq!(TickArray::LEN, 8 + 32 + 4 + 12 + (80 * 64) + 2 + 14 + 32);
    const_assert_eq!(TickArray::LEN, 5184); // Verify exact value
    
    // Verify zero-copy struct sizes match expected layout
    const_assert_eq!(core::mem::size_of::<TickArray>() + 8, TickArray::LEN);
    const_assert_eq!(core::mem::size_of::<TickArray>(), 5184 - 8);
    
    // Verify Market and Position sizes match their LEN constants  
    const_assert_eq!(core::mem::size_of::<Market>() + 8, Market::LEN);
    const_assert_eq!(core::mem::size_of::<Position>() + 8, Position::LEN);
    
    // Buffer size check
    const_assert_eq!(core::mem::size_of::<Buffer>() + 8, Buffer::LEN);
}
