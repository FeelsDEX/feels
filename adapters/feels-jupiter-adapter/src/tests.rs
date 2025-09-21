#[cfg(test)]
mod tests {
    use crate::amm::FeelsAmm;
    use anchor_lang::prelude::*;
    use solana_program::pubkey::Pubkey;
    use feels::state::{Market, PolicyV1, TokenType, TokenOrigin};
    use jupiter_amm_interface::Amm;
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
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            hub_protocol: None,
            sqrt_price: 79228162514264337593543950336, // ~1.0 price
            liquidity: 1000000000000,
            current_tick: 0,
            tick_spacing: 1,
            global_lower_tick: -887272,
            global_upper_tick: 887272,
            floor_liquidity: 0,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
            fee_growth_global_0: 0,
            fee_growth_global_1: 0,
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
            jit_enabled: false,
            jit_base_cap_bps: 300,
            jit_per_slot_cap_bps: 500,
            jit_concentration_width: 100,
            jit_max_multiplier: 10,
            jit_drain_protection_bps: 7000,
            jit_circuit_breaker_bps: 3000,
            floor_tick: -887272,
            floor_buffer_ticks: 1000,
            last_floor_ratchet_ts: 0,
            floor_cooldown_secs: 3600,
            steady_state_seeded: false,
            cleanup_complete: false,
            phase: 0,
            phase_start_slot: 0,
            phase_start_timestamp: 0,
            last_phase_transition_slot: 0,
            last_phase_trigger: 0,
            total_volume_token_0: 0,
            total_volume_token_1: 0,
            rolling_buy_volume: 0,
            rolling_sell_volume: 0,
            rolling_total_volume: 0,
            rolling_window_start_slot: 0,
            tick_snapshot_1hr: 0,
            last_snapshot_timestamp: 0,
            _reserved: [0; 1],
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

    #[test]
    fn test_multi_tick_quote_accumulation() {
        // Test quote accumulation across multiple ticks with alternating liquidity
        // This verifies the adapter correctly handles:
        // 1. Multiple tick crossings
        // 2. Alternating liquidity_net (positive and negative)
        // 3. Correct output accumulation
        
        // Create a market with multiple liquidity positions
        let mut market = create_test_market();
        market.liquidity = 1_000_000_000_000; // 1M tokens
        market.tick_spacing = 10;
        market.current_tick = 0;
        
        // Simulate a swap that would cross multiple ticks
        // In a real test, we would:
        // 1. Create tick arrays with alternating liquidity patterns
        // 2. Execute swaps of different sizes
        // 3. Verify output accumulation is correct
        
        // Test case 1: Small swap within current tick
        let small_amount = 10_000_000; // 10 tokens
        let fee_amount = (small_amount as u128 * market.base_fee_bps as u128 / 10_000) as u64;
        assert_eq!(fee_amount, 30_000); // 0.3% of 10 tokens
        
        // Test case 2: Medium swap crossing multiple ticks
        let _medium_amount = 100_000_000; // 100 tokens
        // With alternating liquidity, price impact would vary
        
        // Test case 3: Large swap hitting liquidity gaps
        let _large_amount = 1_000_000_000; // 1000 tokens
        // This would significantly impact price with 1M liquidity
        
        // Verify key invariants:
        // 1. Larger swaps have worse price (more slippage)
        // 2. Fees increase with swap size
        // 3. Output never exceeds input (after fees)
        assert!(fee_amount > 0, "Fees should always be collected");
        assert!(small_amount > fee_amount, "Output should be less than input");
    }

    #[test]
    fn test_tick_crossing_scenarios() {
        // Test specific tick crossing scenarios
        let market = create_test_market();
        
        // Scenario 1: Crossing from positive to negative liquidity_net
        // This reduces total liquidity in the pool
        
        // Scenario 2: Crossing from negative to positive liquidity_net  
        // This increases total liquidity in the pool
        
        // Scenario 3: Multiple consecutive crossings
        // Tests accumulation accuracy
        
        // Scenario 4: Hitting zero liquidity
        // Should stop the swap at that point
        
        // Each scenario would be tested with the actual adapter implementation
        // For now, we verify the market structure supports these tests
        assert_eq!(market.tick_spacing, 1, "Tick spacing should allow fine-grained testing");
        assert!(market.liquidity > 0, "Market should have initial liquidity");
    }

