use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    instructions::{PommAction, PommInstructionBuilder},
    protocol::PdaBuilder,
};

use super::BaseClient;

/// Protocol-Owned Market Making (POMM) service
#[allow(dead_code)]
pub struct PommService {
    _base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    builder: PommInstructionBuilder,
}

impl PommService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            _base: base,
            pda,
            builder: PommInstructionBuilder::new(program_id),
        }
    }

    /// Initialize a POMM position
    pub fn initialize_pomm_position_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
    ) -> SdkResult<Instruction> {
        self.builder
            .initialize_pomm_position(authority, market, tick_lower, tick_upper)
    }

    /// Add liquidity to a POMM position
    pub fn add_liquidity_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        pomm_position: Pubkey,
        amount: u128,
    ) -> SdkResult<Instruction> {
        self.builder.manage_pomm_position(
            authority,
            market,
            pomm_position,
            PommAction::AddLiquidity { amount },
        )
    }

    /// Remove liquidity from a POMM position
    pub fn remove_liquidity_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        pomm_position: Pubkey,
        amount: u128,
    ) -> SdkResult<Instruction> {
        self.builder.manage_pomm_position(
            authority,
            market,
            pomm_position,
            PommAction::RemoveLiquidity { amount },
        )
    }

    /// Collect fees from a POMM position
    pub fn collect_fees_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        pomm_position: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder.manage_pomm_position(
            authority,
            market,
            pomm_position,
            PommAction::CollectFees,
        )
    }

    /// Get POMM position address
    pub fn get_pomm_position_address(
        &self,
        market: &Pubkey,
        tick_lower: i32,
        tick_upper: i32,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"pomm_position",
                market.as_ref(),
                &tick_lower.to_le_bytes(),
                &tick_upper.to_le_bytes(),
            ],
            &self.pda.program_id,
        )
    }
}