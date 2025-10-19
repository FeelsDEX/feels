//! Test SDK initialize_market instruction

#[cfg(test)]
mod tests {
    use feels_sdk::{instructions, SdkError};
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_initialize_market_instruction_ordering() {
        // Test with FeelsSOL as token_0
        let feelssol_mint = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();

        // Order tokens
        let (token_0, token_1) = if feelssol_mint < token_mint {
            (feelssol_mint, token_mint)
        } else {
            (token_mint, feelssol_mint)
        };

        let authority = Pubkey::new_unique();

        // Create instruction
        let ix = instructions::initialize_market(
            authority,
            token_0,
            token_1,
            feelssol_mint,
            30,                                // base_fee_bps
            10,                                // tick_spacing
            79228162514264337593543950336u128, // initial_sqrt_price (1:1)
            0,                                 // no initial buy
            None,                              // creator_feelssol
            None,                              // creator_token_out
        )
        .unwrap();

        // Verify accounts are in correct order
        assert_eq!(ix.accounts.len(), 19, "Should have 19 accounts");

        // Check that system program is only at position 16 (index 16)
        let system_program_count = ix
            .accounts
            .iter()
            .filter(|meta| meta.pubkey == solana_program::system_program::id())
            .count();
        assert_eq!(
            system_program_count, 1,
            "Should have exactly one System Program account"
        );

        // Verify System Program is at correct position
        assert_eq!(
            ix.accounts[16].pubkey,
            solana_program::system_program::id(),
            "System Program should be at position 16"
        );

        // Verify protocol token accounts are unique dummy PDAs when token is FeelsSOL
        if token_0 == feelssol_mint {
            assert_ne!(
                ix.accounts[10].pubkey,
                solana_program::system_program::id(),
                "Protocol token 0 should not be System Program when token is FeelsSOL"
            );
        }

        if token_1 == feelssol_mint {
            assert_ne!(
                ix.accounts[11].pubkey,
                solana_program::system_program::id(),
                "Protocol token 1 should not be System Program when token is FeelsSOL"
            );
        }
    }

    #[test]
    fn test_both_tokens_as_feelssol_edge_case() {
        // This shouldn't happen in practice, but test the edge case
        let feelssol_mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        // Try to create a market with FeelsSOL/FeelsSOL (invalid but should not panic)
        let result = instructions::initialize_market(
            authority,
            feelssol_mint,
            feelssol_mint,
            feelssol_mint,
            30,
            10,
            79228162514264337593543950336u128,
            0,
            None,
            None,
        );

        // Should fail because you cannot create a market with the same token as both token_0 and token_1
        assert!(result.is_err());

        // Verify the error is the expected token ordering validation
        let error = result.unwrap_err();
        match error {
            SdkError::InvalidParameters(msg) => {
                assert!(msg.contains("Invalid token order"));
            }
            _ => panic!("Expected InvalidParameters error for same token market"),
        }
    }
}