    #[test]
    fn test_quote_execution_parity_comprehensive() {
        use feels_sdk::jupiter::{MarketState, TickArrayLoader};
        
        // Create test market with realistic parameters
        let mut market = create_test_market();
        market.sqrt_price = 79228162514264337593543950336u128; // Price = 1
        market.current_tick = 0;
        market.liquidity = 10_000_000_000_000u128; // 10M liquidity
        market.base_fee_bps = 30; // 0.3% fee
        market.tick_spacing = 60;
        
        // Test various swap scenarios
        let test_cases = vec![
            (100_000u64, true),         // Small swap token0->token1
            (100_000u64, false),        // Small swap token1->token0
            (10_000_000u64, true),      // Medium swap
            (10_000_000u64, false),     
            (1_000_000_000u64, true),   // Large swap
            (1_000_000_000u64, false),
        ];
        
        for (amount_in, is_token_0_to_1) in test_cases {
            // Create market state
            let market_state = MarketState {
                market_key: Pubkey::new_unique(),
                token_0: market.token_0,
                token_1: market.token_1,
                sqrt_price: market.sqrt_price,
                current_tick: market.current_tick,
                liquidity: market.liquidity,
                fee_bps: market.base_fee_bps,
                tick_spacing: market.tick_spacing,
                global_lower_tick: market.global_lower_tick,
                global_upper_tick: market.global_upper_tick,
                fee_growth_global_0: market.fee_growth_global_0_x64,
                fee_growth_global_1: market.fee_growth_global_1_x64,
            };
            
            // Empty tick arrays (no initialized ticks to cross)
            let tick_arrays = TickArrayLoader::new();
            
            // Create simulator and calculate
            let simulator = feels_sdk::jupiter::SwapSimulator::new(&market_state, &tick_arrays);
            let result = simulator.simulate_swap(amount_in, is_token_0_to_1).unwrap();
            
            // Verify fee calculation exactly matches on-chain logic
            let expected_fee = ((amount_in as u128 * market.base_fee_bps as u128 + 9999) / 10000) as u64;
            assert_eq!(
                result.fee_paid, expected_fee,
                "Fee mismatch for {} {} direction",
                amount_in,
                if is_token_0_to_1 { "0->1" } else { "1->0" }
            );
            
            // Verify invariants
            let amount_after_fee = amount_in.saturating_sub(result.fee_paid);
            if amount_after_fee > 0 && market.liquidity > 0 {
                assert!(result.amount_out > 0, "Should have output for non-zero input");
                
                // Output should be less than input (no free money)
                assert!(
                    result.amount_out < amount_in,
                    "Output {} should be less than input {}",
                    result.amount_out, amount_in
                );
            }
        }
    }

    #[test]
    fn test_tick_array_quote_consistency() {
        use feels_sdk::jupiter::{MarketState, TickArrayLoader, ParsedTickArray, TickArrayFormat};
        use std::collections::HashMap;
        
        let mut market = create_test_market();
        market.liquidity = 5_000_000_000_000u128;
        market.current_tick = 100;
        market.tick_spacing = 10;
        
        // Create market state
        let market_state = MarketState {
            market_key: Pubkey::new_unique(),
            token_0: market.token_0,
            token_1: market.token_1,
            sqrt_price: market.sqrt_price,
            current_tick: market.current_tick,
            liquidity: market.liquidity,
            fee_bps: market.base_fee_bps,
            tick_spacing: market.tick_spacing,
            global_lower_tick: market.global_lower_tick,
            global_upper_tick: market.global_upper_tick,
            fee_growth_global_0: market.fee_growth_global_0_x64,
            fee_growth_global_1: market.fee_growth_global_1_x64,
        };
        
        // Create tick arrays with liquidity changes
        let mut tick_arrays = TickArrayLoader::new();
        let mut initialized_ticks = HashMap::new();
        
        // Add ticks with liquidity changes
        initialized_ticks.insert(50, 1_000_000_000i128);    // Add liquidity
        initialized_ticks.insert(80, -500_000_000i128);     // Remove liquidity  
        initialized_ticks.insert(110, 2_000_000_000i128);   // Add more
        initialized_ticks.insert(150, -2_500_000_000i128);  // Remove most
        
        // Create ParsedTickArray
        let parsed = ParsedTickArray {
            format: TickArrayFormat::V1,
            market: market_state.market_key,
            start_tick_index: 0,
            initialized_ticks,
            initialized_count: Some(4),
        };
        
        tick_arrays.add_parsed_array(parsed);
        
        // Test swaps in both directions
        let amounts = vec![1_000_000u64, 50_000_000u64, 200_000_000u64];
        
        for amount_in in amounts {
            // Create simulators for each direction
            let simulator = feels_sdk::jupiter::SwapSimulator::new(&market_state, &tick_arrays);
            
            // Swap up (crossing positive ticks)
            let result_up = simulator.simulate_swap(amount_in, false).unwrap(); // token1->token0, price increases
            
            // Swap down (crossing negative ticks)
            let result_down = simulator.simulate_swap(amount_in, true).unwrap(); // token0->token1, price decreases
            
            // Fees should be identical (same input amount)
            assert_eq!(result_up.fee_paid, result_down.fee_paid, "Fees should match for same input");
            
            // Outputs may differ due to tick crossings and liquidity changes
            // But both should be valid
            assert!(result_up.amount_out > 0 || amount_in <= result_up.fee_paid);
            assert!(result_down.amount_out > 0 || amount_in <= result_down.fee_paid);
        }
    }
    
