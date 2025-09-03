/// Centralized bitmap operations for the Feels Protocol.
/// Provides consistent, safe bit manipulation utilities used throughout the codebase.
use anchor_lang::prelude::*;
use crate::error::FeelsError;

// ============================================================================
// Constants
// ============================================================================

/// Number of bits in a u64 word
pub const WORD_BITS: usize = 64;

/// Maximum bits in a u8 bitmap
pub const U8_BITS: usize = 8;

/// Maximum bits in a u64 bitmap
pub const U64_BITS: usize = 64;

// ============================================================================
// Single-Word Bitmap Operations
// ============================================================================

/// Operations on u8 bitmaps (used for router active slots, duration masks)
pub mod u8_bitmap {
    use super::*;
    
    /// Set a bit in a u8 bitmap
    #[inline(always)]
    pub fn set_bit(bitmap: &mut u8, index: usize) -> Result<()> {
        require!(
            index < U8_BITS,
            FeelsError::ValidationError
        );
        *bitmap |= 1u8 << index;
        Ok(())
    }
    
    /// Clear a bit in a u8 bitmap
    #[inline(always)]
    pub fn clear_bit(bitmap: &mut u8, index: usize) -> Result<()> {
        require!(
            index < U8_BITS,
            FeelsError::ValidationError
        );
        *bitmap &= !(1u8 << index);
        Ok(())
    }
    
    /// Check if a bit is set in a u8 bitmap
    #[inline(always)]
    pub fn is_bit_set(bitmap: u8, index: usize) -> Result<bool> {
        require!(
            index < U8_BITS,
            FeelsError::ValidationError
        );
        Ok((bitmap & (1u8 << index)) != 0)
    }
    
    /// Toggle a bit in a u8 bitmap
    #[inline(always)]
    pub fn toggle_bit(bitmap: &mut u8, index: usize) -> Result<()> {
        require!(
            index < U8_BITS,
            FeelsError::ValidationError
        );
        *bitmap ^= 1u8 << index;
        Ok(())
    }
    
    /// Count set bits in a u8 bitmap
    #[inline(always)]
    pub fn count_set_bits(bitmap: u8) -> u32 {
        bitmap.count_ones()
    }
}

/// Operations on u64 bitmaps (used for single-word tick bitmaps)
pub mod u64_bitmap {
    use super::*;
    
    /// Set a bit in a u64 bitmap
    #[inline(always)]
    pub fn set_bit(bitmap: &mut u64, index: usize) -> Result<()> {
        require!(
            index < U64_BITS,
            FeelsError::ValidationError
        );
        *bitmap |= 1u64 << index;
        Ok(())
    }
    
    /// Clear a bit in a u64 bitmap
    #[inline(always)]
    pub fn clear_bit(bitmap: &mut u64, index: usize) -> Result<()> {
        require!(
            index < U64_BITS,
            FeelsError::ValidationError
        );
        *bitmap &= !(1u64 << index);
        Ok(())
    }
    
    /// Check if a bit is set in a u64 bitmap
    #[inline(always)]
    pub fn is_bit_set(bitmap: u64, index: usize) -> Result<bool> {
        require!(
            index < U64_BITS,
            FeelsError::ValidationError
        );
        Ok((bitmap & (1u64 << index)) != 0)
    }
    
    /// Find the next set bit after the given index
    #[inline(always)]
    pub fn next_set_bit(bitmap: u64, start_index: usize) -> Option<usize> {
        if start_index >= U64_BITS {
            return None;
        }
        
        // Mask off bits before start_index
        let mask = u64::MAX << start_index;
        let masked = bitmap & mask;
        
        if masked == 0 {
            None
        } else {
            Some(masked.trailing_zeros() as usize)
        }
    }
}

// ============================================================================
// Multi-Word Bitmap Operations
// ============================================================================

/// Operations on multi-word bitmaps (used for tick array bitmaps)
pub mod multi_word_bitmap {
    use super::*;
    
    /// Calculate word and bit indices for a given bit position
    #[inline(always)]
    pub fn get_word_and_bit_index(bit_position: usize) -> (usize, usize) {
        let word_index = bit_position / WORD_BITS;
        let bit_index = bit_position % WORD_BITS;
        (word_index, bit_index)
    }
    
    /// Set a bit in a multi-word bitmap
    #[inline(always)]
    pub fn set_bit(bitmap: &mut [u64], bit_position: usize) -> Result<()> {
        let (word_index, bit_index) = get_word_and_bit_index(bit_position);
        require!(
            word_index < bitmap.len(),
            FeelsError::ValidationError
        );
        bitmap[word_index] |= 1u64 << bit_index;
        Ok(())
    }
    
    /// Clear a bit in a multi-word bitmap
    #[inline(always)]
    pub fn clear_bit(bitmap: &mut [u64], bit_position: usize) -> Result<()> {
        let (word_index, bit_index) = get_word_and_bit_index(bit_position);
        require!(
            word_index < bitmap.len(),
            FeelsError::ValidationError
        );
        bitmap[word_index] &= !(1u64 << bit_index);
        Ok(())
    }
    
    /// Check if a bit is set in a multi-word bitmap
    #[inline(always)]
    pub fn is_bit_set(bitmap: &[u64], bit_position: usize) -> Result<bool> {
        let (word_index, bit_index) = get_word_and_bit_index(bit_position);
        require!(
            word_index < bitmap.len(),
            FeelsError::ValidationError
        );
        Ok((bitmap[word_index] & (1u64 << bit_index)) != 0)
    }
    
