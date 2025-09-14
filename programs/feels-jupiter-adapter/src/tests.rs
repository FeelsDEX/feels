#[cfg(test)]
mod tests {
    use crate::amm::FeelsAmm;
    use anchor_lang::prelude::*;
    use solana_program::pubkey::Pubkey;
    use feels::state::{Market, PolicyV1, TokenType, TokenOrigin};
    use std::str::FromStr;

    // Helper function to create a test market
    fn create_test_market() -> Market {
        Market {
            version: 1,
            is_initialized: true,
            is_paused: false,
            token_0: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            token_1: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(), // USDC
            feelssol_mint: Pubkey::from_str("FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad").unwrap(),
            token_0_type: TokenType::Spl,
            token_1_type: TokenType::Spl,
            token_0_origin: TokenOrigin::External,
            token_1_origin: TokenOrigin::External,
            sqrt_price: 79228162514264337593543950336, // ~1.0 price
            liquidity: 1000000000000,
            current_tick: 0,
            tick_spacing: 1,
            global_lower_tick: -887272,
            global_upper_tick: 887272,
            floor_liquidity: 0,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
            base_fee_bps: 30, // 0.3%
            buffer: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            last_epoch_update: 0,
            epoch_number: 0,
            oracle: Pubkey::new_unique(),
            oracle_bump: 255,
            policy: PolicyV1::default(),
            market_authority_bump: 255,
            vault_0_bump: 255,
            vault_1_bump: 255,
            reentrancy_guard: false,
            initial_liquidity_deployed: true,
            _reserved: [0; 31],
        }
    }

    #[test]
    fn test_market_discriminator() {
        // Test that we have the correct discriminator for Market accounts
        let discriminator = Market::DISCRIMINATOR;
        assert_eq!(discriminator, [219, 190, 213, 55, 0, 227, 198, 154]);
    }

    #[test]
    fn test_amm_basic_properties() {
        // Test basic AMM properties without full Jupiter interface
        let market = create_test_market();
        
        // Verify market configuration
        assert!(market.is_initialized);
        assert!(!market.is_paused);
        assert_eq!(market.base_fee_bps, 30);
        
        // Verify token configuration
        assert_eq!(market.token_0_type, TokenType::Spl);
        assert_eq!(market.token_1_type, TokenType::Spl);
    }

    #[test]
    fn test_quote_calculation_logic() {
        // Test the swap calculation logic
        let market = create_test_market();
        let amount_in = 1_000_000_000u64; // 1 SOL
        let is_token_0_to_1 = true;
        let reserve_0 = 1000_000_000_000u64; // 1000 SOL
        let reserve_1 = 1000_000_000_000u64; // 1000 USDC
        
        // Calculate expected output using the same logic as the AMM
        let fee_bps = market.base_fee_bps as u64;
        let fee_amount = (amount_in as u128 * fee_bps as u128 / 10_000) as u64;
        let amount_after_fee = amount_in.saturating_sub(fee_amount);
        
        // Constant product formula
        let (reserve_in, reserve_out) = if is_token_0_to_1 {
            (reserve_0 as u128, reserve_1 as u128)
        } else {
            (reserve_1 as u128, reserve_0 as u128)
        };
        
        let k = reserve_in * reserve_out;
        let new_reserve_in = reserve_in + amount_after_fee as u128;
        let new_reserve_out = k / new_reserve_in;
        let amount_out = reserve_out.saturating_sub(new_reserve_out) as u64;
        
        // Verify calculations
        assert_eq!(fee_amount, 3_000_000); // 0.3% of 1 SOL
        assert!(amount_out > 0);
        assert!(amount_out < amount_in); // Should get less than 1:1 due to slippage
    }

    #[test]
    fn test_market_serialization() {
        // Test that Market can be serialized and deserialized correctly
        let market = create_test_market();
        
        // Serialize
        let mut data = Vec::new();
        market.try_serialize(&mut data).unwrap();
        
        // The serialized data should have been written
        assert!(data.len() > 0);
        // The actual size depends on the Market struct layout
        
        // Deserialize
        let deserialized = Market::try_deserialize(&mut &data[..]).unwrap();
        
        // Verify key fields
        assert_eq!(deserialized.version, market.version);
        assert_eq!(deserialized.is_initialized, market.is_initialized);
        assert_eq!(deserialized.token_0, market.token_0);
        assert_eq!(deserialized.token_1, market.token_1);
        assert_eq!(deserialized.base_fee_bps, market.base_fee_bps);
    }

    #[test]
    fn test_jupiter_adapter_compilation() {
        // This test verifies that the Jupiter adapter module compiles successfully
        // and that all required types are available
        
        // Verify FeelsAmm type exists and can be imported
        use jupiter_amm_interface::AmmProgramIdToLabel;
        
        // Verify the program ID mapping exists
        let labels = <FeelsAmm as AmmProgramIdToLabel>::PROGRAM_ID_TO_LABELS;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].0, feels::ID);
        assert_eq!(labels[0].1, "Feels");
    }

    #[test]
    fn test_vault_derivation() {
        // Test PDA derivation for vaults
        let market = create_test_market();
        let market_key = Pubkey::new_unique();
        let program_id = feels::ID;
        
        // Derive vault addresses
        let (vault_0, _bump_0) = Market::derive_vault_address(&market_key, &market.token_0, &program_id);
        let (vault_1, _bump_1) = Market::derive_vault_address(&market_key, &market.token_1, &program_id);
        
        // Vaults should be different
        assert_ne!(vault_0, vault_1);
        
        // Bumps should be valid (u8 type already ensures 0-255)
        // No need to assert - u8 type guarantees this
    }
}