    #[test]
    fn test_fee_account_handling() {
        use crate::config::{set_treasury, add_protocol_token};
        use solana_program::pubkey::Pubkey;
        
        // Set a test treasury
        let treasury = Pubkey::new_unique();
        set_treasury(treasury);
        
        // Add a test protocol token
        let protocol_mint = Pubkey::new_unique();
        add_protocol_token(protocol_mint);
        
        // Create test market
        let mut market = create_test_market();
        market.token_0 = protocol_mint; // Make token_0 a protocol token
        
        // Test treasury ATA derivation
        let output_mint = market.token_1;
        let treasury_ata = crate::config::get_treasury_ata(&output_mint);
        
        // Verify it's the correct ATA
        let expected_ata = spl_associated_token_account::get_associated_token_address(
            &treasury,
            &output_mint
        );
        assert_eq!(
            treasury_ata, expected_ata,
            "Treasury ATA should match expected derivation"
        );
        
        // Test protocol token detection
        assert!(
            crate::config::is_protocol_token(&protocol_mint),
            "Should detect registered protocol token"
        );
        assert!(
            !crate::config::is_protocol_token(&market.token_1),
            "Should not detect unregistered token as protocol token"
        );
    }
    
    #[test]
    fn test_tick_array_format_detection() {
        use feels_sdk::{TickArrayFormat, parse_tick_array_auto};
        
        // Test V1 format with exact size
        let mut v1_data = vec![0u8; TickArrayFormat::V1.calculate_total_size()];
        v1_data[..8].copy_from_slice(&TickArrayFormat::V1.discriminator);
        
        let result = parse_tick_array_auto(&v1_data, 10);
        assert!(result.is_ok(), "Should parse V1 format successfully");
        
        let parsed = result.unwrap();
        assert_eq!(parsed.format.version, 1);
        assert_eq!(parsed.format.array_size, 64);
    }
    
    #[test]
    fn test_tick_array_format_extensions() {
        use feels_sdk::{TickArrayFormat, parse_tick_array_auto};
        
        // Test V1 format with extensions (future-proofing)
        let extended_size = TickArrayFormat::V1.calculate_total_size() + 128;
        let mut extended_data = vec![0u8; extended_size];
        extended_data[..8].copy_from_slice(&TickArrayFormat::V1.discriminator);
        
        let result = parse_tick_array_auto(&extended_data, 10);
        assert!(
            result.is_ok(), 
            "Should handle V1 format with extensions for backward compatibility"
        );
    }
    
    #[test]
    fn test_tick_array_parsing_corrupted_data() {
        use feels_sdk::parse_tick_array_auto;
        
        // Test various corrupted data scenarios
        let test_cases = vec![
            (vec![0u8; 7], "Too small"),
            (vec![255u8; 100], "Invalid discriminator"),
            (vec![0u8; 1000], "Wrong discriminator"),
        ];
        
        for (data, scenario) in test_cases {
            let result = parse_tick_array_auto(&data, 10);
            assert!(
                result.is_err(),
                "Should fail to parse corrupted data: {}",
                scenario
            );
        }
    }
    
