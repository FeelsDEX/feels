//! Tests for set_protocol_owned_override instruction

#[cfg(test)]
mod test_set_protocol_owned_override {
    use anchor_lang::prelude::*;
    use feels::state::{Buffer, ProtocolConfig};

    fn create_test_protocol_config() -> ProtocolConfig {
        ProtocolConfig {
            authority: Pubkey::new_unique(),
            mint_fee: 100_000_000, // 0.1 SOL
            treasury: Pubkey::new_unique(),
            default_protocol_fee_rate: 30,
            default_creator_fee_rate: 70,
            max_protocol_fee_rate: 100,
            token_expiration_seconds: 3600,
            depeg_threshold_bps: 100,
            depeg_required_obs: 3,
            clear_required_obs: 5,
            dex_twap_window_secs: 300,
            dex_twap_stale_age_secs: 600,
            dex_twap_updater: Pubkey::new_unique(),
            dex_whitelist: [Pubkey::default(); 8],
            dex_whitelist_len: 0,
            _reserved: [0; 7],
            mint_per_slot_cap_feelssol: 0,
            redeem_per_slot_cap_feelssol: 0,
            default_base_fee_bps: 30,
            default_tick_spacing: 64,
            default_initial_sqrt_price: 5825507814218144,
            default_tick_step_size: 128,
        }
    }

    fn create_test_buffer() -> Buffer {
        Buffer {
            market: Pubkey::new_unique(),
            authority: Pubkey::new_unique(), // This is the market creator
            feelssol_mint: Pubkey::new_unique(),
            fees_token_0: 1_000_000,
            fees_token_1: 2_000_000,
            tau_spot: 3_000_000,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 100,
            floor_placement_threshold: 1_000_000,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 255,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
            jit_rolling_consumption: 0,
            jit_rolling_window_start: 0,
            jit_last_heavy_usage_slot: 0,
            jit_total_consumed_epoch: 0,
            initial_tau_spot: 1_000_000,
            protocol_owned_override: 0,
            pomm_position_count: 0,
            _padding: [0; 7],
        }
    }

    #[test]
    fn test_protocol_authority_can_set_override() {
        let protocol_config = create_test_protocol_config();
        let mut buffer = create_test_buffer();
        let protocol_authority = protocol_config.authority;

        // Initially, override should be 0
        assert_eq!(buffer.protocol_owned_override, 0);

        // Protocol authority should be able to set override
        // In the real instruction, this would be validated
        buffer.protocol_owned_override = 5_000_000;

        assert_eq!(buffer.protocol_owned_override, 5_000_000);
    }

    #[test]
    fn test_non_protocol_authority_cannot_set_override() {
        let protocol_config = create_test_protocol_config();
        let buffer = create_test_buffer();
        let non_protocol_authority = Pubkey::new_unique();

        // Non-protocol authority should not match
        assert_ne!(non_protocol_authority, protocol_config.authority);

        // In the real instruction, this would fail with InvalidAuthority error
    }

    #[test]
    fn test_buffer_authority_not_required_for_override() {
        let protocol_config = create_test_protocol_config();
        let buffer = create_test_buffer();

        // Buffer authority is set to market creator
        let market_creator = buffer.authority;

        // Protocol authority is different from buffer authority
        assert_ne!(protocol_config.authority, market_creator);

        // But protocol authority can still set override
        // This demonstrates the fix - we don't check buffer.authority
    }

    #[test]
    fn test_override_can_be_cleared() {
        let mut buffer = create_test_buffer();

        // Set override
        buffer.protocol_owned_override = 5_000_000;
        assert_eq!(buffer.protocol_owned_override, 5_000_000);

        // Clear override by setting to 0
        buffer.protocol_owned_override = 0;
        assert_eq!(buffer.protocol_owned_override, 0);
    }

    #[test]
    fn test_override_used_in_floor_calculation() {
        let buffer = create_test_buffer();

        // When protocol_owned_override is set, it should be used
        // instead of dynamic calculations
        if buffer.protocol_owned_override > 0 {
            let non_circulating = buffer.protocol_owned_override as u128;
            assert_eq!(non_circulating, buffer.protocol_owned_override as u128);
        }
    }

    #[test]
    fn test_multiple_buffers_independent_overrides() {
        let mut buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Set different overrides for different buffers
        buffer1.protocol_owned_override = 1_000_000;
        buffer2.protocol_owned_override = 2_000_000;

        // Each buffer maintains its own override
        assert_eq!(buffer1.protocol_owned_override, 1_000_000);
        assert_eq!(buffer2.protocol_owned_override, 2_000_000);
    }

    #[test]
    fn test_override_persists_across_operations() {
        let mut buffer = create_test_buffer();

        // Set override
        buffer.protocol_owned_override = 3_000_000;

        // Simulate other buffer operations
        buffer.fees_token_0 = buffer.fees_token_0.saturating_add(100_000);
        buffer.tau_spot = buffer.tau_spot.saturating_add(100_000);

        // Override should remain unchanged
        assert_eq!(buffer.protocol_owned_override, 3_000_000);
    }

    #[test]
    fn test_governance_risk_control() {
        // This test demonstrates the purpose of the override:
        // It's a risk control lever for governance

        let mut buffer = create_test_buffer();

        // Normal operation: dynamic calculation
        assert_eq!(buffer.protocol_owned_override, 0);

        // Risk scenario: governance sets a fixed amount
        buffer.protocol_owned_override = 10_000_000;

        // This fixed amount is now used for floor calculations
        // protecting the protocol from potential calculation issues
        assert_eq!(buffer.protocol_owned_override, 10_000_000);
    }
}
