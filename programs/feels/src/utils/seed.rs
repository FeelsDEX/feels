/// Ensures deterministic PDA derivation by enforcing canonical token ordering.
/// Prevents duplicate pools by sorting token mints consistently regardless of
/// user input order. Now includes tick array PDA derivation helpers to centralize
/// all PDA logic. Critical for maintaining pool uniqueness and enabling efficient
/// pool and tick array discovery without requiring additional lookup tables.

use anchor_lang::prelude::*;
use std::cmp::Ordering;

// ============================================================================
// Canonical Seed Implementation
// ============================================================================

pub struct CanonicalSeeds;

impl CanonicalSeeds {
    /// Sort two token mints into canonical order for consistent PDA derivation
    /// Returns (token_0, token_1) where token_0 < token_1 by byte comparison
    pub fn sort_token_mints(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, Pubkey) {
        match mint_a.as_ref().cmp(mint_b.as_ref()) {
            Ordering::Less => (*mint_a, *mint_b),
            Ordering::Greater => (*mint_b, *mint_a),
            Ordering::Equal => (*mint_a, *mint_b), // Same token, though this shouldn't happen
        }
    }
    
    /// Derive pool PDA with canonical seed ordering
    /// This ensures only one pool can exist for any token pair regardless of input order
    pub fn derive_pool_pda(
        mint_a: &Pubkey,
        mint_b: &Pubkey,
        fee_rate: u16,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        let (token_0, token_1) = Self::sort_token_mints(mint_a, mint_b);
        
        Pubkey::find_program_address(
            &[
                b"pool",
                token_0.as_ref(),
                token_1.as_ref(),
                &fee_rate.to_le_bytes(),
            ],
            program_id,
        )
    }
    
    /// Get pool seeds in canonical order for PDA signing
    pub fn get_pool_seeds(
        mint_a: &Pubkey,
        mint_b: &Pubkey,
        fee_rate: u16,
        bump: u8,
    ) -> Vec<Vec<u8>> {
        let (token_0, token_1) = Self::sort_token_mints(mint_a, mint_b);
        
        vec![
            b"pool".to_vec(),
            token_0.as_ref().to_vec(),
            token_1.as_ref().to_vec(),
            fee_rate.to_le_bytes().to_vec(),
            vec![bump],
        ]
    }
    
    /// Validate that tokens are in canonical order
    pub fn validate_canonical_order(token_a: &Pubkey, token_b: &Pubkey) -> Result<()> {
        require!(
            token_a.as_ref() <= token_b.as_ref(),
            ErrorCode::NonCanonicalTokenOrder
        );
        Ok(())
    }
    
    /// Check if token pair needs to be swapped for canonical ordering
    pub fn needs_swap(mint_a: &Pubkey, mint_b: &Pubkey) -> bool {
        mint_a.as_ref() > mint_b.as_ref()
    }
    
    /// Derive tick array PDA for a given pool and start tick
    /// This helper centralizes the tick array PDA derivation logic
    pub fn derive_tick_array_pda(
        pool: &Pubkey,
        start_tick: i32,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"tick_array",
                pool.as_ref(),
                &start_tick.to_le_bytes(),
            ],
            program_id,
        )
    }
    
    /// Get tick array seeds for PDA signing
    pub fn get_tick_array_seeds(
        pool: &Pubkey,
        start_tick: i32,
        bump: u8,
    ) -> Vec<Vec<u8>> {
        vec![
            b"tick_array".to_vec(),
            pool.as_ref().to_vec(),
            start_tick.to_le_bytes().to_vec(),
            vec![bump],
        ]
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Token mints must be in canonical order (sorted by bytes)")]
    NonCanonicalTokenOrder,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_canonical_ordering() {
        let mint_a = Pubkey::new_from_array([1; 32]);
        let mint_b = Pubkey::new_from_array([2; 32]);
        
        // Test ordering is consistent regardless of input order
        let (token_0_ab, token_1_ab) = CanonicalSeeds::sort_token_mints(&mint_a, &mint_b);
        let (token_0_ba, token_1_ba) = CanonicalSeeds::sort_token_mints(&mint_b, &mint_a);
        
        assert_eq!(token_0_ab, token_0_ba);
        assert_eq!(token_1_ab, token_1_ba);
        assert_eq!(token_0_ab, mint_a);
        assert_eq!(token_1_ab, mint_b);
    }
    
    #[test]
    fn test_pda_consistency() {
        let mint_a = Pubkey::new_from_array([5; 32]);
        let mint_b = Pubkey::new_from_array([3; 32]);
        let fee_rate = 30u16;
        let program_id = Pubkey::new_from_array([10; 32]);
        
        // PDAs should be identical regardless of mint order
        let (pda_ab, bump_ab) = CanonicalSeeds::derive_pool_pda(&mint_a, &mint_b, fee_rate, &program_id);
        let (pda_ba, bump_ba) = CanonicalSeeds::derive_pool_pda(&mint_b, &mint_a, fee_rate, &program_id);
        
        assert_eq!(pda_ab, pda_ba);
        assert_eq!(bump_ab, bump_ba);
    }
    
    #[test]
    fn test_needs_swap() {
        let mint_low = Pubkey::new_from_array([1; 32]);
        let mint_high = Pubkey::new_from_array([255; 32]);
        
        assert!(!CanonicalSeeds::needs_swap(&mint_low, &mint_high));
        assert!(CanonicalSeeds::needs_swap(&mint_high, &mint_low));
    }
}