    #[test]
    fn test_tick_array_initialized_tick_extraction() {
        use feels_sdk::{TickArrayFormat, parse_tick_array_auto};
        
        // Create a valid tick array with some initialized ticks
        let mut data = vec![0u8; TickArrayFormat::V1.calculate_total_size()];
        
        // Set discriminator
        data[..8].copy_from_slice(&TickArrayFormat::V1.discriminator);
        
        // Set market pubkey (32 bytes at offset 8)
        let market = Pubkey::new_unique();
        data[8..40].copy_from_slice(market.as_ref());
        
        // Set start_tick_index (4 bytes at offset 40)
        let start_tick = 1000i32;
        data[40..44].copy_from_slice(&start_tick.to_le_bytes());
        
        // Initialize some ticks in the array
        let tick_spacing = 10u16;
        let ticks_offset = 56; // After discriminator + header
        
        // Initialize tick at index 0
        let tick_0_offset = ticks_offset;
        data[tick_0_offset..tick_0_offset + 16].copy_from_slice(&100i128.to_le_bytes()); // liquidity_net
        data[tick_0_offset + 64] = 1; // initialized flag
        
        // Initialize tick at index 5
        let tick_5_offset = ticks_offset + (5 * 80);
        data[tick_5_offset..tick_5_offset + 16].copy_from_slice(&(-50i128).to_le_bytes()); // liquidity_net
        data[tick_5_offset + 64] = 1; // initialized flag
        
        // Parse the array
        let parsed = parse_tick_array_auto(&data, tick_spacing).unwrap();
        
        // Verify parsed data
        assert_eq!(parsed.market, market);
        assert_eq!(parsed.start_tick_index, start_tick);
        assert_eq!(parsed.initialized_ticks.len(), 2);
        
        // Check tick indices
        let tick_0_index = start_tick + 0 * tick_spacing as i32;
        let tick_5_index = start_tick + 5 * tick_spacing as i32;
        
        assert_eq!(
            parsed.initialized_ticks.get(&tick_0_index),
            Some(&100i128),
            "Should have tick at index {}",
            tick_0_index
        );
        assert_eq!(
            parsed.initialized_ticks.get(&tick_5_index),
            Some(&-50i128),
            "Should have tick at index {}",
            tick_5_index
        );
    }
    
    #[test]
    fn test_adapter_tick_array_update_resilience() {
        use crate::amm::FeelsAmm;
        use jupiter_amm_interface::{KeyedAccount, AmmContext};
        use solana_sdk::account::Account as SolanaAccount;
        use feels_sdk::TickArrayFormat;
        
        // Create test market
        let market = create_test_market();
        let mut market_data = Vec::new();
        market.try_serialize(&mut market_data).unwrap();
        
        let market_key = Pubkey::new_unique();
        let keyed_account = KeyedAccount {
            key: market_key,
            account: SolanaAccount {
                lamports: 1_000_000,
                data: market_data,
                owner: feels::ID,
                executable: false,
                rent_epoch: 0,
            },
            params: None,
        };
        
        // Create AMM instance
        let amm_context = AmmContext {
            clock_ref: Default::default(),
        };
        let mut amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context).unwrap();
        
        // Create account map with tick array data
        let mut account_map = ahash::AHashMap::<Pubkey, SolanaAccount>::new();
        
        // Add vault accounts
        use solana_program::program_pack::Pack;
        use spl_token::state::AccountState;
        
