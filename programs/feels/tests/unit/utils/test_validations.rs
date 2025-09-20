//! Tests for validation utilities

#[cfg(test)]
mod test_validations {
    use feels::{
        constants::{MIN_LIQUIDITY, MAX_LIQUIDITY},
        error::FeelsError,
        state::Position,
        utils::validations::*,
    };
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_validate_amount() {
        // Valid amounts
        assert!(validate_amount(1).is_ok());
        assert!(validate_amount(1000).is_ok());
        assert!(validate_amount(u64::MAX / 2).is_ok());

        // Invalid amounts
        assert!(validate_amount(0).unwrap_err().to_string().contains("ZeroAmount"));
        assert!(validate_amount(u64::MAX / 2 + 1).unwrap_err().to_string().contains("AmountOverflow"));
    }

    #[test]
    fn test_validate_liquidity_amounts() {
        // Valid combinations
        assert!(validate_liquidity_amounts(100, 200).is_ok());
        assert!(validate_liquidity_amounts(0, 100).is_ok());
        assert!(validate_liquidity_amounts(100, 0).is_ok());

        // Both zero
        assert!(validate_liquidity_amounts(0, 0).unwrap_err().to_string().contains("ZeroAmount"));

        // Overflow
        assert!(validate_liquidity_amounts(u64::MAX / 2 + 1, 100).unwrap_err().to_string().contains("AmountOverflow"));
    }

    #[test]
    fn test_validate_tick_range() {
        let tick_spacing = 10;

        // Valid range
        assert!(validate_tick_range(-100, 100, tick_spacing).is_ok());
        assert!(validate_tick_range(0, 1000, tick_spacing).is_ok());

        // Invalid: upper < lower
        assert_eq!(
            validate_tick_range(100, -100, tick_spacing).unwrap_err(),
            FeelsError::InvalidTickRange.into()
        );

        // Invalid: not aligned to tick spacing
        assert_eq!(
            validate_tick_range(-95, 100, tick_spacing).unwrap_err(),
            FeelsError::TickNotSpaced.into()
        );
        assert_eq!(
            validate_tick_range(-100, 95, tick_spacing).unwrap_err(),
            FeelsError::TickNotSpaced.into()
        );

        // Invalid: out of bounds
        assert_eq!(
            validate_tick_range(-500000, 100, tick_spacing).unwrap_err(),
            FeelsError::InvalidTick.into()
        );
        assert_eq!(
            validate_tick_range(-100, 500000, tick_spacing).unwrap_err(),
            FeelsError::InvalidTick.into()
        );
    }

    #[test]
    fn test_validate_pool_includes_feelssol() {
        let feelssol_mint = Pubkey::new_unique();
        let other_token_1 = Pubkey::new_unique();
        let other_token_2 = Pubkey::new_unique();

        // Valid: FeelsSOL as token_0
        assert!(validate_pool_includes_feelssol(
            &feelssol_mint,
            &other_token_1,
            &feelssol_mint
        ).is_ok());

        // Valid: FeelsSOL as token_1
        assert!(validate_pool_includes_feelssol(
            &other_token_1,
            &feelssol_mint,
            &feelssol_mint
        ).is_ok());

        // Invalid: Neither token is FeelsSOL
        assert_eq!(
            validate_pool_includes_feelssol(
                &other_token_1,
                &other_token_2,
                &feelssol_mint
            ).unwrap_err(),
            FeelsError::InvalidRoute.into()
        );
    }

    #[test]
    fn test_validate_time_constraint() {
        let last_action = 1000;
        let minimum_interval = 60;

        // Valid: enough time passed
        assert!(validate_time_constraint(1100, last_action, minimum_interval).is_ok());
        assert!(validate_time_constraint(1060, last_action, minimum_interval).is_ok());

        // Invalid: too soon
        assert_eq!(
            validate_time_constraint(1059, last_action, minimum_interval).unwrap_err(),
            FeelsError::CooldownActive.into()
        );
        assert_eq!(
            validate_time_constraint(1000, last_action, minimum_interval).unwrap_err(),
            FeelsError::CooldownActive.into()
        );
    }

