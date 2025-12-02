//! SDK compatibility layer for tests
//!
//! Provides functions that were removed from the SDK during reorganization

use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::str::FromStr;

// Re-export feels_sdk for convenience
// SDK compatibility layer - using internal types instead of external feels-sdk

// Constants
pub const MARKET_SEED: &[u8] = b"market";
pub const BUFFER_SEED: &[u8] = b"buffer";
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const ORACLE_SEED: &[u8] = b"oracle";
pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
pub const FEELS_HUB_SEED: &[u8] = b"feels_hub";
pub const FEELS_MINT_SEED: &[u8] = b"feels_mint";
pub const POSITION_SEED: &[u8] = b"position";

/// Program ID for Feels protocol
pub fn program_id() -> Pubkey {
    feels::id()
}

/// Find market address
pub fn find_market_address(token_0: &Pubkey, token_1: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MARKET_SEED, token_0.as_ref(), token_1.as_ref()],
        &program_id(),
    )
}

/// Find buffer address
pub fn find_buffer_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BUFFER_SEED, market.as_ref()], &program_id())
}

/// Find vault address
pub fn find_vault_address(market: &Pubkey, token: &Pubkey) -> (Pubkey, u8) {
    // Determine vault index based on token ordering in market
    // This is simplified - in reality would need to check market state
    let index = if token.as_ref()[0] < 128 { b"0" } else { b"1" };
    Pubkey::find_program_address(&[VAULT_SEED, market.as_ref(), index], &program_id())
}

/// Find vault authority address
pub fn find_vault_authority_address(market: &Pubkey) -> (Pubkey, u8) {
    find_market_authority_address(market)
}

/// Find market authority address
pub fn find_market_authority_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, market.as_ref()], &program_id())
}

/// Find FeelsSOL mint address
pub fn find_feels_mint_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"feels_mint"], &program_id())
}

/// Find oracle address
pub fn find_oracle_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ORACLE_SEED, market.as_ref()], &program_id())
}

/// Find protocol config address
pub fn find_protocol_config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id())
}

/// Utilities module
pub mod utils {
    use super::*;