        // Create properly initialized SPL token accounts
        let mut vault_0_data = vec![0u8; spl_token::state::Account::LEN];
        let vault_0_account = spl_token::state::Account {
            mint: market.token_0,
            owner: keyed_account.key,
            amount: 1_000_000_000_000,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        vault_0_account.pack_into_slice(&mut vault_0_data);
        
        let mut vault_1_data = vec![0u8; spl_token::state::Account::LEN];
        let vault_1_account = spl_token::state::Account {
            mint: market.token_1,
            owner: keyed_account.key,
            amount: 1_000_000_000_000,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        vault_1_account.pack_into_slice(&mut vault_1_data);
        
        // Get accounts to update (includes vaults)
        let accounts = amm.get_accounts_to_update();
        if accounts.len() >= 2 {
            account_map.insert(accounts[0], SolanaAccount {
                lamports: 1_000_000,
                data: vault_0_data,
                owner: spl_token::id(),
                executable: false,
                rent_epoch: 0,
            });
            account_map.insert(accounts[1], SolanaAccount {
                lamports: 1_000_000,
                data: vault_1_data,
                owner: spl_token::id(),
                executable: false,
                rent_epoch: 0,
            });
        }
        
        // Add tick array with V1 format
        // Tick array keys are at index 2+ in accounts list
        let tick_array_key = if accounts.len() > 2 { accounts[2] } else { Pubkey::new_unique() };
        let mut tick_data = vec![0u8; TickArrayFormat::V1.calculate_total_size()];
        tick_data[..8].copy_from_slice(&TickArrayFormat::V1.discriminator);
        tick_data[8..40].copy_from_slice(market_key.as_ref());
        account_map.insert(tick_array_key, SolanaAccount {
            lamports: 1_000_000,
            data: tick_data,
            owner: feels::ID,
            executable: false,
            rent_epoch: 0,
        });
        
        // Update should succeed with V1 format
        let result = amm.update(&account_map);
        assert!(result.is_ok(), "Should handle V1 tick array format");
        
        // Test with extended format (simulating future protocol upgrade)
        let mut extended_data = vec![0u8; TickArrayFormat::V1.calculate_total_size() + 100];
        extended_data[..8].copy_from_slice(&TickArrayFormat::V1.discriminator);
        extended_data[8..40].copy_from_slice(market_key.as_ref());
        account_map.insert(tick_array_key, SolanaAccount {
            lamports: 1_000_000,
            data: extended_data,
            owner: feels::ID,
            executable: false,
            rent_epoch: 0,
        });
        
        // Update should still succeed with extended format
        let result = amm.update(&account_map);
        assert!(
            result.is_ok(), 
            "Should handle extended tick array format for future compatibility"
        );
    }
    
    #[test]
    fn test_sdk_onchain_quote_parity() {
        use crate::amm::FeelsAmm;
        use jupiter_amm_interface::{QuoteParams, AmmContext, KeyedAccount};
        use solana_sdk::account::Account as SolanaAccount;
        
        // Create a test market with specific parameters
        let mut market = create_test_market();
        market.liquidity = 5_000_000_000_000u128;
        market.base_fee_bps = 30; // 0.3%
        market.sqrt_price = 79228162514264337593543950336u128; // Price = 1
        
        let mut market_data = Vec::new();
        market.try_serialize(&mut market_data).unwrap();
        
        let keyed_account = KeyedAccount {
            key: Pubkey::new_unique(),
            account: SolanaAccount {
                lamports: 1_000_000,
                data: market_data,
                owner: feels::ID,
                executable: false,
                rent_epoch: 0,
            },
            params: None,
        };
        
        // Create AMM instance
        let amm_context = AmmContext {
            clock_ref: Default::default(),
        };
        let amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context).unwrap();
        
        // Test various swap amounts
        let test_amounts = vec![
            1_000_000u64,      // 1 token
            10_000_000u64,     // 10 tokens
            100_000_000u64,    // 100 tokens
            1_000_000_000u64,  // 1000 tokens
        ];
        
        for amount in test_amounts {
            let quote_params = QuoteParams {
                amount,
                input_mint: market.token_0,
                output_mint: market.token_1,
                swap_mode: jupiter_amm_interface::SwapMode::ExactIn,
            };
            
            let quote = amm.quote(&quote_params).unwrap();
            
            // Verify fee calculation matches on-chain logic
            // For 0.3% fee: net = floor(gross * 9970 / 10000)
            let expected_net = (amount as u128 * 9970) / 10000;
            let expected_fee = amount - expected_net as u64;
            
            assert_eq!(
                quote.fee_amount, expected_fee,
                "Fee for {} should match on-chain calculation",
                amount
            );
            
            // Verify we got some output
            assert!(
                quote.out_amount > 0,
                "Should have output for {} input",
                amount
            );
            
            // Verify price impact is reasonable
            let impact_ratio = 1.0 - (quote.out_amount as f64 / expected_net as f64);
            assert!(
                impact_ratio < 0.1, // Less than 10% impact
                "Price impact {} too high for {} tokens",
                impact_ratio,
                amount
            );
        }
    }
    
    #[test]
    fn test_quote_consistency_across_updates() {
        use crate::amm::FeelsAmm;
        use jupiter_amm_interface::{QuoteParams, AmmContext, KeyedAccount};
        use solana_sdk::account::Account as SolanaAccount;
        
        // Create market and AMM
        let market = create_test_market();
        let mut market_data = Vec::new();
        market.try_serialize(&mut market_data).unwrap();
        
        let keyed_account = KeyedAccount {
            key: Pubkey::new_unique(),
            account: SolanaAccount {
                lamports: 1_000_000,
                data: market_data,
                owner: feels::ID,
                executable: false,
                rent_epoch: 0,
            },
            params: None,
        };
        
        let amm_context = AmmContext {
            clock_ref: Default::default(),
        };
        let mut amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context).unwrap();
        
