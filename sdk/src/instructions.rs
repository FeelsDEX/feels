//! Instruction builders for MVP

use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::{
    error::SdkError, find_buffer_address, find_market_address, find_vault_authority_address,
};

type Result<T> = std::result::Result<T, SdkError>;

// Instruction discriminators (8-byte sighashes)
const ENTER_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0xc7, 0xcd, 0x31, 0xad, 0x51, 0x32, 0xba, 0x7e];
const EXIT_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0x69, 0x76, 0xa8, 0x94, 0x3d, 0x98, 0x03, 0xaf];
const INITIALIZE_HUB_DISCRIMINATOR: [u8; 8] = [0xca, 0x1b, 0x7e, 0x1b, 0x36, 0xb6, 0x44, 0xa9];
const SWAP_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];
const INITIALIZE_MARKET_DISCRIMINATOR: [u8; 8] = [0x23, 0x23, 0xbd, 0xc1, 0x9b, 0x30, 0xaa, 0xcb];
const MINT_TOKEN_DISCRIMINATOR: [u8; 8] = [0xac, 0x89, 0xb7, 0x0e, 0xcf, 0x6e, 0xea, 0x38];
const DEPLOY_INITIAL_LIQUIDITY_DISCRIMINATOR: [u8; 8] =
    [0x9f, 0xf7, 0xd1, 0x43, 0xb6, 0x5f, 0x8a, 0x2d];
const INITIALIZE_TRANCHE_TICKS_DISCRIMINATOR: [u8; 8] =
    [0x4b, 0x77, 0x1a, 0x93, 0x2c, 0x10, 0xe5, 0x6f];
const UPDATE_FLOOR_DISCRIMINATOR: [u8; 8] = [0x7a, 0x22, 0x91, 0x56, 0x3c, 0x10, 0xaf, 0x44];
const UPDATE_DEX_TWAP_DISCRIMINATOR: [u8; 8] = [0x64, 0x1b, 0x3e, 0x8b, 0x9a, 0x52, 0x10, 0x77];
const UPDATE_NATIVE_RATE_DISCRIMINATOR: [u8; 8] = [0x3c, 0x7e, 0x92, 0x14, 0xde, 0x20, 0xa1, 0x5f];

// Instruction data types
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EnterFeelssolInstructionData {
    pub amount: u64,
}

impl EnterFeelssolInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = ENTER_FEELSSOL_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ExitFeelssolInstructionData {
    pub amount: u64,
}

impl ExitFeelssolInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = EXIT_FEELSSOL_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeHubInstructionData {}