    #[test]
    fn test_validate_slot_constraint() {
        let last_slot = 1000;
        let minimum_slots = 10;

        // Valid: enough slots passed
        assert!(validate_slot_constraint(1020, last_slot, minimum_slots).is_ok());
        assert!(validate_slot_constraint(1010, last_slot, minimum_slots).is_ok());

        // Invalid: too soon
        assert_eq!(
            validate_slot_constraint(1009, last_slot, minimum_slots).unwrap_err(),
            FeelsError::TooEarly.into()
        );
    }

    #[test]
    fn test_validate_sqrt_price() {
        // Valid prices
        assert!(validate_sqrt_price(MIN_SQRT_PRICE).is_ok());
        assert!(validate_sqrt_price(MAX_SQRT_PRICE).is_ok());
        assert!(validate_sqrt_price(1u128 << 64).is_ok()); // ~1.0

        // Invalid prices
        assert!(validate_sqrt_price(MIN_SQRT_PRICE - 1).unwrap_err().to_string().contains("InvalidPrice"));
        assert!(validate_sqrt_price(MAX_SQRT_PRICE + 1).unwrap_err().to_string().contains("InvalidPrice"));
    }

    #[test]
    fn test_validate_liquidity() {
        // Valid liquidity
        assert!(validate_liquidity(MIN_LIQUIDITY).is_ok());
        assert!(validate_liquidity(1_000_000).is_ok());
        assert!(validate_liquidity(MAX_LIQUIDITY).is_ok());

        // Invalid: too low
        assert!(validate_liquidity(MIN_LIQUIDITY - 1).unwrap_err().to_string().contains("LiquidityBelowMinimum"));
        assert!(validate_liquidity(0).unwrap_err().to_string().contains("LiquidityBelowMinimum"));

        // Invalid: too high
        assert!(validate_liquidity(MAX_LIQUIDITY + 1).unwrap_err().to_string().contains("LiquidityOverflow"));
    }

    #[test]
    fn test_validate_position_ownership() {
        let owner = Pubkey::new_unique();
        let wrong_owner = Pubkey::new_unique();

        let position = Position {
            nft_mint: Pubkey::new_unique(),
            market: Pubkey::new_unique(),
            owner,
            tick_lower: -100,
            tick_upper: 100,
            liquidity: 1000,
            fee_growth_inside_0_last_x64: 0,
            fee_growth_inside_1_last_x64: 0,
            tokens_owed_0: 0,
            tokens_owed_1: 0,
            position_bump: 255,
            is_pomm: false,
            last_updated_slot: 0,
            fee_growth_inside_0_last: 0,
            fee_growth_inside_1_last: 0,
            fees_owed_0: 0,
            fees_owed_1: 0,
        };

        // Valid owner
        assert!(validate_position_ownership(&position, &owner).is_ok());

        // Invalid owner
        assert_eq!(
            validate_position_ownership(&position, &wrong_owner).unwrap_err(),
            FeelsError::InvalidPositionOwner.into()
        );
    }

    #[test]
    fn test_validate_monotonic_increase() {
        // Valid: increasing
        assert!(validate_monotonic_increase(100, 50).is_ok());
        assert!(validate_monotonic_increase(100, 100).is_ok()); // Equal is ok

        // Invalid: decreasing
        assert_eq!(
            validate_monotonic_increase(50, 100).unwrap_err(),
            FeelsError::InvalidUpdate.into()
        );
    }