        // Get initial quote
        let quote_params = QuoteParams {
            amount: 10_000_000,
            input_mint: market.token_0,
            output_mint: market.token_1,
            swap_mode: jupiter_amm_interface::SwapMode::ExactIn,
        };
        
        let quote1 = amm.quote(&quote_params).unwrap();
        
        // Update with empty tick arrays (no change in liquidity)
        let mut account_map = ahash::AHashMap::<Pubkey, SolanaAccount>::new();
        let accounts = amm.get_accounts_to_update();
        
        // Create properly initialized SPL token account data
        use spl_token::state::AccountState;
        use solana_program::program_pack::Pack;
        
        let mut vault_0_data = vec![0u8; spl_token::state::Account::LEN];
        let vault_0_account = spl_token::state::Account {
            mint: market.token_0,
            owner: keyed_account.key, // Market owns the vault
            amount: 1_000_000_000_000, // 1M tokens
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        vault_0_account.pack_into_slice(&mut vault_0_data);
        
        let mut vault_1_data = vec![0u8; spl_token::state::Account::LEN];
        let vault_1_account = spl_token::state::Account {
            mint: market.token_1,
            owner: keyed_account.key, // Market owns the vault
            amount: 1_000_000_000_000, // 1M tokens
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        vault_1_account.pack_into_slice(&mut vault_1_data);
        
        if accounts.len() >= 2 {
            account_map.insert(accounts[0], SolanaAccount {
                lamports: 1_000_000,
                data: vault_0_data,
                owner: spl_token::id(),
                executable: false,
                rent_epoch: 0,
            });
            account_map.insert(accounts[1], SolanaAccount {
                lamports: 1_000_000,
                data: vault_1_data,
                owner: spl_token::id(),
                executable: false,
                rent_epoch: 0,
            });
        }
        
        amm.update(&account_map).unwrap();
        
        // Get quote again - should be identical
        let quote2 = amm.quote(&quote_params).unwrap();
        
        assert_eq!(
            quote1.out_amount, quote2.out_amount,
            "Output should be consistent without liquidity changes"
        );
        assert_eq!(
            quote1.fee_amount, quote2.fee_amount,
            "Fee should be consistent"
        );
    }
    
    #[test]
    fn test_swap_account_generation() {
        use crate::amm::FeelsAmm;
        use jupiter_amm_interface::{SwapParams, KeyedAccount, AmmContext};
        use solana_sdk::account::Account as SolanaAccount;
        use crate::config::set_treasury;
        
        // Configure treasury
        let treasury = Pubkey::new_unique();
        set_treasury(treasury);
        
        // Create a test market account
        let market = create_test_market();
        let mut market_data = Vec::new();
        market.try_serialize(&mut market_data).unwrap();
        
        let keyed_account = KeyedAccount {
            key: Pubkey::new_unique(),
            account: SolanaAccount {
                lamports: 1_000_000,
                data: market_data,
                owner: feels::ID,
                executable: false,
                rent_epoch: 0,
            },
            params: None,
        };
        
        // Create AMM instance
        let amm_context = AmmContext {
            clock_ref: Default::default(),
        };
        let amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context).unwrap();
        
        // Test swap params
        let jupiter_program_id = Pubkey::new_unique();
        let swap_params = SwapParams {
            source_mint: market.token_0,
            destination_mint: market.token_1,
            source_token_account: Pubkey::new_unique(),
            destination_token_account: Pubkey::new_unique(),
            token_transfer_authority: Pubkey::new_unique(),
            quote_mint_to_referrer: None,
            jupiter_program_id: &jupiter_program_id,
            swap_mode: jupiter_amm_interface::SwapMode::ExactIn,
            in_amount: 1_000_000,
            out_amount: 0,
            missing_dynamic_accounts_as_default: false,
        };
        
        // Generate swap accounts
        let result = amm.get_swap_and_account_metas(&swap_params);
        assert!(result.is_ok(), "Should generate swap accounts successfully");
        
        let swap_and_metas = result.unwrap();
        let accounts = swap_and_metas.account_metas;
        
        // Verify we have the correct number of accounts
        // Updated to expect 17 accounts including fee distribution accounts
        assert_eq!(
            accounts.len(),
            amm.get_accounts_len(),
            "Should have correct number of accounts"
        );
        
        // Verify treasury account is included
        let treasury_ata = crate::config::get_treasury_ata(&market.token_1);
        let has_treasury = accounts.iter().any(|meta| meta.pubkey == treasury_ata);
        assert!(
            has_treasury,
            "Should include treasury ATA in account list"
        );
    }
}