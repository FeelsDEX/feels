use crate::core::{constants::TICK_ARRAY_SIZE, SdkError, SdkResult};

/// Calculate amount with slippage tolerance
pub fn calculate_amount_with_slippage(
    amount: u64,
    slippage_bps: u16,
    is_min: bool,
) -> SdkResult<u64> {
    if slippage_bps > 10000 {
        return Err(SdkError::InvalidParameters(
            "Slippage cannot exceed 100%".to_string(),
        ));
    }

    let factor = if is_min {
        10000u64.saturating_sub(slippage_bps as u64)
    } else {
        10000u64.saturating_add(slippage_bps as u64)
    };

    amount
        .checked_mul(factor)
        .and_then(|v| v.checked_div(10000))
        .ok_or(SdkError::MathOverflow)
}

/// Calculate fee amount from basis points
pub fn calculate_fee_amount(amount: u64, fee_bps: u16) -> SdkResult<u64> {
    amount
        .checked_mul(fee_bps as u64)
        .and_then(|v| v.checked_div(10000))
        .ok_or(SdkError::MathOverflow)
}

/// Calculate price impact in basis points
pub fn calculate_price_impact_bps(start_sqrt_price: u128, end_sqrt_price: u128) -> SdkResult<u16> {
    let (higher, lower) = if start_sqrt_price > end_sqrt_price {
        (start_sqrt_price, end_sqrt_price)
    } else {
        (end_sqrt_price, start_sqrt_price)
    };

    let diff = higher.saturating_sub(lower);
    let impact = diff
        .checked_mul(10000)
        .and_then(|v| v.checked_div(start_sqrt_price))
        .ok_or(SdkError::MathOverflow)?;

    Ok(impact.min(10000) as u16)
}

/// Convert sqrt price to human-readable price
pub fn sqrt_price_to_price(sqrt_price: u128, decimals_0: u8, decimals_1: u8) -> f64 {
    let sqrt_price_float = sqrt_price as f64;
    let q_shift = 2f64.powi(64);
    let decimal_shift = 10f64.powi((decimals_1 - decimals_0) as i32);

    ((sqrt_price_float / q_shift) * (sqrt_price_float / q_shift)) * decimal_shift
}

/// Convert human-readable price to sqrt price
pub fn price_to_sqrt_price(price: f64, decimals_0: u8, decimals_1: u8) -> u128 {
    let decimal_shift = 10f64.powi((decimals_1 - decimals_0) as i32);
    let adjusted_price = price / decimal_shift;
    let sqrt_price_float = adjusted_price.sqrt();
    let q_shift = 2f64.powi(64);

    (sqrt_price_float * q_shift) as u128
}

/// Get tick from sqrt price
pub fn sqrt_price_to_tick(sqrt_price: u128) -> SdkResult<i32> {
    // Use the feels program's implementation
    feels::utils::tick_from_sqrt_price(sqrt_price).map_err(|_| SdkError::MathOverflow)
}

/// Get sqrt price from tick
pub fn tick_to_sqrt_price(tick: i32) -> SdkResult<u128> {
    feels::utils::sqrt_price_from_tick(tick).map_err(|_| SdkError::MathOverflow)
}

/// Align tick to spacing
pub fn align_tick(tick: i32, tick_spacing: u16) -> i32 {
    let spacing = tick_spacing as i32;
    if tick >= 0 {
        (tick / spacing) * spacing
    } else {
        ((tick - spacing + 1) / spacing) * spacing
    }
}

/// Check if tick is initialized
pub fn is_tick_initialized(tick: i32, tick_spacing: u16) -> bool {
    tick % (tick_spacing as i32) == 0
}

/// Calculate the tick array start index for a given tick
pub fn get_tick_array_start_index(tick: i32, tick_spacing: u16) -> i32 {
    let ticks_in_array = TICK_ARRAY_SIZE;
    let tick_array_spacing = (tick_spacing as i32) * ticks_in_array;

    if tick >= 0 {
        (tick / tick_array_spacing) * tick_array_spacing
    } else {
        ((tick - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
    }
}

/// Check if a tick spacing requires full range positions only
pub fn is_full_range_only(tick_spacing: u16) -> bool {
    tick_spacing >= 88
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_calculation() {
        // Min amount with 1% slippage
        assert_eq!(
            calculate_amount_with_slippage(1000, 100, true).unwrap(),
            990
        );

        // Max amount with 1% slippage
        assert_eq!(
            calculate_amount_with_slippage(1000, 100, false).unwrap(),
            1010
        );

        // Invalid slippage
        assert!(calculate_amount_with_slippage(1000, 10001, true).is_err());
    }

    #[test]
    fn test_tick_alignment() {
        assert_eq!(align_tick(5, 10), 0);
        assert_eq!(align_tick(15, 10), 10);
        assert_eq!(align_tick(-5, 10), -10);
        assert_eq!(align_tick(-15, 10), -20);
    }
}
