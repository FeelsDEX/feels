use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const INITIALIZE_POMM_POSITION_DISCRIMINATOR: [u8; 8] = [188, 224, 119, 1, 109, 96, 244, 199];
const MANAGE_POMM_POSITION_DISCRIMINATOR: [u8; 8] = [173, 67, 116, 206, 107, 121, 81, 19];

/// Parameters for initializing POMM position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializePommPositionParams {
    pub tick_lower: i32,
    pub tick_upper: i32,
}

impl_instruction!(InitializePommPositionParams, INITIALIZE_POMM_POSITION_DISCRIMINATOR);

/// POMM action enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PommAction {
    AddLiquidity { amount: u128 },
    RemoveLiquidity { amount: u128 },
    CollectFees,
}

/// Parameters for managing POMM position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ManagePommPositionParams {
    pub action: PommAction,
}

impl_instruction!(ManagePommPositionParams, MANAGE_POMM_POSITION_DISCRIMINATOR);

/// POMM instruction builder
pub struct PommInstructionBuilder {
    pda: PdaBuilder,
}

impl PommInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build initialize POMM position instruction
    pub fn initialize_pomm_position(
        &self,
        authority: Pubkey,
        market: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();
        let (buffer, _) = self.pda.buffer(&market);
        let (pomm_position, _) = Pubkey::find_program_address(
            &[
                b"pomm_position",
                market.as_ref(),
                &tick_lower.to_le_bytes(),
                &tick_upper.to_le_bytes(),
            ],
            &self.pda.program_id,
        );

        // Derive tick arrays
        let lower_tick_array = self.get_tick_array_for_tick(&market, tick_lower);
        let upper_tick_array = self.get_tick_array_for_tick(&market, tick_upper);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(authority)
            .add_writable(market)
            .add_readonly(protocol_config)
            .add_writable(buffer)
            .add_writable(pomm_position)
            .add_writable(lower_tick_array)
            .add_writable(upper_tick_array)
            .add_readonly(solana_program::system_program::id())
            .with_data(
                InitializePommPositionParams {
                    tick_lower,
                    tick_upper,
                }
                .build_data()?,
            )
            .build())
    }

    /// Build manage POMM position instruction
    pub fn manage_pomm_position(
        &self,
        authority: Pubkey,
        market: Pubkey,
        pomm_position: Pubkey,
        action: PommAction,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();
        let (buffer, _) = self.pda.buffer(&market);
        let (vault_authority, _) = self.pda.vault_authority(&market);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(authority)
            .add_writable(market)
            .add_readonly(protocol_config)
            .add_writable(buffer)
            .add_writable(pomm_position)
            .add_readonly(vault_authority)
            .add_readonly(spl_token::id())
            .with_data(ManagePommPositionParams { action }.build_data()?)
            .build())
    }

    fn get_tick_array_for_tick(&self, market: &Pubkey, tick: i32) -> Pubkey {
        // Simplified - would need tick spacing to calculate properly
        let start_index = (tick / (crate::core::TICK_ARRAY_SIZE * 10)) * (crate::core::TICK_ARRAY_SIZE * 10);
        let (tick_array, _) = self.pda.tick_array(market, start_index);
        tick_array
    }
}