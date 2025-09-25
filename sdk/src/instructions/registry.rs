use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    impl_instruction,
    instructions::{FeelsInstructionBuilder, InstructionBuilder},
    protocol::PdaBuilder,
};

// Instruction discriminators
const INITIALIZE_POOL_REGISTRY_DISCRIMINATOR: [u8; 8] = [109, 119, 17, 241, 165, 19, 176, 175];
const REGISTER_POOL_DISCRIMINATOR: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const UPDATE_POOL_PHASE_DISCRIMINATOR: [u8; 8] = [67, 208, 79, 72, 239, 112, 73, 232];

/// Parameters for initializing pool registry (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializePoolRegistryParams {}

impl_instruction!(InitializePoolRegistryParams, INITIALIZE_POOL_REGISTRY_DISCRIMINATOR);

/// Parameters for registering pool (no params)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RegisterPoolParams {}

impl_instruction!(RegisterPoolParams, REGISTER_POOL_DISCRIMINATOR);

/// Pool phase enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum PoolPhase {
    PreLaunch,
    Live,
    PostGraduation,
    Expired,
}

/// Parameters for updating pool phase
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdatePoolPhaseParams {
    pub new_phase: PoolPhase,
}

impl_instruction!(UpdatePoolPhaseParams, UPDATE_POOL_PHASE_DISCRIMINATOR);

/// Registry instruction builder
pub struct RegistryInstructionBuilder {
    pda: PdaBuilder,
}

impl RegistryInstructionBuilder {
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            pda: PdaBuilder::new(program_id),
        }
    }

    /// Build initialize pool registry instruction
    pub fn initialize_pool_registry(
        &self,
        authority: Pubkey,
        payer: Pubkey,
    ) -> SdkResult<Instruction> {
        let (protocol_config, _) = self.pda.protocol_config();
        let (pool_registry, _) = Pubkey::find_program_address(
            &[b"pool_registry"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_readonly(protocol_config)
            .add_writable(pool_registry)
            .add_signer(authority)
            .add_signer(payer)
            .add_readonly(solana_program::system_program::id())
            .with_data(InitializePoolRegistryParams {}.build_data()?)
            .build())
    }

    /// Build register pool instruction
    pub fn register_pool(
        &self,
        creator: Pubkey,
        payer: Pubkey,
        market: Pubkey,
        project_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        let (pool_registry, _) = Pubkey::find_program_address(
            &[b"pool_registry"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_writable(pool_registry)
            .add_readonly(market)
            .add_readonly(project_mint)
            .add_signer(creator)
            .add_signer(payer)
            .add_readonly(solana_program::system_program::id())
            .add_readonly(solana_program::sysvar::clock::id())
            .with_data(RegisterPoolParams {}.build_data()?)
            .build())
    }

    /// Build update pool phase instruction
    pub fn update_pool_phase(
        &self,
        authority: Pubkey,
        market: Pubkey,
        new_phase: PoolPhase,
    ) -> SdkResult<Instruction> {
        let (pool_registry, _) = Pubkey::find_program_address(
            &[b"pool_registry"],
            &self.pda.program_id,
        );

        Ok(FeelsInstructionBuilder::new()
            .add_writable(pool_registry)
            .add_readonly(market)
            .add_signer(authority)
            .add_readonly(solana_program::sysvar::clock::id())
            .with_data(UpdatePoolPhaseParams { new_phase }.build_data()?)
            .build())
    }
}