    #[test]
    fn test_validate_buffer_threshold() {
        // Valid thresholds
        assert!(validate_buffer_threshold(MIN_BUFFER_THRESHOLD).is_ok());
        assert!(validate_buffer_threshold(1_000_000).is_ok());
        assert!(validate_buffer_threshold(MAX_BUFFER_THRESHOLD).is_ok());

        // Invalid: too low
        assert!(validate_buffer_threshold(MIN_BUFFER_THRESHOLD - 1).unwrap_err().to_string().contains("InvalidThreshold"));

        // Invalid: too high
        assert!(validate_buffer_threshold(MAX_BUFFER_THRESHOLD + 1).unwrap_err().to_string().contains("InvalidThreshold"));
    }

    #[test]
    fn test_validate_oracle_freshness() {
        let oracle_timestamp = 1000;
        let max_age = 600; // 10 minutes

        // Valid: fresh oracle
        assert!(validate_oracle_freshness(1500, oracle_timestamp, max_age).is_ok());
        assert!(validate_oracle_freshness(1600, oracle_timestamp, max_age).is_ok());

        // Invalid: stale oracle
        assert_eq!(
            validate_oracle_freshness(1700, oracle_timestamp, max_age).unwrap_err(),
            FeelsError::OracleStale.into()
        );
    }

    #[test]
    fn test_validate_fee_cap() {
        let amount_in = 10000;

        // Valid: under cap
        assert!(validate_fee_cap(100, amount_in, 100).is_ok()); // 1% of 10k = 100
        assert!(validate_fee_cap(50, amount_in, 100).is_ok());

        // Invalid: over cap
        assert_eq!(
            validate_fee_cap(101, amount_in, 100).unwrap_err(), // Over 1%
            FeelsError::FeeCapExceeded.into()
        );
    }

    #[test]
    fn test_validate_rate_limit() {
        let current = 1000;
        let cap = 5000;

        // Valid: under limit
        assert!(validate_rate_limit(current, 3000, cap).is_ok()); // 1000 + 3000 = 4000 < 5000
        assert!(validate_rate_limit(current, 4000, cap).is_ok()); // 1000 + 4000 = 5000

        // Invalid: over limit
        assert_eq!(
            validate_rate_limit(current, 4001, cap).unwrap_err(),
            FeelsError::RateLimitExceeded.into()
        );

        // Invalid: overflow
        assert_eq!(
            validate_rate_limit(u64::MAX - 1, 2, cap).unwrap_err(),
            FeelsError::MathOverflow.into()
        );
    }

    #[test]
    fn test_validate_sqrt_price_movement() {
        let old_price = 1u128 << 64; // ~1.0
        
        // Valid: 1% movement
        let new_price_up = old_price + (old_price / 100); // +1%
        assert!(validate_sqrt_price_movement(old_price, new_price_up, 200).is_ok()); // 2% limit
        
        let new_price_down = old_price - (old_price / 100); // -1%
        assert!(validate_sqrt_price_movement(old_price, new_price_down, 200).is_ok());

        // Invalid: 3% movement with 2% limit
        let new_price_large = old_price + (old_price * 3 / 100);
        assert_eq!(
            validate_sqrt_price_movement(old_price, new_price_large, 200).unwrap_err(),
            FeelsError::PriceMovementTooLarge.into()
        );
    }

    #[test]
    fn test_get_tick_array_start_index() {
        let tick_spacing = 10;
        
        // Test various tick indices
        assert_eq!(get_tick_array_start_index(0, tick_spacing), 0);
        assert_eq!(get_tick_array_start_index(50, tick_spacing), 0);
        assert_eq!(get_tick_array_start_index(639, tick_spacing), 0); // 64 * 10 - 1
        assert_eq!(get_tick_array_start_index(640, tick_spacing), 640);
        assert_eq!(get_tick_array_start_index(700, tick_spacing), 640);
        
        // Negative ticks
        assert_eq!(get_tick_array_start_index(-50, tick_spacing), -640);
        assert_eq!(get_tick_array_start_index(-640, tick_spacing), -640);
        assert_eq!(get_tick_array_start_index(-641, tick_spacing), -1280);
    }
}