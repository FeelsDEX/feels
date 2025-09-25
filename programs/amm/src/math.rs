use anchor_lang::prelude::*;

// Constants for tick math (adapted for u128 limits)
pub const MIN_TICK: i32 = -443636; // Minimum tick
pub const MAX_TICK: i32 = 443636; // Maximum tick
pub const MIN_SQRT_RATIO: u128 = 4295128740; // sqrt(1.0001^MIN_TICK) approximately
pub const MAX_SQRT_RATIO: u128 = 79226673515401279992447579055; // Fits in u128

#[error_code]
pub enum MathError {
    #[msg("Price out of bounds")]
    PriceOutOfBounds,
}

/// Convert sqrt price to tick
///
/// Takes a sqrt price in Q64.64 fixed-point format and converts it to the corresponding tick.
/// Uses the formula: tick = floor(log(sqrt_price) / log(sqrt(1.0001)))
pub fn price_to_tick(sqrt_price: u128) -> Result<i32> {
    // Validate that the sqrt_price is within the supported range for our u128 implementation
    // This prevents overflow and ensures we stay within mathematically valid tick bounds
    require!(
        (MIN_SQRT_RATIO..=MAX_SQRT_RATIO).contains(&sqrt_price),
        MathError::PriceOutOfBounds
    );

    // Convert u128 to f64 for logarithmic calculations
    let sqrt_price_f64 = sqrt_price as f64;

    // Take natural logarithm of the sqrt_price
    let log_sqrt_price = sqrt_price_f64.ln();

    // Calculate log(sqrt(1.0001)) as the base for our logarithm
    // This is the denominator in our tick formula: tick = log(sqrt_price) / log(sqrt(1.0001))
    // We use sqrt(1.0001) because each tick represents a 0.01% price change,
    // and sqrt_price = sqrt(1.0001^tick), so tick = log(sqrt_price) / log(sqrt(1.0001))
    let log_sqrt_1_0001 = (1.0001_f64).sqrt().ln();

    // Calculate the exact tick value using change of base formula
    // tick = log(sqrt_price) / log(sqrt(1.0001))
    // This gives us the precise tick corresponding to the given sqrt_price
    let tick_f64 = log_sqrt_price / log_sqrt_1_0001;

    // Floor the result to get the integer tick
    // We use floor() because ticks are discrete integer values, and we want the tick
    // that represents the highest price that is still <= the given sqrt_price
    let tick = tick_f64.floor() as i32;

    // Clamp the result to our valid tick range to handle any edge cases
    // This ensures we never return a tick outside our supported bounds,
    // even if floating-point precision issues occur
    Ok(tick.clamp(MIN_TICK, MAX_TICK))
}