    /// Find the next set bit after the given position
    pub fn next_set_bit(bitmap: &[u64], start_position: usize) -> Option<usize> {
        let (mut word_index, bit_index) = get_word_and_bit_index(start_position);
        
        if word_index >= bitmap.len() {
            return None;
        }
        
        // Check current word first (with bits before start masked)
        let mask = u64::MAX << bit_index;
        let masked = bitmap[word_index] & mask;
        
        if masked != 0 {
            let bit_offset = masked.trailing_zeros() as usize;
            return Some(word_index * WORD_BITS + bit_offset);
        }
        
        // Check subsequent words
        word_index += 1;
        while word_index < bitmap.len() {
            if bitmap[word_index] != 0 {
                let bit_offset = bitmap[word_index].trailing_zeros() as usize;
                return Some(word_index * WORD_BITS + bit_offset);
            }
            word_index += 1;
        }
        
        None
    }
    
    /// Find the previous set bit before the given position
    pub fn prev_set_bit(bitmap: &[u64], start_position: usize) -> Option<usize> {
        if start_position == 0 || bitmap.is_empty() {
            return None;
        }
        
        let search_position = start_position.saturating_sub(1);
        let (mut word_index, bit_index) = get_word_and_bit_index(search_position);
        
        if word_index >= bitmap.len() {
            word_index = bitmap.len() - 1;
        }
        
        // Check current word first (with bits after start masked)
        let mask = if bit_index == WORD_BITS - 1 {
            u64::MAX
        } else {
            (1u64 << (bit_index + 1)) - 1
        };
        let masked = bitmap[word_index] & mask;
        
        if masked != 0 {
            let bit_offset = 63 - masked.leading_zeros() as usize;
            return Some(word_index * WORD_BITS + bit_offset);
        }
        
        // Check previous words
        while word_index > 0 {
            word_index -= 1;
            if bitmap[word_index] != 0 {
                let bit_offset = 63 - bitmap[word_index].leading_zeros() as usize;
                return Some(word_index * WORD_BITS + bit_offset);
            }
        }
        
        None
    }
    
    /// Count total set bits in a multi-word bitmap
    pub fn count_set_bits(bitmap: &[u64]) -> u32 {
        bitmap.iter().map(|word| word.count_ones()).sum()
    }
}

// ============================================================================
// Bit Encoding/Decoding Utilities
// ============================================================================

/// Utilities for encoding multiple values into a single integer
pub mod bit_encoding {
    use super::*;
    
    /// Create a bitmask for a given number of bits
    #[inline(always)]
    pub fn create_mask(bits: u8) -> u64 {
        if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        }
    }
    
    /// Extract a value from a packed integer
    #[inline(always)]
    pub fn extract_bits(packed: u64, shift: u8, bits: u8) -> u64 {
        (packed >> shift) & create_mask(bits)
    }
    
    /// Pack a value into a packed integer
    #[inline(always)]
    pub fn pack_bits(packed: &mut u64, value: u64, shift: u8, bits: u8) -> Result<()> {
        let mask = create_mask(bits);
        require!(
            value <= mask,
            FeelsError::ValidationError
        );
        
        // Clear the target bits
        *packed &= !(mask << shift);
        // Set the new value
        *packed |= (value & mask) << shift;
        
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_u8_bitmap_operations() {
        let mut bitmap: u8 = 0;
        
        // Set bit 3
        assert!(u8_bitmap::set_bit(&mut bitmap, 3).is_ok());
        assert_eq!(bitmap, 0b00001000);
        
        // Check bit 3 is set
        assert!(u8_bitmap::is_bit_set(bitmap, 3).unwrap());
        
        // Clear bit 3
        assert!(u8_bitmap::clear_bit(&mut bitmap, 3).is_ok());
        assert_eq!(bitmap, 0);
        
        // Test bounds checking
        assert!(u8_bitmap::set_bit(&mut bitmap, 8).is_err());
    }
    
    #[test]
    fn test_multi_word_bitmap() {
        let mut bitmap = vec![0u64; 16]; // 1024-bit bitmap
        
        // Set bit 100
        assert!(multi_word_bitmap::set_bit(&mut bitmap, 100).is_ok());
        assert!(multi_word_bitmap::is_bit_set(&bitmap, 100).unwrap());
        
        // Find next set bit
        assert_eq!(multi_word_bitmap::next_set_bit(&bitmap, 0), Some(100));
        assert_eq!(multi_word_bitmap::next_set_bit(&bitmap, 101), None);
        
        // Set bit 500
        assert!(multi_word_bitmap::set_bit(&mut bitmap, 500).is_ok());
        assert_eq!(multi_word_bitmap::next_set_bit(&bitmap, 101), Some(500));
        
        // Count bits
        assert_eq!(multi_word_bitmap::count_set_bits(&bitmap), 2);
    }
    
    #[test]
    fn test_bit_encoding() {
        let mut packed = 0u64;
        
        // Pack 5 into bits 0-3 (4 bits)
        assert!(bit_encoding::pack_bits(&mut packed, 5, 0, 4).is_ok());
        assert_eq!(packed, 0b0101);
        
        // Pack 3 into bits 4-5 (2 bits)
        assert!(bit_encoding::pack_bits(&mut packed, 3, 4, 2).is_ok());
        assert_eq!(packed, 0b110101);
        
        // Extract values
        assert_eq!(bit_encoding::extract_bits(packed, 0, 4), 5);
        assert_eq!(bit_encoding::extract_bits(packed, 4, 2), 3);
        
        // Test overflow
        assert!(bit_encoding::pack_bits(&mut packed, 16, 0, 4).is_err());
    }
}