use std::sync::Arc;

use anchor_lang::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    instructions::{PoolPhase, RegistryInstructionBuilder},
    protocol::PdaBuilder,
};

use super::BaseClient;

/// Pool registry service
pub struct RegistryService {
    base: Arc<BaseClient>,
    pda: Arc<PdaBuilder>,
    builder: RegistryInstructionBuilder,
}

impl RegistryService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            base,
            pda,
            builder: RegistryInstructionBuilder::new(program_id),
        }
    }

    /// Initialize the pool registry
    pub fn initialize_pool_registry_ix(
        &self,
        authority: Pubkey,
        payer: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder.initialize_pool_registry(authority, payer)
    }

    /// Register a new pool in the registry
    pub fn register_pool_ix(
        &self,
        creator: Pubkey,
        payer: Pubkey,
        market: Pubkey,
        project_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder
            .register_pool(creator, payer, market, project_mint)
    }

    /// Update pool phase in the registry
    pub fn update_pool_phase_ix(
        &self,
        authority: Pubkey,
        market: Pubkey,
        new_phase: PoolPhase,
    ) -> SdkResult<Instruction> {
        self.builder.update_pool_phase(authority, market, new_phase)
    }

    /// Get pool registry address
    pub fn get_pool_registry_address(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"pool_registry"], &self.pda.program_id)
    }
}