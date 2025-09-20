use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::{SdkResult, SwapDirection},
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminator
const SWAP_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// Parameters for swap
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapParams {
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub max_ticks_crossed: u8,
    pub max_total_fee_bps: u16,
}

impl_instruction!(SwapParams, SWAP_DISCRIMINATOR);

/// Common swap accounts
pub struct SwapAccounts {
    pub user: Pubkey,
    pub market: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub user_token_in: Pubkey,
    pub user_token_out: Pubkey,
    pub tick_arrays: Vec<Pubkey>,
}

/// Swap instruction builder
pub struct SwapInstructionBuilder {
    pda: PdaBuilder,
}

impl SwapInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build exact input swap instruction
    pub fn swap(
        &self,
        accounts: SwapAccounts,
        params: SwapParams,
    ) -> SdkResult<Instruction> {
        self.build_swap_instruction(accounts, params.build_data()?)
    }


    /// Common swap instruction building logic
    fn build_swap_instruction(
        &self,
        accounts: SwapAccounts,
        data: Vec<u8>,
    ) -> SdkResult<Instruction> {
        let (buffer, _) = self.pda.buffer(&accounts.market);
        let (vault_authority, _) = self.pda.vault_authority(&accounts.market);
        let (oracle, _) = self.pda.oracle(&accounts.market);
        let (protocol_config, _) = self.pda.protocol_config();

        // Derive vault addresses using token mints
        let (vault_0, _) = Pubkey::find_program_address(
            &[b"vault", accounts.token_0.as_ref(), accounts.token_1.as_ref(), b"0"],
            &self.pda.program_id,
        );
        let (vault_1, _) = Pubkey::find_program_address(
            &[b"vault", accounts.token_0.as_ref(), accounts.token_1.as_ref(), b"1"],
            &self.pda.program_id,
        );

        let mut builder = FeelsInstructionBuilder::new()
            .add_signer(accounts.user)
            .add_writable(accounts.market)
            .add_writable(vault_0)
            .add_writable(vault_1)
            .add_readonly(vault_authority)
            .add_writable(buffer)
            .add_writable(oracle)
            .add_writable(accounts.user_token_in)
            .add_writable(accounts.user_token_out)
            .add_readonly(spl_token::id())
            .add_readonly(solana_sdk::sysvar::clock::id())
            .add_readonly(protocol_config)
            // Protocol treasury (required, placeholder for now)
            .add_writable(Pubkey::default())
            // Protocol token (optional)
            .add_optional(None)
            // Creator token account (optional)
            .add_optional(None);

        // Add tick arrays
        for tick_array in accounts.tick_arrays {
            builder = builder.add_writable(tick_array);
        }

        Ok(builder.with_data(data).build())
    }

    /// Derive tick arrays needed for a swap
    pub fn derive_tick_arrays(
        &self,
        market: &Pubkey,
        current_tick: i32,
        tick_spacing: u16,
        direction: SwapDirection,
        max_arrays: usize,
    ) -> Vec<Pubkey> {
        let tick_array_size = crate::core::TICK_ARRAY_SIZE;
        let tick_array_spacing = (tick_spacing as i32) * tick_array_size;

        let mut arrays = Vec::with_capacity(max_arrays);
        let mut current_start = self.get_start_tick_index(current_tick, tick_spacing);

        for _ in 0..max_arrays {
            let (tick_array, _) = self.pda.tick_array(market, current_start);
            arrays.push(tick_array);

            current_start = match direction {
                SwapDirection::ZeroForOne => current_start - tick_array_spacing,
                SwapDirection::OneForZero => current_start + tick_array_spacing,
            };
        }

        arrays
    }

    fn get_start_tick_index(&self, tick: i32, tick_spacing: u16) -> i32 {
        let tick_array_size = crate::core::TICK_ARRAY_SIZE;
        let tick_array_spacing = (tick_spacing as i32) * tick_array_size;

        if tick >= 0 {
            (tick / tick_array_spacing) * tick_array_spacing
        } else {
            ((tick - tick_array_spacing + 1) / tick_array_spacing) * tick_array_spacing
        }
    }
}