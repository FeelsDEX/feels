//! Tests for fee rounding vulnerability fix
//! 
//! Ensures that small swaps cannot bypass fees through rounding

use feels::utils::{calculate_fee_ceil, mul_div_ceil_u64};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceiling_division() {
        // Test exact division
        assert_eq!(mul_div_ceil_u64(100, 30, 10000).unwrap(), 1); // 0.3 rounds up to 1
        
        // Test that would round to 0 with floor division
        assert_eq!(mul_div_ceil_u64(10, 30, 10000).unwrap(), 1); // 0.03 rounds up to 1
        assert_eq!(mul_div_ceil_u64(1, 30, 10000).unwrap(), 1);  // 0.003 rounds up to 1
        
        // Test zero cases
        assert_eq!(mul_div_ceil_u64(0, 30, 10000).unwrap(), 0);
        assert_eq!(mul_div_ceil_u64(100, 0, 10000).unwrap(), 0);
        
        // Test larger amounts
        assert_eq!(mul_div_ceil_u64(10000, 30, 10000).unwrap(), 30); // Exact
        assert_eq!(mul_div_ceil_u64(10001, 30, 10000).unwrap(), 31); // Rounds up
    }
    
    #[test]
    fn test_calculate_fee_ceil() {
        // Test with 30 bps (0.3%) fee
        let fee_bps = 30u16;
        
        // Test amounts that would round to 0 with floor division
        assert_eq!(calculate_fee_ceil(1, fee_bps).unwrap(), 1);     // Min fee = 1
        assert_eq!(calculate_fee_ceil(10, fee_bps).unwrap(), 1);    // 0.03 → 1
        assert_eq!(calculate_fee_ceil(100, fee_bps).unwrap(), 1);   // 0.3 → 1
        assert_eq!(calculate_fee_ceil(333, fee_bps).unwrap(), 1);   // 0.999 → 1
        assert_eq!(calculate_fee_ceil(334, fee_bps).unwrap(), 2);   // 1.002 → 2
        
        // Test zero cases
        assert_eq!(calculate_fee_ceil(0, fee_bps).unwrap(), 0);
        assert_eq!(calculate_fee_ceil(1000, 0).unwrap(), 0);
        
        // Test normal amounts
        assert_eq!(calculate_fee_ceil(1000, fee_bps).unwrap(), 3);    // 3.0
        assert_eq!(calculate_fee_ceil(10000, fee_bps).unwrap(), 30);  // 30.0
        assert_eq!(calculate_fee_ceil(10001, fee_bps).unwrap(), 31);  // 30.003 → 31
    }
    
    #[test]
    fn test_attack_prevention() {
        // Simulate attacker trying to drain fees with small swaps
        let fee_bps = 30u16; // 0.3% fee
        
        // Amounts that attacker might try to use to avoid fees
        let attack_amounts = vec![1, 10, 100, 200, 300, 333];
        
        for amount in attack_amounts {
            let fee = calculate_fee_ceil(amount, fee_bps).unwrap();
            assert!(fee > 0, "Amount {} should have non-zero fee", amount);
            
            // Verify that fee is at least 1 basis point of the amount
            // or 1 token (whichever is larger)
            let min_acceptable_fee = std::cmp::max(1, amount / 10000);
            assert!(fee >= min_acceptable_fee, 
                "Fee {} for amount {} is too low", fee, amount);
        }
    }
    
    #[test]
    fn test_fee_percentage_accuracy() {
        // Test that fees are accurate for larger amounts
        let fee_bps = 30u16;
        
        // Test exact multiples
        assert_eq!(calculate_fee_ceil(10000, fee_bps).unwrap(), 30);
        assert_eq!(calculate_fee_ceil(100000, fee_bps).unwrap(), 300);
        assert_eq!(calculate_fee_ceil(1000000, fee_bps).unwrap(), 3000);
        
        // Test that ceiling works correctly
        assert_eq!(calculate_fee_ceil(10001, fee_bps).unwrap(), 31);  // Not 30
        assert_eq!(calculate_fee_ceil(100001, fee_bps).unwrap(), 301); // Not 300
    }
}