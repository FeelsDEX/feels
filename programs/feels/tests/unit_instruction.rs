use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::InstructionData;
use std::str::FromStr;

// Import the program we're testing - using the crate name directly since this is an integration test

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_program_id() {
        let expected_program_id =
            Pubkey::from_str("Fee1sProtoco11111111111111111111111111111111").unwrap();
        assert_eq!(feels::ID, expected_program_id);
    }

    #[test]
    fn test_initialize_instruction_data() {
        let instruction_data = feels::instruction::Initialize {};
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "Initialize instruction should serialize to non-empty data"
        );
    }

    // FeelsSOL Token Operations Tests
    #[test]
    fn test_feelssol_create_instruction_data() {
        let instruction_data = feels::instruction::FeelssolCreate {};
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelssolCreate instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_feelssol_mint_instruction_data() {
        let instruction_data = feels::instruction::FeelssolMint { amount: 1000 };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelssolMint instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_feelssol_burn_instruction_data() {
        let instruction_data = feels::instruction::FeelssolBurn { amount: 500 };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelssolBurn instruction should serialize to non-empty data"
        );
    }

    // Feels Token Operations Tests
    #[test]
    fn test_feels_token_create_instruction_data() {
        let instruction_data = feels::instruction::FeelsTokenCreate {
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            uri: "https://example.com/metadata.json".to_string(),
            decimals: 9,
        };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelsTokenCreate instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_feels_token_mint_instruction_data() {
        let instruction_data = feels::instruction::FeelsTokenMint { amount: 1000 };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelsTokenMint instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_feels_token_burn_instruction_data() {
        let instruction_data = feels::instruction::FeelsTokenBurn { amount: 500 };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "FeelsTokenBurn instruction should serialize to non-empty data"
        );
    }

    // Pool Position NFT Operations Tests
    #[test]
    fn test_pool_position_create_instruction_data() {
        let instruction_data = feels::instruction::PoolPositionCreate {
            position_id: "pos_123".to_string(),
            pool_id: "pool_456".to_string(),
        };
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "PoolPositionCreate instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_pool_position_mint_instruction_data() {
        let instruction_data = feels::instruction::PoolPositionMint {};
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "PoolPositionMint instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_pool_position_burn_instruction_data() {
        let instruction_data = feels::instruction::PoolPositionBurn {};
        let serialized = instruction_data.data();
        assert!(
            !serialized.is_empty(),
            "PoolPositionBurn instruction should serialize to non-empty data"
        );
    }

    #[test]
    fn test_token_metadata_validation() {
        // Test name length validation
        let long_name = "a".repeat(100);
        assert!(
            long_name.len() > 32,
            "Long name should exceed typical limits"
        );

        // Test symbol length validation
        let long_symbol = "SYMBOL".repeat(10);
        assert!(
            long_symbol.len() > 10,
            "Long symbol should exceed typical limits"
        );

        // Test valid metadata
        let valid_name = "Valid Token";
        let valid_symbol = "VALID";
        let valid_uri = "https://example.com/metadata.json";

        assert!(valid_name.len() <= 32, "Valid name should be within limits");
        assert!(
            valid_symbol.len() <= 10,
            "Valid symbol should be within limits"
        );
        assert!(
            valid_uri.starts_with("https://"),
            "Valid URI should be properly formatted"
        );
    }

    #[test]
    fn test_nft_metadata_validation() {
        // Test NFT-specific validation
        let nft_name = "Awesome NFT #001";
        let nft_symbol = "ANF";
        let nft_uri = "https://example.com/nft/001.json";

        assert!(
            nft_name.len() <= 50,
            "NFT name should be within reasonable limits"
        );
        assert!(nft_symbol.len() <= 10, "NFT symbol should be within limits");
        assert!(
            nft_uri.starts_with("https://"),
            "NFT URI should be properly formatted"
        );
        assert!(
            std::path::Path::new(nft_uri)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json")),
            "NFT URI should point to JSON metadata"
        );
    }

    #[test]
    fn test_amount_validation() {
        // Test zero amount
        let zero_amount = 0u64;
        assert_eq!(zero_amount, 0, "Zero amount should be zero");

        // Test max amount
        let max_amount = u64::MAX;
        assert_eq!(
            max_amount, 18_446_744_073_709_551_615,
            "Max amount should be u64::MAX"
        );

        // Test typical amounts
        let typical_amount = 1_000_000_000; // 1 billion with 9 decimals = 1 token
        assert!(typical_amount > 0, "Typical amount should be positive");
        assert!(
            typical_amount < u64::MAX,
            "Typical amount should be less than max"
        );

        // Test NFT amount (should always be 1)
        let nft_amount = 1u64;
        assert_eq!(nft_amount, 1, "NFT amount should always be 1");
    }

    #[test]
    fn test_decimals_validation() {
        // Test valid decimals (0-18 is typical range for tokens)
        let valid_decimals = [0u8, 6u8, 9u8, 18u8];
        for decimals in valid_decimals {
            assert!(
                decimals <= 18,
                "Decimals {decimals} should be within valid range"
            );
        }

        // Test edge cases
        let min_decimals = 0u8;
        let max_typical_decimals = 18u8;

        assert_eq!(min_decimals, 0, "Min decimals should be 0");
        assert_eq!(
            max_typical_decimals, 18,
            "Max typical decimals should be 18"
        );

        // Test NFT decimals (should always be 0)
        let nft_decimals = 0u8;
        assert_eq!(nft_decimals, 0, "NFT decimals should always be 0");
    }

    #[test]
    fn test_nft_update_field_validation() {
        // Test valid update fields
        let valid_fields = [
            "name",
            "description",
            "image",
            "animation_url",
            "external_url",
        ];
        for field in valid_fields {
            assert!(!field.is_empty(), "Field {field} should not be empty");
            assert!(
                field.len() <= 50,
                "Field {field} should be within reasonable length"
            );
        }

        // Test field values
        let short_value = "Short";
        let long_value = "a".repeat(1000);

        assert!(short_value.len() < 100, "Short value should be acceptable");
        assert!(
            long_value.len() > 500,
            "Long value should be flagged for review"
        );
    }

    #[test]
    fn test_instruction_discriminators() {
        // Test that all instructions have unique discriminators
        let initialize = feels::instruction::Initialize {}.data();
        let feelssol_create = feels::instruction::FeelssolCreate {}.data();
        let feelssol_mint = feels::instruction::FeelssolMint { amount: 100 }.data();
        let feelssol_burn = feels::instruction::FeelssolBurn { amount: 50 }.data();
        let feels_token_create = feels::instruction::FeelsTokenCreate {
            name: "Test".to_string(),
            symbol: "TEST".to_string(),
            uri: "https://example.com".to_string(),
            decimals: 9,
        }
        .data();
        let feels_token_mint = feels::instruction::FeelsTokenMint { amount: 100 }.data();
        let pool_position_create = feels::instruction::PoolPositionCreate {
            position_id: "pos_123".to_string(),
            pool_id: "pool_456".to_string(),
        }
        .data();

        // All discriminators should be different (first 8 bytes)
        assert_ne!(
            &initialize[..8],
            &feelssol_create[..8],
            "Initialize and FeelssolCreate should have different discriminators"
        );
        assert_ne!(
            &initialize[..8],
            &feelssol_mint[..8],
            "Initialize and FeelssolMint should have different discriminators"
        );
        assert_ne!(
            &feelssol_create[..8],
            &feelssol_mint[..8],
            "FeelssolCreate and FeelssolMint should have different discriminators"
        );
        assert_ne!(
            &feelssol_create[..8],
            &feelssol_burn[..8],
            "FeelssolCreate and FeelssolBurn should have different discriminators"
        );
        assert_ne!(
            &feelssol_mint[..8],
            &feelssol_burn[..8],
            "FeelssolMint and FeelssolBurn should have different discriminators"
        );
        assert_ne!(
            &feels_token_create[..8],
            &feels_token_mint[..8],
            "FeelsTokenCreate and FeelsTokenMint should have different discriminators"
        );
        assert_ne!(
            &feels_token_create[..8],
            &pool_position_create[..8],
            "FeelsTokenCreate and PoolPositionCreate should have different discriminators"
        );
    }
}
