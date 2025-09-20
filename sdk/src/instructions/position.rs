use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const OPEN_POSITION_WITH_METADATA_DISCRIMINATOR: [u8; 8] = [242, 29, 134, 48, 58, 110, 14, 60];
const CLOSE_POSITION_WITH_METADATA_DISCRIMINATOR: [u8; 8] = [17, 174, 244, 40, 141, 4, 42, 125];
const UPDATE_POSITION_FEE_LOWER_DISCRIMINATOR: [u8; 8] = [58, 181, 152, 160, 205, 130, 59, 20];
const UPDATE_POSITION_FEE_UPPER_DISCRIMINATOR: [u8; 8] = [162, 48, 161, 22, 95, 7, 191, 252];

/// Parameters for opening position with metadata
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OpenPositionWithMetadataParams {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

impl_instruction!(OpenPositionWithMetadataParams, OPEN_POSITION_WITH_METADATA_DISCRIMINATOR);

/// Parameters for closing position with metadata
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ClosePositionWithMetadataParams {
    pub liquidity: u128,
}

impl_instruction!(ClosePositionWithMetadataParams, CLOSE_POSITION_WITH_METADATA_DISCRIMINATOR);

/// Parameters for updating position fee lower
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdatePositionFeeLowerParams {
    pub fee_growth_inside_0: u128,
    pub fee_growth_inside_1: u128,
}

impl_instruction!(UpdatePositionFeeLowerParams, UPDATE_POSITION_FEE_LOWER_DISCRIMINATOR);

/// Parameters for updating position fee upper
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdatePositionFeeUpperParams {
    pub fee_growth_inside_0: u128,
    pub fee_growth_inside_1: u128,
}

impl_instruction!(UpdatePositionFeeUpperParams, UPDATE_POSITION_FEE_UPPER_DISCRIMINATOR);

/// Position instruction builder
pub struct PositionInstructionBuilder {
    pda: PdaBuilder,
}

impl PositionInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build open position with metadata instruction
    pub fn open_position_with_metadata(
        &self,
        owner: Pubkey,
        market: Pubkey,
        params: OpenPositionWithMetadataParams,
    ) -> SdkResult<Instruction> {
        let (position, _) = self.pda.position(&owner, params.tick_lower, params.tick_upper);
        let (position_metadata, _) = self.pda.position_metadata(&position);
        let (position_mint, _) = Pubkey::find_program_address(
            &[b"position_mint", position.as_ref()],
            &self.pda.program_id,
        );
        let position_token_account = spl_associated_token_account::get_associated_token_address(
            &owner,
            &position_mint,
        );

        // Derive tick arrays for the position range
        let lower_tick_array = self.get_tick_array_for_tick(&market, params.tick_lower);
        let upper_tick_array = self.get_tick_array_for_tick(&market, params.tick_upper);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(owner)
            .add_writable(market)
            .add_writable(position)
            .add_writable(position_metadata)
            .add_writable(position_mint)
            .add_writable(position_token_account)
            .add_writable(lower_tick_array)
            .add_writable(upper_tick_array)
            .add_readonly(solana_sdk::system_program::id())
            .add_readonly(spl_token::id())
            .add_readonly(spl_associated_token_account::id())
            .add_readonly(solana_sdk::sysvar::rent::id())
            .add_readonly(mpl_token_metadata::ID)
            .with_data(params.build_data()?)
            .build())
    }

    /// Build close position with metadata instruction
    pub fn close_position_with_metadata(
        &self,
        owner: Pubkey,
        market: Pubkey,
        position: Pubkey,
        position_mint: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> SdkResult<Instruction> {
        let params = ClosePositionWithMetadataParams { liquidity };
        let (position_metadata, _) = self.pda.position_metadata(&position);
        let position_token_account = spl_associated_token_account::get_associated_token_address(
            &owner,
            &position_mint,
        );

        // Derive tick arrays for the position range
        let lower_tick_array = self.get_tick_array_for_tick(&market, tick_lower);
        let upper_tick_array = self.get_tick_array_for_tick(&market, tick_upper);

        Ok(FeelsInstructionBuilder::new()
            .add_signer(owner)
            .add_writable(market)
            .add_writable(position)
            .add_writable(position_metadata)
            .add_writable(position_mint)
            .add_writable(position_token_account)
            .add_writable(lower_tick_array)
            .add_writable(upper_tick_array)
            .add_readonly(spl_token::id())
            .add_readonly(mpl_token_metadata::ID)
            .with_data(params.build_data()?)
            .build())
    }

    /// Build update position fee lower instruction
    pub fn update_position_fee_lower(
        &self,
        position: Pubkey,
        lower_tick_array: Pubkey,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> SdkResult<Instruction> {
        Ok(FeelsInstructionBuilder::new()
            .add_writable(position)
            .add_readonly(lower_tick_array)
            .with_data(
                UpdatePositionFeeLowerParams {
                    fee_growth_inside_0,
                    fee_growth_inside_1,
                }
                .build_data()?,
            )
            .build())
    }

    /// Build update position fee upper instruction
    pub fn update_position_fee_upper(
        &self,
        position: Pubkey,
        upper_tick_array: Pubkey,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> SdkResult<Instruction> {
        Ok(FeelsInstructionBuilder::new()
            .add_writable(position)
            .add_readonly(upper_tick_array)
            .with_data(
                UpdatePositionFeeUpperParams {
                    fee_growth_inside_0,
                    fee_growth_inside_1,
                }
                .build_data()?,
            )
            .build())
    }

    fn get_tick_array_for_tick(&self, market: &Pubkey, tick: i32) -> Pubkey {
        // Simplified - would need tick spacing to calculate properly
        let start_index = (tick / (crate::core::TICK_ARRAY_SIZE * 10)) * (crate::core::TICK_ARRAY_SIZE * 10);
        let (tick_array, _) = self.pda.tick_array(market, start_index);
        tick_array
    }
}