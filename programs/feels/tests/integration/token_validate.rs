use feels::state::TokenMetadata;
use feels::utils::token_validate::*;

#[cfg(test)]
mod token_validate_integration_tests {
    use super::*;

    // ============================================================================
    // Token Create Validation Tests
    // ============================================================================

    #[test]
    fn test_create_token_with_restricted_ticker() {
        // Test that creating a token with a restricted ticker fails
        let restricted_tickers = vec![
            "SOL", "FEELSSOL", "USDC", "USDT", "BTC", "ETH"
        ];
        
        for ticker in restricted_tickers {
            println!("Testing restricted ticker: {}", ticker);
            
            // Token create should fail with restricted ticker
            let result = validate_ticker_format(ticker);
            assert!(result.is_err(), "Should reject restricted ticker: {}", ticker);
            
            // Verify it's specifically a restrict error
            let error_msg = format!("{:?}", result.err().unwrap());
            assert!(error_msg.contains("RestrictedTicker"), "Should be RestrictedTicker error");
            
            // Test alternatives are provided
            let alternatives = get_ticker_alternatives(ticker);
            assert!(!alternatives.is_empty(), "Should provide alternatives for: {}", ticker);
            assert!(alternatives.contains(&format!("{}2", ticker.to_uppercase())));
            assert!(alternatives.contains(&format!("{}V2", ticker.to_uppercase())));
        }
    }
    
    #[test]
    fn test_create_token_with_valid_ticker() {
        // Test that creating a token with valid ticker succeeds
        let valid_tickers = vec![
            "MYTOKEN", "CUSTOMCOIN", "DEFI123", "MOON", "LAMBO", "TEST1"
        ];
        
        for ticker in valid_tickers {
            println!("Testing valid ticker: {}", ticker);
            
            // Ticker validation should succeed
            let result = validate_ticker_format(ticker);
            assert!(result.is_ok(), "Should accept valid ticker: {}", ticker);
            
            // Should not be restricted
            assert!(!is_ticker_restricted(ticker), "Valid ticker should not be restricted: {}", ticker);
        }
    }
    
    #[test]
    fn test_create_token_ticker_format_validation() {
        // Test ticker format validation rules
        
        // Test valid formats
        let valid_formats = vec![
            ("A", "Single character"),
            ("ABC", "Three characters"),
            ("TOKEN123", "Mixed alphanumeric"),
            ("ABCDEFGHIJKL", "Maximum 12 characters"),
        ];
        
        for (ticker, description) in valid_formats {
            let result = validate_ticker_format(ticker);
            assert!(result.is_ok(), "{} should be valid: {}", description, ticker);
        }
        
        // Test invalid formats
        let invalid_formats = vec![
            ("", "Empty ticker", "InvalidTickerLength"),
            ("ABCDEFGHIJKLM", "13 characters", "InvalidTickerLength"),
            ("MY-TOKEN", "Hyphen", "InvalidTickerFormat"),
            ("MY_TOKEN", "Underscore", "InvalidTickerFormat"),
            ("MY TOKEN", "Space", "InvalidTickerFormat"),
            ("MY@TOKEN", "Special character", "InvalidTickerFormat"),
            ("MY.TOKEN", "Period", "InvalidTickerFormat"),
        ];
        
        for (ticker, description, expected_error) in invalid_formats {
            let result = validate_ticker_format(ticker);
            assert!(result.is_err(), "{} should be invalid: {}", description, ticker);
            
            let error_msg = format!("{:?}", result.err().unwrap());
            assert!(error_msg.contains(expected_error), 
                "Should be {} error for {}: {}", expected_error, description, ticker);
        }
    }
    
    // ============================================================================
    // Case Sensitivity Tests
    // ============================================================================
    
    #[test]
    fn test_ticker_case_insensitive_restriction() {
        // Test that restriction checking is case-insensitive
        let test_cases = vec![
            ("sol", true),
            ("SOL", true),
            ("Sol", true),
            ("sOL", true),
            ("feelssol", true),
            ("FEELSSOL", true),
            ("FeelsSOL", true),
            ("FEELSsol", true),
            ("usdc", true),
            ("USDC", true),
            ("Usdc", true),
            // Valid tickers
            ("MYTOKEN", false),
            ("mytoken", false),
            ("MyToken", false),
        ];
        
        for (ticker, should_be_restricted) in test_cases {
            let is_restricted = is_ticker_restricted(ticker);
            assert_eq!(is_restricted, should_be_restricted, 
                "Ticker '{}' restriction status should be: {}", ticker, should_be_restricted);
        }
    }
    
    // ============================================================================
    // Token Metadata Tests
    // ============================================================================
    