    /// Find position address from position mint
    pub fn find_position_address(position_mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[POSITION_SEED, position_mint.as_ref()], &program_id())
    }

    /// Find tick array address
    pub fn find_tick_array_address(market: &Pubkey, start_tick: i32) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"tick_array", market.as_ref(), &start_tick.to_le_bytes()],
            &program_id(),
        )
    }

    /// Get tick array start index
    pub fn get_tick_array_start_index(tick: i32, tick_spacing: u16) -> i32 {
        let tick_array_size = feels::state::TICK_ARRAY_SIZE as i32;
        let tick_array_spacing = (tick_spacing as i32) * tick_array_size;

        if tick >= 0 {
            (tick / tick_array_spacing) * tick_array_spacing
        } else {
            ((tick - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
        }
    }
}

/// Instructions module additions
pub mod instructions {
    use super::*;
    use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize};
    use solana_sdk::instruction::{AccountMeta, Instruction};

    /// Initialize protocol params
    #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct InitializeProtocolParams {
        pub mint_fee: u64,
        pub treasury: Pubkey,
        pub default_protocol_fee_rate: Option<u16>,
        pub default_creator_fee_rate: Option<u16>,
        pub max_protocol_fee_rate: Option<u16>,
        pub dex_twap_updater: Pubkey,
        pub depeg_threshold_bps: u16,
        pub depeg_required_obs: u8,
        pub clear_required_obs: u8,
        pub dex_twap_window_secs: u32,
        pub dex_twap_stale_age_secs: u32,
        pub dex_whitelist: Vec<Pubkey>,
    }

    /// Build initialize protocol instruction
    pub fn initialize_protocol(
        payer: Pubkey,
        params: InitializeProtocolParams,
    ) -> anchor_lang::Result<Instruction> {
        let (protocol_config, _) = find_protocol_config_address();
        let (protocol_oracle, _) =
            Pubkey::find_program_address(&[b"protocol_oracle"], &program_id());
        let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &program_id());

        let discriminator = [188, 233, 252, 106, 134, 146, 202, 91];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(protocol_config, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                AccountMeta::new(protocol_oracle, false),
                AccountMeta::new(safety, false),
            ],
            data,
        })
    }

    /// Swap params (re-export from feels)
    pub use feels::logic::SwapParams;

    /// Close position params
    pub use feels::instructions::ClosePositionParams;

    /// Build mint token instruction
    pub fn mint_token(
        creator: Pubkey,
        token_mint: Pubkey,
        feelssol_mint: Pubkey,
        creator_feelssol: Pubkey,
        params: feels::instructions::MintTokenParams,
    ) -> anchor_lang::Result<Instruction> {
        // Derive all required PDAs
        let (escrow, _) =
            Pubkey::find_program_address(&[b"escrow", token_mint.as_ref()], &program_id());
        let (escrow_authority, _) = Pubkey::find_program_address(
            &[b"escrow_authority", token_mint.as_ref()],
            &program_id(),
        );
        let (protocol_config, _) = find_protocol_config_address();

        // Escrow vaults
        let escrow_token_vault =
            spl_associated_token_account::get_associated_token_address(&escrow, &token_mint);
        let escrow_feelssol_vault =
            spl_associated_token_account::get_associated_token_address(&escrow, &feelssol_mint);

        // Protocol token registry
        let (protocol_token, _) =
            Pubkey::find_program_address(&[b"protocol_token", token_mint.as_ref()], &program_id());

        // Metadata account
        let metadata_program =
            Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap();
        let (metadata_account, _) = Pubkey::find_program_address(
            &[b"metadata", metadata_program.as_ref(), token_mint.as_ref()],
            &metadata_program,
        );

        let discriminator = [0x84, 0x85, 0x82, 0x50, 0x40, 0xda, 0xba, 0x96];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(creator, true),
                AccountMeta::new(token_mint, true), // Signer because it's being initialized
                AccountMeta::new(escrow, false),
                AccountMeta::new(escrow_token_vault, false),
                AccountMeta::new(escrow_feelssol_vault, false),
                AccountMeta::new_readonly(escrow_authority, false),
                AccountMeta::new(metadata_account, false),
                AccountMeta::new_readonly(feelssol_mint, false),
                AccountMeta::new(creator_feelssol, false),
                AccountMeta::new_readonly(protocol_config, false),
                AccountMeta::new_readonly(metadata_program, false),
                AccountMeta::new(protocol_token, false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data,
        })
    }

    /// Build open position instruction
    pub fn open_position(
        owner: Pubkey,
        market: Pubkey,
        position: Pubkey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: u128,
    ) -> anchor_lang::Result<Instruction> {
        let discriminator = [0x87, 0x80, 0xed, 0xec, 0x53, 0x73, 0x18, 0x0e];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&lower_tick.to_le_bytes());
        data.extend_from_slice(&upper_tick.to_le_bytes());
        data.extend_from_slice(&liquidity.to_le_bytes());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(owner, true),
                AccountMeta::new(market, false),
                AccountMeta::new(position, false),
                // Additional accounts would be needed
            ],
            data,
        })
    }

    /// Build close position instruction
    pub fn close_position(
        owner: Pubkey,
        market: Pubkey,
        position: Pubkey,
        params: ClosePositionParams,
    ) -> anchor_lang::Result<Instruction> {
        let discriminator = [0x7b, 0x86, 0xfd, 0xdc, 0x37, 0xa5, 0xd4, 0xd0];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(owner, true),
                AccountMeta::new(market, false),
                AccountMeta::new(position, false),
                // Additional accounts would be needed
            ],
            data,
        })
    }

    /// Build collect fees instruction
    pub fn collect_fees(
        owner: Pubkey,
        position: Pubkey,
        position_token_account_0: Pubkey,
        position_token_account_1: Pubkey,
    ) -> anchor_lang::Result<Instruction> {
        let discriminator = [0xb1, 0xd2, 0x35, 0x05, 0x63, 0xea, 0x43, 0xfc];

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(owner, true),
                AccountMeta::new(position, false),
                AccountMeta::new(position_token_account_0, false),
                AccountMeta::new(position_token_account_1, false),
                // Additional accounts would be needed
            ],
            data: discriminator.to_vec(),
        })
    }

    /// Build initialize market instruction
    pub fn initialize_market(
        deployer: Pubkey,
        token_0: Pubkey,
        token_1: Pubkey,
        params: feels::instructions::InitializeMarketParams,
    ) -> anchor_lang::Result<Instruction> {
        let (market, _) = find_market_address(&token_0, &token_1);
        let (buffer, _) = find_buffer_address(&market);
        let (vault_authority, _) = find_market_authority_address(&market);
        let (oracle, _) = find_oracle_address(&market);
        let (vault_0, _) = find_vault_address(&market, &token_0);
        let (vault_1, _) = find_vault_address(&market, &token_1);

        // Determine which is FeelsSOL mint
        // In test context, we pass the actual FeelsSOL mint pubkey
        let feelssol_mint =
            if token_0.to_string().starts_with("1") || token_0.to_string().len() < 44 {
                // Likely a low pubkey that's our test FeelsSOL
                token_0
            } else {
                token_1
            };

        // Protocol token accounts
        // For test environments, use system program for FeelsSOL
        let protocol_token_0 = if token_0 == feelssol_mint {
            solana_sdk::system_program::id()
        } else {
            let (pda, _) =
                Pubkey::find_program_address(&[b"protocol_token", token_0.as_ref()], &program_id());
            pda
        };

        let protocol_token_1 = if token_1 == feelssol_mint {
            solana_sdk::system_program::id()
        } else {
            let (pda, _) =
                Pubkey::find_program_address(&[b"protocol_token", token_1.as_ref()], &program_id());
            pda
        };

        // Escrow for non-FeelsSOL token
        let protocol_token_mint = if token_0 == feelssol_mint {
            token_1
        } else {
            token_0
        };
        let (escrow, _) =
            Pubkey::find_program_address(&[b"escrow", protocol_token_mint.as_ref()], &program_id());

        // Escrow authority
        let (escrow_authority, _) =
            Pubkey::find_program_address(&[b"escrow_authority"], &program_id());

        let discriminator = [0x95, 0xf6, 0xc7, 0xee, 0xab, 0x7e, 0xd8, 0x75];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(deployer, true),
                AccountMeta::new_readonly(token_0, false),
                AccountMeta::new_readonly(token_1, false),
                AccountMeta::new(market, false),
                AccountMeta::new(buffer, false),
                AccountMeta::new(oracle, false),
                AccountMeta::new(vault_0, false),
                AccountMeta::new(vault_1, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new_readonly(feelssol_mint, false),
                AccountMeta::new_readonly(protocol_token_0, false),
                AccountMeta::new_readonly(protocol_token_1, false),
                AccountMeta::new_readonly(escrow, false),
                AccountMeta::new_readonly(deployer, false), // creator_feelssol (dummy for tests)
                AccountMeta::new_readonly(deployer, false), // creator_token_out (dummy for tests)
                AccountMeta::new_readonly(escrow_authority, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data,
        })
    }

    /// Build deploy initial liquidity instruction
    pub fn deploy_initial_liquidity(
        deployer: Pubkey,
        market: Pubkey,
        params: feels::instructions::DeployInitialLiquidityParams,
    ) -> anchor_lang::Result<Instruction> {
        let discriminator = [0xc9, 0xf6, 0x66, 0x37, 0x03, 0xa4, 0xd0, 0x72];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&params.try_to_vec().unwrap());

        Ok(Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(deployer, true),
                AccountMeta::new(market, false),
                // Additional accounts would be needed
            ],
            data,
        })
    }
}

