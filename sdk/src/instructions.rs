//! Instruction builders for MVP

use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::{find_market_address, find_buffer_address, find_vault_authority_address, error::SdkError};

type Result<T> = std::result::Result<T, SdkError>;

// Instruction discriminators (8-byte sighashes)
const ENTER_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0xc7, 0xcd, 0x31, 0xad, 0x51, 0x32, 0xba, 0x7e]; 
const EXIT_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0x69, 0x76, 0xa8, 0x94, 0x3d, 0x98, 0x03, 0xaf];
const SWAP_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];
const INITIALIZE_MARKET_DISCRIMINATOR: [u8; 8] = [0x23, 0x23, 0xbd, 0xc1, 0x9b, 0x30, 0xaa, 0xcb];
const MINT_TOKEN_DISCRIMINATOR: [u8; 8] = [0xac, 0x89, 0xb7, 0x0e, 0xcf, 0x6e, 0xea, 0x38];
const DEPLOY_INITIAL_LIQUIDITY_DISCRIMINATOR: [u8; 8] = [0x9f, 0xf7, 0xd1, 0x43, 0xb6, 0x5f, 0x8a, 0x2d];

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
pub struct SwapParams {
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub max_ticks_crossed: u8,
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
    jitosol_mint: Pubkey,
    feelssol_mint: Pubkey,
    amount: u64,
) -> Instruction {
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
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(mint_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: EnterFeelssolInstructionData { amount }.data(),
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
            AccountMeta::new(user_feelssol, false),
            AccountMeta::new(user_jitosol, false),
            AccountMeta::new(feelssol_mint, false),
            AccountMeta::new_readonly(jitosol_mint, false),
            AccountMeta::new(jitosol_vault, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: ExitFeelssolInstructionData { amount }.data(),
    }
}

/// Build swap instruction (unified engine)
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
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = crate::find_vault_0_address(&token_0_mint, &token_1_mint);
    let (vault_1, _) = crate::find_vault_1_address(&token_0_mint, &token_1_mint);
    let (vault_authority, _) = find_vault_authority_address(&market);
    
    let mut accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(market, false),
        AccountMeta::new_readonly(Pubkey::default(), false), // No oracle
        AccountMeta::new(vault_0, false),
        AccountMeta::new(vault_1, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new(buffer, false),
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
            }
        }.data(),
    })
}

/// Build initialize market instruction (includes buffer initialization)
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
    let (market, _) = find_market_address(&token_0, &token_1);
    let (buffer, _) = find_buffer_address(&market);
    let (oracle, _) = Pubkey::find_program_address(
        &[b"oracle", market.as_ref()],
        &crate::program_id(),
    );
    let (vault_0, _) = crate::find_vault_0_address(&token_0, &token_1);
    let (vault_1, _) = crate::find_vault_1_address(&token_0, &token_1);
    let (market_authority, _) = Pubkey::find_program_address(
        &[b"authority", market.as_ref()],
        &crate::program_id(),
    );

    // Derive the escrow PDA for the project token (non-FeelsSOL token)
    let project_token_mint = if token_0 != feelssol_mint { token_0 } else { token_1 };
    let (escrow, _) = Pubkey::find_program_address(
        &[b"escrow", project_token_mint.as_ref()],
        &crate::program_id(),
    );
    
    // Debug logging
    eprintln!("SDK initialize_market: token_0 = {}", token_0);
    eprintln!("SDK initialize_market: token_1 = {}", token_1);
    eprintln!("SDK initialize_market: feelssol_mint = {}", feelssol_mint);
    eprintln!("SDK initialize_market: project_token_mint = {}", project_token_mint);
    eprintln!("SDK initialize_market: Escrow PDA: {}", escrow);
    
    // Protocol token accounts - these can be dummy accounts if token is FeelsSOL
    let protocol_token_0 = if token_0 == feelssol_mint {
        // Use a unique dummy PDA for FeelsSOL to avoid conflicts with system program
        let (dummy_protocol_0, _) = Pubkey::find_program_address(
            &[b"dummy_protocol_0"],
            &crate::program_id(),
        );
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
        let (dummy_protocol_1, _) = Pubkey::find_program_address(
            &[b"dummy_protocol_1"],
            &crate::program_id(),
        );
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
    let dummy_feelssol = Pubkey::find_program_address(
        &[b"dummy_feelssol"],
        &crate::program_id(),
    ).0;
    let dummy_token_out = Pubkey::find_program_address(
        &[b"dummy_token_out"],
        &crate::program_id(),
    ).0;
    
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
    accounts.push(AccountMeta::new_readonly(solana_sdk::system_program::id(), false)); // system_program
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false)); // token_program
    accounts.push(AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false)); // rent
    
    // Debug: Print all accounts in order
    eprintln!("SDK initialize_market accounts in order:");
    for (i, account) in accounts.iter().enumerate() {
        eprintln!("  {}: {} (writable: {})", i, account.pubkey, account.is_writable);
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
            }
        }.data(),
    })
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
            }
        }.data(),
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
    let (escrow, _) = Pubkey::find_program_address(
        &[b"escrow", token_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (escrow_authority, _) = Pubkey::find_program_address(
        &[b"escrow_authority", escrow.as_ref()],
        &crate::program_id(),
    );
    
    let (protocol_token, _) = Pubkey::find_program_address(
        &[b"protocol_token", token_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (protocol_config, _) = Pubkey::find_program_address(
        &[b"protocol_config"],
        &crate::program_id(),
    );
    
    // Get escrow token vault
    let escrow_token_vault = spl_associated_token_account::get_associated_token_address(
        &escrow_authority,
        &token_mint,
    );
    
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
    ).0;
    
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
    let (protocol_config, _) = Pubkey::find_program_address(
        &[b"protocol_config"],
        &crate::program_id(),
    );
    
    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(protocol_config, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: InitializeProtocolInstructionData { params }.data(),
    })
}