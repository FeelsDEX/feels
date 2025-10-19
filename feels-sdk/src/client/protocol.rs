use std::sync::Arc;

use crate::prelude::*;
use solana_sdk::instruction::Instruction;

use crate::{
    core::SdkResult,
    instructions::{
        InitializeHubParams, InitializeProtocolParams, ProtocolInstructionBuilder,
        UpdateProtocolParams,
    },
    protocol::PdaBuilder,
};

use super::BaseClient;

/// Protocol management service
#[allow(dead_code)]
pub struct ProtocolService {
    _base: Arc<BaseClient>,
    _pda: Arc<PdaBuilder>,
    builder: ProtocolInstructionBuilder,
}

impl ProtocolService {
    pub fn new(base: Arc<BaseClient>, pda: Arc<PdaBuilder>, program_id: Pubkey) -> Self {
        Self {
            _base: base,
            _pda: pda,
            builder: ProtocolInstructionBuilder::new(program_id),
        }
    }

    /// Initialize the protocol (one-time setup)
    pub fn initialize_protocol_ix(
        &self,
        authority: Pubkey,
        params: InitializeProtocolParams,
    ) -> SdkResult<Instruction> {
        self.builder.initialize_protocol(authority, params)
    }

    /// Update protocol configuration
    pub fn update_protocol_ix(
        &self,
        authority: Pubkey,
        params: UpdateProtocolParams,
    ) -> SdkResult<Instruction> {
        self.builder.update_protocol(authority, params)
    }

    /// Initialize the FeelsSOL hub
    pub fn initialize_hub_ix(
        &self,
        authority: Pubkey,
        jitosol_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder
            .initialize_hub(authority, InitializeHubParams { jitosol_mint })
    }

    /// Update floor price for a market
    pub fn update_floor_ix(
        &self,
        market: Pubkey,
        vault_0: Pubkey,
        vault_1: Pubkey,
        project_mint: Pubkey,
        escrow_token_account: Option<Pubkey>,
    ) -> SdkResult<Instruction> {
        self.builder
            .update_floor(market, vault_0, vault_1, project_mint, escrow_token_account)
    }

    /// Set protocol owned override for floor calculation
    pub fn set_protocol_owned_override_ix(
        &self,
        authority: Pubkey,
        buffer: Pubkey,
        override_amount: u64,
    ) -> SdkResult<Instruction> {
        self.builder
            .set_protocol_owned_override(authority, buffer, override_amount)
    }

    /// Update native rate for a market
    pub fn update_native_rate_ix(
        &self,
        market: Pubkey,
        feelssol_mint: Pubkey,
    ) -> SdkResult<Instruction> {
        self.builder.update_native_rate(market, feelssol_mint)
    }
}