/// Build initialize hub instruction
pub fn initialize_hub(payer: Pubkey, feelssol_mint: Pubkey, jitosol_mint: Pubkey) -> Instruction {
    let (feels_hub, _) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint.as_ref()], &program_id());
    let (jitosol_vault, _) =
        Pubkey::find_program_address(&[b"jitosol_vault", feelssol_mint.as_ref()], &program_id());
    let (vault_authority, _) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, feelssol_mint.as_ref()],
        &program_id(),
    );

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(feelssol_mint, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(feels_hub, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: vec![202, 27, 126, 27, 54, 182, 68, 169], // initialize_hub discriminator
    }
}

/// Build enter feelssol instruction
pub fn enter_feelssol(
    user: Pubkey,
    user_jitosol: Pubkey,
    user_feelssol: Pubkey,
    feelssol_mint: Pubkey,
    jitosol_mint: Pubkey,
    amount: u64,
) -> Instruction {
    // Derive PDAs
    let (hub, _) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint.as_ref()], &program_id());
    let (jitosol_vault, _) =
        Pubkey::find_program_address(&[b"jitosol_vault", feelssol_mint.as_ref()], &program_id());
    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"mint_authority", feelssol_mint.as_ref()], &program_id());

    let discriminator = [0xc7, 0xcd, 0x31, 0xad, 0x51, 0x32, 0xba, 0x7e];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(user_jitosol, false),
            AccountMeta::new(user_feelssol, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(feelssol_mint, false),
            AccountMeta::new(hub, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(mint_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    }
}

/// Build exit feelssol instruction
pub fn exit_feelssol(
    user: Pubkey,
    user_feelssol: Pubkey,
    user_jitosol: Pubkey,
    feelssol_mint: Pubkey,
    jitosol_mint: Pubkey,
    amount: u64,
) -> Instruction {
    // Derive PDAs
    let (hub, _) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint.as_ref()], &program_id());
    let (jitosol_vault, _) =
        Pubkey::find_program_address(&[b"jitosol_vault", feelssol_mint.as_ref()], &program_id());
    let (vault_authority, _) = Pubkey::find_program_address(
        &[VAULT_AUTHORITY_SEED, feelssol_mint.as_ref()],
        &program_id(),
    );
    let (safety_controller, _) =
        Pubkey::find_program_address(&[b"safety_controller"], &program_id());
    let (protocol_config, _) = Pubkey::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id());
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &program_id());

    let discriminator = [0x69, 0x76, 0xa8, 0x94, 0x3d, 0x98, 0x03, 0xaf];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(user_feelssol, false),
            AccountMeta::new(user_jitosol, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(feelssol_mint, false),
            AccountMeta::new(hub, false),
            AccountMeta::new_readonly(safety_controller, false),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new_readonly(protocol_oracle, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ],
        data,
    }
}

