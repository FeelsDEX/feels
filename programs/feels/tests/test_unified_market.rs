//! Basic tests for unified Market account

use anchor_lang::prelude::*;
use feels_protocol::state::*;
use feels_protocol::error::FeelsError;
use feels_core::constants::Q64;

#[cfg(test)]
mod unified_market_tests {
    use super::*;

    #[test]
    fn test_market_initialization() {
        // Test creating a unified market account
        let mut market = Market::default();
        
        // Initialize with test values
        let result = market.initialize(
            Pubkey::new_unique(), // pool
            Pubkey::new_unique(), // token_0
            Pubkey::new_unique(), // token_1
            Pubkey::new_unique(), // vault_0
            Pubkey::new_unique(), // vault_1
            Q64,                  // sqrt_price = 1.0
            DomainWeights {
                w_s: 4000,
                w_t: 3000,
                w_l: 2000,
                w_tau: 1000,
            },
            Pubkey::new_unique(), // authority
        );
        
        assert!(result.is_ok());
        assert!(market.is_initialized);
        assert!(!market.is_paused);
        
        // Check initial state
        assert_eq!(market.S, Q64);
        assert_eq!(market.T, Q64);
        assert_eq!(market.L, Q64);
        assert_eq!(market.sqrt_price, Q64);
        assert_eq!(market.liquidity, 0);
        
        // Check weights
        assert_eq!(market.w_s, 4000);
        assert_eq!(market.w_t, 3000);
        assert_eq!(market.w_l, 2000);
        assert_eq!(market.w_tau, 1000);
    }

    #[test]
    fn test_domain_weights_validation() {
        let valid_weights = DomainWeights {
            w_s: 4000,
            w_t: 3000,
            w_l: 3000,
            w_tau: 1000,
        };
        
        assert!(valid_weights.validate().is_ok());
        
        // Test invalid weights (don't sum to 10000)
        let invalid_weights = DomainWeights {
            w_s: 3000,
            w_t: 3000,
            w_l: 3000,
            w_tau: 1000,
        };
        
        assert!(invalid_weights.validate().is_err());
        
        // Test invalid tau weight
        let invalid_tau = DomainWeights {
            w_s: 4000,
            w_t: 3000,
            w_l: 3000,
            w_tau: 6000, // Too high
        };
        
        assert!(invalid_tau.validate().is_err());
    }

    #[test]
    fn test_market_price_updates() {
        let mut market = create_test_market();
        
        // Update price and tick
        let new_sqrt_price = Q64 * 2; // sqrt(4) = 2
        let new_tick = 6932; // approximate tick for price 4
        
        market.update_price(new_sqrt_price, new_tick);
        
        assert_eq!(market.sqrt_price, new_sqrt_price);
        assert_eq!(market.current_tick, new_tick);
    }

    #[test]
    fn test_market_liquidity_operations() {
        let mut market = create_test_market();
        
        // Add liquidity
        let liquidity_delta = 1000000u128;
        let result = market.add_liquidity(liquidity_delta);
        assert!(result.is_ok());
        assert_eq!(market.liquidity, liquidity_delta);
        
        // Add more liquidity
        let result = market.add_liquidity(500000);
        assert!(result.is_ok());
        assert_eq!(market.liquidity, 1500000);
        
        // Remove liquidity
        let result = market.remove_liquidity(300000);
        assert!(result.is_ok());
        assert_eq!(market.liquidity, 1200000);
        
        // Try to remove too much liquidity
        let result = market.remove_liquidity(2000000);
        assert!(result.is_err());
    }

    #[test]
    fn test_market_volume_tracking() {
        let mut market = create_test_market();
        
        // Record some volume
        let result = market.record_volume(1000, 2000);
        assert!(result.is_ok());
        assert_eq!(market.total_volume_0, 1000);
        assert_eq!(market.total_volume_1, 2000);
        
        // Record more volume
        let result = market.record_volume(500, 750);
        assert!(result.is_ok());
        assert_eq!(market.total_volume_0, 1500);
        assert_eq!(market.total_volume_1, 2750);
    }

    #[test]
    fn test_market_scalar_updates() {
        let mut market = create_test_market();
        
        // Update thermodynamic scalars
        let new_s = Q64 * 2;
        let new_t = Q64 * 3;
        let new_l = Q64 / 2;
        
        market.update_scalars(new_s, new_t, new_l);
        
        assert_eq!(market.S, new_s);
        assert_eq!(market.T, new_t);
        assert_eq!(market.L, new_l);
    }

    #[test]
    fn test_market_pause_functionality() {
        let mut market = create_test_market();
        
        assert!(!market.is_paused);
        
        // Pause market
        market.is_paused = true;
        assert!(market.is_paused);
        
        // Operations should check is_paused before proceeding
        // This would be enforced in the instruction handlers
    }

    // Helper function to create a test market
    fn create_test_market() -> Market {
        let mut market = Market::default();
        
        market.initialize(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Q64,
            DomainWeights {
                w_s: 3333,
                w_t: 3333,
                w_l: 3334,
                w_tau: 0,
            },
            Pubkey::new_unique(),
        ).unwrap();
        
        market
    }
}

#[cfg(test)]
mod unified_state_access_tests {
    use super::*;
    use feels_protocol::logic::unified_state_access::*;

    #[test]
    fn test_market_access_getters() {
        let market = create_test_market_account();
        let market_access = MarketAccess::new(&market).unwrap();
        
        // Test thermodynamic getters
        assert_eq!(market_access.s(), Q64);
        assert_eq!(market_access.t(), Q64);
        assert_eq!(market_access.l(), Q64);
        
        // Test AMM getters
        assert_eq!(market_access.sqrt_price(), Q64);
        assert_eq!(market_access.current_tick(), 0);
        assert_eq!(market_access.liquidity(), 0);
        
        // Test fee parameters
        assert_eq!(market_access.base_fee_bps(), 30);
        assert_eq!(market_access.max_fee_bps(), 300);
        
        // Test volatility parameters
        assert_eq!(market_access.sigma_price(), 100);
        assert_eq!(market_access.sigma_rate(), 50);
        assert_eq!(market_access.sigma_leverage(), 200);
    }

    #[test]
    fn test_buffer_state_access() {
        let buffer = create_test_buffer_account();
        let mut buffer_access = BufferStateAccess::new(&buffer);
        
        // Test fee collection
        let result = buffer_access.collect_fees(true, 1000);
        assert!(result.is_ok());
        
        let result = buffer_access.collect_fees(false, 2000);
        assert!(result.is_ok());
        
        // Test rebate payment
        let result = buffer_access.pay_rebate(true, 100);
        assert!(result.is_ok());
        
        let result = buffer_access.pay_rebate(false, 200);
        assert!(result.is_ok());
    }

    // Helper functions
    fn create_test_market_account() -> Account<'static, Market> {
        // This would need proper setup in a real test environment
        // For now, using a placeholder
        panic!("Test helper not implemented - would need test framework setup");
    }
    
    fn create_test_buffer_account() -> Account<'static, BufferAccount> {
        // This would need proper setup in a real test environment
        panic!("Test helper not implemented - would need test framework setup");
    }
}