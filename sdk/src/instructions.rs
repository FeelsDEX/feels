//! Instruction builders for MVP

use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::{find_market_address, find_buffer_address, find_vault_address, find_vault_authority_address};

// Instruction discriminators (8-byte sighashes)
const ENTER_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0xc7, 0xcd, 0x31, 0xad, 0x51, 0x32, 0xba, 0x7e]; 
const EXIT_FEELSSOL_DISCRIMINATOR: [u8; 8] = [0x69, 0x76, 0xa8, 0x94, 0x3d, 0x98, 0x03, 0xaf];
const SWAP_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];
const INITIALIZE_MARKET_DISCRIMINATOR: [u8; 8] = [0x23, 0x23, 0xbd, 0xc1, 0x9b, 0x30, 0xaa, 0xcb];

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

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeMarketInstructionData {
    pub base_fee_bps: u16,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
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
    let (vault_0, _) = find_vault_address(&market, &token_0_mint);
    let (vault_1, _) = find_vault_address(&market, &token_1_mint);
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
    authority: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    feelssol_mint: Pubkey,
    base_fee_bps: u16,
    tick_spacing: u16,
    initial_sqrt_price: u128,
) -> Result<Instruction> {
    let (market, _) = find_market_address(&token_0, &token_1);
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = find_vault_address(&market, &token_0);
    let (vault_1, _) = find_vault_address(&market, &token_1);
    let (buffer_vault_0, _) = Pubkey::find_program_address(
        &[b"buffer_vault", buffer.as_ref(), token_0.as_ref()],
        &crate::program_id(),
    );
    let (buffer_vault_1, _) = Pubkey::find_program_address(
        &[b"buffer_vault", buffer.as_ref(), token_1.as_ref()],
        &crate::program_id(),
    );
    let (vault_authority, _) = find_vault_authority_address(&market);
    let (buffer_authority, _) = Pubkey::find_program_address(
        &[b"buffer_authority", buffer.as_ref()],
        &crate::program_id(),
    );
    
    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(market, false),
            AccountMeta::new(buffer, false),
            AccountMeta::new_readonly(token_0, false),
            AccountMeta::new_readonly(token_1, false),
            AccountMeta::new_readonly(feelssol_mint, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new(buffer_vault_0, false),
            AccountMeta::new(buffer_vault_1, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(buffer_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ],
        data: InitializeMarketInstructionData {
            base_fee_bps,
            tick_spacing,
            initial_sqrt_price,
        }.data(),
    })
}

/// Build place floor liquidity instruction
pub fn place_floor_liquidity(
    caller: Pubkey,
    market: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
) -> Result<Instruction> {
    let (buffer, _) = find_buffer_address(&market);
    let (vault_0, _) = find_vault_address(&market, &token_0_mint);
    let (vault_1, _) = find_vault_address(&market, &token_1_mint);
    
    let (buffer_vault_0, _) = Pubkey::find_program_address(
        &[b"buffer_vault", buffer.as_ref(), token_0_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (buffer_vault_1, _) = Pubkey::find_program_address(
        &[b"buffer_vault", buffer.as_ref(), token_1_mint.as_ref()],
        &crate::program_id(),
    );
    
    let (buffer_authority, _) = Pubkey::find_program_address(
        &[b"buffer_authority", buffer.as_ref()],
        &crate::program_id(),
    );
    
    Ok(Instruction {
        program_id: crate::program_id(),
        accounts: vec![
            AccountMeta::new_readonly(caller, true),
            AccountMeta::new(market, false),
            AccountMeta::new(buffer, false),
            AccountMeta::new(vault_0, false),
            AccountMeta::new(vault_1, false),
            AccountMeta::new(buffer_vault_0, false),
            AccountMeta::new(buffer_vault_1, false),
            AccountMeta::new_readonly(buffer_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ],
        data: vec![0; 8], // Placeholder discriminator for place_floor_liquidity
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
        pub params: crate::types::InitializeMarketParams,
    }
    
    
    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct PlaceFloorLiquidity {}
}