/// Build initialize market instruction
pub fn initialize_market(
    deployer: Pubkey,
    market: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    params: feels::instructions::InitializeMarketParams,
) -> Instruction {
    instructions::initialize_market(deployer, token_0, token_1, params).unwrap()
}

/// Build deploy initial liquidity instruction
pub fn deploy_initial_liquidity(
    deployer: Pubkey,
    market: Pubkey,
    params: feels::instructions::DeployInitialLiquidityParams,
) -> Instruction {
    instructions::deploy_initial_liquidity(deployer, market, params).unwrap()
}

/// Sort tokens with FeelsSOL always as token_0
pub fn sort_tokens_with_feelssol(
    token_a: Pubkey,
    token_b: Pubkey,
    feelssol_mint: Pubkey,
) -> std::result::Result<(Pubkey, Pubkey), &'static str> {
    if token_a == token_b {
        return Err("Cannot create market with identical tokens");
    }

    if token_a == feelssol_mint {
        Ok((feelssol_mint, token_b))
    } else if token_b == feelssol_mint {
        Ok((feelssol_mint, token_a))
    } else {
        Err("Market must include FeelsSOL as one of the tokens")
    }
}

/// Build update native rate instruction
pub fn update_native_rate(authority: Pubkey, native_rate_q64: u128) -> Instruction {
    let (protocol_config, _) = find_protocol_config_address();
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &program_id());
    let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &program_id());

    let params = feels::instructions::UpdateNativeRateParams { native_rate_q64 };

    let discriminator = [100, 175, 161, 10, 254, 80, 99, 77]; // update_native_rate
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&params.try_to_vec().unwrap());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new(protocol_oracle, false),
            AccountMeta::new(safety, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ],
        data,
    }
}

// Types needed for tests
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub price_impact: f64,
    pub fee_amount: u64,
    pub fee_paid: u64, // Alias for fee_amount for backwards compatibility
}

#[derive(Debug)]
pub struct TestMarketSetup {
    pub market_id: Pubkey,
    pub market: Pubkey, // Derived from market_id for backwards compatibility
    pub oracle_id: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub market_authority: Pubkey,
    pub buffer_id: Pubkey,
    pub protocol_config: Pubkey,
    pub protocol_treasury: Pubkey,
    pub feelssol_mint: Pubkey,
    pub custom_token_mint: Pubkey,
    pub custom_token_keypair: Keypair,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub token_mint: Pubkey, // Alias for custom_token_mint
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub address: Pubkey,
    pub pubkey: Pubkey, // Alias for address
    pub market: Pubkey,
    pub owner: Pubkey,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity: u128,
    pub fee_growth_inside_0: u128,
    pub fee_growth_inside_1: u128,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
    pub mint: Pubkey, // Position NFT mint
    pub token_account: Pubkey, // Position NFT token account
}

#[derive(Debug, Clone)]
pub struct CollectFeesResult {
    pub amount_0: u64,
    pub amount_1: u64,
    pub fee_a_collected: u64, // Alias for amount_0
    pub fee_b_collected: u64, // Alias for amount_1
}

#[derive(Debug)]
pub struct SwapParams {
    pub market: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub trader: Keypair,
}

/// Build update dex twap instruction
pub fn update_dex_twap(updater: Pubkey, dex_twap_rate_q64: u128, venue_id: Pubkey) -> Instruction {
    let (protocol_config, _) = find_protocol_config_address();
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &program_id());
    let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &program_id());

    let params = feels::instructions::UpdateDexTwapParams {
        dex_twap_rate_q64,
        window_secs: 300, // 5 minutes default
        obs: 1,
        venue_id,
    };

    let discriminator = [144, 64, 180, 12, 223, 33, 140, 232]; // update_dex_twap
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&params.try_to_vec().unwrap());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(updater, true),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new(protocol_oracle, false),
            AccountMeta::new(safety, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ],
        data,
    }
}
