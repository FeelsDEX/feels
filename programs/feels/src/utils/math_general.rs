/// General mathematical utilities used across the protocol for common operations.
/// Includes integer square root for price calculations and other helper functions
/// that don't fit into specialized math modules. Optimized implementations that
/// balance precision with gas efficiency for on-chain execution.

// ============================================================================
// Core Implementation
// ============================================================================

/// Integer square root calculation using Newton's method
/// This is a general-purpose utility that can be used across the codebase
pub fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    
    let mut x = value;
    let mut y = (value + 1) / 2;
    
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    
    x
}

// ------------------------------------------------------------------------
// Helper Functions
// ------------------------------------------------------------------------

/// Safe percentage calculation with basis points precision and rounding options
/// Returns (value * percentage_bps) / 10000 with overflow protection
pub fn calculate_percentage_bp(value: u128, percentage_bps: u16, round_up: bool) -> Result<u128, Box<dyn std::error::Error>> {
    if percentage_bps > 10000 {
        return Err("Invalid percentage: must be <= 10000 basis points".into());
    }
    
    let numerator = value
        .checked_mul(percentage_bps as u128)
        .ok_or("Multiplication overflow")?;
    
    let result = if round_up {
        numerator
            .checked_add(9999)
            .ok_or("Addition overflow")?
            .checked_div(10000)
            .ok_or("Division by zero")?
    } else {
        numerator
            .checked_div(10000)
            .ok_or("Division by zero")?
    };
    
    Ok(result)
}

/// Calculate percentage with standard precision (for backwards compatibility)
/// Returns (value * percentage) / 100
pub fn calculate_percentage(value: u64, percentage: u8) -> u64 {
    ((value as u128 * percentage as u128) / 100) as u64
}

/// Clamp a value between min and max bounds
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_sqrt() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(16), 4);
        assert_eq!(integer_sqrt(25), 5);
        assert_eq!(integer_sqrt(100), 10);
        assert_eq!(integer_sqrt(10000), 100);
    }

    #[test]
    fn test_calculate_percentage_bp() {
        assert_eq!(calculate_percentage_bp(10000, 100, false).unwrap(), 100);     // 1% of 10000
        assert_eq!(calculate_percentage_bp(10000, 30, false).unwrap(), 30);       // 0.3% of 10000
        assert_eq!(calculate_percentage_bp(10000, 10000, false).unwrap(), 10000); // 100% of 10000
        
        // Test rounding up
        assert_eq!(calculate_percentage_bp(1001, 2000, true).unwrap(), 201);  // Rounds up
        assert_eq!(calculate_percentage_bp(1001, 2000, false).unwrap(), 200); // Rounds down
        
        // Test invalid percentage
        assert!(calculate_percentage_bp(1000, 10001, false).is_err());
    }

    #[test]
    fn test_calculate_percentage() {
        assert_eq!(calculate_percentage(1000, 10), 100);   // 10% of 1000
        assert_eq!(calculate_percentage(1000, 50), 500);   // 50% of 1000
        assert_eq!(calculate_percentage(1000, 100), 1000); // 100% of 1000
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 1, 10), 5);   // within range
        assert_eq!(clamp(0, 1, 10), 1);   // below min
        assert_eq!(clamp(15, 1, 10), 10); // above max
    }
}