use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    instructions::{OpenPositionWithMetadataParams, PositionInstructionBuilder},
    protocol::PdaBuilder,
};

use super::BaseClient;

/// Position management service (with NFT support)
#[allow(dead_code)]
pub struct PositionService {
    _base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    builder: PositionInstructionBuilder,
}

impl PositionService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            _base: base,
            pda,
            builder: PositionInstructionBuilder::new(program_id),
        }
    }

    /// Open a position with NFT metadata
    pub fn open_position_with_metadata_ix(
        &self,
        owner: Pubkey,
        market: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
        name: String,
        symbol: String,
        uri: String,
    ) -> SdkResult<Instruction> {
        let params = OpenPositionWithMetadataParams {
            tick_lower,
            tick_upper,
            liquidity,
            name,
            symbol,
            uri,
        };
        self.builder.open_position_with_metadata(owner, market, params)
    }

    /// Close a position with NFT metadata
    pub fn close_position_with_metadata_ix(
        &self,
        owner: Pubkey,
        market: Pubkey,
        position: Pubkey,
        position_mint: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> SdkResult<Instruction> {
        self.builder.close_position_with_metadata(
            owner,
            market,
            position,
            position_mint,
            tick_lower,
            tick_upper,
            liquidity,
        )
    }

    /// Update position fee accounting (lower tick)
    pub fn update_position_fee_lower_ix(
        &self,
        position: Pubkey,
        lower_tick_array: Pubkey,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> SdkResult<Instruction> {
        self.builder.update_position_fee_lower(
            position,
            lower_tick_array,
            fee_growth_inside_0,
            fee_growth_inside_1,
        )
    }

    /// Update position fee accounting (upper tick)
    pub fn update_position_fee_upper_ix(
        &self,
        position: Pubkey,
        upper_tick_array: Pubkey,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> SdkResult<Instruction> {
        self.builder.update_position_fee_upper(
            position,
            upper_tick_array,
            fee_growth_inside_0,
            fee_growth_inside_1,
        )
    }

    /// Get position NFT mint address
    pub fn get_position_mint(&self, position: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"position_mint", position.as_ref()],
            &self.pda.program_id,
        )
    }

    /// Get position token account for owner
    pub fn get_position_token_account(&self, owner: &Pubkey, position_mint: &Pubkey) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(owner, position_mint)
    }
}