impl InitializeHubInstructionData {
    fn data(&self) -> Vec<u8> {
        INITIALIZE_HUB_DISCRIMINATOR.to_vec()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapParams {
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub max_ticks_crossed: u8,
    pub max_total_fee_bps: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapInstructionData {
    pub params: SwapParams,
}

impl SwapInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = SWAP_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeMarketParams {
    pub base_fee_bps: u16,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
    pub initial_buy_feelssol_amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeMarketInstructionData {
    pub params: InitializeMarketParams,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateDexTwapParams {
    pub dex_twap_rate_q64: u128,
    pub window_secs: u32,
    pub obs: u16,
    pub venue_id: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateDexTwapInstructionData { pub params: UpdateDexTwapParams }
impl UpdateDexTwapInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut v = UPDATE_DEX_TWAP_DISCRIMINATOR.to_vec();
        v.extend_from_slice(&self.try_to_vec().unwrap());
        v
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateNativeRateParams { pub native_rate_q64: u128 }
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateNativeRateInstructionData { pub params: UpdateNativeRateParams }
impl UpdateNativeRateInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut v = UPDATE_NATIVE_RATE_DISCRIMINATOR.to_vec();
        v.extend_from_slice(&self.try_to_vec().unwrap());
        v
    }
}

impl InitializeMarketInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = INITIALIZE_MARKET_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

/// Build enter FeelsSOL instruction
pub fn enter_feelssol(
    user: Pubkey,
    user_jitosol: Pubkey,
    user_feelssol: Pubkey,
    feelssol_mint: Pubkey,
    jitosol_mint: Pubkey,
    amount: u64,
) -> Instruction {
    let (hub, _) = Pubkey::find_program_address(
        &[b"feels_hub", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    let (jitosol_vault, _) = Pubkey::find_program_address(
        &[b"jitosol_vault", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    let (mint_authority, _) = Pubkey::find_program_address(
        &[b"mint_authority", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    Instruction {
        program_id: crate::program_id(),
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
        data: EnterFeelssolInstructionData { amount }.data(),
    }
}

/// Build initialize hub instruction
pub fn initialize_hub(payer: Pubkey, feelssol_mint: Pubkey, jitosol_mint: Pubkey) -> Instruction {
    let (hub, _) = Pubkey::find_program_address(
        &[b"feels_hub", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    let (jitosol_vault, _) = Pubkey::find_program_address(
        &[b"jitosol_vault", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(feelssol_mint, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(hub, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: InitializeHubInstructionData {}.data(),
    }
}

/// Build exit FeelsSOL instruction
pub fn exit_feelssol(
    user: Pubkey,
    user_feelssol: Pubkey,
    user_jitosol: Pubkey,
    feelssol_mint: Pubkey,
    jitosol_mint: Pubkey,
    amount: u64,
) -> Instruction {
    // Derive PDAs based on IDL account structure
    let (hub, _) = Pubkey::find_program_address(
        &[b"feels_hub", feelssol_mint.as_ref()],
        &crate::program_id(),
    );
    let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &crate::program_id());
    let (protocol_config, _) =
        Pubkey::find_program_address(&[b"protocol_config"], &crate::program_id());
    let (protocol_oracle, _) =
        Pubkey::find_program_address(&[b"protocol_oracle"], &crate::program_id());
    let (jitosol_vault, _) = Pubkey::find_program_address(
        &[b"jitosol_vault", feelssol_mint.as_ref()],
        &crate::program_id(),
    );
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault_authority", feelssol_mint.as_ref()],
        &crate::program_id(),
    );

    Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(user_jitosol, false),
            AccountMeta::new(user_feelssol, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(feelssol_mint, false),
            AccountMeta::new(hub, false),
            AccountMeta::new(safety, false),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new(protocol_oracle, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: ExitFeelssolInstructionData { amount }.data(),
    }
}

/// Build swap instruction (unified engine)
#[allow(clippy::too_many_arguments)]
pub fn swap(
    user: Pubkey,
    market: Pubkey,
    user_token_in: Pubkey,
    user_token_out: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
    tick_arrays: Vec<Pubkey>,
    amount_in: u64,
    minimum_amount_out: u64,
    max_ticks_crossed: u8,
) -> Result<Instruction> {
    // Validate token ordering
    if token_0_mint >= token_1_mint {
        return Err(SdkError::InvalidParameters(
            "Invalid token order: token_0 must be less than token_1 in market.".to_string(),
        ));
    }
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = crate::find_vault_0_address(&token_0_mint, &token_1_mint);
    let (vault_1, _) = crate::find_vault_1_address(&token_0_mint, &token_1_mint);
    let (vault_authority, _) = find_vault_authority_address(&market);
    let (oracle, _) =
        Pubkey::find_program_address(&[b"oracle", market.as_ref()], &crate::program_id());

    // Accounts must match on-chain order
    let mut accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(market, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new(oracle, false),
        AccountMeta::new(user_token_in, false),
        AccountMeta::new(user_token_out, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];

    // Add tick arrays as remaining accounts
    for tick_array in tick_arrays {
        accounts.push(AccountMeta::new(tick_array, false));
    }

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts,
        data: SwapInstructionData {
            params: SwapParams {
                amount_in,
                minimum_amount_out,
                max_ticks_crossed,
                // Default to 0 (no cap) unless a higher-level helper wraps this
                max_total_fee_bps: 0,
            },
        }
        .data(),
    })
}

/// Build swap instruction with strict fee cap enforcement on-chain
pub fn swap_with_fee_cap(
    user: Pubkey,
    market: Pubkey,
    user_token_in: Pubkey,
    user_token_out: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
    tick_arrays: Vec<Pubkey>,
    amount_in: u64,
    minimum_amount_out: u64,
    max_ticks_crossed: u8,
    max_total_fee_bps: u16,
) -> Result<Instruction> {
    // Validate token ordering
    if token_0_mint >= token_1_mint {
        return Err(SdkError::InvalidParameters(
            "Invalid token order: token_0 must be less than token_1 in market.".to_string(),
        ));
    }
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = crate::find_vault_0_address(&token_0_mint, &token_1_mint);
    let (vault_1, _) = crate::find_vault_1_address(&token_0_mint, &token_1_mint);
    let (vault_authority, _) = find_vault_authority_address(&market);
    let (oracle, _) =
        Pubkey::find_program_address(&[b"oracle", market.as_ref()], &crate::program_id());

    let mut accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(market, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new(oracle, false),
        AccountMeta::new(user_token_in, false),
        AccountMeta::new(user_token_out, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];
    for ta in tick_arrays {
        accounts.push(AccountMeta::new(ta, false));
    }

    let params = SwapInstructionData {
        params: SwapParams {
            amount_in,
            minimum_amount_out,
            max_ticks_crossed,
            max_total_fee_bps,
        },
    };

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts,
        data: params.data(),
    })
}

/// Build graduate_pool instruction
pub fn graduate_pool(authority: Pubkey, market: Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new(market, false),
    ];
    // Use a simple discriminator for graduate_pool
    let data = vec![0x8a, 0xfe, 0x97, 0xd3, 0x84, 0xcc, 0x40, 0x3f]; // graduate_pool discriminator
    Instruction {
        program_id: crate::program_id(),
        accounts,
        data,
    }
}

/// Build cleanup_bonding_curve instruction (closes tranche plan)
pub fn cleanup_bonding_curve(authority: Pubkey, market: Pubkey) -> Instruction {
    let (tranche_plan, _) =
        Pubkey::find_program_address(&[b"tranche_plan", market.as_ref()], &crate::program_id());
    Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(market, false),
            AccountMeta::new(tranche_plan, false), // closed to authority
        ],
        data: vec![0x31, 0x55, 0x26, 0x90, 0xaa, 0x7e, 0x01, 0x42], // cleanup_bonding_curve discriminator
    }
}

/// Build initialize market instruction (includes buffer initialization)
#[allow(clippy::too_many_arguments)]
pub fn initialize_market(
    creator: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    base_fee_bps: u16,
    tick_spacing: u16,
    initial_sqrt_price: u128,
    initial_buy_feelssol_amount: u64,
    creator_feelssol: Option<Pubkey>,
    creator_token_out: Option<Pubkey>,
) -> Result<Instruction> {
    // Validate parameters
    if base_fee_bps > 10000 {
        return Err(SdkError::InvalidParameters(
            "Invalid fee: base_fee_bps must be <= 10000 (100%)".to_string(),
        ));
    }

    // Validate FeelsSOL is one of the tokens
    if token_0 != feelssol_mint && token_1 != feelssol_mint {
        return Err(SdkError::InvalidParameters(
            "Invalid market: One token must be FeelsSOL. All markets require FeelsSOL as one of the tokens due to the hub-and-spoke architecture.".to_string()
        ));
    }

    // Validate FeelsSOL is token_0 (hub-and-spoke requirement)
    if token_0 != feelssol_mint {
        return Err(SdkError::InvalidParameters(
            "Invalid token order: FeelsSOL must be token_0 in the market pair. Please swap the token order.".to_string()
        ));
    }

    // Validate token ordering (token_0 < token_1)
    if token_0 >= token_1 {
        return Err(SdkError::InvalidParameters(
            "Invalid token order: token_0 must be less than token_1. Ensure tokens are sorted by their public key values.".to_string()
        ));
    }
    let (market, _) = find_market_address(&token_0, &token_1);
    let (buffer, _) = find_buffer_address(&market);
    let (oracle, _) =
        Pubkey::find_program_address(&[b"oracle", market.as_ref()], &crate::program_id());
    let (vault_0, _) = crate::find_vault_0_address(&token_0, &token_1);
    let (vault_1, _) = crate::find_vault_1_address(&token_0, &token_1);
    let (market_authority, _) =
        Pubkey::find_program_address(&[b"authority", market.as_ref()], &crate::program_id());

    // Derive the escrow PDA for the project token (non-FeelsSOL token)
    let project_token_mint = if token_0 != feelssol_mint {
        token_0
    } else {
        token_1
    };
    let (escrow, _) = Pubkey::find_program_address(
        &[b"escrow", project_token_mint.as_ref()],
        &crate::program_id(),
    );

    // Debug logging
    eprintln!("SDK initialize_market: token_0 = {}", token_0);
    eprintln!("SDK initialize_market: token_1 = {}", token_1);
    eprintln!("SDK initialize_market: feelssol_mint = {}", feelssol_mint);
    eprintln!(
        "SDK initialize_market: project_token_mint = {}",
        project_token_mint
    );
    eprintln!("SDK initialize_market: Escrow PDA: {}", escrow);

    // Protocol token accounts - these can be dummy accounts if token is FeelsSOL
    let protocol_token_0 = if token_0 == feelssol_mint {
        // Use a unique dummy PDA for FeelsSOL to avoid conflicts with system program
        let (dummy_protocol_0, _) =
            Pubkey::find_program_address(&[b"dummy_protocol_0"], &crate::program_id());
        dummy_protocol_0
    } else {
        let (protocol_token_0, _) = Pubkey::find_program_address(
            &[b"protocol_token", token_0.as_ref()],
            &crate::program_id(),
        );
        eprintln!("SDK: Calculated protocol_token_0 PDA: {}", protocol_token_0);
        protocol_token_0
    };

    let protocol_token_1 = if token_1 == feelssol_mint {
        // Use a unique dummy PDA for FeelsSOL to avoid conflicts with system program
        let (dummy_protocol_1, _) =
            Pubkey::find_program_address(&[b"dummy_protocol_1"], &crate::program_id());
        dummy_protocol_1
    } else {
        let (protocol_token_1, _) = Pubkey::find_program_address(
            &[b"protocol_token", token_1.as_ref()],
            &crate::program_id(),
        );
        eprintln!("SDK: Calculated protocol_token_1 PDA: {}", protocol_token_1);
        protocol_token_1
    };

    // Use dummy accounts if not doing initial buy
    // Create unique dummy accounts to avoid conflicts
    // These are just placeholder accounts that won't be used
    let dummy_feelssol = Pubkey::find_program_address(&[b"dummy_feelssol"], &crate::program_id()).0;
    let dummy_token_out =
        Pubkey::find_program_address(&[b"dummy_token_out"], &crate::program_id()).0;

    let creator_feelssol_account = creator_feelssol.unwrap_or(dummy_feelssol);
    let creator_token_out_account = creator_token_out.unwrap_or(dummy_token_out);

    let mut accounts = vec![
        AccountMeta::new(creator, true),
        AccountMeta::new(token_0, false),
        AccountMeta::new(token_1, false),
        AccountMeta::new(market, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new(oracle, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(market_authority, false),
        AccountMeta::new_readonly(feelssol_mint, false),
    ];

    // Add protocol token accounts
    // If it's a dummy account (for FeelsSOL), make it readonly
    if token_0 == feelssol_mint {
        accounts.push(AccountMeta::new_readonly(protocol_token_0, false));
    } else {
        accounts.push(AccountMeta::new(protocol_token_0, false));
    }

    if token_1 == feelssol_mint {
        accounts.push(AccountMeta::new_readonly(protocol_token_1, false));
    } else {
        accounts.push(AccountMeta::new(protocol_token_1, false));
    }

    // Add remaining accounts in exact order as defined in the instruction
    accounts.push(AccountMeta::new(escrow, false)); // escrow
    accounts.push(AccountMeta::new_readonly(creator_feelssol_account, false)); // creator_feelssol (dummy, so readonly)
    accounts.push(AccountMeta::new_readonly(creator_token_out_account, false)); // creator_token_out (dummy, so readonly)
    accounts.push(AccountMeta::new_readonly(
        solana_sdk::system_program::id(),
        false,
    )); // system_program
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false)); // token_program
    accounts.push(AccountMeta::new_readonly(
        solana_sdk::sysvar::rent::id(),
        false,
    )); // rent

    // Debug: Print all accounts in order
    eprintln!("SDK initialize_market accounts in order:");
    for (i, account) in accounts.iter().enumerate() {
        eprintln!(
            "  {}: {} (writable: {})",
            i, account.pubkey, account.is_writable
        );
    }

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts,
        data: InitializeMarketInstructionData {
            params: InitializeMarketParams {
                base_fee_bps,
                tick_spacing,
                initial_sqrt_price,
                initial_buy_feelssol_amount,
            },
        }
        .data(),
    })
}

/// Build update_dex_twap instruction
pub fn update_dex_twap(
    updater: Pubkey,
    dex_twap_rate_q64: u128,
    window_secs: u32,
    obs: u16,
    venue_id: Pubkey,
) -> Instruction {
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol_config"], &crate::program_id());
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &crate::program_id());
    let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &crate::program_id());
    let metas = vec![
        AccountMeta::new(updater, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(protocol_oracle, false),
        AccountMeta::new(safety, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];
    Instruction { program_id: crate::program_id(), accounts: metas, data: UpdateDexTwapInstructionData { params: UpdateDexTwapParams { dex_twap_rate_q64, window_secs, obs, venue_id } }.data() }
}

/// Build update_native_rate instruction
pub fn update_native_rate(
    authority: Pubkey,
    native_rate_q64: u128,
) -> Instruction {
    let (protocol_config, _) = Pubkey::find_program_address(&[b"protocol_config"], &crate::program_id());
    let (protocol_oracle, _) = Pubkey::find_program_address(&[b"protocol_oracle"], &crate::program_id());
    let (safety, _) = Pubkey::find_program_address(&[b"safety_controller"], &crate::program_id());
    let metas = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(protocol_oracle, false),
        AccountMeta::new(safety, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];
    Instruction { program_id: crate::program_id(), accounts: metas, data: UpdateNativeRateInstructionData { params: UpdateNativeRateParams { native_rate_q64 } }.data() }
}

/// Build update_floor crank instruction
pub fn update_floor(
    market: Pubkey,
    buffer: Pubkey,
    vault_0: Pubkey,
    vault_1: Pubkey,
    project_mint: Pubkey,
) -> Instruction {
    let metas = vec![
        AccountMeta::new(market, false),
        AccountMeta::new_readonly(buffer, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(project_mint, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];
    Instruction { program_id: crate::program_id(), accounts: metas, data: UPDATE_FLOOR_DISCRIMINATOR.to_vec() }
}

/// Namespace for instruction discriminators (auto-generated by Anchor)
pub mod instruction {
    use super::*;

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct EnterFeelssol {
        pub amount: u64,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct ExitFeelssol {
        pub amount: u64,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct SwapParams {
        pub amount_in: u64,
        pub minimum_amount_out: u64,
        pub max_ticks_crossed: u8,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct Swap {
        pub params: SwapParams,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct InitializeMarket {
        pub params: super::InitializeMarketParams,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct PlaceFloorLiquidity {}

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct MintToken {
        pub params: MintTokenParams,
    }
}

// Re-export types from the program
pub use feels::instructions::MintTokenParams;

// Instruction data structure for mint_token
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MintTokenInstructionData {
    pub params: MintTokenParams,
}

impl MintTokenInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = MINT_TOKEN_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

// Deploy initial liquidity types
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DeployInitialLiquidityParams {
    /// Number of ticks between each stair step
    pub tick_step_size: i32,
    /// Optional initial buy amount in FeelsSOL (0 = no initial buy)
    pub initial_buy_feelssol_amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DeployInitialLiquidityInstructionData {
    pub params: DeployInitialLiquidityParams,
}

impl DeployInitialLiquidityInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = DEPLOY_INITIAL_LIQUIDITY_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

/// Build deploy_initial_liquidity instruction
#[allow(clippy::too_many_arguments)]
pub fn deploy_initial_liquidity(
    deployer: Pubkey,
    market: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    tick_step_size: i32,
    initial_buy_feelssol_amount: u64,
    deployer_feelssol: Option<Pubkey>,
    deployer_token_out: Option<Pubkey>,
) -> Result<Instruction> {
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = crate::find_vault_0_address(&token_0, &token_1);
    let (vault_1, _) = crate::find_vault_1_address(&token_0, &token_1);
    let (market_authority, _) = find_vault_authority_address(&market);

    // Derive buffer authority and vaults
    let (buffer_authority, _) = Pubkey::find_program_address(
        &[b"buffer_authority", buffer.as_ref()],
        &crate::program_id(),
    );

    let buffer_token_vault = if token_0 != feelssol_mint {
        spl_associated_token_account::get_associated_token_address(&buffer_authority, &token_0)
    } else {
        spl_associated_token_account::get_associated_token_address(&buffer_authority, &token_1)
    };

    let buffer_feelssol_vault = spl_associated_token_account::get_associated_token_address(
        &buffer_authority,
        &feelssol_mint,
    );

    // Use provided accounts or dummy accounts for initial buy
    let deployer_feelssol_account = deployer_feelssol.unwrap_or(solana_sdk::system_program::id());
    let deployer_token_out_account = deployer_token_out.unwrap_or(solana_sdk::system_program::id());

    let accounts = vec![
        AccountMeta::new(deployer, true),
        AccountMeta::new(market, false),
        AccountMeta::new(deployer_feelssol_account, false),
        AccountMeta::new(deployer_token_out_account, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(market_authority, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new(buffer_token_vault, false),
        AccountMeta::new(buffer_feelssol_vault, false),
        AccountMeta::new_readonly(buffer_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts,
        data: DeployInitialLiquidityInstructionData {
            params: DeployInitialLiquidityParams {
                tick_step_size,
                initial_buy_feelssol_amount,
            },
        }
        .data(),
    })
}

/// Build mint_token instruction
pub fn mint_token(
    creator: Pubkey,
    creator_feelssol: Pubkey,
    token_mint: Pubkey,
    feelssol_mint: Pubkey,
    params: MintTokenParams,
) -> Result<Instruction> {
    // Derive PDAs
    let (escrow, _) =
        Pubkey::find_program_address(&[b"escrow", token_mint.as_ref()], &crate::program_id());

    let (escrow_authority, _) = Pubkey::find_program_address(
        &[b"escrow_authority", escrow.as_ref()],
        &crate::program_id(),
    );

    let (protocol_token, _) = Pubkey::find_program_address(
        &[b"protocol_token", token_mint.as_ref()],
        &crate::program_id(),
    );

    let (protocol_config, _) =
        Pubkey::find_program_address(&[b"protocol_config"], &crate::program_id());

    // Get escrow token vault
    let escrow_token_vault =
        spl_associated_token_account::get_associated_token_address(&escrow_authority, &token_mint);

    // Get escrow feelssol vault
    let escrow_feelssol_vault = spl_associated_token_account::get_associated_token_address(
        &escrow_authority,
        &feelssol_mint,
    );

    // Get metadata account
    let metadata = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            token_mint.as_ref(),
        ],
        &mpl_token_metadata::ID,
    )
    .0;

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(creator, true),
            AccountMeta::new(token_mint, true),
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_token_vault, false),
            AccountMeta::new(escrow_feelssol_vault, false),
            AccountMeta::new_readonly(escrow_authority, false),
            AccountMeta::new(metadata, false),
            AccountMeta::new_readonly(feelssol_mint, false),
            AccountMeta::new(creator_feelssol, false),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new_readonly(mpl_token_metadata::ID, false),
            AccountMeta::new(protocol_token, false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: MintTokenInstructionData { params }.data(),
    })
}

// Add InitializeProtocolParams re-export
pub use feels::instructions::InitializeProtocolParams;

// Position instruction re-exports
pub use feels::instructions::ClosePositionParams;

// Position instruction discriminators
const OPEN_POSITION_DISCRIMINATOR: [u8; 8] = [135, 128, 47, 77, 15, 152, 240, 49];
const CLOSE_POSITION_DISCRIMINATOR: [u8; 8] = [123, 134, 65, 47, 68, 3, 183, 1];
const COLLECT_FEES_DISCRIMINATOR: [u8; 8] = [164, 152, 207, 99, 99, 103, 65, 10];

// Instruction discriminator for initialize_protocol
const INITIALIZE_PROTOCOL_DISCRIMINATOR: [u8; 8] = [0xbc, 0xe9, 0xfc, 0x6a, 0x86, 0x92, 0xca, 0x5b];

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeProtocolInstructionData {
    pub params: InitializeProtocolParams,
}

impl InitializeProtocolInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = INITIALIZE_PROTOCOL_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

/// Build initialize_protocol instruction
pub fn initialize_protocol(
    authority: Pubkey,
    params: InitializeProtocolParams,
) -> Result<Instruction> {
    let (protocol_config, _) =
        Pubkey::find_program_address(&[b"protocol_config"], &crate::program_id());

    let (protocol_oracle, _) =
        Pubkey::find_program_address(&[b"protocol_oracle"], &crate::program_id());

    let (safety_controller, _) =
        Pubkey::find_program_address(&[b"safety_controller"], &crate::program_id());

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(protocol_config, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new(protocol_oracle, false),
            AccountMeta::new(safety_controller, false),
        ],
        data: InitializeProtocolInstructionData { params }.data(),
    })
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeTrancheTicksParams {
    pub tick_step_size: i32,
    pub num_steps: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeTrancheTicksInstructionData {
    pub params: InitializeTrancheTicksParams,
}

impl InitializeTrancheTicksInstructionData {
    fn data(&self) -> Vec<u8> {
        let mut data = INITIALIZE_TRANCHE_TICKS_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&self.try_to_vec().unwrap());
        data
    }
}

/// Build initialize_tranche_ticks crank instruction
pub fn initialize_tranche_ticks(
    crank: Pubkey,
    market: Pubkey,
    tick_step_size: i32,
    num_steps: u8,
    tick_arrays: Vec<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(crank, true),
        AccountMeta::new(market, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];
    for ta in tick_arrays {
        accounts.push(AccountMeta::new(ta, false));
    }
    Instruction {
        program_id: crate::program_id(),
        accounts,
        data: InitializeTrancheTicksInstructionData {
            params: InitializeTrancheTicksParams {
                tick_step_size,
                num_steps,
            },
        }
        .data(),
    }
}

/// Build open_position instruction  
#[allow(clippy::too_many_arguments)]
pub fn open_position(
    provider: Pubkey,
    market: Pubkey,
    position_mint: Pubkey,
    position_token_account: Pubkey,
    provider_token_0: Pubkey,
    provider_token_1: Pubkey,
    vault_0: Pubkey,
    vault_1: Pubkey,
    lower_tick_array: Pubkey,
    upper_tick_array: Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
) -> Result<Instruction> {
    // Derive position PDA
    let (position, _) = Pubkey::find_program_address(
        &[b"position", position_mint.as_ref()],
        &crate::program_id(),
    );

    // Serialize instruction data
    let mut data = OPEN_POSITION_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&tick_lower.to_le_bytes());
    data.extend_from_slice(&tick_upper.to_le_bytes());
    data.extend_from_slice(&liquidity_amount.to_le_bytes());

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(provider, true),
            AccountMeta::new(market, false),
            AccountMeta::new(position_mint, true),
            AccountMeta::new(position_token_account, false),
            AccountMeta::new(position, false),
            AccountMeta::new(provider_token_0, false),
            AccountMeta::new(provider_token_1, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new(lower_tick_array, false),
            AccountMeta::new(upper_tick_array, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    })
}

/// Build close_position instruction
#[allow(clippy::too_many_arguments)]
pub fn close_position(
    owner: Pubkey,
    market: Pubkey,
    position_mint: Pubkey,
    position_token_account: Pubkey,
    owner_token_0: Pubkey,
    owner_token_1: Pubkey,
    vault_0: Pubkey,
    vault_1: Pubkey,
    lower_tick_array: Pubkey,
    upper_tick_array: Pubkey,
    params: ClosePositionParams,
) -> Result<Instruction> {
    // Derive PDAs
    let (position, _) = Pubkey::find_program_address(
        &[b"position", position_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"authority", market.as_ref()],
        &crate::program_id(),
    );

    // Serialize instruction data
    let mut data = CLOSE_POSITION_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&params.try_to_vec().unwrap());

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(owner, true),
            AccountMeta::new(market, false),
            AccountMeta::new(position_mint, false),
            AccountMeta::new(position_token_account, false),
            AccountMeta::new(position, false),
            AccountMeta::new(owner_token_0, false),
            AccountMeta::new(owner_token_1, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new_readonly(market_authority, false),
            AccountMeta::new(lower_tick_array, false),
            AccountMeta::new(upper_tick_array, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    })
}

/// Build collect_fees instruction
pub fn collect_fees(
    owner: Pubkey,
    market: Pubkey,
    position_mint: Pubkey,
    position_token_account: Pubkey,
    owner_token_0: Pubkey,
    owner_token_1: Pubkey,
    vault_0: Pubkey,
    vault_1: Pubkey,
    tick_arrays: Option<(Pubkey, Pubkey)>, // Optional tick arrays for fee calculation
) -> Result<Instruction> {
    // Derive PDAs
    let (position, _) = Pubkey::find_program_address(
        &[b"position", position_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"authority", market.as_ref()],
        &crate::program_id(),
    );

    let mut accounts = vec![
        AccountMeta::new(owner, true),
        AccountMeta::new(market, false),
        AccountMeta::new_readonly(position_mint, false),
        AccountMeta::new_readonly(position_token_account, false),
        AccountMeta::new(position, false),
        AccountMeta::new(owner_token_0, false),
        AccountMeta::new(owner_token_1, false),
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(market_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    // Add tick arrays as remaining accounts if provided
    if let Some((lower_array, upper_array)) = tick_arrays {
        accounts.push(AccountMeta::new_readonly(lower_array, false));
        accounts.push(AccountMeta::new_readonly(upper_array, false));
    }

    Ok(Instruction {
        program_id: crate::program_id(),
        accounts,
        data: COLLECT_FEES_DISCRIMINATOR.to_vec(),
    })
}
