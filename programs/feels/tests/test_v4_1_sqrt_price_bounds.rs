#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::liquidity::LiquidityMath;
    use crate::state::PoolError;
    use crate::utils::{MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64};

    #[test]
    fn test_v4_1_sqrt_price_bounds_validation() {
        // Test that function rejects sqrt_price below minimum
        let result = LiquidityMath::get_next_sqrt_price_from_input(
            MIN_SQRT_PRICE_X64 - 1, // Below minimum
            1000000,  // Valid liquidity
            100,      // Valid amount_in
            true,     // zero_for_one
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string().contains("PriceOutOfBounds"), true);

        // Test that function rejects sqrt_price above maximum
        let result = LiquidityMath::get_next_sqrt_price_from_input(
            MAX_SQRT_PRICE_X64 + 1, // Above maximum
            1000000,  // Valid liquidity
            100,      // Valid amount_in
            true,     // zero_for_one
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string().contains("PriceOutOfBounds"), true);

        // Test that function accepts valid sqrt_price at minimum bound
        let result = LiquidityMath::get_next_sqrt_price_from_input(
            MIN_SQRT_PRICE_X64, // At minimum
            1000000,  // Valid liquidity
            100,      // Valid amount_in
            true,     // zero_for_one
        );
        assert!(result.is_ok());

        // Test that function accepts valid sqrt_price at maximum bound
        let result = LiquidityMath::get_next_sqrt_price_from_input(
            MAX_SQRT_PRICE_X64, // At maximum
            1000000,  // Valid liquidity
            100,      // Valid amount_in
            true,     // zero_for_one
        );
        assert!(result.is_ok());

        // Test that function accepts valid sqrt_price in normal range
        let mid_price = (MIN_SQRT_PRICE_X64 + MAX_SQRT_PRICE_X64) / 2;
        let result = LiquidityMath::get_next_sqrt_price_from_input(
            mid_price,
            1000000,  // Valid liquidity
            100,      // Valid amount_in
            false,    // zero_for_one = false
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_v4_1_get_next_sqrt_price_from_output_bounds() {
        // Also test the output version has the same validation
        let result = LiquidityMath::get_next_sqrt_price_from_output(
            MAX_SQRT_PRICE_X64 + 1, // Above maximum
            1000000,  // Valid liquidity
            100,      // Valid amount_out
            true,     // zero_for_one
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string().contains("PriceOutOfBounds"), true);
    }
}