    #[test]
    fn test_token_metadata_structure() {
        // Test that TokenMetadata has the expected structure and size
        println!("TokenMetadata SIZE: {}", TokenMetadata::SIZE);
        
        // Verify size calculation is reasonable
        // These are compile-time checks that will fail if the conditions are false
        const _: () = assert!(TokenMetadata::SIZE > 100);
        const _: () = assert!(TokenMetadata::SIZE < 1000);
        
        // Test individual field sizes
        let expected_min_size = 8 + // discriminator
            4 + 1 + // ticker (min 1 char + length)
            4 + 1 + // name (min 1 char + length)
            4 + 1 + // symbol (min 1 char + length)
            32 + // mint
            32 + // authority  
            8 + // created_at
            64; // reserved
        
        assert!(TokenMetadata::SIZE >= expected_min_size, 
            "TokenMetadata SIZE should be at least {}, got {}", 
            expected_min_size, TokenMetadata::SIZE);
    }
    
    // ============================================================================
    // Integration Workflow Tests
    // ============================================================================
    
    #[test]
    fn test_complete_token_create_validation_workflow() {
        println!("Testing complete token create validation workflow");
        
        // Step 1: Test restricted ticker rejection
        let restricted_ticker = "SOL";
        let validation_result = validate_ticker_format(restricted_ticker);
        assert!(validation_result.is_err());
        
        // Step 2: Get alternatives
        let alternatives = get_ticker_alternatives(restricted_ticker);
        assert!(!alternatives.is_empty());
        println!("Alternatives for '{}': {:?}", restricted_ticker, alternatives);
        
        // Step 3: Test that alternative is valid (if it's not also restricted)
        let alternative = "SOL2"; // One of the suggested alternatives
        if !is_ticker_restricted(alternative) {
            let alt_validation = validate_ticker_format(alternative);
            assert!(alt_validation.is_ok(), "Alternative '{}' should be valid", alternative);
        }
        
        // Step 4: Test valid custom ticker
        let valid_ticker = "MYTOKEN";
        let valid_result = validate_ticker_format(valid_ticker);
        assert!(valid_result.is_ok(), "Valid ticker '{}' should pass validation", valid_ticker);
        
        println!("✓ Complete token create validation workflow working");
    }
    
    // ============================================================================
    // Error Message Tests
    // ============================================================================
    
    #[test]
    fn test_error_message_clarity() {
        // Test that error messages are clear and actionable
        
        // Restricted ticker error
        let restrict_error = validate_ticker_format("SOL").err().unwrap();
        let error_string = format!("{}", restrict_error);
        assert!(error_string.contains("restricted"), 
            "Error message should mention restriction: {}", error_string);
        
        // Invalid length error
        let length_error = validate_ticker_format("").err().unwrap();
        let length_string = format!("{}", length_error);
        assert!(length_string.contains("length"), 
            "Error message should mention length: {}", length_string);
        
        // Invalid format error
        let format_error = validate_ticker_format("MY-TOKEN").err().unwrap();
        let format_string = format!("{}", format_error);
        assert!(format_string.contains("characters") || format_string.contains("format"), 
            "Error message should mention format: {}", format_string);
        
        println!("✓ Error messages are clear and actionable");
    }
    
    // ============================================================================
    // Comprehensive Restriction Coverage Tests
    // ============================================================================
    
    #[test]
    fn test_comprehensive_restriction_coverage() {
        // Verify all required categories are covered
        let required_tickers = vec!["FEELSSOL", "SOL", "USDC", "USDT"]; // User's requirements
        
        for ticker in required_tickers {
            assert!(is_ticker_restricted(ticker), 
                "Required ticker '{}' should be restricted", ticker);
        }
        
        // Test major categories
        let protocol_tokens = ["FEELSSOL", "FEELS"];
        let blockchain_tokens = vec!["SOL", "ETH", "BTC"];
        let stablecoins = vec!["USDC", "USDT", "DAI"];
        let reserved_words = vec!["TOKEN", "COIN", "MONEY"];
        
        for ticker in protocol_tokens.iter().chain(&blockchain_tokens).chain(&stablecoins).chain(&reserved_words) {
            assert!(is_ticker_restricted(ticker), 
                "Category ticker '{}' should be restricted", ticker);
        }
        
        println!("✓ Comprehensive restriction coverage validated");
    }
    
    // ============================================================================
    // Performance Tests
    // ============================================================================
    
    #[test]
    fn test_restrict_lookup_performance() {
        // Test that restriction lookups are reasonably fast
        use std::time::Instant;
        
        let test_tickers = vec![
            "SOL", "USDC", "BTC", "ETH", "MYTOKEN", "CUSTOM", "TEST123"
        ];
        
        let start = Instant::now();
        for _ in 0..1000 {
            for ticker in &test_tickers {
                let _ = is_ticker_restricted(ticker);
            }
        }
        let duration = start.elapsed();
        
        println!("1000 iterations x {} tickers took: {:?}", test_tickers.len(), duration);
        assert!(duration.as_millis() < 100, "Restriction lookups should be fast");
    }
}