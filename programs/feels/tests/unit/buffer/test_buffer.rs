#[cfg(test)]
mod tests {
    use feels::state::{Buffer, FeeDomain};
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_floor_placement_due_no_overflow() {
        // Create a buffer with a reasonable threshold
        let buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: 0,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000_000, // 1 billion
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };

        // Test with values that would overflow u64 if added naively
        let token_0_value: u64 = u64::MAX / 2 + 1000;
        let token_1_value: u64 = u64::MAX / 2 + 1000;
        
        // This should not panic - saturating_add prevents overflow
        let result = buffer.floor_placement_due(token_0_value, token_1_value);
        
        // The result should be true since the sum exceeds the threshold
        assert!(result);
    }

    #[test]
    fn test_floor_placement_due_normal_case() {
        let buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: 0,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };

        // Test with values below threshold
        let token_0_value: u64 = 400_000;
        let token_1_value: u64 = 500_000;
        
        let result = buffer.floor_placement_due(token_0_value, token_1_value);
        
        // Should be false since 400k + 500k = 900k < 1M
        assert!(!result);
        
        // Test with values at threshold
        let token_0_value: u64 = 600_000;
        let token_1_value: u64 = 400_000;
        
        let result = buffer.floor_placement_due(token_0_value, token_1_value);
        
        // Should be true since 600k + 400k = 1M
        assert!(result);
    }

    #[test]
    fn test_buffer_tau_overflow_protection() {
        let mut buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: u128::MAX - 1000,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };

        // Test get_total_tau with near-max values
        let total = buffer.get_total_tau();
        assert_eq!(total, u128::MAX - 1000);

        // Test adding more would saturate, not overflow
        buffer.tau_time = 2000;
        let total = buffer.get_total_tau();
        assert_eq!(total, u128::MAX); // Should saturate at MAX
    }

    #[test]
    fn test_buffer_fee_collection_overflow_protection() {
        let mut buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: u128::MAX - 100,
            fees_token_1: 0,
            tau_spot: 0,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };

        // Test that collect_fee handles overflow correctly
        
        // Try to add 200 when we're at MAX - 100
        let result = buffer.collect_fee(200, 0, FeeDomain::Spot);
        
        // Should return error for overflow
        assert!(result.is_err());
        
        // FIXED: collect_fee is now transactional - no state is modified on error
        assert_eq!(buffer.tau_spot, 0); // tau should remain unchanged
        assert_eq!(buffer.fees_token_0, u128::MAX - 100); // fees should remain unchanged
        
        // But adding 50 should work
        let initial_fees = buffer.fees_token_0;
        let initial_tau = buffer.tau_spot;
        let result = buffer.collect_fee(50, 0, FeeDomain::Spot);
        assert!(result.is_ok());
        assert_eq!(buffer.tau_spot, initial_tau + 50);
        assert_eq!(buffer.tau_spot, 50); // tau_spot should be 0 + 50
        // fees_token_0 should be initial + 50
        assert_eq!(buffer.fees_token_0, initial_fees + 50);
        // Verify the exact value
        assert_eq!(buffer.fees_token_0, u128::MAX - 50);
    }

    #[test]
    fn test_buffer_collect_fee_transactional() {
        // Test that collect_fee is fully transactional
        let mut buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 100,
            fees_token_1: 200,
            tau_spot: u128::MAX - 50,  // Near overflow for tau
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };

        // Test case 1: tau overflow - should fail without modifying any state
        let initial_tau = buffer.tau_spot;
        let initial_fees = buffer.fees_token_0;
        
        let result = buffer.collect_fee(100, 0, FeeDomain::Spot);
        assert!(result.is_err());
        assert_eq!(buffer.tau_spot, initial_tau); // tau unchanged
        assert_eq!(buffer.fees_token_0, initial_fees); // fees unchanged
        
        // Test case 2: fee overflow - should fail without modifying any state
        let mut buffer2 = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: u128::MAX - 50,  // Near overflow for fees
            fees_token_1: 200,
            tau_spot: 100,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
        };
        
        let initial_tau2 = buffer2.tau_spot;
        let initial_fees2 = buffer2.fees_token_0;
        
        let result = buffer2.collect_fee(100, 0, FeeDomain::Spot);
        assert!(result.is_err());
        assert_eq!(buffer2.tau_spot, initial_tau2); // tau unchanged
        assert_eq!(buffer2.fees_token_0, initial_fees2); // fees unchanged
        
        // Test case 3: successful update - both should be modified
        let result = buffer2.collect_fee(40, 0, FeeDomain::Spot);
        assert!(result.is_ok());
        assert_eq!(buffer2.tau_spot, initial_tau2 + 40); // tau updated
        assert_eq!(buffer2.fees_token_0, initial_fees2 + 40); // fees updated
    }
}